//! The **env config/hook layer** — the first verb-independent seam (ADR 0009 §2/§5/§6).
//!
//! Environment-specific settings do **not** belong hardcoded in the conformance logic. This
//! public artifact deliberately ships neutral defaults: no GitHub account, repository-root
//! convention, or family-tool map. Operators supply their own values through configuration when a
//! verb needs them. The layer also carries tw registration, a `.workmux.yaml` emoji prefix, and a
//! documented extension point for a future CI release pattern.
//!
//! This is **orthogonal to [`Model`](crate::Model)**: a verb reads a `Model` (what conformance
//! means) *and* an [`EnvConfig`] (where this environment's repos/account/registration live). The
//! two-layer model semantics (`resolved(repo) = BASE ∪ PROFILE[archetype]`) are untouched.
//!
//! ## Resolution order
//!
//! ```text
//! EnvConfig::resolve(&[&file_layer, &env_layer])
//!   = builtin_defaults()      // step 1 — the single source of the defaults, in ONE place
//!       .apply(file_layer)    // step 2 — a parsed config file (lowest override)
//!       .apply(env_layer)     // step 3 — process env vars (highest precedence)
//! ```
//!
//! This is the AI-first CLI §8 precedence **flag > env > file > default** minus the flag rung.
//! [`resolve`](EnvConfig::resolve) takes an *ordered slice* of layers, so a future verb adds the
//! flag rung by appending it (`&[&file, &env, &flags]`) — the fourth, highest layer needs no
//! change to this module.
//!
//! ## I/O stays at the edge
//!
//! Core is zero-dependency and I/O-free (the crate's existing "serde is a deferred seam"
//! discipline). The override layers are *produced* at the CLI edge and handed to core as pure
//! [`EnvConfigLayer`] values: the env layer via [`EnvConfigLayer::from_env_vars`] (pure over a
//! map), the file layer by the CLI edge's `config` TOML parser.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The environment-variable prefix for env overrides — the §8 "env mirrors flag" naming.
const ENV_PREFIX: &str = "PROJECT_CANON_";

/// tw / `projects.conf` registration settings.
///
/// `#[non_exhaustive]`: a future registration knob (e.g. a `tw` binary path) must not break
/// downstream `match`/read sites — this is a foundational seam three verbs inherit.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TwRegistration {
    /// Whether new repos are registered with `tw` at all.
    pub enabled: bool,
    /// Path to the `tw` `projects.conf` registry (may be `~`-relative — see
    /// [`EnvConfig::expand_home`]).
    pub projects_conf: String,
}

/// **Documented extension point** for the future `hauis` CI release pattern (ADR 0009).
///
/// Unpopulated at v0 (`pattern == None`). The field exists so the seam is stable when `hauis`
/// lands. `#[non_exhaustive]` so `hauis` can add fields (workflow, channel, …) without breaking
/// construction/read sites in the inheriting verbs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct CiReleaseHook {
    /// The CI release pattern name (e.g. `Some("hauis")` in the future), or `None` when no CI
    /// release pattern is configured — the portable default.
    pub pattern: Option<String>,
}

/// The fully-resolved environment config a verb consumes.
///
/// Built with [`EnvConfig::resolve`] (defaults → file → env). Built-in defaults are neutral
/// because this crate is public. A verb that requires an account or repository convention must
/// report which configuration key or environment variable the operator needs to set.
///
/// `#[non_exhaustive]`: verbs read this resolved type but never construct it (they call
/// [`resolve`](Self::resolve)); marking it lets new environment specifics land without breaking
/// downstream read sites.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EnvConfig {
    /// The GitHub account family repos live under, if configured.
    pub gh_account: Option<String>,
    /// The base of a `<repo_root>/<name>` location convention, if configured. See
    /// [`EnvConfig::repo_location`].
    pub repo_root: Option<String>,
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
    /// Step 1 of resolution: neutral built-in defaults for this public artifact.
    pub fn builtin_defaults() -> Self {
        EnvConfig {
            gh_account: None,
            repo_root: None,
            family_tools: BTreeSet::new(),
            repo_overrides: BTreeMap::new(),
            tw: TwRegistration {
                enabled: false,
                projects_conf: "~/.config/tw/projects.conf".to_string(),
            },
            workmux_emoji_prefix: None,
            ci_release: CiReleaseHook::default(),
        }
    }

    /// Resolve the config: [`builtin_defaults`](Self::builtin_defaults) with `layers` applied in
    /// order, each overriding the previous. The canonical call is
    /// `resolve(&[&file_layer, &env_layer])` (defaults → file → env); a future verb adds a flag
    /// layer simply by appending it (`&[&file, &env, &flags]`) — the promised fourth rung needs
    /// no change to this module. Applying an already-validated layer never fails, so the merge is
    /// infallible; validation lives at each source's parse edge (see
    /// [`EnvConfigLayer::from_env_vars`]).
    pub fn resolve(layers: &[&EnvConfigLayer]) -> Self {
        let mut cfg = Self::builtin_defaults();
        for layer in layers {
            cfg.apply(layer);
        }
        cfg
    }

    /// Merge one override layer in place. Present fields win; absent (`None`) fields are left
    /// untouched. The family-repo override map *extends* (add/repoint one tool without
    /// redeclaring the whole set). Note: a sparse layer cannot yet *clear* an optional value or
    /// *remove* an override a lower layer set — a tri-state patch is a deferred seam for when the
    /// config-file layer lands (env, the only wired source, cannot express a clear anyway).
    pub(crate) fn apply(&mut self, layer: &EnvConfigLayer) {
        if let Some(v) = &layer.gh_account {
            self.gh_account = Some(v.clone());
        }
        if let Some(v) = &layer.repo_root {
            self.repo_root = Some(v.clone());
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

    /// The configured `<repo_root>/<name>` location convention, computed in one place. Applying a
    /// `repo_root` override re-derives every convention path through here.
    ///
    /// The result is a **config string**, still `~`-relative when `repo_root` is — pass it
    /// through [`expand_home`](Self::expand_home) at the I/O edge before touching the filesystem.
    /// A trailing slash on `repo_root` is trimmed so the join never doubles the separator. It
    /// returns `None` until an operator configures `repo_root`.
    pub fn repo_location(&self, name: &str) -> Option<String> {
        self.repo_root
            .as_ref()
            .map(|root| format!("{}/{}", root.trim_end_matches('/'), name))
    }

    /// Expand a leading `~` / `~/` against `home` — the **pure** half of tilde resolution, so the
    /// whole family inherits one expansion behavior. Core stays I/O-free: the CLI edge supplies
    /// `home` (e.g. from `$HOME`); a path that is not `~`-prefixed is returned unchanged.
    ///
    /// Verbs must call this before handing a config path (`repo_location(..)`, `tw.projects_conf`)
    /// to `std::fs`/`git` — a literal `~` is not a filesystem path on any OS.
    pub fn expand_home(path: &str, home: &str) -> String {
        let home = home.trim_end_matches('/');
        if path == "~" {
            home.to_string()
        } else if let Some(rest) = path.strip_prefix("~/") {
            format!("{home}/{rest}")
        } else {
            path.to_string()
        }
    }

    /// Resolve one family tool to its repo path: an explicit override if present, else the
    /// `repo_root/<name>` convention if `name` is a known family tool, else `None`.
    pub fn family_repo(&self, name: &str) -> Option<String> {
        if let Some(path) = self.repo_overrides.get(name) {
            return Some(path.clone());
        }
        if self.family_tools.contains(name) {
            return self.repo_location(name);
        }
        None
    }

    /// The full family-repo map (`tool → repo path`), materialized on demand from the known tools
    /// and any overrides. Off-convention override tools not in `family_tools` are included too.
    pub fn family_repos(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for tool in &self.family_tools {
            if let Some(path) = self.repo_location(tool) {
                map.insert(tool.clone(), path);
            }
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
    /// **Strict (§1/§4), consistently:** a malformed `_ENABLED` value, an empty/whitespace-only
    /// scalar value, or a `FAMILY_TOOLS` list with an empty element are all [`EnvConfigError`]s
    /// echoing the offending variable — never a silent coerce or a silent drop. To *keep* a
    /// default, omit the variable; setting it to empty is an error, not an "unset". (Surrounding
    /// whitespace around each `FAMILY_TOOLS` element is trimmed as list syntax.)
    pub fn from_env_vars(vars: &BTreeMap<String, String>) -> Result<Self, EnvConfigError> {
        let get = |key: &str| vars.get(&format!("{ENV_PREFIX}{key}")).map(String::as_str);

        Ok(EnvConfigLayer {
            gh_account: non_empty("GH_ACCOUNT", get("GH_ACCOUNT"))?,
            repo_root: non_empty("REPO_ROOT", get("REPO_ROOT"))?,
            family_tools: match get("FAMILY_TOOLS") {
                None => None,
                Some(raw) => Some(parse_family_tools(raw)?),
            },
            repo_overrides: BTreeMap::new(),
            tw_enabled: match get("TW_ENABLED") {
                None => None,
                Some(raw) => Some(parse_bool("TW_ENABLED", raw)?),
            },
            tw_projects_conf: non_empty("TW_PROJECTS_CONF", get("TW_PROJECTS_CONF"))?,
            workmux_emoji_prefix: non_empty("WORKMUX_EMOJI_PREFIX", get("WORKMUX_EMOJI_PREFIX"))?,
            ci_release_pattern: non_empty("CI_RELEASE_PATTERN", get("CI_RELEASE_PATTERN"))?,
        })
    }
}

/// Accept an optional scalar env value, rejecting an empty/whitespace-only one (§1: no silent
/// coerce of a blank override into "unset"). Absent (`None`) stays absent; a present value is
/// kept verbatim (no trimming — that would be a silent fixup).
fn non_empty(suffix: &str, raw: Option<&str>) -> Result<Option<String>, EnvConfigError> {
    match raw {
        None => Ok(None),
        Some(v) if v.trim().is_empty() => Err(EnvConfigError::EmptyValue {
            var: format!("{ENV_PREFIX}{suffix}"),
        }),
        Some(v) => Ok(Some(v.to_string())),
    }
}

/// Parse the comma-separated `FAMILY_TOOLS` list. Surrounding whitespace per element is trimmed
/// as list syntax; an empty element (`foo,,bar`, a bare `,`, or an empty string) is an error, not
/// a silent drop (§1).
fn parse_family_tools(raw: &str) -> Result<BTreeSet<String>, EnvConfigError> {
    let mut set = BTreeSet::new();
    for part in raw.split(',') {
        let tool = part.trim();
        if tool.is_empty() {
            return Err(EnvConfigError::InvalidList {
                var: format!("{ENV_PREFIX}FAMILY_TOOLS"),
                value: raw.to_string(),
            });
        }
        set.insert(tool.to_string());
    }
    Ok(set)
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

/// An error resolving the env config layer. `#[non_exhaustive]`: strictness will grow (tool-name
/// grammar, path validation), and a new variant must not break downstream `match` arms.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EnvConfigError {
    /// A boolean env var held a value outside `{true, false}`. Carries the variable name and the
    /// actual bad value so the message can echo both (§4 informative errors).
    InvalidBool { var: String, value: String },
    /// A scalar env var was set to an empty/whitespace-only value (to keep the default, omit it).
    EmptyValue { var: String },
    /// A list-valued env var had an empty element. Carries the variable and the raw value.
    InvalidList { var: String, value: String },
}

impl fmt::Display for EnvConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvConfigError::InvalidBool { var, value } => write!(
                f,
                "{var}: invalid boolean {value:?} (expected \"true\" or \"false\")"
            ),
            EnvConfigError::EmptyValue { var } => {
                write!(
                    f,
                    "{var}: empty value (omit the variable to leave this setting unset)"
                )
            }
            EnvConfigError::InvalidList { var, value } => write!(
                f,
                "{var}: invalid list {value:?} (empty element; use non-empty comma-separated items)"
            ),
        }
    }
}

impl std::error::Error for EnvConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_neutral_for_a_public_artifact() {
        let cfg = EnvConfig::builtin_defaults();
        assert_eq!(cfg.gh_account, None);
        assert_eq!(cfg.repo_root, None);
        assert!(cfg.family_tools.is_empty());
        assert!(cfg.family_repos().is_empty());
        assert!(!cfg.tw.enabled);
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
        let cfg = EnvConfig::resolve(&[&file, &env]);
        assert_eq!(
            cfg.gh_account.as_deref(),
            Some("env-acct"),
            "env wins over file"
        );
        assert_eq!(
            cfg.repo_root.as_deref(),
            Some("/file/root"),
            "file wins over default"
        );
        assert_eq!(
            cfg.tw.projects_conf, "~/.config/tw/projects.conf",
            "untouched field keeps the default"
        );
    }

    #[test]
    fn empty_layers_resolve_to_the_defaults() {
        let cfg = EnvConfig::resolve(&[&EnvConfigLayer::empty(), &EnvConfigLayer::empty()]);
        assert_eq!(cfg, EnvConfig::builtin_defaults());
    }

    #[test]
    fn overriding_repo_root_rederives_every_convention_path() {
        let env = EnvConfigLayer {
            repo_root: Some("/work".to_string()),
            ..EnvConfigLayer::empty()
        };
        let cfg = EnvConfig::resolve(&[&EnvConfigLayer::empty(), &env]);
        // No stale absolute paths baked at default-time: the map re-derives from the new root.
        assert_eq!(
            cfg.repo_location("example-tool").as_deref(),
            Some("/work/example-tool")
        );
        assert_eq!(cfg.family_repo("example-tool"), None);
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
        let cfg = EnvConfig::resolve(&[&file, &EnvConfigLayer::empty()]);
        // The override wins over the convention for that one tool.
        assert_eq!(
            cfg.family_repo("issuectl").as_deref(),
            Some("/elsewhere/issuectl")
        );
        // Unconfigured tools do not silently acquire a convention path.
        assert_eq!(cfg.family_repo("another-tool"), None);
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
        let cfg = EnvConfig::resolve(&[&file, &EnvConfigLayer::empty()]);
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
        let cfg = EnvConfig::resolve(&[&EnvConfigLayer::empty(), &layer]);

        assert_eq!(cfg.gh_account.as_deref(), Some("octocat"));
        assert_eq!(cfg.repo_root.as_deref(), Some("/src"));
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

    #[test]
    fn empty_scalar_env_value_is_rejected_not_silently_accepted() {
        // §1: a blank REPO_ROOT is an error (which would otherwise resolve repos under `/`), not
        // a silent "unset". To keep the default you omit the variable.
        for var in [
            "PROJECT_CANON_REPO_ROOT",
            "PROJECT_CANON_GH_ACCOUNT",
            "PROJECT_CANON_TW_PROJECTS_CONF",
        ] {
            let vars = BTreeMap::from([(var.to_string(), "   ".to_string())]);
            let err = EnvConfigLayer::from_env_vars(&vars).expect_err("empty rejected");
            assert_eq!(
                err,
                EnvConfigError::EmptyValue {
                    var: var.to_string()
                }
            );
        }
    }

    #[test]
    fn family_tools_with_an_empty_element_is_rejected() {
        // `foo,,bar` is a malformed list — an error, not a silent drop of the empty element (§1).
        let vars = BTreeMap::from([(
            "PROJECT_CANON_FAMILY_TOOLS".to_string(),
            "foo,,bar".to_string(),
        )]);
        let err = EnvConfigLayer::from_env_vars(&vars).expect_err("strict list");
        assert_eq!(
            err,
            EnvConfigError::InvalidList {
                var: "PROJECT_CANON_FAMILY_TOOLS".to_string(),
                value: "foo,,bar".to_string(),
            }
        );
        // An empty FAMILY_TOOLS is likewise rejected (omit it to keep the default set).
        let empty = BTreeMap::from([("PROJECT_CANON_FAMILY_TOOLS".to_string(), String::new())]);
        assert!(EnvConfigLayer::from_env_vars(&empty).is_err());
    }

    #[test]
    fn resolve_applies_an_arbitrary_number_of_ordered_layers() {
        // Demonstrates the flag-rung claim: a third (highest) layer overrides the env layer with
        // no change to this module.
        let file = EnvConfigLayer {
            gh_account: Some("file".to_string()),
            ..EnvConfigLayer::empty()
        };
        let env = EnvConfigLayer {
            gh_account: Some("env".to_string()),
            ..EnvConfigLayer::empty()
        };
        let flags = EnvConfigLayer {
            gh_account: Some("flag".to_string()),
            ..EnvConfigLayer::empty()
        };
        let cfg = EnvConfig::resolve(&[&file, &env, &flags]);
        assert_eq!(cfg.gh_account.as_deref(), Some("flag"), "last layer wins");
        // Zero layers resolves to the built-in defaults.
        assert_eq!(EnvConfig::resolve(&[]), EnvConfig::builtin_defaults());
    }

    #[test]
    fn expand_home_resolves_only_a_leading_tilde() {
        assert_eq!(
            EnvConfig::expand_home("~/Projects/x", "/home/j"),
            "/home/j/Projects/x"
        );
        assert_eq!(EnvConfig::expand_home("~", "/home/j"), "/home/j");
        // A trailing slash on home does not double the separator.
        assert_eq!(
            EnvConfig::expand_home("~/Projects", "/home/j/"),
            "/home/j/Projects"
        );
        // Non-tilde paths pass through untouched; a mid-string `~` is not expanded.
        assert_eq!(EnvConfig::expand_home("/abs/path", "/home/j"), "/abs/path");
        assert_eq!(EnvConfig::expand_home("a/~/b", "/home/j"), "a/~/b");
        // A configured repository convention becomes a usable filesystem path only after expansion.
        let layer = EnvConfigLayer {
            repo_root: Some("~/Projects".to_string()),
            ..EnvConfigLayer::empty()
        };
        let cfg = EnvConfig::resolve(&[&layer]);
        assert_eq!(
            EnvConfig::expand_home(&cfg.repo_location("example-tool").unwrap(), "/home/j"),
            "/home/j/Projects/example-tool"
        );
    }

    #[test]
    fn repo_location_does_not_double_a_trailing_separator() {
        let env = EnvConfigLayer {
            repo_root: Some("/work/".to_string()),
            ..EnvConfigLayer::empty()
        };
        let cfg = EnvConfig::resolve(&[&env]);
        assert_eq!(cfg.repo_location("x").as_deref(), Some("/work/x"));
    }
}
