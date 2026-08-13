//! The `project-canon` binary — deliberately a **thin stub** at v0.
//!
//! This issue (`profile-and-base-canon-model`) delivers the two-layer conformance *model* in
//! `project-canon-core`, not the verbs. The `new` / `doctor` / `review` verbs — and with them
//! the clap surface, the §2 error→exit map, and the §10 `--json` envelope — land in their own
//! (blocked) issues. Half-implementing the CLI canon here would be worse than an honest stub,
//! so `main` only reports status and a one-line smoke summary proving the core is wired.

use std::process::ExitCode;

use project_canon_core::{Archetype, EnvConfig, EnvConfigLayer, Model, Questionnaire};

fn main() -> ExitCode {
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
        "No verbs yet — `new`, `doctor`, and `review` are tracked as separate issues in this repo."
    );
    ExitCode::SUCCESS
}
