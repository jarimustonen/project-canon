//! End-to-end golden-shape tests for canon §14 machine-readable help.

use std::process::{Command, Output};

const REQUIRED_KEYS: &str = include_str!("fixtures/help-json.required-keys.golden");

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_project-canon"))
        .args(args)
        .output()
        .expect("run project-canon")
}

fn assert_help_shape(args: &[&str], expected_path: &str) {
    let out = run(args);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let stdout = String::from_utf8(out.stdout).expect("help is UTF-8 JSON");
    assert!(
        stdout.starts_with('{') && stdout.ends_with("}\n"),
        "{stdout}"
    );
    for key in REQUIRED_KEYS.lines() {
        assert!(
            stdout.contains(&format!("\"{key}\"")),
            "missing {key}: {stdout}"
        );
    }
    assert!(
        stdout.contains(&format!("\"command_path\":{expected_path}")),
        "{stdout}"
    );
    assert!(
        stdout.contains("\"examples\":[{"),
        "every help document has examples: {stdout}"
    );
    assert!(
        stdout.contains("\"deprecated\":"),
        "flags declare deprecation status: {stdout}"
    );
}

#[test]
fn root_help_json_matches_the_golden_shape_and_lists_verbs() {
    let out = run(&["--help", "--json"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    for key in REQUIRED_KEYS.lines() {
        assert!(
            stdout.contains(&format!("\"{key}\"")),
            "missing {key}: {stdout}"
        );
    }
    assert!(stdout.contains("\"command_path\":[\"project-canon\"]"));
    for verb in ["doctor", "new", "review", "skill", "version"] {
        assert!(stdout.contains(&format!("\"name\":\"{verb}\"")), "{stdout}");
    }
}

#[test]
fn verb_and_nested_help_json_are_structured() {
    assert_help_shape(
        &["doctor", "--help", "--json"],
        "[\"project-canon\",\"doctor\"]",
    );
    assert_help_shape(&["new", "--help", "--json"], "[\"project-canon\",\"new\"]");
    assert_help_shape(
        &["review", "--help", "--json"],
        "[\"project-canon\",\"review\"]",
    );
    assert_help_shape(
        &["skill", "--help", "--json"],
        "[\"project-canon\",\"skill\"]",
    );
    assert_help_shape(
        &["skill", "install", "--help", "--json"],
        "[\"project-canon\",\"skill\",\"install\"]",
    );
    assert_help_shape(
        &["version", "--help", "--json"],
        "[\"project-canon\",\"version\"]",
    );
}

#[test]
fn text_help_remains_human_readable() {
    let out = run(&["doctor", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("USAGE:"));
    assert!(!stdout.starts_with('{'));
}

#[test]
fn json_failure_still_uses_the_central_error_envelope() {
    let out = run(&["doctor", "--json", "--unknown"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("\"schema_version\":1"));
    assert!(stderr.contains("\"code\":\"usage_error\""));
}
