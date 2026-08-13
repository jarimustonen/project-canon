//! # project-canon-core
//!
//! The two-layer conformance model for project-canon (ADR 0009 §1/§4/§6):
//!
//! ```text
//! resolved(repo) = BASE CANON  ∪  PROFILE[archetype-of-repo]
//!                  (repo-invariant)   (archetype-specific, additive)
//! ```
//!
//! - A [`Dimension`](dimension::Dimension) is the unit of conformance — a canon §N section or a
//!   create-project scaffold requirement, unified so base and profiles are plain id-sets.
//! - The **base canon** holds the repo-invariant dimensions: the repo-general canon sections
//!   (§10, §15–§17, §22) plus the create-project scaffold dims.
//! - A [`Profile`](profile::Profile) is a named section-set + a probe registry. Only
//!   [`Archetype::Cli`](dimension::Archetype::Cli) has content at v0 (the `AGENTS-AI-FIRST-CLI.md`
//!   §1–§22 lift); `service`/`library`/`release` are named-but-empty extension points.
//! - The [`Questionnaire`](questionnaire::Questionnaire) *characterizes* a repo (archetype + the
//!   eight yes/no questions, mirrored from `cli-canon`) and [`Model::resolve`](profile::Model::resolve)
//!   turns it into a [`Resolution`](resolve::Resolution).
//!
//! Orthogonal to the model is the [`env`] config/hook layer ([`EnvConfig`]): the non-portable
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
