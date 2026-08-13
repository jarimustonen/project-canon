//! Done-criteria integration tests for the env config/hook layer.
//!
//! These assert the seam's public contract against the crate's public API only:
//! - the resolution order is defaults → config file → env override;
//! - the enumerated homebase specifics resolve through the layer with overridable defaults;
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

    let cfg = EnvConfig::resolve(&file, &env);

    assert_eq!(cfg.gh_account, "env-acct", "env > file");
    assert_eq!(cfg.repo_root, "/file/root", "file > default");
    assert_eq!(
        cfg.tw.projects_conf, "~/.config/tw/projects.conf",
        "untouched field keeps the default"
    );
}

#[test]
fn enumerated_specifics_resolve_through_the_layer_with_defaults() {
    let cfg = EnvConfig::resolve(&EnvConfigLayer::empty(), &EnvConfigLayer::empty());

    // gh account + ~/Sources/<name> convention + family-repo map.
    assert_eq!(cfg.gh_account, "jarimustonen");
    assert_eq!(
        cfg.repo_location("project-canon"),
        "~/Sources/project-canon"
    );
    let repos = cfg.family_repos();
    assert_eq!(
        repos.get("orchestratectl").map(String::as_str),
        Some("~/Sources/orchestratectl")
    );

    // tw / projects.conf registration.
    assert!(cfg.tw.enabled);
    assert_eq!(cfg.tw.projects_conf, "~/.config/tw/projects.conf");

    // .workmux.yaml prefix — portable default (no glyph), overridable.
    assert_eq!(cfg.workmux_emoji_prefix, None);

    // The hauis CI-release extension point exists but is unpopulated at v0.
    assert_eq!(cfg.ci_release.pattern, None);
}

#[test]
fn a_portable_override_replaces_every_homebase_specific() {
    // Someone else's environment: a full env override with none of the homebase values left.
    let env = EnvConfigLayer::from_env_vars(&BTreeMap::from([
        (
            "PROJECT_CANON_GH_ACCOUNT".to_string(),
            "octo-org".to_string(),
        ),
        ("PROJECT_CANON_REPO_ROOT".to_string(), "/w".to_string()),
        (
            "PROJECT_CANON_FAMILY_TOOLS".to_string(),
            "onectl,twoctl".to_string(),
        ),
        ("PROJECT_CANON_TW_ENABLED".to_string(), "false".to_string()),
    ]))
    .expect("valid env vars");
    let cfg = EnvConfig::resolve(&EnvConfigLayer::empty(), &env);

    assert_eq!(cfg.gh_account, "octo-org");
    assert_eq!(cfg.family_repo("onectl").as_deref(), Some("/w/onectl"));
    assert_eq!(cfg.family_repo("issuectl"), None, "homebase tools gone");
    assert!(!cfg.tw.enabled);
}

#[test]
fn env_config_is_orthogonal_to_the_conformance_model() {
    // Resolving the env layer touches nothing in the two-layer model: a cli resolution still
    // covers §1–§22 regardless of the env config.
    let _cfg = EnvConfig::resolve(&EnvConfigLayer::empty(), &EnvConfigLayer::empty());
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
