//! The `project-canon` binary — the thin CLI over `project-canon-core`.
//!
//! The `doctor` verb (mechanical conformance gate) lives in [`doctor`] and the `new` scaffold
//! generator in [`new`]; `main` dispatches subcommands and, for a bare invocation, prints the
//! no-verb smoke summary proving the core + configuration layer are wired.

mod config;
mod doctor;
mod error;
mod help;
mod json;
mod new;
mod probes;
mod review;
mod shell;
mod skill;
mod version;

use std::process::ExitCode;

use project_canon_core::{Archetype, EnvConfig, Model, Questionnaire};

use crate::error::{fail, json_requested, CliError};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(status) = help::render_if_requested(&args) {
        return status;
    }
    match args.first().map(String::as_str) {
        Some("config") => config::run(&args[1..]),
        Some("doctor") => doctor::run(&args[1..]),
        Some("new") => new::run(&args[1..]),
        Some("review") => review::run(&args[1..]),
        Some("skill") => skill::run(&args[1..]),
        Some("version") => version::run(&args[1..]),
        Some("--version") => version::legacy(&args[1..]),
        // Only the bare invocation and top-level help hit the stub; an unknown subcommand OR an
        // unknown leading flag is a strict usage error (§1), not a silent exit-0 — a typo like
        // `project-canon --doctr` must not look like success.
        None | Some("--help") => smoke_summary(&args),
        Some(other) => {
            fail(
                json_requested(&args),
                CliError::actionable(
                    "usage_error",
                    format!(
                        "unknown subcommand or flag: {other:?}; known verbs: config, doctor, new, review, skill, version"
                    ),
                ),
            )
        }
    }
}

/// The no-verb smoke summary: prove the core model + configuration layer are wired.
fn smoke_summary(_args: &[String]) -> ExitCode {
    // Smoke summary from core: resolve the widest `cli` characterization and report the size of
    // its section-set. This keeps the core dependency real and gives an at-a-glance sanity line.
    let model = Model::standard();
    let cli_sections = model
        .resolve(
            &Questionnaire::builder(Archetype::Cli)
                .all_conditionals_yes()
                .build(),
        )
        .canon_section_set(&model)
        .len();

    // Resolve the same defaults → config file → environment layer consumed by commands.
    let cfg = match config::resolve() {
        Ok(config) => config,
        Err(error) => return fail(json_requested(_args), error.into_cli()),
    };
    // `~`-relative config paths become usable filesystem paths only after edge expansion; the
    // home dir comes from the environment (an I/O-edge concern, never core's).
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());

    eprintln!(
        "project-canon {}: model loaded ({} base dimensions; cli profile covers {} canon sections).",
        env!("CARGO_PKG_VERSION"),
        model.base_members().len(),
        cli_sections,
    );
    eprintln!(
        "env layer resolved (gh: {}; repo root: {}; {} family repos).",
        cfg.gh_account.as_deref().unwrap_or("not configured"),
        cfg.repo_root
            .as_deref()
            .map(|root| EnvConfig::expand_home(root, &home))
            .unwrap_or_else(|| "not configured".to_string()),
        cfg.family_repos().len(),
    );
    eprintln!(
        "Run `project-canon doctor --help`, `project-canon new --help`, `project-canon review --help`, `project-canon skill --help`, or `project-canon version --help`."
    );
    ExitCode::SUCCESS
}
