//! End-to-end integration tests for the `project-canon review` verb — run the built binary
//! against fixture repos and assert the **advisory** exit-code contract, the `--json` schema, and
//! the **never-act** guarantees (no target-repo writes, no `issuectl` execution, no filed issues).
//!
//! Hermetic: fixtures are throwaway temp dirs (no checked-in nested `.git`), the environment is
//! scrubbed of `PROJECT_CANON_*`, and the tests assert on the *printed* output — nothing here (or
//! in the binary) runs `issuectl`, writes to the target, or hits the network.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// A throwaway fixture repo under the OS temp root; removed on drop.
struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("pc-review-it-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Fixture { path }
    }

    fn touch(&self, rel: &str) -> &Self {
        std::fs::write(self.path.join(rel), b"x").unwrap();
        self
    }

    fn mkdir(&self, rel: &str) -> &Self {
        std::fs::create_dir_all(self.path.join(rel)).unwrap();
        self
    }

    /// A repo satisfying every mechanical MUST (and SHOULD) probe.
    fn conformant(tag: &str) -> Fixture {
        let f = Fixture::new(tag);
        f.touch("AGENTS.md")
            .touch("CLAUDE.md")
            .touch("README.md")
            .touch(".gitignore")
            .mkdir("issues")
            .mkdir(".git")
            .mkdir("crates/pc-core")
            .mkdir("crates/pc-cli");
        f
    }

    /// A recursively-sorted snapshot of every path under the fixture **and every file's bytes**,
    /// for a before/after equality check that review mutated nothing. Capturing contents (not just
    /// path names) catches an in-place overwrite that leaves the tree shape unchanged.
    fn snapshot(&self) -> Vec<(String, Option<Vec<u8>>)> {
        fn walk(
            dir: &std::path::Path,
            base: &std::path::Path,
            out: &mut Vec<(String, Option<Vec<u8>>)>,
        ) {
            let mut entries: Vec<_> = std::fs::read_dir(dir)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            entries.sort();
            for p in entries {
                let rel = p.strip_prefix(base).unwrap().display().to_string();
                if p.is_dir() {
                    out.push((rel, None));
                    walk(&p, base, out);
                } else {
                    out.push((rel, Some(std::fs::read(&p).unwrap())));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.path, &self.path, &mut out);
        out
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A `project-canon` command with every `PROJECT_CANON_*` variable scrubbed, so a stray override
/// on the dev/CI machine can't perturb these hermetic fixtures.
fn base_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_project-canon"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("PROJECT_CANON_") {
            cmd.env_remove(key);
        }
    }
    cmd
}

fn run_review(args: &[&str]) -> Output {
    base_command()
        .arg("review")
        .args(args)
        .output()
        .expect("run project-canon review")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("exited with a code")
}

// ===== advisory exit-code contract ==================================================

#[test]
fn a_repo_with_must_gaps_still_exits_zero() {
    // The defining difference from doctor: findings NEVER flip review's exit code.
    let f = Fixture::conformant("gaps");
    std::fs::remove_file(f.path.join("AGENTS.md")).unwrap(); // a MUST gap
    std::fs::remove_dir_all(f.path.join("issues")).unwrap(); // another MUST gap
    let out = run_review(&[f.path.to_str().unwrap()]);
    assert_eq!(
        code(&out),
        0,
        "review is advisory — a conformance gap must NOT be a non-zero exit; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("must-fix"), "{stdout}");
    assert!(stdout.contains("advisory"), "{stdout}");
}

#[test]
fn a_conformant_repo_exits_zero_and_stages_nothing() {
    let f = Fixture::conformant("clean");
    let out = run_review(&[f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("no confirmed gaps"), "{stdout}");
}

#[test]
fn bad_profile_is_a_usage_error_exit_one() {
    let f = Fixture::conformant("badprof");
    let out = run_review(&["--profile", "webapp", f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("webapp"));
}

#[test]
fn missing_target_is_a_usage_error_exit_one() {
    let out = run_review(&["/no/such/repo/anywhere"]);
    assert_eq!(code(&out), 1);
}

#[test]
fn unknown_flag_is_a_usage_error_exit_one() {
    let f = Fixture::conformant("flag");
    let out = run_review(&["--nope", f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("--nope"));
}

#[test]
fn help_exits_zero() {
    let out = run_review(&["--help"]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("USAGE"));
    assert!(
        stdout.contains("never"),
        "help must state the never-act contract"
    );
}

// ===== staged commands are PRINTED, not run =========================================

#[test]
fn staged_issuectl_command_is_printed_scoped_and_not_executed() {
    let f = Fixture::conformant("stage");
    std::fs::remove_file(f.path.join("AGENTS.md")).unwrap();
    let out = run_review(&[f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The command is present as text, scoped to the target repo, and clearly not executed.
    assert!(stdout.contains("issuectl new"), "{stdout}");
    assert!(stdout.contains("NOT executed"), "{stdout}");
    let canon = std::fs::canonicalize(&f.path).unwrap();
    assert!(
        stdout.contains(&format!("cd -- '{}'", canon.display())),
        "staged command must cd into the target repo: {stdout}"
    );
    // review filed NOTHING: no issues/<slug>/ tree was created for the staged command.
    assert!(!f
        .path
        .join("issues")
        .join("canon-base-doc-pattern")
        .exists());
}

#[test]
fn review_writes_nothing_to_the_target_repo() {
    // Snapshot before/after a full review (with gaps that produce staged commands): identical,
    // down to file *contents* (the snapshot hashes bytes, not just path names).
    let f = Fixture::conformant("nowrite");
    std::fs::remove_file(f.path.join("AGENTS.md")).unwrap();
    // Give an existing file distinctive bytes so an in-place overwrite would be caught.
    std::fs::write(f.path.join("README.md"), b"ORIGINAL-CONTENT-DO-NOT-TOUCH").unwrap();
    let before = f.snapshot();
    let out = run_review(&["--json", f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    let after = f.snapshot();
    assert_eq!(
        before, after,
        "review must not mutate the target repo (paths OR contents)"
    );
}

/// The structural proof of the never-act contract: even with a runnable `issuectl` first on
/// `PATH`, review must NOT execute it. A poisoned `issuectl` touches a sentinel when run; after a
/// review that stages several commands, the sentinel must be absent. Unix-only (shell shim).
#[cfg(unix)]
#[test]
fn review_never_executes_issuectl_even_when_it_is_on_path() {
    use std::os::unix::fs::PermissionsExt;

    let f = Fixture::conformant("nopexec");
    std::fs::remove_file(f.path.join("AGENTS.md")).unwrap(); // force confirmed gaps → staged cmds
    std::fs::remove_dir_all(f.path.join("issues")).unwrap();

    // A scratch bin dir with a poisoned `issuectl` that records the fact it ran, then a sentinel
    // path the shim will create. Both live OUTSIDE the target repo.
    let bin = Fixture::new("nopexec-bin");
    let sentinel = bin.path.join("issuectl-was-executed");
    let shim = bin.path.join("issuectl");
    std::fs::write(
        &shim,
        format!(
            "#!/bin/sh\ntouch {:?}\nexit 0\n",
            sentinel.to_str().unwrap()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();

    let orig_path = std::env::var("PATH").unwrap_or_default();
    let out = base_command()
        .arg("review")
        .arg(f.path.to_str().unwrap())
        .env("PATH", format!("{}:{}", bin.path.display(), orig_path))
        .output()
        .expect("run project-canon review");

    assert_eq!(code(&out), 0);
    // Review printed the staged issuectl commands…
    assert!(String::from_utf8_lossy(&out.stdout).contains("issuectl new"));
    // …but NEVER executed one: the poisoned shim was not run.
    assert!(
        !sentinel.exists(),
        "review executed issuectl — the never-act contract is violated"
    );
}

// ===== --json envelope ==============================================================

#[test]
fn json_envelope_is_well_formed_advisory_and_carries_findings() {
    let f = Fixture::conformant("json");
    std::fs::remove_file(f.path.join("AGENTS.md")).unwrap();
    let out = run_review(&["--json", f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stdout = stdout.trim();
    // Structural sanity of the §10 envelope (dependency-free — no JSON parser in this crate).
    assert!(stdout.starts_with('{') && stdout.ends_with('}'), "{stdout}");
    assert!(stdout.contains("\"schema_version\":1"));
    assert!(stdout.contains("\"verb\":\"review\""));
    assert!(stdout.contains("\"advisory\":true"));
    assert!(stdout.contains("\"exit_code\":0"));
    assert!(stdout.contains("\"kind\":\"confirmed-gap\""));
    assert!(stdout.contains("\"fix_class\":\"must-fix\""));
    assert!(stdout.contains("\"kind\":\"manual-verify\""));
    assert!(stdout.contains("\"discovery_candidates\":[]"));
    assert!(stdout.contains("\"staged_commands\":[\"( cd "));
    // Data on stdout only; diagnostics (if any) on stderr (§2).
    assert!(String::from_utf8_lossy(&out.stderr).is_empty());
}

#[test]
fn run_against_project_canons_built_binary_passes_every_runtime_probe() {
    let f = Fixture::conformant("self-runtime");
    let binary = env!("CARGO_BIN_EXE_project-canon");
    let before = f.snapshot();
    let out = run_review(&["--run", binary, "--json", f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 0, "{}", String::from_utf8_lossy(&out.stderr));
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let outcomes = payload["runtime_probe"]["outcomes"].as_array().unwrap();
    assert_eq!(outcomes.len(), 8);
    assert!(
        outcomes.iter().all(|outcome| outcome["status"] == "pass"),
        "{outcomes:?}"
    );
    assert_eq!(payload["summary"]["could_not_probe"], 0);
    assert_eq!(
        before,
        f.snapshot(),
        "runtime review must remain target-read-only"
    );
}

#[test]
fn a_missing_run_target_is_an_honest_could_not_probe_report() {
    let f = Fixture::conformant("missing-runtime");
    let missing = f.path.join("missing-binary");
    let out = run_review(&[
        "--run",
        missing.to_str().unwrap(),
        "--json",
        f.path.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 0, "review remains advisory");
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["summary"]["could_not_probe"], 8);
    assert!(payload["runtime_probe"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|outcome| outcome["status"] == "could-not-probe"));
    assert!(payload["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["kind"] == "could-not-probe"));
}

#[cfg(unix)]
#[test]
fn assume_defaults_cannot_be_combined_with_run_and_executes_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::conformant("assume-static");
    let sentinel = f.path.join("executed");
    let binary = f.path.join("target-binary");
    std::fs::write(
        &binary,
        format!("#!/bin/sh\ntouch {:?}\n", sentinel.to_str().unwrap()),
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out = run_review(&[
        "--assume-defaults",
        "--run",
        binary.to_str().unwrap(),
        f.path.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("static-only"));
    assert!(!sentinel.exists());
}
