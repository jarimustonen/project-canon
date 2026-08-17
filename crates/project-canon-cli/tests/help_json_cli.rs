//! End-to-end golden-shape tests for canon §14 machine-readable help.

use std::process::{Command, Output};

use serde_json::Value;

const REQUIRED_KEYS: &str = include_str!("fixtures/help-json.required-keys.golden");

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_project-canon"))
        .args(args)
        .output()
        .expect("run project-canon")
}

fn json_help(args: &[&str]) -> Value {
    let out = run(args);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    serde_json::from_slice(&out.stdout).expect("valid help JSON")
}

fn assert_help_shape(args: &[&str], expected_path: &[&str]) {
    let json = json_help(args);
    let object = json.as_object().expect("help document object");
    for key in REQUIRED_KEYS.lines() {
        assert!(object.contains_key(key), "missing top-level {key}: {json}");
    }
    assert_eq!(
        object["command_path"],
        Value::Array(
            expected_path
                .iter()
                .map(|part| Value::String((*part).into()))
                .collect()
        )
    );
    assert!(object["examples"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    for flag in object["flags"].as_array().unwrap() {
        assert!(
            flag.get("deprecated").is_some(),
            "flag lacks deprecation status: {flag}"
        );
    }
}

#[test]
fn root_help_json_matches_the_golden_shape_and_lists_verbs() {
    let json = json_help(&["--help", "--json"]);
    let object = json.as_object().unwrap();
    for key in REQUIRED_KEYS.lines() {
        assert!(object.contains_key(key), "missing top-level {key}: {json}");
    }
    assert_eq!(object["command_path"], serde_json::json!(["project-canon"]));
    let names: Vec<_> = object["subcommands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["config", "doctor", "new", "review", "skill", "version"]
    );
    let version_flag = object["flags"]
        .as_array()
        .unwrap()
        .iter()
        .find(|flag| flag["name"] == "--version")
        .expect("root help documents --version");
    let summary = version_flag["summary"].as_str().unwrap();
    assert!(
        summary.contains("Full alias of the version verb"),
        "{summary}"
    );
    assert!(summary.contains("honors --json"), "{summary}");
    assert!(summary.contains("Prefer `version`"), "{summary}");
    assert!(!summary.contains("text-only"), "{summary}");
}

#[test]
fn verb_and_nested_help_json_are_structured() {
    for (args, path) in [
        (
            &["config", "--help", "--json"][..],
            &["project-canon", "config"][..],
        ),
        (
            &["config", "path", "--help", "--json"][..],
            &["project-canon", "config", "path"][..],
        ),
        (
            &["config", "show", "--help", "--json"][..],
            &["project-canon", "config", "show"][..],
        ),
        (
            &["doctor", "--help", "--json"][..],
            &["project-canon", "doctor"][..],
        ),
        (
            &["new", "--help", "--json"][..],
            &["project-canon", "new"][..],
        ),
        (
            &["review", "--help", "--json"][..],
            &["project-canon", "review"][..],
        ),
        (
            &["skill", "--help", "--json"][..],
            &["project-canon", "skill"][..],
        ),
        (
            &["skill", "install", "--help", "--json"][..],
            &["project-canon", "skill", "install"][..],
        ),
        (
            &["skill", "list", "--help", "--json"][..],
            &["project-canon", "skill", "list"][..],
        ),
        (
            &["skill", "print", "--help", "--json"][..],
            &["project-canon", "skill", "print"][..],
        ),
        (
            &["skill", "show", "--help", "--json"][..],
            &["project-canon", "skill", "show"][..],
        ),
        (
            &["version", "--help", "--json"][..],
            &["project-canon", "version"][..],
        ),
    ] {
        assert_help_shape(args, path);
    }
}

#[test]
fn unknown_help_paths_are_json_errors() {
    for args in [
        ["unknown", "--help", "--json"].as_slice(),
        ["skill", "unknown", "--help", "--json"].as_slice(),
    ] {
        let out = run(args);
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
        let stderr: Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(stderr["error"]["code"], "usage_error");
    }
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
    let stderr: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(stderr["schema_version"], 1);
    assert_eq!(stderr["error"]["code"], "usage_error");
}
