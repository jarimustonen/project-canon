//! End-to-end integration tests for the `project-canon doctor` verb — run the built binary
//! against fixture repos and assert the exit-code contract + `--json` schema.
//!
//! Fixtures are built in a throwaway temp dir at test time (no checked-in nested `.git`), keeping
//! the workspace dependency-free (no `tempfile`/`assert_cmd`).

use std::path::{Path, PathBuf};
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
            std::env::temp_dir().join(format!("pc-doctor-it-{tag}-{}-{n}", std::process::id()));
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
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A `project-canon` command with every `PROJECT_CANON_*` variable scrubbed, so a stray override
/// on the dev/CI machine can't perturb these hermetic fixtures (the env layer is validated
/// strictly and would otherwise fail the run with exit 2).
fn base_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_project-canon"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("PROJECT_CANON_") {
            cmd.env_remove(key);
        }
    }
    cmd
}

/// Run the built `project-canon` binary with `doctor` + `args`.
fn run_doctor(args: &[&str]) -> Output {
    base_command()
        .arg("doctor")
        .args(args)
        .output()
        .expect("run project-canon doctor")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("exited with a code")
}

#[test]
fn conformant_repo_exits_zero() {
    let f = Fixture::conformant("ok");
    let out = run_doctor(&[f.path.to_str().unwrap()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("conformant"), "{stdout}");
}

#[test]
fn must_gap_exits_one() {
    let f = Fixture::conformant("gap");
    std::fs::remove_dir_all(f.path.join("issues")).unwrap(); // remove a MUST scaffold
    let out = run_doctor(&[f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("FAIL"), "{stdout}");
    assert!(stdout.contains("non-conformant"), "{stdout}");
}

#[test]
fn bad_profile_is_a_usage_error_exit_two() {
    let f = Fixture::conformant("badprof");
    let out = run_doctor(&["--profile", "webapp", f.path.to_str().unwrap()]);
    assert_eq!(
        code(&out),
        2,
        "bad --profile must be distinct from a conformance gap"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("webapp"), "{stderr}");
}

#[test]
fn missing_target_is_a_usage_error_exit_two() {
    let out = run_doctor(&["/no/such/repo/anywhere"]);
    assert_eq!(code(&out), 2);
}

#[test]
fn unknown_flag_is_a_usage_error_exit_two() {
    let f = Fixture::conformant("flag");
    let out = run_doctor(&["--nope", f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("--nope"));
}

#[test]
fn help_exits_zero() {
    let out = run_doctor(&["--help"]);
    assert_eq!(code(&out), 0);
    assert!(String::from_utf8_lossy(&out.stdout).contains("USAGE"));
}

#[test]
fn json_output_is_well_formed_and_matches_the_gate() {
    // Conformant → conformant:true, exit_code:0.
    let f = Fixture::conformant("json-ok");
    let out = run_doctor(&["--json", f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stdout = stdout.trim();
    // Structural sanity of the §10 envelope (dependency-free — no JSON parser in this crate).
    assert!(stdout.starts_with('{') && stdout.ends_with('}'), "{stdout}");
    assert!(stdout.contains("\"schema_version\":1"));
    assert!(stdout.contains("\"verb\":\"doctor\""));
    assert!(stdout.contains("\"profile\":\"cli\""));
    assert!(stdout.contains("\"conformant\":true"));
    assert!(stdout.contains("\"exit_code\":0"));
    // Every §1–§22 canon check id is present in the matrix.
    for section in ["canon.s01", "canon.s18", "canon.s22"] {
        assert!(stdout.contains(section), "missing {section}");
    }

    // A MUST gap → conformant:false, exit_code:1.
    let g = Fixture::conformant("json-gap");
    std::fs::remove_dir_all(g.path.join(".git")).unwrap();
    let out = run_doctor(&["--json", g.path.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"conformant\":false"));
    assert!(stdout.contains("\"exit_code\":1"));
    assert!(stdout.contains("\"status\":\"fail\""));
}

#[test]
fn profile_cli_resolves_all_22_canon_sections() {
    let f = Fixture::conformant("resolve");
    let out = run_doctor(&["--profile", "cli", "--json", f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    for n in 1u8..=22 {
        let id = format!("canon.s{n:02}");
        assert!(stdout.contains(&id), "cli profile should resolve {id}");
    }
}

#[test]
fn default_target_is_cwd() {
    // With no positional, doctor probes the current directory. Run it in a conformant fixture.
    let f = Fixture::conformant("cwd");
    let out = base_command()
        .arg("doctor")
        .current_dir(&f.path)
        .output()
        .unwrap();
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn inline_value_on_a_valueless_flag_is_a_usage_error() {
    // §1: `--json=false` must not silently emit JSON. Exit 2, echoing the offending flag.
    let f = Fixture::conformant("inline");
    let out = run_doctor(&["--json=false", f.path.to_str().unwrap()]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("--json"));
}

#[test]
fn double_dash_lets_a_dashy_repo_path_through() {
    // A conformant fixture reached via `--` still exits 0 (path parsing stopped at `--`).
    let f = Fixture::conformant("dashy");
    let out = run_doctor(&["--", f.path.to_str().unwrap()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn help_exits_zero_even_with_a_malformed_env() {
    // §2: help is an exit-0 event, independent of a broken PROJECT_CANON_* override.
    let out = base_command()
        .arg("doctor")
        .arg("--help")
        .env("PROJECT_CANON_TW_ENABLED", "not-a-bool")
        .output()
        .unwrap();
    assert_eq!(code(&out), 0);
    assert!(String::from_utf8_lossy(&out.stdout).contains("USAGE"));
}

#[test]
fn top_level_unknown_flag_is_a_usage_error() {
    // `project-canon --bogus` must not run the smoke stub and exit 0.
    let out = base_command().arg("--bogus").output().unwrap();
    assert_eq!(code(&out), 2);
}

/// Sanity: the fixture helpers point at a real directory (guards the temp-dir plumbing).
#[test]
fn fixture_paths_exist() {
    let f = Fixture::conformant("sanity");
    assert!(Path::new(&f.path).is_dir());
}
