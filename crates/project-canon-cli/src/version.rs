//! The `version` meta-verb: a schema-versioned drift contract (canon §10).

use std::process::ExitCode;

use project_canon_core::Archetype;

use crate::error::{fail, json_requested, write_stdout, CliError};
use crate::json::Json;
use crate::skill::bundled_skill_metadata;

/// The version-payload schema. Bump only for a breaking payload change.
const SCHEMA_VERSION: i64 = 1;
const TOOL: &str = "project-canon";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_COMMIT: &str = env!("PROJECT_CANON_BUILD_COMMIT");
const BUILD_PROVENANCE_KIND: &str = env!("PROJECT_CANON_BUILD_PROVENANCE_KIND");
const BUILD_PROVENANCE_NOTE: &str = env!("PROJECT_CANON_BUILD_PROVENANCE_NOTE");

/// Run `project-canon version [--json]`.
pub fn run(args: &[String]) -> ExitCode {
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--help" => {
                print!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "--json" if !json => json = true,
            "--json" => {
                return fail(
                    true,
                    CliError::actionable("usage_error", "version: repeated flag: --json"),
                );
            }
            flag if flag.starts_with("--json=") => {
                return fail(
                    false,
                    CliError::actionable(
                        "usage_error",
                        format!("version: flag --json does not take a value (got {flag:?})"),
                    ),
                );
            }
            flag if flag.starts_with('-') => {
                return fail(
                    json_requested(args),
                    CliError::actionable("usage_error", format!("version: unknown flag: {flag}")),
                );
            }
            value => {
                return fail(
                    json_requested(args),
                    CliError::actionable(
                        "usage_error",
                        format!("version: unexpected argument: {value:?}"),
                    ),
                );
            }
        }
    }

    if json {
        write_stdout(&format!("{}\n", payload()), true)
    } else {
        write_stdout(&format!("{TOOL} {CLI_VERSION}\n"), false)
    }
}

/// The parser-level `--version` compatibility alias is intentionally text-only (canon §10).
pub fn legacy(args: &[String]) -> ExitCode {
    if args.is_empty() {
        return write_stdout(&format!("{TOOL} {CLI_VERSION}\n"), false);
    }
    fail(
        json_requested(args),
        CliError::actionable(
            "usage_error",
            "--version is text-only; use `project-canon version --json` for structured version data",
        ),
    )
}

fn payload() -> Json {
    let skills = bundled_skill_metadata()
        .into_iter()
        .map(|(name, cli_version, schema_version)| {
            Json::Object(vec![
                ("name".into(), Json::str(name)),
                ("cli_version".into(), Json::str(cli_version)),
                ("schema_version".into(), Json::Int(schema_version)),
            ])
        })
        .collect();

    Json::Object(vec![
        ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
        ("tool".into(), Json::str(TOOL)),
        ("version".into(), Json::str(CLI_VERSION)),
        (
            "commit".into(),
            (!BUILD_COMMIT.is_empty())
                .then_some(BUILD_COMMIT)
                .map_or(Json::Null, Json::str),
        ),
        (
            "build_provenance".into(),
            Json::Object(vec![
                ("kind".into(), Json::str(BUILD_PROVENANCE_KIND)),
                ("note".into(), Json::str(BUILD_PROVENANCE_NOTE)),
            ]),
        ),
        (
            "supported_schemas".into(),
            Json::Array(vec![Json::Int(SCHEMA_VERSION)]),
        ),
        (
            "supported_profiles".into(),
            Json::Array(
                Archetype::ALL
                    .iter()
                    .map(|archetype| Json::str(archetype.slug()))
                    .collect(),
            ),
        ),
        (
            "supported_surfaces".into(),
            Json::Array(
                ["doctor", "new", "review", "skill", "version"]
                    .into_iter()
                    .map(Json::str)
                    .collect(),
            ),
        ),
        ("skills".into(), Json::Array(skills)),
    ])
}

const HELP: &str = "\
project-canon version — print build and schema compatibility information

USAGE:
    project-canon version [--json]

FLAGS:
    --json     Emit the schema-versioned drift contract on stdout.
    --help     Show this help.

The parser-level `--version` alias is text-only. Agents needing structured data use
`project-canon version --json`.
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_carries_the_drift_contract() {
        let json = payload().to_string();
        for key in [
            "schema_version",
            "tool",
            "version",
            "commit",
            "build_provenance",
            "supported_schemas",
            "supported_profiles",
            "supported_surfaces",
            "skills",
        ] {
            assert!(
                json.contains(&format!("\"{key}\"")),
                "missing {key}: {json}"
            );
        }
        assert!(json.contains("\"name\":\"ai-first-cli-canon\""));
        assert!(json.contains("\"schema_version\":1"));
        assert!(
            BUILD_COMMIT.is_empty()
                || (BUILD_COMMIT.len() == 40
                    && BUILD_COMMIT.bytes().all(|byte| byte.is_ascii_hexdigit()))
        );
    }
}
