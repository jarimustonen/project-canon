//! The `project-canon` binary — the thin CLI over `project-canon-core`.
//!
//! The `doctor` verb (mechanical conformance gate) lives in [`doctor`] and the `new` scaffold
//! generator in [`new`]; `review` is still tracked as a separate issue. `main` dispatches the
//! subcommand and, for a bare invocation, prints the no-verb smoke summary proving the core +
//! env layer are wired.

mod doctor;
mod json;
mod new;
mod probes;
mod review;
mod shell;
mod skill;

use std::process::ExitCode;

use project_canon_core::{Archetype, EnvConfig, EnvConfigLayer, Model, Questionnaire};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("doctor") => doctor::run(&args[1..]),
        Some("new") => new::run(&args[1..]),
        Some("review") => review::run(&args[1..]),
        Some("skill") => skill::run(&args[1..]),
        // Only the bare invocation (and the not-yet-parsed top-level status flags) hit the stub;
        // an unknown subcommand OR an unknown leading flag is a strict usage error (§1), not a
        // silent exit-0 — a typo like `project-canon --doctr` must not look like success.
        None | Some("--help") | Some("--version") => smoke_summary(),
        Some(other) => {
            eprintln!("project-canon: unknown subcommand or flag: {other:?}");
            eprintln!("known verbs: doctor, new, review, skill");
            ExitCode::from(2)
        }
    }
}

/// The no-verb smoke summary: prove the core model + env config layer are wired.
fn smoke_summary() -> ExitCode {
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

    // Exercise the env config/hook layer at the I/O edge (the only place I/O belongs): read the
    // process env into an override layer over the built-in defaults. The config-file layer is a
    // deferred `config` surface, so it is empty here. A malformed `PROJECT_CANON_*` value is a
    // §1/§4 strict-validation error, not a silent coerce.
    let env_layer = match EnvConfigLayer::from_env_vars(&std::env::vars().collect()) {
        Ok(layer) => layer,
        Err(err) => {
            eprintln!("project-canon: {err}");
            return ExitCode::from(2); // §2: caller-actionable bad input.
        }
    };
    let cfg = EnvConfig::resolve(&[&EnvConfigLayer::empty(), &env_layer]);
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
        cfg.gh_account,
        EnvConfig::expand_home(&cfg.repo_root, &home),
        cfg.family_repos().len(),
    );
    eprintln!(
        "Run `project-canon doctor --help`, `project-canon new --help`, `project-canon review --help`, or `project-canon skill --help`."
    );
    ExitCode::SUCCESS
}
