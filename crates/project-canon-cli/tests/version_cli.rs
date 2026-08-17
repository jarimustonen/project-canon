//! End-to-end contract tests for `project-canon version`.

use std::process::{Command, Output};

const GOLDEN: &str = include_str!("fixtures/version-json.golden");

fn base_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_project-canon"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("PROJECT_CANON_") {
            cmd.env_remove(key);
        }
    }
    cmd
}

fn run(args: &[&str]) -> Output {
    base_command()
        .args(args)
        .output()
        .expect("run project-canon")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("exited with a code")
}

/// Normalize values which legitimately change between builds, while preserving the complete
/// shape, stable keys, ordering, and every non-volatile value in the golden contract.
fn normalize_json(output: &str) -> String {
    let version = env!("CARGO_PKG_VERSION");
    let mut normalized = output.replace(&format!("\"{version}\""), "\"<VERSION>\"");

    let commit_prefix = "\"commit\":\"";
    if let Some(start) = normalized.find(commit_prefix) {
        let value_start = start + commit_prefix.len();
        let value_end = normalized[value_start..]
            .find('"')
            .map(|offset| value_start + offset)
            .expect("commit string closes");
        normalized.replace_range(value_start..value_end, "<COMMIT_OR_NULL>");
    } else {
        normalized = normalized.replace("\"commit\":null", "\"commit\":\"<COMMIT_OR_NULL>\"");
    }

    for key in ["kind", "note"] {
        let prefix = format!("\"{key}\":\"");
        let start = normalized.find(&prefix).expect("provenance field exists") + prefix.len();
        let end = normalized[start..]
            .find('"')
            .map(|offset| start + offset)
            .expect("provenance string closes");
        normalized.replace_range(
            start..end,
            if key == "kind" {
                "<PROVENANCE_KIND>"
            } else {
                "<PROVENANCE_NOTE>"
            },
        );
    }
    normalized
}

#[test]
fn version_json_spellings_are_identical_and_match_the_stable_golden_contract() {
    let verb = run(&["version", "--json"]);
    for args in [
        ["--version", "--json"].as_slice(),
        ["--json", "--version"].as_slice(),
    ] {
        let alias = run(args);
        assert_eq!(code(&alias), code(&verb), "args: {args:?}");
        assert_eq!(alias.stdout, verb.stdout, "args: {args:?}");
        assert_eq!(alias.stderr, verb.stderr, "args: {args:?}");
    }
    assert_eq!(
        code(&verb),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&verb.stderr)
    );
    assert!(
        verb.stderr.is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&verb.stderr)
    );

    let stdout = String::from_utf8(verb.stdout).expect("version JSON is UTF-8");
    assert_eq!(normalize_json(&stdout), GOLDEN);
}

#[test]
fn version_json_has_valid_commit_or_explicit_null_provenance() {
    let out = run(&["version", "--json"]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    let git_commit = stdout
        .split("\"commit\":\"")
        .nth(1)
        .and_then(|tail| tail.split('"').next());
    match git_commit {
        Some(commit) => {
            assert!(
                commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "commit must be a full hex SHA: {commit}"
            );
            assert!(
                stdout.contains("\"build_provenance\":{\"kind\":\"git\""),
                "a git commit must report git provenance: {stdout}"
            );
        }
        None => {
            assert!(stdout.contains("\"commit\":null"));
            assert!(stdout.contains("\"build_provenance\":{\"kind\":\"tarball\""));
        }
    }
}

#[test]
fn version_text_spellings_are_identical_and_human_readable() {
    let verb = run(&["version"]);
    let alias = run(&["--version"]);
    assert_eq!(code(&alias), code(&verb));
    assert_eq!(alias.stdout, verb.stdout);
    assert_eq!(alias.stderr, verb.stderr);
    assert_eq!(code(&verb), 0);
    assert!(verb.stderr.is_empty());
    let stdout = String::from_utf8(verb.stdout).unwrap();
    assert_eq!(
        stdout,
        format!("project-canon {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn json_failure_uses_the_central_error_envelope() {
    for args in [
        ["version", "--json", "--unknown"].as_slice(),
        ["version", "--json", "--json=invalid"].as_slice(),
    ] {
        let out = run(args);
        assert_eq!(code(&out), 1);
        assert!(out.stdout.is_empty());
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.contains("\"schema_version\":1"), "{stderr}");
        assert!(stderr.contains("\"code\":\"usage_error\""), "{stderr}");
    }
}
