//! The repo-general **scaffold dimensions** — the base-canon members that come from what
//! homebase's `create-project` already sets up for *every* repo, regardless of archetype.
//!
//! These are declared **abstractly** — "the repo has a consolidated `AGENTS.md` with a
//! `CLAUDE.md` symlink", never a configured repository path or a `projects.conf` path. Probing an actual
//! repo for them (and resolving homebase env specifics: `tw`, the `gh` account, `.workmux`
//! emoji) is the later `env-config-hook-layer` issue's job. This crate hardcodes no filesystem
//! path, account, or host.

use crate::dimension::{
    Applicability, Dimension, DimensionSource, EffectClass, Layer, Probe, Severity,
};

/// The base-canon scaffold dimensions (all [`Layer::Base`], [`DimensionSource::Scaffold`]).
pub(crate) fn scaffold_dimensions() -> Vec<Dimension> {
    use Applicability::Always;
    use EffectClass::Static;
    use Severity::{Must, Should};

    let dim = |id, title, severity, signal, fail| Dimension {
        id,
        title,
        severity,
        applicability: Always,
        layer: Layer::Base,
        source: DimensionSource::Scaffold,
        probe: Probe {
            effect: Static,
            signal,
            // Scaffold dims are structural repo facts; the hint is a file-shape check, not a
            // `$TOOL` invocation. Real probing is deferred to env-config-hook-layer.
            command_hint: "inspect the repo tree",
            fail,
        },
    };

    vec![
        dim(
            "base.doc-pattern",
            "Consolidated AGENTS.md per directory, with CLAUDE.md a symlink to it",
            Must,
            "every directory has AGENTS.md (all AI-relevant info) and CLAUDE.md → AGENTS.md; complex topics split to AGENTS-<TOPIC>.md",
            "docs scattered across README-only or CLAUDE.md-as-file; no per-directory AGENTS.md",
        ),
        dim(
            "base.issue-tracking",
            "issuectl-managed issues under issues/<slug>/item.md",
            Must,
            "issuectl init done; flat issues/<slug>/item.md layout; status in frontmatter, not the path; planning docs under the parent issue",
            "issues tracked ad hoc; open/closed/ split dirs; numeric-prefixed slugs; standalone plan files",
        ),
        dim(
            "base.git-hygiene",
            "A git repo with main kept clean for parallel worktrees",
            Must,
            "initialized git repo; main is the integration branch; feature work happens on branches/worktrees",
            "no git repo; work committed directly to a dirty main that blocks parallel worktrees",
        ),
        dim(
            "base.readme",
            "A README.md front door",
            Should,
            "a top-level README.md describing the project's purpose and status",
            "no README, or an empty placeholder",
        ),
        dim(
            "base.gitignore",
            "gitignore covers build artifacts and the scratchpad",
            Should,
            "build artifacts (e.g. /target) and the history/ scratchpad are gitignored",
            "build output or ephemeral scratch dirs tracked in git",
        ),
    ]
}
