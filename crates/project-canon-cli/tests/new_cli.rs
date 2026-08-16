//! End-to-end integration tests for the `project-canon new` verb — run the built binary against
//! throwaway temp dirs and assert the file generation, the dry-run plan, the `--json` shape, the
//! clobber guard, and a `new` → `doctor` conformance round-trip.
//!
//! Hermetic and side-effect-safe by construction: `new` only ever writes files, and these tests
//! never let it (or doctor) reach the network, `gh`, or `tw`. The bootstrap hooks (`git init`,
//! `issuectl init`, …) are only *printed* by `new`; the round-trip simulates the two local hooks
//! by creating `.git` and `issues/` itself before invoking doctor.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// A throwaway target dir under the OS temp root; removed on drop. `new` creates it.
struct Tmp {
    path: PathBuf,
}

impl Tmp {
    fn new(tag: &str) -> Tmp {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("pc-new-it-{tag}-{}-{n}", std::process::id()));
        Tmp { path }
    }
    fn str(&self) -> &str {
        self.path.to_str().unwrap()
    }
}

impl Drop for Tmp {
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
    cmd.env("PROJECT_CANON_GH_ACCOUNT", "example-user");
    cmd
}

fn run_new(args: &[&str]) -> Output {
    base_command()
        .arg("new")
        .args(args)
        .output()
        .expect("run project-canon new")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("exited with a code")
}

#[test]
fn generates_the_base_and_cli_scaffold() {
    let t = Tmp::new("gen");
    let out = run_new(&[t.str()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // Base scaffold.
    for rel in [
        "AGENTS.md",
        "CLAUDE.md",
        "AGENTS-AI-FIRST-CLI.md",
        "README.md",
        ".gitignore",
        ".workmux.yaml",
        "CONFORMANCE.md",
    ] {
        assert!(t.path.join(rel).exists(), "missing {rel}");
    }
    // cli-profile surface: the §22 core/cli split (name derived from the dir's final component).
    let name = t.path.file_name().unwrap().to_string_lossy().to_string();
    assert!(t.path.join("Cargo.toml").is_file());
    assert!(t
        .path
        .join(format!("crates/{name}-core/src/lib.rs"))
        .is_file());
    assert!(t
        .path
        .join(format!("crates/{name}-cli/src/main.rs"))
        .is_file());
    // The canon copy is byte-identical to the single source (`project_canon_core::CANON`) — the
    // same bytes `new` embeds. Asserting against the const, not a repo-relative path, keeps this
    // test self-contained inside the published crate tarball and free of root-symlink fragility.
    let bundled = std::fs::read_to_string(t.path.join("AGENTS-AI-FIRST-CLI.md")).unwrap();
    assert_eq!(
        bundled,
        project_canon_core::CANON,
        "bundled canon must match the single-source canon"
    );
    // `new` does NOT create .git or issues/ (those are hook products).
    assert!(!t.path.join(".git").exists());
    assert!(!t.path.join("issues").exists());
}

#[test]
fn dry_run_writes_nothing() {
    let t = Tmp::new("dry");
    let out = run_new(&["--dry-run", t.str()]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dry-run"), "{stdout}");
    assert!(stdout.contains("would-create"), "{stdout}");
    // Nothing was written — not even the target directory.
    assert!(!t.path.exists(), "dry-run must not create the target dir");
}

#[test]
fn json_plan_is_well_formed() {
    let t = Tmp::new("json");
    let out = run_new(&["--json", "--dry-run", t.str()]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stdout = stdout.trim();
    assert!(stdout.starts_with('{') && stdout.ends_with('}'), "{stdout}");
    assert!(stdout.contains("\"schema_version\":1"));
    assert!(stdout.contains("\"verb\":\"new\""));
    assert!(stdout.contains("\"profile\":\"cli\""));
    assert!(stdout.contains("\"dry_run\":true"));
    assert!(stdout.contains("\"surface_shape\":\"flat-verb\""));
    // The hook plan is present, and marks external actions.
    assert!(stdout.contains("\"id\":\"github-create\""));
    assert!(stdout.contains("\"class\":\"external\""));
    assert!(stdout.contains("\"exit_code\":0"));
}

#[test]
fn hooks_are_printed_not_run() {
    // The human view lists the bootstrap hooks under a "not run" banner and never executes them.
    let t = Tmp::new("hooks");
    let out = run_new(&["--dry-run", t.str()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("not run"), "{stdout}");
    assert!(stdout.contains("gh repo create"), "{stdout}");
    assert!(stdout.contains("issuectl init"), "{stdout}");
}

#[test]
fn non_empty_target_is_refused_without_force() {
    let t = Tmp::new("nonempty");
    std::fs::create_dir_all(&t.path).unwrap();
    std::fs::write(t.path.join("preexisting"), b"x").unwrap();
    let out = run_new(&[t.str()]);
    assert_eq!(
        code(&out),
        1,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("not empty"));
    // Nothing generated.
    assert!(!t.path.join("AGENTS.md").exists());
}

#[test]
fn force_fills_gaps_without_overwriting() {
    let t = Tmp::new("force");
    std::fs::create_dir_all(&t.path).unwrap();
    std::fs::write(t.path.join("README.md"), b"KEEP ME").unwrap();
    let out = run_new(&["--force", t.str()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The gap is filled…
    assert!(t.path.join("AGENTS.md").is_file());
    // …but the pre-existing file is untouched.
    assert_eq!(
        std::fs::read_to_string(t.path.join("README.md")).unwrap(),
        "KEEP ME"
    );
}

#[test]
fn new_requires_a_configured_gh_account() {
    let t = Tmp::new("missing-account");
    let xdg = std::env::temp_dir().join(format!("pc-new-missing-account-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&xdg);
    let mut cmd = base_command();
    let out = cmd
        .env("XDG_CONFIG_HOME", &xdg)
        .env_remove("PROJECT_CANON_GH_ACCOUNT")
        .args(["new", "--json", "--dry-run", t.str()])
        .output()
        .expect("run project-canon new");
    assert_eq!(code(&out), 1);
    assert!(out.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(error["error"]["code"], "required_config_missing");
    assert!(error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("PROJECT_CANON_GH_ACCOUNT"));
    let _ = std::fs::remove_dir_all(xdg);
}

#[test]
fn missing_dir_is_a_usage_error() {
    let out = run_new(&["--json"]);
    assert_eq!(code(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("target directory"));
}

#[test]
fn bad_profile_is_a_usage_error() {
    let t = Tmp::new("badprof");
    let out = run_new(&["--profile", "webapp", t.str()]);
    assert_eq!(code(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("webapp"));
    assert!(!t.path.exists());
}

#[test]
fn unknown_flag_is_a_usage_error() {
    let t = Tmp::new("flag");
    let out = run_new(&["--nope", t.str()]);
    assert_eq!(code(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("--nope"));
}

#[test]
fn unsafe_name_is_rejected() {
    // A `--name` that would traverse out of the target, inject a flag, or break Cargo is refused
    // at exit 1 — nothing is generated. This is the security boundary in action.
    let t = Tmp::new("badname");
    for bad in ["../escape", "foo bar", "-flag", "1tool", "foo;rm"] {
        let out = run_new(&["--name", bad, t.str()]);
        assert_eq!(
            code(&out),
            1,
            "name {bad:?} should be rejected; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert!(!t.path.exists(), "no scaffold on a rejected name");
}

#[test]
fn empty_positional_is_a_usage_error() {
    // The design forbids a cwd default; an empty `<dir>` must not silently scaffold into cwd.
    let out = run_new(&["--name", "foo", ""]);
    assert_eq!(code(&out), 1);
}

#[test]
fn flag_like_value_is_a_usage_error() {
    // `--name --json <dir>` is a forgotten value, not name="--json" — strict §1.
    let t = Tmp::new("flagval");
    let out = run_new(&["--name", "--json", t.str()]);
    assert_eq!(code(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("flag-like"));
}

#[test]
fn generated_cli_scaffold_builds() {
    // The whole point of `new` is a repo that starts conformant — prove the emitted cli scaffold
    // actually compiles. The generated crates have zero dependencies, so `cargo build --offline`
    // is hermetic (no network); an isolated target dir keeps it off the workspace's own target.
    let t = Tmp::new("build");
    let out = run_new(&["--name", "gentool", t.str()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let target_dir = t.path.join("_build_target");
    let build = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--offline")
        .current_dir(&t.path)
        .env("CARGO_TARGET_DIR", &target_dir)
        .output();
    let build = match build {
        Ok(o) => o,
        Err(_) => return, // no cargo on PATH — skip rather than fail the suite
    };
    assert!(
        build.status.success(),
        "generated cli scaffold must build; stdout: {} stderr: {}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
}

#[cfg(unix)]
#[test]
fn symlink_target_root_is_refused() {
    // A symlinked target path is refused outright — following it could let writes escape.
    let t = Tmp::new("symroot");
    let real = Tmp::new("symroot-real");
    std::fs::create_dir_all(&real.path).unwrap();
    std::os::unix::fs::symlink(&real.path, &t.path).unwrap();
    let out = run_new(&[t.str()]);
    assert_eq!(
        code(&out),
        1,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // Nothing written through the link.
    assert!(!real.path.join("AGENTS.md").exists());
    let _ = std::fs::remove_file(&t.path);
}

#[test]
fn help_exits_zero_even_with_a_malformed_env() {
    let out = base_command()
        .arg("new")
        .arg("--help")
        .env("PROJECT_CANON_TW_ENABLED", "not-a-bool")
        .output()
        .unwrap();
    assert_eq!(code(&out), 0);
    assert!(String::from_utf8_lossy(&out.stdout).contains("USAGE"));
}

#[test]
fn new_then_doctor_is_conformant() {
    // Generate a fresh cli repo, then simulate the two LOCAL bootstrap hooks (git init +
    // issuectl init) by creating `.git` and `issues/` — exactly what those hooks would produce —
    // and assert the real `doctor` binary reports the repo mechanically conformant (exit 0).
    let t = Tmp::new("roundtrip");
    let out = run_new(&[t.str()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    std::fs::create_dir_all(t.path.join(".git")).unwrap();
    std::fs::create_dir_all(t.path.join("issues")).unwrap();

    let doctor = base_command()
        .arg("doctor")
        .arg(t.str())
        .output()
        .expect("run project-canon doctor");
    assert_eq!(
        code(&doctor),
        0,
        "doctor should pass a freshly-bootstrapped repo; stdout: {} stderr: {}",
        String::from_utf8_lossy(&doctor.stdout),
        String::from_utf8_lossy(&doctor.stderr)
    );
    assert!(String::from_utf8_lossy(&doctor.stdout).contains("conformant"));
}
