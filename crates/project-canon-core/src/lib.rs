//! # project-canon-core
//!
//! The two-layer conformance model for project-canon (ADR 0009 §1/§4/§6):
//!
//! ```text
//! resolved(repo) = BASE CANON  ∪  PROFILE[archetype-of-repo]
//!                  (repo-invariant)   (archetype-specific, additive)
//! ```
//!
//! - A [`Dimension`] is the unit of conformance — a canon §N section or a create-project scaffold
//!   requirement, unified so base and profiles are plain id-sets.
//! - The **base canon** holds the repo-invariant dimensions: the repo-general canon sections
//!   (§10, §15–§17, §22) plus the create-project scaffold dims.
//! - A [`Profile`] is a named section-set + a probe registry. Only [`Archetype::Cli`] has content
//!   at v0 (the `AGENTS-AI-FIRST-CLI.md` §1–§22 lift); `service`/`library`/`release` are
//!   named-but-empty extension points.
//! - The [`Questionnaire`] *characterizes* a repo (archetype + the eight yes/no questions,
//!   mirrored from `cli-canon`) and [`Model::resolve`](profile::Model::resolve) turns it into a
//!   [`Resolution`].
//!
//! Orthogonal to the model is the [`mod@env`] config/hook layer ([`EnvConfig`]): the non-portable
//! homebase environment specifics (family-repo map, gh account, `~/Sources/<name>` convention,
//! tw registration, `.workmux.yaml` prefix) resolved through *defaults → config file → env
//! overrides*, with sensible overridable defaults. A verb reads both a [`Model`] and an
//! `EnvConfig`; the layer does not change the model's `BASE ∪ PROFILE` semantics.
//!
//! This crate is dependency-light and free of clap/I/O (§22 core/cli split); JSON serialization
//! of a `Resolution` (§10, for the future `doctor`/`review` verbs) is a deferred seam.

mod canon;
pub mod dimension;
pub mod env;
pub mod profile;
pub mod questionnaire;
pub mod resolve;
pub mod routing;
mod scaffold;

pub use dimension::{
    Applicability, Archetype, Dimension, DimensionSource, EffectClass, Layer, Probe, Severity,
};
pub use env::{CiReleaseHook, EnvConfig, EnvConfigError, EnvConfigLayer, TwRegistration};
pub use profile::{Model, Profile};
pub use questionnaire::{Question, Questionnaire};
pub use resolve::{AppStatus, Resolution, ResolvedDimension, SurfaceShape};
pub use routing::{suggested_layer, Breadth};

/// The `AGENTS-AI-FIRST-CLI.md` canon, bundled verbatim as the single source of truth
/// (ADR 0009 §6). The physical master lives beside this crate
/// (`crates/project-canon-core/AGENTS-AI-FIRST-CLI.md`) so it packages *inside* core and ships on
/// crates.io; the repo-root `AGENTS-AI-FIRST-CLI.md` is a symlink to it for external consumers.
/// Every canon consumer (the `new` scaffolder, the `skill` installer, both here and in the CLI)
/// reads exactly these bytes, so no second copy can drift.
pub const CANON: &str = include_str!("../AGENTS-AI-FIRST-CLI.md");

#[cfg(test)]
mod canon_tests {
    /// The one physical copy of the canon lives *inside this crate* as a regular file (never a
    /// symlink — the repo-root path is the symlink, not this one) and is exactly what
    /// [`super::CANON`] embeds. Guards the packaging invariant: if someone moved the master back to
    /// the repo root and left a symlink here, or edited the file out from under `include_str!`,
    /// this fails. Self-contained — the file ships in the core tarball, so it holds downstream too.
    #[test]
    fn canon_master_is_a_regular_file_inside_the_crate() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/AGENTS-AI-FIRST-CLI.md");
        let meta = std::fs::symlink_metadata(path).expect("canon master must exist in the crate");
        assert!(
            meta.file_type().is_file(),
            "the canon master in project-canon-core must be a regular file, not a symlink"
        );
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            super::CANON,
            "the packaged canon file must be byte-identical to the embedded CANON const"
        );
    }
}
