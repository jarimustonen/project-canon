//! Done-criteria integration tests for the env config/hook layer.
//!
//! These assert the seam's public contract against the crate's public API only:
//! - the resolution order is defaults → config file → env override;
//! - neutral defaults remain absent until a config layer supplies the environment;
//! - resolving the layer does not touch the two-layer model's invariants.

use std::collections::BTreeMap;

use project_canon_core::{Archetype, EnvConfig, EnvConfigLayer, Model, Questionnaire};

#[test]
fn resolution_order_defaults_then_file_then_env() {
    // File sets two values; env overrides one of them and adds a third. The third field the
    // file set but env did not must survive at the file value (env doesn't clobber unset fields),
    // and an entirely untouched field must stay at the built-in default.
    let file = EnvConfigLayer {
        gh_account: Some("file-acct".to_string()),
        repo_root: Some("/file/root".to_string()),
        ..EnvConfigLayer::empty()
    };
    let env = EnvConfigLayer::from_env_vars(&BTreeMap::from([(
        "PROJECT_CANON_GH_ACCOUNT".to_string(),
        "env-acct".to_string(),
    )]))
    .expect("valid env vars");

    let cfg = EnvConfig::resolve(&[&file, &env]);

    assert_eq!(cfg.gh_account.as_deref(), Some("env-acct"), "env > file");
    assert_eq!(
        cfg.repo_root.as_deref(),
        Some("/file/root"),
        "file > default"
    );
    assert_eq!(
        cfg.tw.projects_conf, "~/.config/tw/projects.conf",
        "untouched field keeps the default"
    );
}

#[test]
fn defaults_are_neutral_until_a_user_config_supplies_the_family_map() {
    let cfg = EnvConfig::resolve(&[&EnvConfigLayer::empty(), &EnvConfigLayer::empty()]);

    assert_eq!(cfg.gh_account, None);
    assert_eq!(cfg.repo_root, None);
    assert!(cfg.family_tools.is_empty());
    assert!(cfg.family_repos().is_empty());

    // tw / projects.conf registration.
    assert!(cfg.tw.enabled);
    assert_eq!(cfg.tw.projects_conf, "~/.config/tw/projects.conf");

    // .workmux.yaml prefix — portable default (no glyph), overridable.
    assert_eq!(cfg.workmux_emoji_prefix, None);

    // The hauis CI-release extension point exists but is unpopulated at v0.
    assert_eq!(cfg.ci_release.pattern, None);
}

#[test]
fn user_config_supplies_a_portable_family_environment() {
    // A user's environment is wholly supplied by configuration.
    let env = EnvConfigLayer::from_env_vars(&BTreeMap::from([
        (
            "PROJECT_CANON_GH_ACCOUNT".to_string(),
            "example-user".to_string(),
        ),
        (
            "PROJECT_CANON_REPO_ROOT".to_string(),
            "~/Projects".to_string(),
        ),
        (
            "PROJECT_CANON_FAMILY_TOOLS".to_string(),
            "alpha-tool,beta-tool".to_string(),
        ),
        ("PROJECT_CANON_TW_ENABLED".to_string(), "false".to_string()),
        (
            "PROJECT_CANON_TW_PROJECTS_CONF".to_string(),
            "/w/tw.conf".to_string(),
        ),
    ]))
    .expect("valid env vars");
    let cfg = EnvConfig::resolve(&[&EnvConfigLayer::empty(), &env]);

    assert_eq!(cfg.gh_account.as_deref(), Some("example-user"));
    assert_eq!(
        cfg.family_repo("alpha-tool").as_deref(),
        Some("~/Projects/alpha-tool")
    );
    assert_eq!(cfg.family_repo("unknown-tool"), None);
    assert!(!cfg.tw.enabled);
    assert_eq!(cfg.tw.projects_conf, "/w/tw.conf", "registry repointed too");
}

#[test]
fn strict_validation_happens_at_the_parse_edge() {
    // The seam rejects malformed env input up front (§1) — a blank override is an error, not a
    // silently-accepted value that would later resolve repos under `/`.
    assert!(EnvConfigLayer::from_env_vars(&BTreeMap::from([(
        "PROJECT_CANON_REPO_ROOT".to_string(),
        String::new(),
    )]))
    .is_err());
}

#[test]
fn convention_paths_are_tilde_expanded_at_the_edge() {
    // A verb turns a `~`-relative config path into a usable filesystem path via the pure helper,
    // supplying the home dir from its own I/O edge (core stays I/O-free).
    let layer = EnvConfigLayer {
        repo_root: Some("~/Projects".to_string()),
        family_tools: Some(["alpha-tool".to_string()].into()),
        ..EnvConfigLayer::empty()
    };
    let cfg = EnvConfig::resolve(&[&layer]);
    assert_eq!(
        EnvConfig::expand_home(&cfg.repo_location("alpha-tool").unwrap(), "/home/dev"),
        "/home/dev/Projects/alpha-tool"
    );
    assert_eq!(
        EnvConfig::expand_home(&cfg.tw.projects_conf, "/home/dev"),
        "/home/dev/.config/tw/projects.conf"
    );
}

#[test]
fn env_config_is_orthogonal_to_the_conformance_model() {
    // Resolving the env layer touches nothing in the two-layer model: a cli resolution still
    // covers §1–§22 regardless of the env config.
    let _cfg = EnvConfig::resolve(&[&EnvConfigLayer::empty(), &EnvConfigLayer::empty()]);
    let model = Model::standard();
    let resolution = model.resolve(
        &Questionnaire::builder(Archetype::Cli)
            .all_conditionals_yes()
            .build(),
    );
    assert_eq!(
        resolution.canon_section_set(&model),
        (1u8..=22).collect::<Vec<_>>()
    );
}
