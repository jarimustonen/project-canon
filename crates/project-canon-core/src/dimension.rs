//! The unit of conformance: a [`Dimension`].
//!
//! Everything resolvable in the two-layer model — a canon §N section or a create-project
//! scaffold requirement — is a `Dimension` with a stable id, a severity, an applicability
//! rule, a layer, a source, and a machine-shaped probe. Unifying canon sections and scaffold
//! requirements under one type lets the base canon and the profiles be plain sets of
//! dimension ids (see [`crate::profile`]).
//!
//! We store the compact *probe* here, never the section's multi-paragraph prose — the prose
//! lives in `AGENTS-AI-FIRST-CLI.md`, referenced by [`DimensionSource::Canon`]. This is the
//! "reference/registry, not a re-copy" rule from the issue's scope discipline.

use crate::questionnaire::Question;

/// The conformance archetype a repo is measured as. Only [`Archetype::Cli`] carries content at
/// v0; the rest are named-but-empty extension points (see [`crate::profile::Profile`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Archetype {
    /// AI-first command-line tool — the `AGENTS-AI-FIRST-CLI.md` §1–§24 surface.
    Cli,
    /// Long-running / hosted service. Empty extension point at v0.
    Service,
    /// Reusable library crate. Empty extension point at v0.
    Library,
    /// Release-artifact / distribution repo. Empty extension point at v0.
    Release,
}

impl Archetype {
    /// Every archetype, in a stable order. Useful for exhaustiveness checks and iteration.
    pub const ALL: [Archetype; 4] = [
        Archetype::Cli,
        Archetype::Service,
        Archetype::Library,
        Archetype::Release,
    ];

    /// The stable, lowercase token used on the CLI surface (`--archetype cli`) and in ids.
    pub fn slug(self) -> &'static str {
        match self {
            Archetype::Cli => "cli",
            Archetype::Service => "service",
            Archetype::Library => "library",
            Archetype::Release => "release",
        }
    }
}

/// Which conformance layer a dimension is *rooted* in.
///
/// Base dims are repo-invariant; profile dims belong to one archetype's surface. Note this is
/// where a dimension is rooted, not the only place it is *cited*: the `cli` profile's declared
/// section-set is its profile-rooted members, and the base layer independently contributes the
/// repo-general canon sections (§10, §15–§17, §22–§24), so their union is the full §1–§24.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Applies to every project-canon-managed repo regardless of archetype.
    Base,
    /// Rooted in a single archetype's profile.
    Profile(Archetype),
}

/// The origin of a dimension — kept inspectable so a later `doctor`/`review` can explain *why*
/// a dimension is in scope (canon citation vs. scaffold obligation vs. discovered practice).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionSource {
    /// A section of `AGENTS-AI-FIRST-CLI.md`. `section` is the stable §N citation number.
    Canon { section: u8 },
    /// A requirement homebase's `create-project` scaffolds for every repo it makes.
    Scaffold,
    /// A practice admitted via dimension-discovery (cleared the ≥2 recurrence bar).
    Discovered,
}

/// The canon's three-tier severity model (see `AGENTS-AI-FIRST-CLI.md` preamble and
/// `cli-canon`'s `conformance-probes.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Unconditional agent-facing surface. Absence is a conformance failure.
    Must,
    /// A hard gate, but only when the dimension's `Applies-when` holds.
    MustWhenApplies,
    /// A strong convention, never a hard readiness gate.
    Should,
}

/// When a dimension is in scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applicability {
    /// Always in scope for any repo of the owning layer/profile.
    Always,
    /// In scope only when the questionnaire answers `question = true`.
    Conditional(Question),
}

impl Applicability {
    /// The gating question, if this dimension is conditional.
    pub fn question(self) -> Option<Question> {
        match self {
            Applicability::Always => None,
            Applicability::Conditional(q) => Some(q),
        }
    }
}

/// The safety class of a probe, lifted from `conformance-probes.md`. Governs whether observing
/// the dimension may mutate state — consumed by the later `review` verb, declared here so the
/// probe registry carries it from the start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectClass {
    /// Reads source/manifests only.
    Static,
    /// Runs a read-only verb (`list`/`show`/`version`/`--help`/`doctor`/`config show`/…).
    ExecRo,
    /// Mutates state — must run only against an isolated scratch `--home`.
    SandboxWrite,
}

/// The machine-shaped probe descriptor for a dimension — the compact form of a
/// `conformance-probes.md` entry. Illustrative, not executed here: the `command_hint` is a
/// `$TOOL`-shaped hint (no real path/host), and running it is the later `review` verb's job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    /// Safety class governing how the probe may be run.
    pub effect: EffectClass,
    /// The conformant shape to look for.
    pub signal: &'static str,
    /// A `$TOOL`-shaped hint for how to observe it. Never executed in this crate.
    pub command_hint: &'static str,
    /// The anti-pattern that constitutes a failure.
    pub fail: &'static str,
}

/// A single conformance requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimension {
    /// Stable registry key (e.g. `"canon.s10"`, `"base.doc-pattern"`). Never renumbered.
    pub id: &'static str,
    /// One-line human title.
    pub title: &'static str,
    /// Severity tier.
    pub severity: Severity,
    /// When it is in scope.
    pub applicability: Applicability,
    /// The layer it is rooted in.
    pub layer: Layer,
    /// Where it came from.
    pub source: DimensionSource,
    /// The machine-shaped probe descriptor.
    pub probe: Probe,
}

impl Dimension {
    /// The canon §N number, if this dimension is a canon section.
    pub fn canon_section(&self) -> Option<u8> {
        match self.source {
            DimensionSource::Canon { section } => Some(section),
            _ => None,
        }
    }
}
