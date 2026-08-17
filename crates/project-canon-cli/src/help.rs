//! Schema-versioned, machine-readable command help (canon §14).
//!
//! The CLI's small handwritten parsers deliberately do not use clap. This module is their shared
//! command-description registry: one stable, ordered description tree renders every `--help
//! --json` payload, including nested `skill` subcommands. Keep entries aligned with parser flags;
//! integration tests exercise each documented example's help surface.

use std::process::ExitCode;

use crate::error::{fail, write_stdout, CliError};
use crate::json::Json;

const SCHEMA_VERSION: i64 = 1;
const TOOL: &str = "project-canon";

struct Arg {
    name: &'static str,
    summary: &'static str,
    required: bool,
    default: Option<&'static str>,
    accepted_values: &'static [&'static str],
}

struct Flag {
    name: &'static str,
    summary: &'static str,
    value_name: Option<&'static str>,
    default: Option<&'static str>,
    accepted_values: &'static [&'static str],
    env_var: Option<&'static str>,
    deprecated: bool,
}

struct Example {
    description: &'static str,
    argv: &'static [&'static str],
}

struct Command {
    path: &'static [&'static str],
    summary: &'static str,
    arguments: &'static [Arg],
    flags: &'static [Flag],
    examples: &'static [Example],
    subcommands: &'static [&'static str],
    exit_codes: &'static [(&'static str, &'static str)],
}

const NO_ARGS: &[Arg] = &[];
const NO_SUBCOMMANDS: &[&str] = &[];
const VALUES_ARCHETYPE: &[&str] = &["cli", "service", "library", "release"];
const VALUES_AGENT: &[&str] = &["claude", "codex"];
const VALUES_AGENT_ALL: &[&str] = &["claude", "codex", "all"];
const COMMON_EXITS: &[(&str, &str)] = &[
    ("0", "success, including help display"),
    ("1", "caller-actionable usage or validation error"),
    ("2", "operational or internal error"),
];

const ROOT: Command = Command {
    path: &[],
    summary: "Project-scoped conformance tool for the AI-first CLI / project family.",
    arguments: NO_ARGS,
    flags: &[
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--version",
            summary: "Full alias of the version verb; honors --json.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--json",
            summary: "With --help, emit this schema-versioned help document.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[Example {
        description: "Discover the available command surface.",
        argv: &["project-canon", "--help", "--json"],
    }],
    subcommands: &["config", "doctor", "new", "review", "skill", "version"],
    exit_codes: COMMON_EXITS,
};

const CONFIG: Command = Command {
    path: &["config"],
    summary: "Inspect effective configuration without changing it.",
    arguments: NO_ARGS,
    flags: &[
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--json",
            summary: "With --help, emit this help document.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[Example {
        description: "Inspect resolved configuration.",
        argv: &["project-canon", "config", "show", "--json"],
    }],
    subcommands: &["path", "show"],
    exit_codes: COMMON_EXITS,
};
const CONFIG_PATH: Command = Command {
    path: &["config", "path"],
    summary: "Print the effective configuration-file path.",
    arguments: NO_ARGS,
    flags: &[
        Flag {
            name: "--json",
            summary: "Emit the path inspection payload.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[Example {
        description: "Inspect the config file location.",
        argv: &["project-canon", "config", "path", "--json"],
    }],
    subcommands: NO_SUBCOMMANDS,
    exit_codes: COMMON_EXITS,
};
const CONFIG_SHOW: Command = Command {
    path: &["config", "show"],
    summary: "Show resolved configuration values with per-value provenance.",
    arguments: NO_ARGS,
    flags: &[
        Flag {
            name: "--json",
            summary: "Emit complete, schema-versioned config inspection data.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[Example {
        description: "Explain effective configuration values.",
        argv: &["project-canon", "config", "show", "--json"],
    }],
    subcommands: NO_SUBCOMMANDS,
    exit_codes: COMMON_EXITS,
};

const DOCTOR: Command = Command {
    path: &["doctor"],
    summary: "Mechanical conformance gate (read-only, non-interactive).",
    arguments: &[Arg {
        name: "repo",
        summary: "Target repository to probe.",
        required: false,
        default: Some("."),
        accepted_values: &[],
    }],
    flags: &[
        Flag {
            name: "--profile",
            summary: "Archetype used for conformance resolution.",
            value_name: Some("archetype"),
            default: Some("cli"),
            accepted_values: VALUES_ARCHETYPE,
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--assume-defaults",
            summary: "Characterize non-interactively with conservative defaults.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--json",
            summary: "Emit the structured doctor report.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--verbose",
            summary: "Show skipped checks in human output.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[Example {
        description: "Check the current repository as a CLI.",
        argv: &["project-canon", "doctor", "--json", "."],
    }],
    subcommands: NO_SUBCOMMANDS,
    exit_codes: &[
        ("0", "all mechanically decided MUST checks conform"),
        ("1", "a conformance gap or caller-actionable error"),
        ("2", "operational error"),
    ],
};

const NEW: Command = Command {
    path: &["new"],
    summary: "Scaffold a repo that starts conformant.",
    arguments: &[Arg {
        name: "dir",
        summary: "Target directory to scaffold into.",
        required: true,
        default: None,
        accepted_values: &[],
    }],
    flags: &[
        Flag {
            name: "--profile",
            summary: "Archetype to scaffold.",
            value_name: Some("archetype"),
            default: Some("cli"),
            accepted_values: VALUES_ARCHETYPE,
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--name",
            summary: "Project name.",
            value_name: Some("name"),
            default: Some("final component of dir"),
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--description",
            summary: "One-line project description.",
            value_name: Some("text"),
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--emoji",
            summary: "Workmux/tw window prefix.",
            value_name: Some("glyph"),
            default: None,
            accepted_values: &[],
            env_var: Some("PROJECT_CANON_WORKMUX_EMOJI_PREFIX"),
            deprecated: false,
        },
        Flag {
            name: "--assume-defaults",
            summary: "Characterize non-interactively with conservative defaults.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--dry-run",
            summary: "Print the plan without writing files.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--force",
            summary: "Fill gaps in a non-empty directory without overwriting files.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--json",
            summary: "Emit the structured scaffold plan.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--verbose",
            summary: "Show skipped rows and resolved sections in human output.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[Example {
        description: "Preview a CLI project scaffold.",
        argv: &[
            "project-canon",
            "new",
            "--dry-run",
            "--profile",
            "cli",
            "my-tool",
        ],
    }],
    subcommands: NO_SUBCOMMANDS,
    exit_codes: COMMON_EXITS,
};

const REVIEW: Command = Command {
    path: &["review"],
    summary: "Advisory conformance audit that recommends and stages, never acts.",
    arguments: &[Arg {
        name: "repo",
        summary: "Target repository to audit.",
        required: false,
        default: Some("."),
        accepted_values: &[],
    }],
    flags: &[
        Flag {
            name: "--profile",
            summary: "Archetype used for conformance resolution.",
            value_name: Some("archetype"),
            default: Some("cli"),
            accepted_values: VALUES_ARCHETYPE,
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--assume-defaults",
            summary: "Characterize non-interactively with conservative defaults.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--run",
            summary: "Opt in to timeout-bounded read-only probes of the named binary; requires profile cli and conflicts with --assume-defaults.",
            value_name: Some("binary"),
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--json",
            summary: "Emit the structured advisory review report.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--verbose",
            summary: "Show manual-verify, passing, and n/a rows in human output.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[
        Example {
            description: "Audit the current repository without executing a target.",
            argv: &["project-canon", "review", "--json", "."],
        },
        Example {
            description: "Also run read-only runtime probes against an explicitly named binary.",
            argv: &[
                "project-canon",
                "review",
                "--run",
                "./target/debug/example-tool",
                "--json",
                ".",
            ],
        },
    ],
    subcommands: NO_SUBCOMMANDS,
    exit_codes: &[
        ("0", "review completed, regardless of findings"),
        ("1", "caller-actionable usage or validation error"),
        ("2", "operational error"),
    ],
};

const SKILL: Command = Command {
    path: &["skill"],
    summary: "Install, list, or print companion AI skills.",
    arguments: NO_ARGS,
    flags: &[
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--json",
            summary: "With --help, emit this help document.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[Example {
        description: "List bundled skills.",
        argv: &["project-canon", "skill", "list", "--json"],
    }],
    subcommands: &["install", "list", "print", "show"],
    exit_codes: COMMON_EXITS,
};
const SKILL_INSTALL: Command = Command {
    path: &["skill", "install"],
    summary: "Install one or all companion skills.",
    arguments: &[Arg {
        name: "name",
        summary: "Optional bundled skill name; omit for all.",
        required: false,
        default: Some("all"),
        accepted_values: &["ai-first-cli-canon", "all"],
    }],
    flags: &[
        Flag {
            name: "--target",
            summary: "Installation base directory.",
            value_name: Some("dir"),
            default: Some("$HOME"),
            accepted_values: &[],
            env_var: Some("HOME"),
            deprecated: false,
        },
        Flag {
            name: "--agent",
            summary: "Agent runtime layout(s) to install.",
            value_name: Some("agent"),
            default: Some("all"),
            accepted_values: VALUES_AGENT_ALL,
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--force",
            summary: "Overwrite a blocking existing file.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--dry-run",
            summary: "Print the installation plan without writing.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--json",
            summary: "Emit the structured installation report.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[Example {
        description: "Preview a Claude skill installation.",
        argv: &[
            "project-canon",
            "skill",
            "install",
            "ai-first-cli-canon",
            "--agent",
            "claude",
            "--dry-run",
        ],
    }],
    subcommands: NO_SUBCOMMANDS,
    exit_codes: COMMON_EXITS,
};
const SKILL_LIST: Command = Command {
    path: &["skill", "list"],
    summary: "List bundled companion skills.",
    arguments: NO_ARGS,
    flags: &[
        Flag {
            name: "--json",
            summary: "Emit the structured skill catalog.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[Example {
        description: "List bundled skills as JSON.",
        argv: &["project-canon", "skill", "list", "--json"],
    }],
    subcommands: NO_SUBCOMMANDS,
    exit_codes: COMMON_EXITS,
};
const SKILL_PRINT: Command = Command {
    path: &["skill", "print"],
    summary: "Print a bundled skill without installing it.",
    arguments: &[Arg {
        name: "name",
        summary: "Bundled skill name.",
        required: true,
        default: None,
        accepted_values: &["ai-first-cli-canon"],
    }],
    flags: &[
        Flag {
            name: "--agent",
            summary: "Rendered agent format.",
            value_name: Some("agent"),
            default: Some("claude"),
            accepted_values: VALUES_AGENT,
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--json",
            summary: "Emit skill metadata and content as JSON.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[Example {
        description: "Read the canon skill as JSON.",
        argv: &[
            "project-canon",
            "skill",
            "print",
            "ai-first-cli-canon",
            "--json",
        ],
    }],
    subcommands: NO_SUBCOMMANDS,
    exit_codes: COMMON_EXITS,
};
const SKILL_SHOW: Command = Command {
    path: &["skill", "show"],
    summary: "Alias for `skill print`: print a bundled skill without installing it.",
    arguments: SKILL_PRINT.arguments,
    flags: SKILL_PRINT.flags,
    examples: &[Example {
        description: "Read the canon skill through the compatibility alias.",
        argv: &[
            "project-canon",
            "skill",
            "show",
            "ai-first-cli-canon",
            "--json",
        ],
    }],
    subcommands: NO_SUBCOMMANDS,
    exit_codes: COMMON_EXITS,
};
const VERSION: Command = Command {
    path: &["version"],
    summary: "Print build and schema compatibility information.",
    arguments: NO_ARGS,
    flags: &[
        Flag {
            name: "--json",
            summary: "Emit the schema-versioned drift contract.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
        Flag {
            name: "--help",
            summary: "Show this help.",
            value_name: None,
            default: None,
            accepted_values: &[],
            env_var: None,
            deprecated: false,
        },
    ],
    examples: &[Example {
        description: "Read the drift contract.",
        argv: &["project-canon", "version", "--json"],
    }],
    subcommands: NO_SUBCOMMANDS,
    exit_codes: COMMON_EXITS,
};

const COMMANDS: &[&Command] = &[
    &ROOT,
    &CONFIG,
    &CONFIG_PATH,
    &CONFIG_SHOW,
    &DOCTOR,
    &NEW,
    &REVIEW,
    &SKILL,
    &SKILL_INSTALL,
    &SKILL_LIST,
    &SKILL_PRINT,
    &SKILL_SHOW,
    &VERSION,
];

/// Intercept a syntactically explicit `--help --json` request before verb parsers render prose.
pub(crate) fn render_if_requested(args: &[String]) -> Option<ExitCode> {
    if !args.iter().any(|arg| arg == "--help") || !args.iter().any(|arg| arg == "--json") {
        return None;
    }
    // Positionals (for example `doctor . --help --json`) are not command-path components.
    // The command grammar is at most two levels deep today, with `skill` as the sole group.
    let path: Vec<&str> = match args.first().map(String::as_str) {
        None | Some("--help") | Some("--json") => vec![],
        Some("config") => match args.get(1).map(String::as_str) {
            None | Some("--help") | Some("--json") => vec!["config"],
            Some("path" | "show") => vec!["config", args[1].as_str()],
            Some(other) if other.starts_with('-') => vec!["config"],
            Some(other) => {
                return Some(fail(
                    true,
                    CliError::actionable(
                        "usage_error",
                        format!("unknown config subcommand for --help --json: {other}"),
                    ),
                ));
            }
        },
        Some("skill") => match args.get(1).map(String::as_str) {
            None | Some("--help") | Some("--json") => vec!["skill"],
            Some("install" | "list" | "print" | "show") => vec!["skill", args[1].as_str()],
            Some(other) if other.starts_with('-') => vec!["skill"],
            Some(other) => {
                return Some(fail(
                    true,
                    CliError::actionable(
                        "usage_error",
                        format!("unknown skill subcommand for --help --json: {other}"),
                    ),
                ));
            }
        },
        Some("doctor" | "new" | "review" | "version") => vec![args[0].as_str()],
        Some(other) => {
            return Some(fail(
                true,
                CliError::actionable(
                    "usage_error",
                    format!("unknown command path for --help --json: {other}"),
                ),
            ));
        }
    };
    let command = COMMANDS
        .iter()
        .copied()
        .find(|command| command.path == path.as_slice());
    Some(match command {
        Some(command) => write_stdout(&format!("{}\n", payload(command)), true),
        None => fail(
            true,
            CliError::actionable(
                "usage_error",
                format!("unknown command path for --help --json: {}", path.join(" ")),
            ),
        ),
    })
}

fn payload(command: &Command) -> Json {
    let arguments = command
        .arguments
        .iter()
        .map(|arg| {
            Json::Object(vec![
                ("name".into(), Json::str(arg.name)),
                ("summary".into(), Json::str(arg.summary)),
                ("required".into(), Json::Bool(arg.required)),
                ("default".into(), arg.default.map_or(Json::Null, Json::str)),
                (
                    "accepted_values".into(),
                    Json::Array(arg.accepted_values.iter().map(|v| Json::str(*v)).collect()),
                ),
            ])
        })
        .collect();
    let flags = command
        .flags
        .iter()
        .map(|flag| {
            Json::Object(vec![
                ("name".into(), Json::str(flag.name)),
                ("summary".into(), Json::str(flag.summary)),
                (
                    "value_name".into(),
                    flag.value_name.map_or(Json::Null, Json::str),
                ),
                ("default".into(), flag.default.map_or(Json::Null, Json::str)),
                (
                    "accepted_values".into(),
                    Json::Array(flag.accepted_values.iter().map(|v| Json::str(*v)).collect()),
                ),
                ("env_var".into(), flag.env_var.map_or(Json::Null, Json::str)),
                ("deprecated".into(), Json::Bool(flag.deprecated)),
            ])
        })
        .collect();
    let examples = command
        .examples
        .iter()
        .map(|example| {
            Json::Object(vec![
                ("description".into(), Json::str(example.description)),
                (
                    "argv".into(),
                    Json::Array(example.argv.iter().map(|v| Json::str(*v)).collect()),
                ),
            ])
        })
        .collect();
    let subcommands = command
        .subcommands
        .iter()
        .map(|name| {
            Json::Object(vec![
                ("name".into(), Json::str(*name)),
                (
                    "path".into(),
                    Json::Array(
                        command
                            .path
                            .iter()
                            .chain(std::iter::once(name))
                            .map(|part| Json::str(*part))
                            .collect(),
                    ),
                ),
            ])
        })
        .collect();
    let exit_codes = command
        .exit_codes
        .iter()
        .map(|(code, note)| {
            Json::Object(vec![
                ("code".into(), Json::str(*code)),
                ("note".into(), Json::str(*note)),
            ])
        })
        .collect();
    Json::Object(vec![
        ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
        ("tool".into(), Json::str(TOOL)),
        (
            "command_path".into(),
            Json::Array(
                std::iter::once(TOOL)
                    .chain(command.path.iter().copied())
                    .map(Json::str)
                    .collect(),
            ),
        ),
        ("summary".into(), Json::str(command.summary)),
        ("arguments".into(), Json::Array(arguments)),
        ("flags".into(), Json::Array(flags)),
        ("subcommands".into(), Json::Array(subcommands)),
        ("examples".into(), Json::Array(examples)),
        (
            "env_var_mappings".into(),
            Json::Array(
                command
                    .flags
                    .iter()
                    .filter_map(|flag| {
                        flag.env_var.map(|env_var| {
                            Json::Object(vec![
                                ("flag".into(), Json::str(flag.name)),
                                ("env_var".into(), Json::str(env_var)),
                            ])
                        })
                    })
                    .collect(),
            ),
        ),
        ("exit_codes".into(), Json::Array(exit_codes)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_command_has_an_example_and_help_has_required_keys() {
        for command in COMMANDS {
            assert!(!command.examples.is_empty(), "{}", command.path.join(" "));
            let text = payload(command).to_string();
            for key in [
                "schema_version",
                "command_path",
                "summary",
                "arguments",
                "flags",
                "subcommands",
                "examples",
                "env_var_mappings",
                "exit_codes",
            ] {
                assert!(text.contains(&format!("\"{key}\"")), "{text}");
            }
        }
    }
}
