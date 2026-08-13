//! The **env config/hook layer** — the first verb-independent seam (ADR 0009 §2/§5/§6).
//!
//! The non-portable homebase environment specifics (the `~/Sources` family-repo map, the gh
//! account, the `~/Sources/<name>` location convention, tw / `projects.conf` registration, the
//! `.workmux.yaml` emoji prefix, and a documented extension point for the future `hauis` CI
//! release pattern) do **not** belong hardcoded in the conformance logic. They live here behind
//! one resolved struct with sensible, overridable defaults.
//!
//! This is **orthogonal to [`Model`](crate::Model)**: a verb reads a `Model` (what conformance
//! means) *and* an [`EnvConfig`] (where this environment's repos/account/registration live). The
//! two-layer model semantics (`resolved(repo) = BASE ∪ PROFILE[archetype]`) are untouched.
//!
//! ## Resolution order
//!
//! ```text
//! EnvConfig::resolve(file_layer, env_layer)
//!   = builtin_defaults()      // step 1 — the single source of the defaults, in ONE place
//!       .apply(file_layer)    // step 2 — a parsed config file (lowest override)
//!       .apply(env_layer)     // step 3 — process env vars (highest precedence)
//! ```
//!
//! This is the AI-first CLI §8 precedence **flag > env > file > default** minus the flag rung —
//! a future verb layers `--gh-account`-style flags on top as a fourth, highest layer without
//! changing this module.
//!
//! ## I/O stays at the edge
//!
//! Core is zero-dependency and I/O-free (the crate's existing "serde is a deferred seam"
//! discipline). The override layers are *produced* at the CLI edge and handed to core as pure
//! [`EnvConfigLayer`] values: the env layer via [`EnvConfigLayer::from_env_vars`] (pure over a
//! map), the file layer by whatever the edge parses (the on-disk format + read are a future
//! `config` surface's concern, deferred like serde).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The environment-variable prefix for env overrides — the §8 "env mirrors flag" naming.
const ENV_PREFIX: &str = "PROJECT_CANON_";

/// The family CLI tools that follow the `~/Sources/<name>` location convention by default. The
/// map itself (`tool → repo path`) is materialized on demand from these + `repo_root`, so it is
/// never a set of stale absolute paths (see [`EnvConfig::family_repos`]).
const DEFAULT_FAMILY_TOOLS: [&str; 7] = [
    "issuectl",
    "orchestratectl",
    "crmctl",
    "tilictl",
    "ossctl",
    "intakectl",
    "glasspad",
];

/// tw / `projects.conf` registration settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwRegistration {
    /// Whether new repos are registered with `tw` at all.
    pub enabled: bool,
    /// Path to the `tw` `projects.conf` registry.
    pub projects_conf: String,
}

/// **Documented extension point** for the future `hauis` CI release pattern (ADR 0009).
///
/// Unpopulated at v0 (`pattern == None`). The field exists so the seam is stable when `hauis`
/// lands — a verb can branch on `ci_release.pattern` without a later breaking change to the
/// config shape.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CiReleaseHook {
    /// The CI release pattern name (e.g. `Some("hauis")` in the future), or `None` when no CI
    /// release pattern is configured — the portable default.
    pub pattern: Option<String>,
}

/// The fully-resolved environment config a verb consumes.
///
/// Built with [`EnvConfig::resolve`] (defaults → file → env). The defaults preserve today's
/// homebase behavior, but they now live in **one** place ([`EnvConfig::builtin_defaults`]) and
/// every value is overridable — that, plus core no longer hardcoding any of them, is the
/// portability win.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvConfig {
    /// The gh account family repos live under.
    pub gh_account: String,
    /// The base of the `~/Sources/<name>` location convention. See [`EnvConfig::repo_location`].
    pub repo_root: String,
    /// Known family tool names that resolve to a repo by the `repo_root/<name>` convention.
    pub family_tools: BTreeSet<String>,
    /// Explicit `tool → repo path` overrides for off-convention repos (usually empty).
    pub repo_overrides: BTreeMap<String, String>,
    /// tw / `projects.conf` registration.
    pub tw: TwRegistration,
    /// The `.workmux.yaml` emoji prefix. `None` = no glyph (the portable default); homebase sets
    /// its own via config.
    pub workmux_emoji_prefix: Option<String>,
    /// Extension point for the future `hauis` CI release pattern.
    pub ci_release: CiReleaseHook,
}

impl EnvConfig {
    /// Step 1 of resolution: the built-in defaults — the single source of the homebase values.
    pub fn builtin_defaults() -> Self {
        EnvConfig {
            gh_account: "jarimustonen".to_string(),
            repo_root: "~/Sources".to_string(),
            family_tools: DEFAULT_FAMILY_TOOLS.iter().map(|s| s.to_string()).collect(),
            repo_overrides: BTreeMap::new(),
            tw: TwRegistration {
                enabled: true,
                projects_conf: "~/.config/tw/projects.conf".to_string(),
            },
            workmux_emoji_prefix: None,
            ci_release: CiReleaseHook::default(),
        }
    }

    /// Resolve the config: [`builtin_defaults`](Self::builtin_defaults), then the file layer,
    /// then the env layer (highest precedence). See the module docs for the precedence rationale.
    pub fn resolve(file: &EnvConfigLayer, env: &EnvConfigLayer) -> Self {
        let mut cfg = Self::builtin_defaults();
        cfg.apply(file);
        cfg.apply(env);
        cfg
    }

    /// Merge one override layer in place. Present fields win; absent (`None`) fields are left
    /// untouched. The family-repo override map *extends* (add/repoint one tool without
    /// redeclaring the whole set).
    fn apply(&mut self, layer: &EnvConfigLayer) {
        if let Some(v) = &layer.gh_account {
            self.gh_account = v.clone();
        }
        if let Some(v) = &layer.repo_root {
            self.repo_root = v.clone();
        }
        if let Some(v) = &layer.family_tools {
            self.family_tools = v.clone();
        }
        for (tool, path) in &layer.repo_overrides {
            self.repo_overrides.insert(tool.clone(), path.clone());
        }
        if let Some(v) = layer.tw_enabled {
            self.tw.enabled = v;
        }
        if let Some(v) = &layer.tw_projects_conf {
            self.tw.projects_conf = v.clone();
        }
        if let Some(v) = &layer.workmux_emoji_prefix {
            self.workmux_emoji_prefix = Some(v.clone());
        }
        if let Some(v) = &layer.ci_release_pattern {
            self.ci_release.pattern = Some(v.clone());
        }
    }

    /// The `~/Sources/<name>` location convention, computed in one place. Applying a `repo_root`
    /// override re-derives every convention path through here.
    pub fn repo_location(&self, name: &str) -> String {
        format!("{}/{}", self.repo_root, name)
    }

    /// Resolve one family tool to its repo path: an explicit override if present, else the
    /// `repo_root/<name>` convention if `name` is a known family tool, else `None`.
    pub fn family_repo(&self, name: &str) -> Option<String> {
        if let Some(path) = self.repo_overrides.get(name) {
            return Some(path.clone());
        }
        if self.family_tools.contains(name) {
            return Some(self.repo_location(name));
        }
        None
    }

    /// The full family-repo map (`tool → repo path`), materialized on demand from the known tools
    /// and any overrides. Off-convention override tools not in `family_tools` are included too.
    pub fn family_repos(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for tool in &self.family_tools {
            map.insert(tool.clone(), self.repo_location(tool));
        }
        for (tool, path) in &self.repo_overrides {
            map.insert(tool.clone(), path.clone());
        }
        map
    }
}

/// A sparse override layer: every field is optional, and only present fields override.
///
/// Produced at the I/O edge (a parsed config file, or [`from_env_vars`](Self::from_env_vars))
/// and merged by [`EnvConfig::resolve`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvConfigLayer {
    /// Override for [`EnvConfig::gh_account`].
    pub gh_account: Option<String>,
    /// Override for [`EnvConfig::repo_root`].
    pub repo_root: Option<String>,
    /// Full replacement for the known family tool set.
    pub family_tools: Option<BTreeSet<String>>,
    /// Per-tool repo overrides (merged/extended into [`EnvConfig::repo_overrides`]).
    pub repo_overrides: BTreeMap<String, String>,
    /// Override for `tw.enabled`.
    pub tw_enabled: Option<bool>,
    /// Override for `tw.projects_conf`.
    pub tw_projects_conf: Option<String>,
    /// Override for the `.workmux.yaml` emoji prefix.
    pub workmux_emoji_prefix: Option<String>,
    /// Override for the CI release pattern extension point.
    pub ci_release_pattern: Option<String>,
}

impl EnvConfigLayer {
    /// An empty layer that overrides nothing (the "no config file" state).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build an override layer from process environment variables (the `PROJECT_CANON_*` set).
    ///
    /// Pure over the supplied map — the CLI passes `std::env::vars().collect()`, tests pass a
    /// synthetic map. Recognized keys (`§8` env-mirrors-flag naming):
    ///
    /// - `PROJECT_CANON_GH_ACCOUNT`
    /// - `PROJECT_CANON_REPO_ROOT`
    /// - `PROJECT_CANON_FAMILY_TOOLS` (comma-separated; replaces the tool set)
    /// - `PROJECT_CANON_TW_ENABLED` (`true`/`false` — strict, §1)
    /// - `PROJECT_CANON_TW_PROJECTS_CONF`
    /// - `PROJECT_CANON_WORKMUX_EMOJI_PREFIX`
    /// - `PROJECT_CANON_CI_RELEASE_PATTERN`
    ///
    /// A malformed `_ENABLED` value is an [`EnvConfigError`], never a silent coerce (§1/§4).
    pub fn from_env_vars(vars: &BTreeMap<String, String>) -> Result<Self, EnvConfigError> {
        let get = |key: &str| vars.get(&format!("{ENV_PREFIX}{key}")).map(String::as_str);
        let mut layer = EnvConfigLayer::empty();

        layer.gh_account = get("GH_ACCOUNT").map(str::to_string);
        layer.repo_root = get("REPO_ROOT").map(str::to_string);
        layer.family_tools = get("FAMILY_TOOLS").map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        });
        layer.tw_enabled = match get("TW_ENABLED") {
            None => None,
            Some(raw) => Some(parse_bool("TW_ENABLED", raw)?),
        };
        layer.tw_projects_conf = get("TW_PROJECTS_CONF").map(str::to_string);
        layer.workmux_emoji_prefix = get("WORKMUX_EMOJI_PREFIX").map(str::to_string);
        layer.ci_release_pattern = get("CI_RELEASE_PATTERN").map(str::to_string);

        Ok(layer)
    }
}

/// Parse a strict boolean env value, echoing the offending value on failure (§1/§4).
fn parse_bool(key: &str, raw: &str) -> Result<bool, EnvConfigError> {
    match raw {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(EnvConfigError::InvalidBool {
            var: format!("{ENV_PREFIX}{key}"),
            value: raw.to_string(),
        }),
    }
}

/// An error resolving the env config layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvConfigError {
    /// A boolean env var held a value outside `{true, false}`. Carries the variable name and the
    /// actual bad value so the message can echo both (§4 informative errors).
    InvalidBool { var: String, value: String },
}

impl fmt::Display for EnvConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvConfigError::InvalidBool { var, value } => write!(
                f,
                "{var}: invalid boolean {value:?} (expected \"true\" or \"false\")"
            ),
        }
    }
}

impl std::error::Error for EnvConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_hold_the_homebase_values_in_one_place() {
        let cfg = EnvConfig::builtin_defaults();
        assert_eq!(cfg.gh_account, "jarimustonen");
        assert_eq!(cfg.repo_root, "~/Sources");
        assert_eq!(
            cfg.repo_location("project-canon"),
            "~/Sources/project-canon"
        );
        assert_eq!(
            cfg.family_repo("issuectl").as_deref(),
            Some("~/Sources/issuectl")
        );
        assert!(cfg.tw.enabled);
        assert_eq!(cfg.tw.projects_conf, "~/.config/tw/projects.conf");
        // Portable defaults for the truly non-portable specifics.
        assert_eq!(cfg.workmux_emoji_prefix, None);
        // The extension point is present but unpopulated at v0.
        assert_eq!(cfg.ci_release.pattern, None);
    }

    #[test]
    fn unknown_tool_has_no_family_repo() {
        let cfg = EnvConfig::builtin_defaults();
        assert_eq!(cfg.family_repo("not-a-family-tool"), None);
    }

    #[test]
    fn resolution_order_is_defaults_then_file_then_env() {
        let file = EnvConfigLayer {
            gh_account: Some("file-acct".to_string()),
            repo_root: Some("/file/root".to_string()),
            ..EnvConfigLayer::empty()
        };
        // Env overrides the file for gh_account, but leaves repo_root (env doesn't set it) at the
        // file value — proving env > file > default without env clobbering unset fields.
        let env = EnvConfigLayer {
            gh_account: Some("env-acct".to_string()),
            ..EnvConfigLayer::empty()
        };
        let cfg = EnvConfig::resolve(&file, &env);
        assert_eq!(cfg.gh_account, "env-acct", "env wins over file");
        assert_eq!(cfg.repo_root, "/file/root", "file wins over default");
        assert_eq!(
            cfg.tw.projects_conf, "~/.config/tw/projects.conf",
            "untouched field keeps the default"
        );
    }

    #[test]
    fn empty_layers_resolve_to_the_defaults() {
        let cfg = EnvConfig::resolve(&EnvConfigLayer::empty(), &EnvConfigLayer::empty());
        assert_eq!(cfg, EnvConfig::builtin_defaults());
    }

    #[test]
    fn overriding_repo_root_rederives_every_convention_path() {
        let env = EnvConfigLayer {
            repo_root: Some("/work".to_string()),
            ..EnvConfigLayer::empty()
        };
        let cfg = EnvConfig::resolve(&EnvConfigLayer::empty(), &env);
        // No stale absolute paths baked at default-time: the map re-derives from the new root.
        assert_eq!(cfg.repo_location("issuectl"), "/work/issuectl");
        assert_eq!(
            cfg.family_repo("issuectl").as_deref(),
            Some("/work/issuectl")
        );
        assert!(cfg.family_repos().values().all(|p| p.starts_with("/work/")));
    }

    #[test]
    fn repo_override_pins_an_off_convention_repo() {
        let file = EnvConfigLayer {
            repo_overrides: BTreeMap::from([(
                "issuectl".to_string(),
                "/elsewhere/issuectl".to_string(),
            )]),
            ..EnvConfigLayer::empty()
        };
        let cfg = EnvConfig::resolve(&file, &EnvConfigLayer::empty());
        // The override wins over the convention for that one tool.
        assert_eq!(
            cfg.family_repo("issuectl").as_deref(),
            Some("/elsewhere/issuectl")
        );
        // Other tools still follow the convention.
        assert_eq!(
            cfg.family_repo("orchestratectl").as_deref(),
            Some("~/Sources/orchestratectl")
        );
    }

    #[test]
    fn override_admits_an_off_convention_tool_not_in_the_known_set() {
        let file = EnvConfigLayer {
            repo_overrides: BTreeMap::from([(
                "customctl".to_string(),
                "/opt/customctl".to_string(),
            )]),
            ..EnvConfigLayer::empty()
        };
        let cfg = EnvConfig::resolve(&file, &EnvConfigLayer::empty());
        assert_eq!(
            cfg.family_repo("customctl").as_deref(),
            Some("/opt/customctl")
        );
        assert!(cfg.family_repos().contains_key("customctl"));
    }

    #[test]
    fn from_env_vars_maps_the_recognized_keys() {
        let vars = BTreeMap::from([
            (
                "PROJECT_CANON_GH_ACCOUNT".to_string(),
                "octocat".to_string(),
            ),
            ("PROJECT_CANON_REPO_ROOT".to_string(), "/src".to_string()),
            (
                "PROJECT_CANON_FAMILY_TOOLS".to_string(),
                "foo, bar ,baz".to_string(),
            ),
            ("PROJECT_CANON_TW_ENABLED".to_string(), "false".to_string()),
            (
                "PROJECT_CANON_WORKMUX_EMOJI_PREFIX".to_string(),
                "🚀".to_string(),
            ),
            (
                "PROJECT_CANON_CI_RELEASE_PATTERN".to_string(),
                "hauis".to_string(),
            ),
            // An unrelated var is ignored.
            ("HOME".to_string(), "/home/x".to_string()),
        ]);
        let layer = EnvConfigLayer::from_env_vars(&vars).expect("valid vars");
        let cfg = EnvConfig::resolve(&EnvConfigLayer::empty(), &layer);

        assert_eq!(cfg.gh_account, "octocat");
        assert_eq!(cfg.repo_root, "/src");
        assert_eq!(
            cfg.family_tools,
            BTreeSet::from(["foo".to_string(), "bar".to_string(), "baz".to_string()])
        );
        assert!(!cfg.tw.enabled);
        assert_eq!(cfg.workmux_emoji_prefix.as_deref(), Some("🚀"));
        assert_eq!(cfg.ci_release.pattern.as_deref(), Some("hauis"));
    }

    #[test]
    fn from_env_vars_with_no_recognized_keys_is_an_empty_layer() {
        let vars = BTreeMap::from([("HOME".to_string(), "/home/x".to_string())]);
        let layer = EnvConfigLayer::from_env_vars(&vars).expect("valid");
        assert_eq!(layer, EnvConfigLayer::empty());
    }

    #[test]
    fn invalid_bool_env_value_errors_and_echoes_the_value() {
        let vars = BTreeMap::from([("PROJECT_CANON_TW_ENABLED".to_string(), "yes".to_string())]);
        let err = EnvConfigLayer::from_env_vars(&vars).expect_err("strict bool");
        assert_eq!(
            err,
            EnvConfigError::InvalidBool {
                var: "PROJECT_CANON_TW_ENABLED".to_string(),
                value: "yes".to_string(),
            }
        );
        // The message names the var and echoes the bad value (§4).
        let msg = err.to_string();
        assert!(msg.contains("PROJECT_CANON_TW_ENABLED"));
        assert!(msg.contains("yes"));
    }
}
