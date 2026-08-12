//! The `project-canon` binary — deliberately a **thin stub** at v0.
//!
//! This issue (`profile-and-base-canon-model`) delivers the two-layer conformance *model* in
//! `project-canon-core`, not the verbs. The `new` / `doctor` / `review` verbs — and with them
//! the clap surface, the §2 error→exit map, and the §10 `--json` envelope — land in their own
//! (blocked) issues. Half-implementing the CLI canon here would be worse than an honest stub,
//! so `main` only reports status and a one-line smoke summary proving the core is wired.

use project_canon_core::{Archetype, Model, Questionnaire};

fn main() {
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

    eprintln!(
        "project-canon {}: model loaded ({} base dimensions; cli profile covers {} canon sections).",
        env!("CARGO_PKG_VERSION"),
        model.base_members().len(),
        cli_sections,
    );
    eprintln!(
        "No verbs yet — `new`, `doctor`, and `review` are tracked as separate issues in this repo."
    );
}
