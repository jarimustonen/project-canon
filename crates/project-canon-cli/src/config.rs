//! Read-only inspection of the environment configuration resolved by the CLI.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use project_canon_core::{EnvConfig, EnvConfigLayer};

use crate::error::{fail, json_requested, write_stdout, CliError};
use crate::json::Json;

const SCHEMA_VERSION: i64 = 1;

#[derive(Clone, Copy)]
enum Source {
    Default,
    File,
    Env,
}

impl Source {
    const fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::File => "file",
            Self::Env => "env",
        }
    }
}

struct ResolvedConfig {
    path: PathBuf,
    exists: bool,
    file: EnvConfigLayer,
    env: EnvConfigLayer,
    config: EnvConfig,
}

pub fn run(args: &[String]) -> ExitCode {
    let command = match parse_args(args) {
        Ok(Command::Help) => {
            print!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Ok(command) => command,
        Err(message) => {
            return fail(
                json_requested(args),
                CliError::actionable("usage_error", format!("config: {message}")),
            );
        }
    };

    match command {
        // Finding the file must work even when its contents are malformed: `path` is the repair
        // discovery command, not a configuration consumer.
        Command::Path { json } => match config_path() {
            Ok(path) => {
                let payload = path_payload(&path);
                if json {
                    write_stdout(&format!("{payload}\n"), true)
                } else {
                    write_stdout(&format!("{}\n", path.display()), false)
                }
            }
            Err(error) => fail(json, error.into_cli()),
        },
        Command::Show { json } => match resolve_with_details() {
            Ok(resolved) => {
                if json {
                    write_stdout(&format!("{}\n", show_payload(&resolved)), true)
                } else {
                    write_stdout(&render_human(&resolved), false)
                }
            }
            Err(error) => fail(json, error.into_cli()),
        },
        Command::Help => unreachable!("handled before resolving configuration"),
    }
}

pub(crate) fn resolve() -> Result<EnvConfig, ConfigError> {
    resolve_with_details().map(|resolved| resolved.config)
}

fn resolve_with_details() -> Result<ResolvedConfig, ConfigError> {
    let path = config_path()?;
    let exists = path.exists();
    let file = if exists {
        parse_file(&path)?
    } else {
        EnvConfigLayer::empty()
    };
    let env = EnvConfigLayer::from_env_vars(&std::env::vars().collect())
        .map_err(|error| ConfigError::Validation(format!("config: {error}")))?;
    let config = EnvConfig::resolve(&[&file, &env]);
    Ok(ResolvedConfig {
        path,
        exists,
        file,
        env,
        config,
    })
}

fn config_path() -> Result<PathBuf, ConfigError> {
    config_path_from(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

fn config_path_from(xdg: Option<&OsStr>, home: Option<&OsStr>) -> Result<PathBuf, ConfigError> {
    if let Some(xdg) = xdg.filter(|path| !path.is_empty()) {
        let path = PathBuf::from(xdg);
        if path.is_absolute() {
            return Ok(path.join("project-canon/config.toml"));
        }
        return Err(ConfigError::Validation(
            "config: XDG_CONFIG_HOME must be an absolute path".to_string(),
        ));
    }
    let home = home.filter(|path| !path.is_empty()).ok_or_else(|| {
        ConfigError::Validation(
            "config: HOME is not set; set HOME or XDG_CONFIG_HOME to locate the config file"
                .to_string(),
        )
    })?;
    Ok(PathBuf::from(home).join(".config/project-canon/config.toml"))
}

fn parse_file(path: &PathBuf) -> Result<EnvConfigLayer, ConfigError> {
    let contents = std::fs::read_to_string(path).map_err(|error| {
        ConfigError::Io(format!("config: cannot read {}: {error}", path.display()))
    })?;
    let value: toml::Value = contents.parse().map_err(|error| {
        ConfigError::Validation(format!(
            "config: invalid TOML in {}: {error}",
            path.display()
        ))
    })?;
    let table = value.as_table().ok_or_else(|| {
        ConfigError::Validation(format!(
            "config: {} must contain a TOML table",
            path.display()
        ))
    })?;

    for key in table.keys() {
        if !matches!(
            key.as_str(),
            "gh_account"
                | "repo_root"
                | "family_tools"
                | "tw_enabled"
                | "tw_projects_conf"
                | "workmux_emoji_prefix"
                | "ci_release_pattern"
                | "user_specific_deny_list"
                | "repo_overrides"
        ) {
            return Err(ConfigError::Validation(format!(
                "config: unknown key {key:?} in {}",
                path.display()
            )));
        }
    }

    let string = |key: &str| -> Result<Option<String>, ConfigError> {
        match table.get(key) {
            None => Ok(None),
            Some(value) => value
                .as_str()
                .map(|value| value.to_string())
                .map(Some)
                .ok_or_else(|| {
                    ConfigError::Validation(format!(
                        "config: {key} in {} must be a string",
                        path.display()
                    ))
                }),
        }
    };
    let bool_value = |key: &str| -> Result<Option<bool>, ConfigError> {
        match table.get(key) {
            None => Ok(None),
            Some(value) => value.as_bool().map(Some).ok_or_else(|| {
                ConfigError::Validation(format!(
                    "config: {key} in {} must be a boolean",
                    path.display()
                ))
            }),
        }
    };
    let family_tools = match table.get("family_tools") {
        None => None,
        Some(value) => Some(
            value
                .as_array()
                .ok_or_else(|| {
                    ConfigError::Validation(format!(
                        "config: family_tools in {} must be an array of strings",
                        path.display()
                    ))
                })?
                .iter()
                .map(|value| {
                    let tool = value.as_str().ok_or_else(|| {
                        ConfigError::Validation(format!(
                            "config: family_tools in {} must be an array of strings",
                            path.display()
                        ))
                    })?;
                    if tool.trim().is_empty() {
                        return Err(ConfigError::Validation(format!(
                            "config: family_tools in {} must not contain empty values",
                            path.display()
                        )));
                    }
                    Ok(tool.to_string())
                })
                .collect::<Result<_, _>>()?,
        ),
    };
    let user_specific_deny_list = match table.get("user_specific_deny_list") {
        None => None,
        Some(value) => Some(parse_string_set(value, "user_specific_deny_list", path)?),
    };
    let repo_overrides = match table.get("repo_overrides") {
        None => BTreeMap::new(),
        Some(value) => value
            .as_table()
            .ok_or_else(|| {
                ConfigError::Validation(format!(
                    "config: repo_overrides in {} must be a table",
                    path.display()
                ))
            })?
            .iter()
            .map(|(tool, value)| {
                let override_path = value.as_str().ok_or_else(|| {
                    ConfigError::Validation(format!(
                        "config: repo_overrides.{tool} in {} must be a string",
                        path.display()
                    ))
                })?;
                if tool.trim().is_empty() || override_path.trim().is_empty() {
                    return Err(ConfigError::Validation(format!(
                        "config: repo_overrides.{tool} in {} must use non-empty tool and path values",
                        path.display()
                    )));
                }
                Ok((tool.clone(), override_path.to_string()))
            })
            .collect::<Result<_, _>>()?,
    };

    Ok(EnvConfigLayer {
        gh_account: non_empty_file_value("gh_account", string("gh_account")?, path)?,
        repo_root: non_empty_file_value("repo_root", string("repo_root")?, path)?,
        family_tools,
        repo_overrides,
        tw_enabled: bool_value("tw_enabled")?,
        tw_projects_conf: non_empty_file_value(
            "tw_projects_conf",
            string("tw_projects_conf")?,
            path,
        )?,
        workmux_emoji_prefix: non_empty_file_value(
            "workmux_emoji_prefix",
            string("workmux_emoji_prefix")?,
            path,
        )?,
        ci_release_pattern: non_empty_file_value(
            "ci_release_pattern",
            string("ci_release_pattern")?,
            path,
        )?,
        user_specific_deny_list,
    })
}

fn parse_string_set(
    value: &toml::Value,
    key: &str,
    path: &Path,
) -> Result<std::collections::BTreeSet<String>, ConfigError> {
    value
        .as_array()
        .ok_or_else(|| {
            ConfigError::Validation(format!(
                "config: {key} in {} must be an array of strings",
                path.display()
            ))
        })?
        .iter()
        .map(|value| {
            let item = value.as_str().ok_or_else(|| {
                ConfigError::Validation(format!(
                    "config: {key} in {} must be an array of strings",
                    path.display()
                ))
            })?;
            if item.trim().is_empty() {
                return Err(ConfigError::Validation(format!(
                    "config: {key} in {} must not contain empty values",
                    path.display()
                )));
            }
            Ok(item.trim().to_string())
        })
        .collect()
}

fn non_empty_file_value(
    key: &str,
    value: Option<String>,
    path: &Path,
) -> Result<Option<String>, ConfigError> {
    match value {
        Some(value) if value.trim().is_empty() => Err(ConfigError::Validation(format!(
            "config: {key} in {} must not be empty",
            path.display()
        ))),
        value => Ok(value),
    }
}

fn source(file: bool, env: bool) -> Source {
    if env {
        Source::Env
    } else if file {
        Source::File
    } else {
        Source::Default
    }
}

fn value(value: Json, source: Source, detail: Option<String>, secret: bool) -> Json {
    Json::Object(vec![
        (
            "value".into(),
            if secret {
                Json::str("<redacted>")
            } else {
                value
            },
        ),
        ("source".into(), Json::str(source.name())),
        ("source_detail".into(), detail.map_or(Json::Null, Json::str)),
        ("secret".into(), Json::Bool(secret)),
    ])
}

fn detail(source: Source, path: &Path, env: &str) -> Option<String> {
    match source {
        Source::Default => None,
        Source::File => Some(path.display().to_string()),
        Source::Env => Some(env.to_string()),
    }
}

fn path_payload(path: &Path) -> Json {
    Json::Object(vec![
        ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
        ("config_path".into(), Json::str(path.display().to_string())),
        ("exists".into(), Json::Bool(path.exists())),
    ])
}

fn show_payload(resolved: &ResolvedConfig) -> Json {
    let file = &resolved.file;
    let env = &resolved.env;
    let cfg = &resolved.config;
    let gh_source = source(file.gh_account.is_some(), env.gh_account.is_some());
    let root_source = source(file.repo_root.is_some(), env.repo_root.is_some());
    let tools_source = source(file.family_tools.is_some(), env.family_tools.is_some());
    let tw_enabled_source = source(file.tw_enabled.is_some(), env.tw_enabled.is_some());
    let tw_conf_source = source(
        file.tw_projects_conf.is_some(),
        env.tw_projects_conf.is_some(),
    );
    let emoji_source = source(
        file.workmux_emoji_prefix.is_some(),
        env.workmux_emoji_prefix.is_some(),
    );
    let ci_source = source(
        file.ci_release_pattern.is_some(),
        env.ci_release_pattern.is_some(),
    );
    let deny_source = source(
        file.user_specific_deny_list.is_some(),
        env.user_specific_deny_list.is_some(),
    );
    let family_repos = cfg
        .family_repos()
        .into_iter()
        .map(|(tool, path)| {
            let repo_source = if file.repo_overrides.contains_key(&tool) {
                Source::File
            } else if cfg.family_tools.contains(&tool) {
                if matches!(root_source, Source::Env) || matches!(tools_source, Source::Env) {
                    Source::Env
                } else if matches!(root_source, Source::File)
                    || matches!(tools_source, Source::File)
                {
                    Source::File
                } else {
                    Source::Default
                }
            } else {
                Source::File
            };
            let provenance = match repo_source {
                Source::Default => None,
                Source::File if file.repo_overrides.contains_key(&tool) => Some(format!(
                    "{} (repo_overrides.{tool})",
                    resolved.path.display()
                )),
                Source::File => Some(resolved.path.display().to_string()),
                Source::Env => {
                    Some("PROJECT_CANON_REPO_ROOT and/or PROJECT_CANON_FAMILY_TOOLS".to_string())
                }
            };
            (tool, value(Json::str(path), repo_source, provenance, false))
        })
        .collect();
    let values = Json::Object(vec![
        (
            "gh_account".into(),
            value(
                cfg.gh_account.as_deref().map_or(Json::Null, Json::str),
                gh_source,
                detail(gh_source, &resolved.path, "PROJECT_CANON_GH_ACCOUNT"),
                false,
            ),
        ),
        (
            "repo_root".into(),
            value(
                cfg.repo_root.as_deref().map_or(Json::Null, Json::str),
                root_source,
                detail(root_source, &resolved.path, "PROJECT_CANON_REPO_ROOT"),
                false,
            ),
        ),
        (
            "family_tools".into(),
            value(
                Json::Array(cfg.family_tools.iter().map(Json::str).collect()),
                tools_source,
                detail(tools_source, &resolved.path, "PROJECT_CANON_FAMILY_TOOLS"),
                false,
            ),
        ),
        ("family_repos".into(), Json::Object(family_repos)),
        (
            "tw_enabled".into(),
            value(
                Json::Bool(cfg.tw.enabled),
                tw_enabled_source,
                detail(
                    tw_enabled_source,
                    &resolved.path,
                    "PROJECT_CANON_TW_ENABLED",
                ),
                false,
            ),
        ),
        (
            "tw_projects_conf".into(),
            value(
                Json::str(&cfg.tw.projects_conf),
                tw_conf_source,
                detail(
                    tw_conf_source,
                    &resolved.path,
                    "PROJECT_CANON_TW_PROJECTS_CONF",
                ),
                false,
            ),
        ),
        (
            "workmux_emoji_prefix".into(),
            value(
                cfg.workmux_emoji_prefix
                    .as_deref()
                    .map_or(Json::Null, Json::str),
                emoji_source,
                detail(
                    emoji_source,
                    &resolved.path,
                    "PROJECT_CANON_WORKMUX_EMOJI_PREFIX",
                ),
                false,
            ),
        ),
        (
            "ci_release_pattern".into(),
            value(
                cfg.ci_release
                    .pattern
                    .as_deref()
                    .map_or(Json::Null, Json::str),
                ci_source,
                detail(
                    ci_source,
                    &resolved.path,
                    "PROJECT_CANON_CI_RELEASE_PATTERN",
                ),
                false,
            ),
        ),
        (
            "user_specific_deny_list".into(),
            value(
                Json::Array(cfg.user_specific_deny_list.iter().map(Json::str).collect()),
                deny_source,
                detail(
                    deny_source,
                    &resolved.path,
                    "PROJECT_CANON_USER_SPECIFIC_DENY_LIST",
                ),
                true,
            ),
        ),
    ]);
    Json::Object(vec![
        ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
        (
            "config_path".into(),
            Json::str(resolved.path.display().to_string()),
        ),
        ("config_exists".into(), Json::Bool(resolved.exists)),
        ("values".into(), values),
    ])
}

fn render_human(resolved: &ResolvedConfig) -> String {
    let cfg = &resolved.config;
    let gh_source = source(
        resolved.file.gh_account.is_some(),
        resolved.env.gh_account.is_some(),
    );
    let root_source = source(
        resolved.file.repo_root.is_some(),
        resolved.env.repo_root.is_some(),
    );
    let tw_source = source(
        resolved.file.tw_enabled.is_some(),
        resolved.env.tw_enabled.is_some(),
    );
    format!(
        "config: {} ({})\ngh account: {} [{}]\nrepo root: {} [{}]\nfamily repos: {}\ntw registration: {} [{}]\n",
        resolved.path.display(),
        if resolved.exists { "present" } else { "not found" },
        cfg.gh_account.as_deref().unwrap_or("not configured"),
        human_source(gh_source, &resolved.path, "PROJECT_CANON_GH_ACCOUNT"),
        cfg.repo_root.as_deref().unwrap_or("not configured"),
        human_source(root_source, &resolved.path, "PROJECT_CANON_REPO_ROOT"),
        cfg.family_repos().len(),
        if cfg.tw.enabled { "enabled" } else { "disabled" },
        human_source(tw_source, &resolved.path, "PROJECT_CANON_TW_ENABLED"),
    )
}

fn human_source(source: Source, path: &Path, env: &str) -> String {
    match source {
        Source::Default => "default".to_string(),
        Source::File => format!("file: {}", path.display()),
        Source::Env => format!("env: {env}"),
    }
}

enum Command {
    Path { json: bool },
    Show { json: bool },
    Help,
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut json = false;
    let mut subcommand = None;
    for arg in args {
        match arg.as_str() {
            "--help" => return Ok(Command::Help),
            "--json" if !json => json = true,
            "--json" => return Err("repeated flag: --json".to_string()),
            flag if flag.starts_with("--json=") => {
                return Err(format!("flag --json does not take a value (got {flag:?})"))
            }
            flag if flag.starts_with('-') => return Err(format!("unknown flag: {flag}")),
            value if subcommand.is_none() => subcommand = Some(value),
            value => return Err(format!("unexpected argument: {value:?}")),
        }
    }
    match subcommand {
        Some("path") => Ok(Command::Path { json }),
        Some("show") => Ok(Command::Show { json }),
        Some(other) => Err(format!(
            "unknown subcommand: {other:?}; expected path or show"
        )),
        None => Err("missing subcommand; expected path or show".to_string()),
    }
}

#[derive(Debug)]
pub(crate) enum ConfigError {
    Validation(String),
    Io(String),
}

impl ConfigError {
    pub(crate) fn into_cli(self) -> CliError {
        match self {
            Self::Validation(message) => CliError::actionable("validation_error", message),
            Self::Io(message) => CliError::system("io_error", message),
        }
    }
}

const HELP: &str = "\
project-canon config — inspect effective configuration without changing it

USAGE:
    project-canon config <path|show> [--json]

SUBCOMMANDS:
    path    Print the effective config-file path.
    show    Print resolved settings and their provenance.

EXAMPLES:
    project-canon config path
    project-canon config show --json

The default config path is $XDG_CONFIG_HOME/project-canon/config.toml, or
$HOME/.config/project-canon/config.toml when XDG_CONFIG_HOME is unset. TOML values use
built-in default < config file < PROJECT_CANON_* environment-variable precedence.
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_uses_an_absolute_xdg_path_or_home() {
        assert_eq!(
            config_path_from(Some(OsStr::new("/tmp/project-canon-config-test")), None).unwrap(),
            PathBuf::from("/tmp/project-canon-config-test/project-canon/config.toml")
        );
        assert!(matches!(
            config_path_from(Some(OsStr::new("relative")), None),
            Err(ConfigError::Validation(_))
        ));
        assert!(matches!(
            config_path_from(None, None),
            Err(ConfigError::Validation(_))
        ));
    }

    #[test]
    fn parser_rejects_unknown_or_invalid_file_values() {
        let path =
            std::env::temp_dir().join(format!("project-canon-config-{}", std::process::id()));
        std::fs::write(&path, "unknown = true").unwrap();
        assert!(matches!(parse_file(&path), Err(ConfigError::Validation(_))));
        std::fs::write(&path, "tw_enabled = \"yes\"").unwrap();
        assert!(matches!(parse_file(&path), Err(ConfigError::Validation(_))));
        let _ = std::fs::remove_file(path);
    }
}
