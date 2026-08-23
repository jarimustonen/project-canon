//! The `AGENTS-AI-FIRST-CLI.md` §1–§24 sections as [`Dimension`]s — the `cli` profile's
//! section-set, plus the repo-general canon sections routed to base.
//!
//! Each entry is a **reference**, not a re-copy: the compact [`Probe`] is lifted from
//! `cli-canon`'s `conformance-probes.md`, while the full prose stays in the canon and is
//! reachable via [`DimensionSource::Canon`]`{ section }`. §N is a stable citation surface — the
//! numbers here are never renumbered.
//!
//! **Layer routing.** §10 (schema/versioning contract), §15–§17 (companion-skill
//! install/print/sync), and §22 (internal `core`/`cli` layout) are repo-general — homebase's
//! `create-project` applies them to every repo — so they are rooted in [`Layer::Base`]. §23 is
//! likewise repo-general because it governs every publicly distributed artifact, and §24 governs
//! tracked deferrals in every project. The remaining 17 sections shape the CLI surface and are
//! rooted in [`Layer::Profile`]`(Cli)`. Their union is the full §1–§24 (asserted by test).

use crate::dimension::{
    Applicability, Archetype, Dimension, DimensionSource, EffectClass, Layer, Probe, Severity,
};
use crate::questionnaire::Question;

/// The canon sections routed to the base layer (repo-general, archetype-invariant).
const BASE_SECTIONS: [u8; 7] = [10, 15, 16, 17, 22, 23, 24];

/// True if §N is routed to base (vs. the `cli` profile).
pub(crate) fn is_base_section(section: u8) -> bool {
    BASE_SECTIONS.contains(&section)
}

/// The stable registry id for canon §N, e.g. `"canon.s10"`.
///
/// Section numbers are **zero-padded** (`canon.s01`..`canon.s24`) so that lexicographic id
/// order — the order the `BTreeMap` registry and every sorted id list use — equals canon
/// section order. Without the pad, `canon.s10` sorts before `canon.s2`, scrambling downstream
/// doctor/review listings.
pub(crate) const fn canon_id(section: u8) -> &'static str {
    // A small const lookup keeps the ids `&'static str` without allocation.
    match section {
        1 => "canon.s01",
        2 => "canon.s02",
        3 => "canon.s03",
        4 => "canon.s04",
        5 => "canon.s05",
        6 => "canon.s06",
        7 => "canon.s07",
        8 => "canon.s08",
        9 => "canon.s09",
        10 => "canon.s10",
        11 => "canon.s11",
        12 => "canon.s12",
        13 => "canon.s13",
        14 => "canon.s14",
        15 => "canon.s15",
        16 => "canon.s16",
        17 => "canon.s17",
        18 => "canon.s18",
        19 => "canon.s19",
        20 => "canon.s20",
        21 => "canon.s21",
        22 => "canon.s22",
        23 => "canon.s23",
        24 => "canon.s24",
        // §N is 1..=24 (a closed citation surface). A section outside that range is a
        // programming error at edit time, not a runtime input — fail loudly rather than
        // aliasing to a bogus shared id that would collide in the registry.
        _ => panic!("canon section out of the 1..=24 range"),
    }
}

/// Layer for §N: base for the repo-general sections, else the `cli` profile.
fn section_layer(section: u8) -> Layer {
    if is_base_section(section) {
        Layer::Base
    } else {
        Layer::Profile(Archetype::Cli)
    }
}

/// Assemble one canon §N dimension.
fn section(
    section: u8,
    title: &'static str,
    severity: Severity,
    applicability: Applicability,
    probe: Probe,
) -> Dimension {
    Dimension {
        id: canon_id(section),
        title,
        severity,
        applicability,
        layer: section_layer(section),
        source: DimensionSource::Canon { section },
        probe,
    }
}

/// All §1–§24 canon sections as dimensions.
pub(crate) fn canon_dimensions() -> Vec<Dimension> {
    use Applicability::{Always, Conditional};
    use EffectClass::{ExecRo, SandboxWrite, Static};
    use Severity::{Must, MustWhenApplies, Should};

    vec![
        section(
            1,
            "Strict input validation — no silent fixups",
            Must,
            Always,
            Probe {
                effect: ExecRo,
                signal: "empty/whitespace/unknown-flag/out-of-range → error echoing the bad value",
                command_hint: "$TOOL <cmd> \"\"  ·  $TOOL <cmd> --no-such-flag",
                fail: "silent trim/default/coerce; ignored unknown flag; message omits the value",
            },
        ),
        section(
            2,
            "Structured, parseable output + exit codes + JSONL logs",
            Must,
            Always,
            Probe {
                effect: ExecRo,
                signal: "global --json; data→stdout, errors→stderr; one central exit map (0/1/2/130/143); JSONL logs",
                command_hint: "$TOOL <cmd> --json | jq .  ·  $TOOL bogus-subcmd; echo $?",
                fail: "every error collapsed to one code; 2 for a caller-actionable not_found; prose in --json",
            },
        ),
        section(
            3,
            "No interactive prompts",
            Must,
            Always,
            Probe {
                effect: ExecRo,
                signal: "no y/N confirms, no TTY-dependence, no pager/$EDITOR; destructive gated by --force/--yes",
                command_hint: "$TOOL <ro-cmd> </dev/null  ·  grep read_line|prompt|confirm",
                fail: "a Y/N prompt; a spawned pager or editor; a hang waiting on stdin",
            },
        ),
        section(
            4,
            "Informative error messages",
            Must,
            Always,
            Probe {
                effect: ExecRo,
                signal: "error carries the actual invalid value AND the expected set/format; names the failing step",
                command_hint: "trigger each error path; read the message",
                fail: "\"invalid input\" with no value and no expected set",
            },
        ),
        section(
            5,
            "Composable commands",
            Must,
            Always,
            Probe {
                effect: ExecRo,
                signal: "reads write stdout by default; --output FILE; - as stdin; consistent flag names across subcommands",
                command_hint: "$TOOL <fetch> | head  ·  echo … | $TOOL <ro-cmd> -",
                fail: "a fetch that only writes a file; per-subcommand flag drift",
            },
        ),
        section(
            6,
            "CLI surface: noun-verb imperative, apply opt-in",
            Must,
            // Applies *always* (shape of the whole surface). Q1 selects the shape (noun-verb
            // vs. flat), it does not switch §6 on/off — see resolve.rs and the design doc.
            Always,
            Probe {
                effect: Static,
                signal: "resource-first for multi-resource tools; flat verb only for single-resource; apply -f additional-only",
                command_hint: "$TOOL --help; inspect the subcommand tree",
                fail: "apply-only surface for an imperative op; a noun layer for a one-resource CLI",
            },
        ),
        section(
            7,
            "Subcommand verbs: one set, no synonyms",
            Must,
            Always,
            Probe {
                effect: Static,
                signal: "exactly list/show/create/update/delete + the two closed exception sets; update is selective-patch",
                command_hint: "$TOOL --help; grep for ls|get|new|add|edit|set|patch|rm|remove|destroy",
                fail: "a synonym (get/new/rm); a domain word masking a plain update; one verb for list-many and show-one",
            },
        ),
        section(
            8,
            "Config precedence + inspectable config and data root",
            MustWhenApplies,
            Conditional(Question::Q2ConfigOrDataRoot),
            Probe {
                effect: ExecRo,
                signal: "per-key flag>env>file>default; env mirrors flag; config path/show mandatory; secrets redacted; --home 5-layer",
                command_hint: "$TOOL config path  ·  $TOOL config show --json | jq",
                fail: "no config path/show; secrets printed; a --repo/--home synonym pair; silent operate-on-cwd",
            },
        ),
        section(
            9,
            "Output format is fixed, not TTY-detected",
            Must,
            Always,
            Probe {
                effect: ExecRo,
                signal: "format set only by --json/--output; identical bytes piped vs. tty; color off, no --color=auto",
                command_hint: "$TOOL <cmd> | cat vs. in a terminal — bytes must match; grep isatty|is_terminal",
                fail: "table-vs-line or color switching on isatty(); auto-pagination",
            },
        ),
        section(
            10,
            "Schema versioning, errors, warnings, deprecation, provenance",
            Must,
            Always,
            Probe {
                effect: ExecRo,
                signal: "every --json carries schema_version; top-level version/--version are byte-identical full aliases in text and JSON, with --json accepted in either alias order; commit is 40-hex or null+build_provenance; error envelope on stderr",
                command_hint: "compare stdout/stderr/exit: $TOOL version vs --version, then version --json vs --version --json vs --json --version  ·  inspect JSON with jq",
                fail: "version/--version output or exit drift; commit:\"unknown\"; missing supported_schemas; a private JSON toggle; warnings on stderr under --json",
            },
        ),
        section(
            11,
            "Dry-run, idempotency, retry safety",
            MustWhenApplies,
            Conditional(Question::Q3Mutates),
            Probe {
                effect: SandboxWrite,
                signal: "--dry-run emits the planning envelope, never partially applies; dry_run_unsupported if impossible; a convergence affordance",
                command_hint: "$TOOL <mut> --dry-run --json | jq  ·  (sandbox) create twice with one --idempotency-key",
                fail: "no --dry-run on a mutating command; a fake dry-run that writes; dry-run envelope == real envelope",
            },
        ),
        section(
            12,
            "Long-running: streaming events, progress query, signals",
            MustWhenApplies,
            Conditional(Question::Q4LongRunning),
            Probe {
                effect: SandboxWrite,
                signal: "--output=jsonl one event/line with one terminal event; paired show/status; SIGINT/SIGTERM → cancelled, exit 130/143; no spinners",
                command_hint: "(sandbox) $TOOL <long> --output=jsonl | jq -c .  ·  SIGTERM the child PID",
                fail: "a spinner/progress bar; format switching by elapsed time; no progress query for a detached job",
            },
        ),
        section(
            13,
            "Large outputs go to a queryable file",
            // SHOULD in general, MUST when results can be large — modelled as MustWhenApplies
            // gated on Q8 (the "results can be large" trigger).
            MustWhenApplies,
            Conditional(Question::Q8LargeResults),
            Probe {
                effect: ExecRo,
                signal: "--output FILE.jsonl or FILE.db; stdout prints only file metadata; --limit is a guardrail, not the paging mechanism",
                command_hint: "$TOOL list --output \"$sandbox/x.jsonl\"; wc -l / jq",
                fail: "10k rows dumped inline; cursor-paging as the only escape hatch",
            },
        ),
        section(
            14,
            "--help is agent-first, structured, drill-down",
            Must,
            Always,
            Probe {
                effect: ExecRo,
                signal: "top-level lists subcommands + small global-flag set; <sub> --help drills down; --help --json with examples[]",
                command_hint: "$TOOL --help  ·  $TOOL <sub> --help --json | jq",
                fail: "a top-level flag firehose; no machine-readable help (SHOULD-strong family gap)",
            },
        ),
        section(
            15,
            "skill subcommand: install companion AI-skills",
            Must,
            Always,
            Probe {
                // `skill install` mutates state (writes skill files) — conformance-probes.md
                // tags it [sandbox-write]. The coarse single effect-class per section takes the
                // most-dangerous member, so a review runner sandboxes the whole §15 probe.
                effect: SandboxWrite,
                signal: "skill list; skill install [<name>] into ~/.claude/skills by default, --target <dir>; skills live in-repo; every frontmatter description is at most 1024 characters",
                command_hint: "$TOOL skill list  ·  inspect located SKILL.md descriptions  ·  (sandbox) $TOOL skill install --target \"$sandbox/skills\"",
                fail: "no skill door; a skill list referencing a removed flag; a frontmatter description over 1024 characters",
            },
        ),
        section(
            16,
            "skill print: stream skill content, no side effects",
            Must,
            Always,
            Probe {
                effect: ExecRo,
                signal: "skill print <name> → SKILL.md byte-identical to install; --json structured payload; unknown → §10 envelope; no writes/network",
                command_hint: "$TOOL skill print <name> | head  ·  $TOOL skill print <name> --json | jq keys",
                fail: "a rendered-vs-raw distinction; a side effect on print",
            },
        ),
        section(
            17,
            "Skill–CLI version synchronization",
            Must,
            Always,
            Probe {
                effect: Static,
                signal: "SKILL.md frontmatter has cli_version + schema_version; skill print pinned to running binary; version --json exposes skills[]",
                command_hint: "$TOOL version --json | jq .skills  ·  inspect a shipped SKILL.md frontmatter",
                fail: "skill frontmatter without cli_version; skill print served from a stale copy",
            },
        ),
        section(
            18,
            "doctor: read-only self-diagnostic",
            Must,
            Always,
            Probe {
                effect: ExecRo,
                signal: "doctor one line/check OK/WARN/FAIL + summary; --json structured; exit 0 unless a FAIL; read-only by default, --fix opt-in",
                command_hint: "$TOOL doctor --json | jq '.checks[].id, .summary'",
                fail: "no doctor; a doctor that mutates state by default",
            },
        ),
        section(
            19,
            "Deterministic clock: inject time, never read it ad hoc",
            MustWhenApplies,
            Conditional(Question::Q5Timestamps),
            Probe {
                effect: Static,
                signal: "a single hidden global --frozen-time; an injected Clock/FakeClock seam in core; golden/byte-stable fixtures",
                command_hint: "$TOOL <cmd> --help --json | jq '..|.name?' | grep frozen-time; grep Utc::now vs a Clock trait",
                fail: "ad-hoc now() in domain logic; a --now/--frozen-time synonym pair; forging a record's provenance",
            },
        ),
        section(
            20,
            "fmt: idempotent canonicalizer for on-disk records",
            MustWhenApplies,
            Conditional(Question::Q6Records),
            Probe {
                effect: SandboxWrite,
                signal: "fmt rewrites to canonical form, idempotent; sorts only order-insensitive arrays; never bumps updated:; atomic; --strict CI mode",
                command_hint: "$TOOL fmt --dry-run --json  ·  (sandbox) fmt twice, hash tree before/after",
                fail: "a fmt that bumps updated:; non-idempotent output; reordering a semantic array",
            },
        ),
        section(
            21,
            "init: idempotent, no-clobber bootstrap",
            MustWhenApplies,
            Conditional(Question::Q7Scaffolds),
            Probe {
                effect: SandboxWrite,
                signal: "init writes the .<tool>/ marker + scaffold; home resolved explicitly; skill install not implicit; idempotent fill-in; --force spares records",
                command_hint: "$TOOL init --dry-run --json  ·  (sandbox) init twice into a fresh empty dir",
                fail: "an init that clobbers/resets a home; a silent global skill write; a scaffold into a surprising cwd",
            },
        ),
        section(
            22,
            "Internal layout: library-first core/cli split",
            Should,
            Always,
            Probe {
                effect: Static,
                signal: "crates/<tool>-core (pure domain, no clap/I/O) + <tool>-cli; injected Clock in core; shared plumbing in a thin <family>-cli-common",
                command_hint: "read Cargo.toml members; grep core for clap/I/O imports",
                fail: "never a readiness failure; report as a SHOULD recommendation only",
            },
        ),
        section(
            23,
            "Public artifacts contain no user-specific facts",
            Must,
            Always,
            Probe {
                effect: Static,
                signal: "public shipped defaults/templates/skills are neutral; configured private markers absent; own public coordinates exempt; judgment remainder reviewed",
                command_hint: "$TOOL doctor --json | jq '.checks[] | select(.id == \"canon.s23\")'  ·  review hostnames/internal URLs by hand",
                fail: "a configured private marker in distributed text; a maintainer path/account/private repo shipped as a default, fixture, scaffold, or skill",
            },
        ),
        section(
            24,
            "A stated blocker is re-verified, never inherited",
            Must,
            Always,
            Probe {
                effect: Static,
                signal: "deferral issue slugs resolve to open local issues; stated credentials/permissions/dependencies re-verified",
                command_hint: "$TOOL doctor --json | jq '.checks[] | select(.id == \"canon.s24\")'  ·  inspect each remaining blocker by hand",
                fail: "a missing/closed owning issue; a cross-repo issue with no open local owner; an inherited blocker premise with no current evidence",
            },
        ),
    ]
}
