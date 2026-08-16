//! End-to-end contracts for the read-only `config` inspection surface.

use std::fs;
use std::process::{Command, Output};

use serde_json::Value;

const GOLDEN: &str = include_str!("fixtures/config-show-json.golden");

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let path =
        std::env::temp_dir().join(format!("project-canon-config-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn command(xdg: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_project-canon"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("PROJECT_CANON_") {
            cmd.env_remove(key);
        }
    }
    cmd.env("XDG_CONFIG_HOME", xdg);
    cmd
}

fn run(xdg: &std::path::Path, args: &[&str]) -> Output {
    command(xdg).args(args).output().expect("run project-canon")
}

#[test]
fn config_show_json_matches_the_provenance_golden() {
    let xdg = temp_dir("golden");
    let config_dir = xdg.join("project-canon");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        "gh_account = \"file-account\"\nrepo_root = \"/file/root\"\nfamily_tools = [\"alpha\", \"beta\"]\ntw_enabled = false\ntw_projects_conf = \"/file/tw.conf\"\nworkmux_emoji_prefix = \"🧪\"\nci_release_pattern = \"file-ci\"\n\n[repo_overrides]\nalpha = \"/override/alpha\"\n",
    )
    .unwrap();
    let out = command(&xdg)
        .env("PROJECT_CANON_GH_ACCOUNT", "env-account")
        .env("PROJECT_CANON_REPO_ROOT", "/env/root")
        .env("PROJECT_CANON_FAMILY_TOOLS", "beta,gamma")
        .args(["config", "show", "--json"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let output = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        output.replace(&config_path.display().to_string(), "<CONFIG_PATH>"),
        GOLDEN
    );
    let _ = fs::remove_dir_all(xdg);
}

#[test]
fn config_path_and_missing_config_are_inspectable_without_writing() {
    let xdg = temp_dir("path");
    let expected = xdg.join("project-canon/config.toml");
    let out = run(&xdg, &["config", "path", "--json"]);
    assert_eq!(out.status.code(), Some(0));
    let payload: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["schema_version"], 1);
    assert_eq!(payload["config_path"], expected.display().to_string());
    assert_eq!(payload["exists"], false);
    assert!(
        !expected.exists(),
        "inspection must not create config files"
    );
    let _ = fs::remove_dir_all(xdg);
}

#[test]
fn config_path_does_not_parse_a_malformed_config_file() {
    let xdg = temp_dir("malformed-path");
    let config_dir = xdg.join("project-canon");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.toml"), "not valid = [").unwrap();
    let out = run(&xdg, &["config", "path", "--json"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["exists"], true);
    let _ = fs::remove_dir_all(xdg);
}

#[test]
fn config_json_failures_use_the_central_error_envelope() {
    let xdg = temp_dir("invalid");
    let out = run(&xdg, &["config", "show", "--json", "--unknown"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let error: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(error["schema_version"], 1);
    assert_eq!(error["error"]["code"], "usage_error");
    let _ = fs::remove_dir_all(xdg);
}
