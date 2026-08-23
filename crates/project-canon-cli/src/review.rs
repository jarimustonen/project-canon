//! The `review` verb — the **advisory, human-facing conformance audit**.
//!
//! Where [`doctor`](crate::doctor) is a mechanical pass/fail *gate*, `review` is the recommending
//! *audit*: it reads the same two-layer model + the shared mechanical-probe substrate
//! ([`crate::probes`]), triages every in-scope dimension by the canon's **severity model**, and
//! emits severity-ranked **findings** plus **staged** `issuectl` commands and dimension-discovery
//! pointers. It folds in `cli-canon`'s *review* mode (`skills/cli-canon/templates/review-report.md`).
//!
//! **Critical contract (ADR 0009 §2): recommend & stage, NEVER act.** review makes **no** change
//! to the target repo (no writes) and files **no** issue. Every side-effecting suggestion is
//! *rendered as a shell-safe command string and printed* for a human/agent to run — review never
//! shells out and never executes what it stages. A conformance outcome never flips the exit code
//! (advisory, not a gate — see the design note's exit-code contract); only a usage/operational
//! error exits non-zero.
//!
//! Verb logic lives at the CLI edge (not the I/O-free core) because probing reads the filesystem.
//! The core model is consumed unchanged.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use project_canon_core::{
    AppStatus, Archetype, Dimension, EffectClass, Layer, Model, Questionnaire, Resolution,
    Severity, SurfaceShape,
};

use crate::error::{fail, json_requested, write_stdout, CliError};
use crate::json::Json;
use crate::probes::{
    mechanical_probe, runtime_probes, ProbeContext, RuntimeProbeOutcome, RuntimeProbeStatus,
    RUNTIME_TIMEOUT_MS,
};
use crate::shell::shell_quote;

/// The `--json` payload schema version (§10). Bump on any breaking shape change.
const SCHEMA_VERSION: i64 = 1;

// ===== exit codes (see design.md "Exit-code contract — advisory, NOT a gate") =======
/// The report was produced — **regardless of how many gaps were found** (advisory).
const EXIT_OK: u8 = 0;

/// Run `project-canon review <args…>` (the args *after* the `review` subcommand). Owns all of
/// review's I/O and returns the process exit code.
pub fn run(args: &[String]) -> ExitCode {
    // Parse first so `--help` is always an exit-0 event (§2), even under a malformed environment.
    let parsed = match parse_args(args) {
        Ok(Command::Help) => {
            print!("{HELP}");
            return ExitCode::from(EXIT_OK); // §2: help is an exit-0 event.
        }
        Ok(Command::Run(a)) => a,
        Err(err) => {
            return fail(
                json_requested(args),
                CliError::actionable("usage_error", format!("review: {err}")),
            );
        }
    };

    // Resolve the §8 user-config layer because §23's deny-list is operator knowledge, never a
    // shipped default. Help remains independent of configuration.
    let env_config = match crate::config::resolve() {
        Ok(config) => config,
        Err(error) => return fail(parsed.json, error.into_cli()),
    };

    // Read-only target validation (§1): the repo must be an existing directory.
    let repo = Path::new(&parsed.repo);
    if !repo.is_dir() {
        return fail(
            parsed.json,
            CliError::actionable(
                "invalid_target",
                format!("review: target repo is not a directory: {:?}", parsed.repo),
            ),
        );
    }
    // Absolute, symlink-resolved path for the report's `target` field and the staged command's
    // `cd`. A failure here (a race or permission error after the `is_dir` check) is operational.
    let target = match std::fs::canonicalize(repo) {
        Ok(p) => p.display().to_string(),
        Err(err) => {
            return fail(
                parsed.json,
                CliError::system(
                    "io_error",
                    format!("review: cannot resolve target {:?}: {err}", parsed.repo),
                ),
            );
        }
    };

    // Resolve the model with the conservative questionnaire (all conditionals off — §3, review
    // never prompts). `--assume-defaults` names this explicitly; it is the only mode at v0.
    let model = Model::standard();
    let questionnaire = Questionnaire::builder(parsed.profile).build();
    let resolution = model.resolve(&questionnaire);

    // An operational I/O fault while probing (permission denied, transient error, target vanished)
    // means review could not evaluate a dimension → exit 2, never a fabricated finding.
    let probe_context = ProbeContext {
        user_specific_deny_list: &env_config.user_specific_deny_list,
    };
    // Runtime execution is strictly opt-in. Resolve a relative --run value against review's own
    // working directory before children use the audited repo as cwd; a missing/non-executable
    // path remains a reported could-not-probe outcome rather than aborting the advisory report.
    let runtime_binary = match parsed.run.as_deref() {
        Some(binary) => match absolute_lexical(binary) {
            Ok(path) => Some(path),
            Err(error) => {
                return fail(
                    parsed.json,
                    CliError::system(
                        "io_error",
                        format!(
                            "review: cannot resolve --run path against current directory: {error}"
                        ),
                    ),
                )
            }
        },
        None => None,
    };
    let runtime = runtime_binary
        .as_deref()
        .map(|binary| runtime_probes(binary, Path::new(&target)))
        .unwrap_or_default();
    let report = match build_report(
        &model,
        &resolution,
        parsed.profile,
        &target,
        &probe_context,
        runtime_binary.as_deref(),
        &runtime,
    ) {
        Ok(r) => r,
        Err(fault) => {
            return fail(
                parsed.json,
                CliError::system(
                    "io_error",
                    format!(
                        "review: cannot probe {} ({}): {}",
                        fault.dim_id, target, fault.source
                    ),
                ),
            );
        }
    };

    let output = if parsed.json {
        format!("{}\n", report.to_json())
    } else {
        report.render_human(parsed.verbose)
    };
    let write_status = write_stdout(&output, parsed.json);
    if write_status != ExitCode::SUCCESS {
        return write_status;
    }
    // Advisory: the exit code never reflects the conformance outcome. Only usage/operational
    // errors (handled above) exit non-zero.
    ExitCode::from(EXIT_OK)
}

// ===== argument parsing =============================================================

/// A parsed review invocation. (Mirrors doctor's surface — the two verbs share a shape.)
#[derive(Debug, PartialEq, Eq)]
struct ReviewArgs {
    repo: String,
    profile: Archetype,
    json: bool,
    verbose: bool,
    #[allow(dead_code)] // Accepted & validated; a no-op affirmation of the static default mode.
    assume_defaults: bool,
    run: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Run(ReviewArgs),
}

/// Strict argument parsing (§1): unknown flags, bad `--profile`, missing values, repeated flags,
/// and extra positionals are all errors echoing the offending token — never a silent fixup.
fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut repo: Option<String> = None;
    let mut profile: Option<Archetype> = None;
    let mut json = false;
    let mut verbose = false;
    let mut assume_defaults = false;
    let mut run: Option<String> = None;

    // `--` stops flag parsing so a repo path beginning with `-` can still be addressed.
    let mut positional_only = false;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if positional_only {
            set_positional(&mut repo, arg)?;
            continue;
        }
        if arg == "--" {
            positional_only = true;
            continue;
        }
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v)),
            _ => (arg.as_str(), None),
        };
        match flag {
            "--help" => {
                reject_inline("--help", inline)?;
                return Ok(Command::Help);
            }
            "--json" => {
                reject_inline("--json", inline)?;
                set_flag(&mut json, "--json")?;
            }
            "--verbose" => {
                reject_inline("--verbose", inline)?;
                set_flag(&mut verbose, "--verbose")?;
            }
            "--assume-defaults" => {
                reject_inline("--assume-defaults", inline)?;
                set_flag(&mut assume_defaults, "--assume-defaults")?;
            }
            "--run" => {
                if run.is_some() {
                    return Err("repeated flag: --run".to_string());
                }
                let value = match inline {
                    Some(value) if !value.is_empty() => value.to_string(),
                    Some(_) => return Err("--run requires a non-empty path".to_string()),
                    None => {
                        let value = iter
                            .next()
                            .cloned()
                            .ok_or_else(|| "--run requires a path to a binary".to_string())?;
                        if value.starts_with('-') {
                            return Err(format!(
                                "--run requires a path, got flag-like token {value:?}"
                            ));
                        }
                        value
                    }
                };
                run = Some(value);
            }
            "--profile" => {
                if profile.is_some() {
                    return Err("repeated flag: --profile".to_string());
                }
                let value = match inline {
                    Some(v) => v.to_string(),
                    None => {
                        let value = iter.next().cloned().ok_or_else(|| {
                            "--profile requires a value (cli/service/library/release)".to_string()
                        })?;
                        if value.starts_with('-') {
                            return Err(format!(
                                "--profile requires a value, got flag-like token {value:?}"
                            ));
                        }
                        value
                    }
                };
                profile = Some(parse_archetype(&value)?);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag: {other}"));
            }
            _ => set_positional(&mut repo, arg)?,
        }
    }

    if assume_defaults && run.is_some() {
        return Err(
            "--assume-defaults is static-only and cannot be combined with --run".to_string(),
        );
    }

    let profile = profile.unwrap_or(Archetype::Cli);
    if run.is_some() && profile != Archetype::Cli {
        return Err(format!(
            "--run probes CLI surfaces and requires --profile cli (got {:?})",
            profile.slug()
        ));
    }

    Ok(Command::Run(ReviewArgs {
        repo: repo.unwrap_or_else(|| ".".to_string()),
        profile,
        json,
        verbose,
        assume_defaults,
        run,
    }))
}

/// Record the single positional `<repo>` argument, rejecting a second one (§1).
fn set_positional(repo: &mut Option<String>, arg: &str) -> Result<(), String> {
    if repo.is_some() {
        return Err(format!("unexpected extra argument: {arg:?}"));
    }
    *repo = Some(arg.to_string());
    Ok(())
}

/// Reject an inline `=value` on a valueless flag (§1: no silently-discarded value).
fn reject_inline(flag: &str, inline: Option<&str>) -> Result<(), String> {
    match inline {
        Some(value) => Err(format!("flag {flag} does not take a value (got {value:?})")),
        None => Ok(()),
    }
}

/// Set a boolean flag, rejecting a repeat (§1: no silent last-wins).
fn set_flag(slot: &mut bool, name: &str) -> Result<(), String> {
    if *slot {
        return Err(format!("repeated flag: {name}"));
    }
    *slot = true;
    Ok(())
}

/// Parse an archetype slug, echoing the bad value and the valid set on failure (§4).
fn parse_archetype(s: &str) -> Result<Archetype, String> {
    Archetype::ALL
        .into_iter()
        .find(|a| a.slug() == s)
        .ok_or_else(|| {
            let valid = Archetype::ALL
                .iter()
                .map(|a| a.slug())
                .collect::<Vec<_>>()
                .join("/");
            format!("invalid --profile {s:?} (expected one of {valid})")
        })
}

const HELP: &str = "\
project-canon review — recommending conformance audit (advisory; recommends & stages, never acts)

USAGE:
    project-canon review [--profile <archetype>] [--assume-defaults | --run <binary>] [--json] [--verbose] [<repo>]

ARGS:
    <repo>                  Target repo to audit (default: current directory). Read-only.

FLAGS:
    --profile <archetype>   cli | service | library | release  (default: cli)
    --assume-defaults       Static-only conservative characterization; never executes a target.
    --run <binary>          Opt in to read-only runtime probes of this explicitly named binary.
    --json                  Emit the structured §10 report on stdout.
    --verbose               Also list manual-verify coverage notes, passing, and n/a rows.
    --help                  Show this help.

SIDE EFFECTS:
    review NEVER edits the target repo and NEVER files an issue. Without --run it executes nothing.
    With --run it invokes only the named binary, directly (no shell), with read-only probe arguments,
    captured output, null stdin, and a per-invocation timeout. Every issue command is only PRINTED.

EXIT CODES:
    0   review ran and produced its report — regardless of how many gaps it found (advisory)
    2   usage/operational error (bad flag, bad --profile, missing target, I/O fault, malformed env)
";

// ===== the report ===================================================================

/// A finding's kind: an evidence-backed mechanical gap, or a verify-by-hand coverage note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FindingKind {
    /// A mechanical probe exists and **failed** — a real gap with evidence. Gets a staged command.
    ConfirmedGap,
    /// The dimension applies but has no mechanical probe (a behavioral section). The review-mode
    /// `unknown`: a manual-verify coverage note, carrying the probe's how-to. **Never staged.**
    ManualVerify,
    /// An explicitly requested runtime probe could not execute (missing/non-executable, timeout,
    /// crash, or wait failure). Never treated as either a pass or a gap and never staged.
    CouldNotProbe,
    /// A mechanical probe exists and **passed** — not a finding; listed only under `--verbose`.
    Pass,
    /// A conditional whose trigger is off — n/a, never a gap; listed only under `--verbose`.
    NotApplicable,
}

/// The severity-derived fix class (the canon's triage). Only meaningful for a gap/verify row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixClass {
    /// A MUST or MUST-when-applies dimension — a hard conformance requirement.
    MustFix,
    /// A SHOULD dimension — a strong convention, never a hard gate.
    ShouldFix,
}

impl FixClass {
    fn from_severity(severity: Severity) -> FixClass {
        match severity {
            Severity::Must | Severity::MustWhenApplies => FixClass::MustFix,
            Severity::Should => FixClass::ShouldFix,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            FixClass::MustFix => "must-fix",
            FixClass::ShouldFix => "should-fix",
        }
    }
}

/// One resolved dimension, triaged into a review finding.
#[derive(Debug, Clone)]
struct Finding {
    id: &'static str,
    title: &'static str,
    severity: Severity,
    layer: Layer,
    canon_section: Option<u8>,
    kind: FindingKind,
    fix_class: FixClass,
    effect: EffectClass,
    /// The observation: the probe's evidence (confirmed gap) or the verify-by-hand prompt.
    observed: String,
    /// The conformant shape to look for (the model probe's `signal`).
    expected: &'static str,
    /// The anti-pattern that constitutes a failure (the model probe's `fail`).
    fail_mode: &'static str,
    /// How to observe the dimension (the model probe's `command_hint`).
    command_hint: &'static str,
    /// The staged `issuectl` command — `Some` for a confirmed gap, `None` for a manual-verify note
    /// (the review-report rule: `unknown` is never a filed gap). PRINTED, never executed.
    staged_command: Option<String>,
}

impl Finding {
    /// Ranking key (ascending → most-severe first): confirmed gaps outrank verify notes; within a
    /// kind, must-fix outranks should-fix; then by canon §N (base scaffold dims sort after canon).
    fn sort_key(&self) -> (u8, u8, u16, &'static str) {
        let kind_rank = match self.kind {
            FindingKind::ConfirmedGap => 0,
            FindingKind::CouldNotProbe => 1,
            FindingKind::ManualVerify => 2,
            FindingKind::Pass => 3,
            FindingKind::NotApplicable => 4,
        };
        let fix_rank = match self.fix_class {
            FixClass::MustFix => 0,
            FixClass::ShouldFix => 1,
        };
        // Canon sections first (in §N order), scaffold dims (no section) after.
        let section_rank = self.canon_section.map_or(u16::MAX, u16::from);
        (kind_rank, fix_rank, section_rank, self.id)
    }
}

/// The whole review report.
#[derive(Debug, Clone)]
struct Report {
    tool: &'static str,
    target: String,
    profile: Archetype,
    surface_shape: Option<SurfaceShape>,
    runtime_binary: Option<String>,
    runtime_probes: Vec<RuntimeProbeOutcome>,
    findings: Vec<Finding>,
}

impl Report {
    fn count(&self, kind: FindingKind) -> usize {
        self.findings.iter().filter(|f| f.kind == kind).count()
    }

    /// Confirmed gaps by fix class.
    fn confirmed_of(&self, fix: FixClass) -> usize {
        self.findings
            .iter()
            .filter(|f| f.kind == FindingKind::ConfirmedGap && f.fix_class == fix)
            .count()
    }

    /// The ordered list of staged commands (the confirmed-gap `issuectl` commands) — the
    /// "would-run list" the human approves. Every one is PRINTED, never executed.
    fn staged_commands(&self) -> Vec<&str> {
        self.findings
            .iter()
            .filter_map(|f| f.staged_command.as_deref())
            .collect()
    }

    /// The actionable rows — confirmed gaps + manual-verify coverage notes. `pass`/`n/a` are not
    /// findings (they live only in the summary counts and, for a human, under `--verbose`), so the
    /// JSON `findings[]` carries `kind ∈ {confirmed-gap, manual-verify}` only.
    fn actionable(&self) -> impl Iterator<Item = &Finding> {
        self.findings.iter().filter(|f| {
            matches!(
                f.kind,
                FindingKind::ConfirmedGap | FindingKind::CouldNotProbe | FindingKind::ManualVerify
            )
        })
    }

    /// The §10 structured payload.
    fn to_json(&self) -> Json {
        let findings = self
            .actionable()
            .map(|f| {
                Json::Object(vec![
                    ("id".into(), Json::str(f.id)),
                    ("title".into(), Json::str(f.title)),
                    ("severity".into(), Json::str(severity_str(f.severity))),
                    ("layer".into(), Json::str(layer_str(f.layer))),
                    (
                        "canon_section".into(),
                        f.canon_section.map_or(Json::Null, |n| Json::Int(n as i64)),
                    ),
                    ("kind".into(), Json::str(kind_str(f.kind))),
                    ("fix_class".into(), Json::str(f.fix_class.as_str())),
                    ("effect".into(), Json::str(effect_str(f.effect))),
                    ("observed".into(), Json::str(f.observed.clone())),
                    ("expected".into(), Json::str(f.expected)),
                    ("fail_mode".into(), Json::str(f.fail_mode)),
                    ("command_hint".into(), Json::str(f.command_hint)),
                    (
                        "staged_command".into(),
                        Json::opt_str(f.staged_command.clone()),
                    ),
                ])
            })
            .collect();

        let staged = self
            .staged_commands()
            .into_iter()
            .map(Json::str)
            .collect::<Vec<_>>();

        let summary = Json::Object(vec![
            (
                "confirmed_gaps".into(),
                Json::Int(self.count(FindingKind::ConfirmedGap) as i64),
            ),
            (
                "must_fix".into(),
                Json::Int(self.confirmed_of(FixClass::MustFix) as i64),
            ),
            (
                "should_fix".into(),
                Json::Int(self.confirmed_of(FixClass::ShouldFix) as i64),
            ),
            (
                "could_not_probe".into(),
                Json::Int(self.count(FindingKind::CouldNotProbe) as i64),
            ),
            (
                "manual_verify".into(),
                Json::Int(self.count(FindingKind::ManualVerify) as i64),
            ),
            (
                "pass".into(),
                Json::Int(self.count(FindingKind::Pass) as i64),
            ),
            (
                "not_applicable".into(),
                Json::Int(self.count(FindingKind::NotApplicable) as i64),
            ),
            ("staged".into(), Json::Int(staged.len() as i64)),
        ]);

        Json::Object(vec![
            ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
            ("tool".into(), Json::str(self.tool)),
            ("verb".into(), Json::str("review")),
            ("advisory".into(), Json::Bool(true)),
            ("target".into(), Json::str(self.target.clone())),
            ("profile".into(), Json::str(self.profile.slug())),
            (
                "surface_shape".into(),
                Json::opt_str(self.surface_shape.map(surface_shape_str)),
            ),
            (
                "runtime_probe".into(),
                Json::Object(vec![
                    ("enabled".into(), Json::Bool(self.runtime_binary.is_some())),
                    ("binary".into(), Json::opt_str(self.runtime_binary.clone())),
                    ("timeout_ms".into(), Json::Int(RUNTIME_TIMEOUT_MS as i64)),
                    (
                        "outcomes".into(),
                        Json::Array(
                            self.runtime_probes
                                .iter()
                                .map(|probe| {
                                    Json::Object(vec![
                                        ("id".into(), Json::str(probe.id)),
                                        ("status".into(), Json::str(probe.status.as_str())),
                                        ("message".into(), Json::str(probe.message.clone())),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                ]),
            ),
            ("findings".into(), Json::Array(findings)),
            ("staged_commands".into(), Json::Array(staged)),
            // Named-but-empty at v0: real dimension-discovery candidates need ≥2-tool recurrence
            // (judgment), staged against homebase's cli-canon-consolidate. See render_human.
            ("discovery_candidates".into(), Json::Array(vec![])),
            ("summary".into(), summary),
            ("exit_code".into(), Json::Int(EXIT_OK as i64)),
        ])
    }

    /// The human view: severity-ranked findings, then the staged commands, then a summary.
    /// Terse by default (confirmed gaps + staged commands); `--verbose` adds manual-verify,
    /// passing, and n/a rows.
    fn render_human(&self, verbose: bool) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "project-canon review: {} (profile: {})  [advisory — recommends & stages, never acts]\n",
            self.target,
            self.profile.slug()
        ));

        out.push_str("findings (severity-triaged; most severe first):\n");
        let mut shown = 0usize;
        for f in &self.findings {
            let listed = match f.kind {
                FindingKind::ConfirmedGap => true,
                FindingKind::CouldNotProbe => true,
                FindingKind::ManualVerify | FindingKind::Pass | FindingKind::NotApplicable => {
                    verbose
                }
            };
            if !listed {
                continue;
            }
            shown += 1;
            out.push_str(&render_finding_row(f));
        }
        if shown == 0 {
            out.push_str("  (no confirmed gaps — run with --verbose for the manual-verify list)\n");
        }

        // The would-run list — printed, NEVER executed.
        let staged = self.staged_commands();
        out.push_str(
            "\nstaged issue commands (printed, NOT executed — review never files; run these yourself):\n",
        );
        if staged.is_empty() {
            out.push_str("  (none — no confirmed gaps to stage)\n");
        } else {
            for cmd in &staged {
                out.push_str(&format!("  {cmd}\n"));
            }
        }

        // Dimension-discovery pointer (named-but-empty at v0).
        out.push_str(
            "\ndimension-discovery candidates: none at v0 \
             (a canon addition needs recurrence across \u{2265}2 tools \u{2014} a judgment call; \
             stage real candidates against homebase's cli-canon-consolidate).\n",
        );

        out.push_str(&format!(
            "\nsummary: {} confirmed gap{} ({} must-fix, {} should-fix), \
             {} could-not-probe, {} manual-verify, {} pass, {} n/a  \u{2192}  advisory (exit 0)\n",
            self.count(FindingKind::ConfirmedGap),
            if self.count(FindingKind::ConfirmedGap) == 1 {
                ""
            } else {
                "s"
            },
            self.confirmed_of(FixClass::MustFix),
            self.confirmed_of(FixClass::ShouldFix),
            self.count(FindingKind::CouldNotProbe),
            self.count(FindingKind::ManualVerify),
            self.count(FindingKind::Pass),
            self.count(FindingKind::NotApplicable),
        ));
        out
    }
}

/// One finding's human block — the label line plus the actionable detail.
fn render_finding_row(f: &Finding) -> String {
    let tag = match f.kind {
        FindingKind::ConfirmedGap => f.fix_class.as_str(),
        FindingKind::CouldNotProbe => "could-not",
        FindingKind::ManualVerify => "verify",
        FindingKind::Pass => "pass",
        FindingKind::NotApplicable => "n/a",
    };
    let section = f
        .canon_section
        .map_or_else(|| "§--".to_string(), |n| format!("§{n}"));
    // The label line carries the human-readable title (not just the id) so an auditor need not
    // memorize dimension ids to know what a row is about.
    let mut row = format!("  [{:<9}] {:<4} {:<20} {}\n", tag, section, f.id, f.title);
    // Actionable detail per kind.
    match f.kind {
        FindingKind::ConfirmedGap => {
            row.push_str(&format!("      observed: {}\n", f.observed));
            row.push_str(&format!("      expected: {}\n", f.expected));
            if let Some(cmd) = &f.staged_command {
                row.push_str(&format!("      stage:    {cmd}\n"));
            }
        }
        FindingKind::CouldNotProbe => {
            row.push_str(&format!("      outcome:  {}\n", f.observed));
            row.push_str("      status:   could-not-probe (not pass, not gap)\n");
        }
        FindingKind::ManualVerify => {
            row.push_str(&format!(
                "      how:      {}  ({})\n",
                f.command_hint,
                effect_str(f.effect)
            ));
            row.push_str(&format!("      expected: {}\n", f.expected));
            row.push_str(&format!("      fail:     {}\n", f.fail_mode));
        }
        // pass / n/a rows (verbose only) stay one-liners — show the evidence inline.
        FindingKind::Pass | FindingKind::NotApplicable => {
            row.push_str(&format!("      {}\n", f.observed));
        }
    }
    row
}

fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Must => "must",
        Severity::MustWhenApplies => "must-when-applies",
        Severity::Should => "should",
    }
}

fn layer_str(layer: Layer) -> String {
    match layer {
        Layer::Base => "base".to_string(),
        Layer::Profile(a) => format!("profile:{}", a.slug()),
    }
}

fn effect_str(effect: EffectClass) -> &'static str {
    match effect {
        EffectClass::Static => "static",
        EffectClass::ExecRo => "exec-ro",
        EffectClass::SandboxWrite => "sandbox-write",
    }
}

fn surface_shape_str(shape: SurfaceShape) -> &'static str {
    match shape {
        SurfaceShape::NounVerb => "noun-verb",
        SurfaceShape::FlatVerb => "flat-verb",
    }
}

fn kind_str(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::ConfirmedGap => "confirmed-gap",
        FindingKind::CouldNotProbe => "could-not-probe",
        FindingKind::ManualVerify => "manual-verify",
        FindingKind::Pass => "pass",
        FindingKind::NotApplicable => "not-applicable",
    }
}

// ===== building the report ==========================================================

/// An operational I/O fault while probing a dimension. Routed to exit `2` — it is review failing
/// to *evaluate* the repo, never a fabricated finding.
#[derive(Debug)]
struct ProbeFault {
    dim_id: &'static str,
    source: std::io::Error,
}

/// Turn a [`Resolution`] into a [`Report`]: triage each resolved dimension into a [`Finding`],
/// running the shared mechanical probe where one exists. A probe's operational I/O fault
/// short-circuits to `Err` (→ exit 2). Findings are ranked most-severe first.
fn build_report(
    model: &Model,
    resolution: &Resolution,
    profile: Archetype,
    target: &str,
    probe_context: &ProbeContext<'_>,
    runtime_binary: Option<&Path>,
    runtime: &[RuntimeProbeOutcome],
) -> Result<Report, ProbeFault> {
    let repo = Path::new(target);
    let mut findings = Vec::with_capacity(resolution.entries().len());
    for rd in resolution.entries() {
        let dim = model
            .dimension(rd.id)
            .expect("resolution ids resolve in the model");
        findings.push(triage(
            dim,
            rd.status,
            repo,
            target,
            probe_context,
            runtime.iter().find(|probe| probe.id == dim.id),
        )?);
    }
    findings.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));

    Ok(Report {
        tool: "project-canon",
        target: target.to_string(),
        profile,
        surface_shape: resolution.surface_shape(),
        runtime_binary: runtime_binary.map(|path| path.display().to_string()),
        runtime_probes: runtime.to_vec(),
        findings,
    })
}

/// Triage one resolved dimension into a [`Finding`], propagating an operational probe fault as
/// `Err`. The shared mechanical probe decides confirmed-gap vs. pass; a dimension that applies but
/// has no mechanical probe becomes a manual-verify coverage note; an n/a conditional is dropped
/// from the gap list (kept for the summary/verbose).
fn triage(
    dim: &Dimension,
    status: AppStatus,
    repo: &Path,
    target: &str,
    probe_context: &ProbeContext<'_>,
    runtime: Option<&RuntimeProbeOutcome>,
) -> Result<Finding, ProbeFault> {
    let fix_class = FixClass::from_severity(dim.severity);
    let base = |kind, observed: String, staged_command| Finding {
        id: dim.id,
        title: dim.title,
        severity: dim.severity,
        layer: dim.layer,
        canon_section: dim.canon_section(),
        kind,
        fix_class,
        effect: dim.probe.effect,
        observed,
        expected: dim.probe.signal,
        fail_mode: dim.probe.fail,
        command_hint: dim.probe.command_hint,
        staged_command,
    };

    // An explicit runtime probe is stronger evidence than the conservative questionnaire. This
    // intentionally lets §8 be decided when --run observes a config surface even though Q2 is off
    // under static defaults.
    if let Some(runtime) = runtime {
        return Ok(match runtime.status {
            RuntimeProbeStatus::Pass => base(FindingKind::Pass, runtime.message.clone(), None),
            RuntimeProbeStatus::Gap => base(
                FindingKind::ConfirmedGap,
                runtime.message.clone(),
                Some(stage_command(dim, target)),
            ),
            RuntimeProbeStatus::CouldNotProbe => {
                base(FindingKind::CouldNotProbe, runtime.message.clone(), None)
            }
        });
    }

    // Out of scope for this repo (a conditional gated off) — never a gap.
    if let AppStatus::NotApplicable { gated_by } = status {
        return Ok(base(
            FindingKind::NotApplicable,
            format!("n/a — conditional trigger off ({} = no)", gated_by.label()),
            None,
        ));
    }

    // Applies — a mechanical probe decides pass vs. confirmed gap; otherwise it is a manual-verify
    // coverage note carrying the probe's how-to.
    match mechanical_probe(dim.id) {
        None => Ok(base(
            FindingKind::ManualVerify,
            "no mechanical probe — verify by hand".to_string(),
            None,
        )),
        Some(probe) => {
            let outcome = probe(repo, probe_context).map_err(|source| ProbeFault {
                dim_id: dim.id,
                source,
            })?;
            if outcome.passed {
                if let Some(remainder) = judgment_remainder(dim.id) {
                    Ok(base(
                        FindingKind::ManualVerify,
                        format!("{}; {remainder}", outcome.message),
                        None,
                    ))
                } else {
                    Ok(base(FindingKind::Pass, outcome.message, None))
                }
            } else {
                // A confirmed gap: stage (print, never run) an issuectl command scoped to the repo.
                let staged = stage_command(dim, target);
                let observed = match judgment_remainder(dim.id) {
                    Some(remainder) => format!("{}; also {remainder}", outcome.message),
                    None => outcome.message,
                };
                Ok(base(FindingKind::ConfirmedGap, observed, Some(staged)))
            }
        }
    }
}

fn absolute_lexical(path: &str) -> std::io::Result<PathBuf> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        std::env::current_dir().map(|cwd| cwd.join(path))
    }
}

fn judgment_remainder(id: &str) -> Option<&'static str> {
    match id {
        "canon.s15" => Some(
            "verify the skill list/install surface, install target behavior, and guidance freshness beyond the static description-length check",
        ),
        "canon.s23" => Some(
            "review hostnames, internal URLs, borderline names, and dependency legitimacy",
        ),
        "canon.s24" => Some(
            "re-verify stated credentials, permissions, dependencies, and any blocker the local issue scan cannot settle",
        ),
        _ => None,
    }
}

/// Render — never execute — the `issuectl` command that would file a confirmed gap, scoped to the
/// target repo (`( cd '<repo>' && issuectl new … )`, the `review-report.md` idiom). Every
/// interpolated value is POSIX single-quoted; the stable `--slug` makes re-runs idempotent
/// (issuectl dedups on slug) so review does not spam the tracker. review NEVER runs this.
fn stage_command(dim: &Dimension, target: &str) -> String {
    let (title, slug) = match dim.canon_section() {
        Some(section) => (
            format!("cli-canon: §{section} {}", dim.title),
            format!("cli-canon-s{section:02}"),
        ),
        None => {
            // A base scaffold dim (e.g. `base.doc-pattern`) → a `canon-<full-id>` slug. Deriving
            // the slug from the *whole* dimension id (not just its final segment) keeps every
            // staged slug distinct — two dims that share a final segment (e.g. a future
            // `profile.x.readme` vs. `base.readme`) must not collapse to one issue when a human
            // runs the commands (issuectl dedups on slug). Uniqueness is asserted by test.
            (
                format!("project-canon: {}", dim.title),
                format!("canon-{}", dim.id.replace('.', "-")),
            )
        }
    };
    // `cd --` ends option parsing so an absolute path is never mistaken for a flag (canonicalized
    // paths start with `/` today, so this is belt-and-suspenders — but free and strictly safer).
    format!(
        "( cd -- {} && issuectl new --type improvement --title {} --slug {} --label tooling --label cli-canon )",
        shell_quote(target),
        shell_quote(&title),
        shell_quote(&slug),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- arg parsing -------------------------------------------------------------

    fn parse(args: &[&str]) -> Result<Command, String> {
        parse_args(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn defaults_are_cli_profile_and_cwd() {
        assert_eq!(
            parse(&[]).unwrap(),
            Command::Run(ReviewArgs {
                repo: ".".to_string(),
                profile: Archetype::Cli,
                json: false,
                verbose: false,
                assume_defaults: false,
                run: None,
            })
        );
    }

    #[test]
    fn parses_all_flags_and_positional() {
        let cmd = parse(&[
            "--profile",
            "service",
            "--json",
            "--verbose",
            "--assume-defaults",
            "/some/repo",
        ])
        .unwrap();
        assert_eq!(
            cmd,
            Command::Run(ReviewArgs {
                repo: "/some/repo".to_string(),
                profile: Archetype::Service,
                json: true,
                verbose: true,
                assume_defaults: true,
                run: None,
            })
        );
    }

    #[test]
    fn help_short_circuits() {
        assert_eq!(parse(&["--help"]).unwrap(), Command::Help);
        assert_eq!(parse(&["--json", "--help"]).unwrap(), Command::Help);
    }

    #[test]
    fn strict_validation_rejects_bad_input() {
        assert!(parse(&["--nope"]).unwrap_err().contains("--nope"));
        let bad = parse(&["--profile", "webapp"]).unwrap_err();
        assert!(bad.contains("webapp") && bad.contains("cli"), "{bad}");
        assert!(parse(&["--profile"]).is_err());
        assert!(parse(&["--json", "--json"]).is_err());
        assert!(parse(&["a", "b"]).is_err());
        assert!(parse(&["--json=false"])
            .unwrap_err()
            .contains("does not take a value"));
        assert!(parse(&["--profile="]).is_err());
        assert!(parse(&["--profile", "service", "--run", "/bin/tool"])
            .unwrap_err()
            .contains("requires --profile cli"));
    }

    #[test]
    fn double_dash_stops_flag_parsing() {
        let Command::Run(a) = parse(&["--", "-weird-repo"]).unwrap() else {
            panic!("expected Run");
        };
        assert_eq!(a.repo, "-weird-repo");
        assert!(parse(&["--", "a", "b"]).is_err());
    }

    // ---- report fixtures ---------------------------------------------------------

    /// A throwaway temp dir under the OS temp root; removed on drop.
    struct TmpRepo {
        path: std::path::PathBuf,
    }

    impl TmpRepo {
        fn new(tag: &str) -> TmpRepo {
            use std::sync::atomic::{AtomicU32, Ordering};
            static N: AtomicU32 = AtomicU32::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("pc-review-{tag}-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            TmpRepo { path }
        }
        fn touch(&self, rel: &str) -> &Self {
            let p = self.path.join(rel);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&p, b"x").unwrap();
            self
        }
        fn mkdir(&self, rel: &str) -> &Self {
            std::fs::create_dir_all(self.path.join(rel)).unwrap();
            self
        }
        fn target(&self) -> String {
            self.path.display().to_string()
        }
    }

    impl Drop for TmpRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// A repo satisfying every mechanical MUST (and SHOULD) probe.
    fn conformant_repo(tag: &str) -> TmpRepo {
        let repo = TmpRepo::new(tag);
        repo.touch("AGENTS.md")
            .touch("CLAUDE.md")
            .touch("README.md")
            .touch(".gitignore")
            .mkdir("issues")
            .mkdir(".git")
            .mkdir("crates/pc-core")
            .mkdir("crates/pc-cli");
        repo
    }

    fn report_for(repo: &TmpRepo, profile: Archetype) -> Report {
        let model = Model::standard();
        let resolution = model.resolve(&Questionnaire::builder(profile).build());
        let deny_list = std::collections::BTreeSet::new();
        let context = ProbeContext {
            user_specific_deny_list: &deny_list,
        };
        build_report(
            &model,
            &resolution,
            profile,
            &repo.target(),
            &context,
            None,
            &[],
        )
        .expect("no I/O fault")
    }

    fn find<'a>(report: &'a Report, id: &str) -> &'a Finding {
        report
            .findings
            .iter()
            .find(|f| f.id == id)
            .unwrap_or_else(|| panic!("finding {id} present"))
    }

    // ---- triage & severity -------------------------------------------------------

    #[test]
    fn a_missing_must_scaffold_is_a_confirmed_must_fix_gap() {
        let repo = conformant_repo("gap");
        std::fs::remove_file(repo.path.join("AGENTS.md")).unwrap();
        let report = report_for(&repo, Archetype::Cli);
        let doc = find(&report, "base.doc-pattern");
        assert_eq!(doc.kind, FindingKind::ConfirmedGap);
        assert_eq!(doc.fix_class, FixClass::MustFix);
        assert!(doc.observed.contains("AGENTS.md"));
        assert!(doc.staged_command.is_some(), "a confirmed gap is staged");
    }

    #[test]
    fn a_missing_should_is_a_confirmed_should_fix_gap() {
        let repo = conformant_repo("should");
        std::fs::remove_file(repo.path.join("README.md")).unwrap();
        let report = report_for(&repo, Archetype::Cli);
        let readme = find(&report, "base.readme");
        assert_eq!(readme.kind, FindingKind::ConfirmedGap);
        assert_eq!(readme.fix_class, FixClass::ShouldFix);
    }

    #[test]
    fn a_passing_probe_is_a_pass_not_a_gap() {
        let repo = conformant_repo("pass");
        let report = report_for(&repo, Archetype::Cli);
        assert_eq!(find(&report, "base.doc-pattern").kind, FindingKind::Pass);
        assert_eq!(report.count(FindingKind::ConfirmedGap), 0);
    }

    #[test]
    fn public_artifact_gap_also_surfaces_the_judgment_remainder() {
        let repo = conformant_repo("s23-gap-judgment");
        std::fs::write(repo.path.join("README.md"), "private-widget").unwrap();
        let model = Model::standard();
        let resolution = model.resolve(&Questionnaire::builder(Archetype::Cli).build());
        let deny_list = std::collections::BTreeSet::from(["private-widget".to_string()]);
        let context = ProbeContext {
            user_specific_deny_list: &deny_list,
        };
        let report = build_report(
            &model,
            &resolution,
            Archetype::Cli,
            &repo.target(),
            &context,
            None,
            &[],
        )
        .unwrap();
        let s23 = find(&report, "canon.s23");
        assert_eq!(s23.kind, FindingKind::ConfirmedGap);
        assert!(s23.observed.contains("hostnames"));
    }

    #[test]
    fn public_artifact_check_surfaces_the_judgment_remainder_after_mechanical_pass() {
        let repo = conformant_repo("s23-judgment");
        let report = report_for(&repo, Archetype::Cli);
        let s23 = find(&report, "canon.s23");
        assert_eq!(s23.kind, FindingKind::ManualVerify);
        assert!(s23.observed.contains("hostnames"));
        assert!(s23.staged_command.is_none());
    }

    #[test]
    fn verified_deferral_check_surfaces_the_judgment_remainder_after_mechanical_pass() {
        let repo = conformant_repo("s24-judgment");
        let report = report_for(&repo, Archetype::Cli);
        let s24 = find(&report, "canon.s24");
        assert_eq!(s24.kind, FindingKind::ManualVerify);
        assert!(s24.observed.contains("credentials"));
        assert!(s24.staged_command.is_none());
    }

    #[test]
    fn a_behavioral_section_is_a_manual_verify_note_never_staged() {
        let repo = conformant_repo("verify");
        let report = report_for(&repo, Archetype::Cli);
        // §1 (strict input validation) is behavioral — no mechanical probe → manual-verify.
        let s1 = find(&report, "canon.s01");
        assert_eq!(s1.kind, FindingKind::ManualVerify);
        assert_eq!(s1.fix_class, FixClass::MustFix);
        assert!(
            s1.staged_command.is_none(),
            "manual-verify (unknown) is never a filed gap"
        );
        // It carries the probe's how-to so the auditor can run it by hand.
        assert!(!s1.command_hint.is_empty());
        assert!(!s1.expected.is_empty());
    }

    #[test]
    fn conditional_sections_are_na_under_conservative_defaults() {
        let repo = conformant_repo("na");
        let report = report_for(&repo, Archetype::Cli);
        // §11 (dry-run) is gated by Q3, off by default → n/a, never a gap.
        let s11 = find(&report, "canon.s11");
        assert_eq!(s11.kind, FindingKind::NotApplicable);
        assert!(s11.staged_command.is_none());
    }

    #[test]
    fn findings_are_ranked_most_severe_first() {
        // A repo missing a MUST scaffold (doc-pattern) and a SHOULD (readme): the confirmed
        // must-fix gap must sort before the confirmed should-fix gap, both before manual-verify.
        let repo = conformant_repo("rank");
        std::fs::remove_file(repo.path.join("AGENTS.md")).unwrap();
        std::fs::remove_file(repo.path.join("README.md")).unwrap();
        let report = report_for(&repo, Archetype::Cli);
        let confirmed: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::ConfirmedGap)
            .collect();
        assert_eq!(confirmed[0].id, "base.doc-pattern"); // must-fix first
        assert_eq!(confirmed[1].id, "base.readme"); // should-fix second
                                                    // The first manual-verify appears only after every confirmed gap.
        let first_verify = report
            .findings
            .iter()
            .position(|f| f.kind == FindingKind::ManualVerify)
            .unwrap();
        let last_confirmed = report
            .findings
            .iter()
            .rposition(|f| f.kind == FindingKind::ConfirmedGap)
            .unwrap();
        assert!(last_confirmed < first_verify);
    }

    // ---- staging (recommend & stage, never act) ----------------------------------

    #[test]
    fn staged_command_is_scoped_shell_safe_and_not_executed() {
        let repo = conformant_repo("stage");
        std::fs::remove_file(repo.path.join("AGENTS.md")).unwrap();
        let report = report_for(&repo, Archetype::Cli);
        let cmd = find(&report, "base.doc-pattern")
            .staged_command
            .clone()
            .unwrap();
        // Scoped to the target repo, uses issuectl new, carries the stable slug + labels.
        assert!(cmd.contains("issuectl new"));
        assert!(cmd.contains(&format!("cd -- '{}'", repo.target())));
        assert!(cmd.contains("--slug 'canon-base-doc-pattern'"));
        assert!(cmd.contains("--label tooling"));
        assert!(cmd.contains("--label cli-canon"));
        // It is a STRING to print — building the report ran no issuectl / wrote nothing.
        // (Proven structurally: build_report only reads via probes; see never_act tests below.)
    }

    #[test]
    fn canon_section_gap_stages_a_section_scoped_slug() {
        // Force a canon-section mechanical gap: §22 (core/cli split) has a mechanical probe.
        let repo = conformant_repo("s22");
        std::fs::remove_dir_all(repo.path.join("crates")).unwrap();
        let report = report_for(&repo, Archetype::Cli);
        let s22 = find(&report, "canon.s22");
        assert_eq!(s22.kind, FindingKind::ConfirmedGap);
        let cmd = s22.staged_command.clone().unwrap();
        assert!(cmd.contains("--slug 'cli-canon-s22'"), "{cmd}");
        assert!(cmd.contains("§22"), "{cmd}");
    }

    #[test]
    fn every_dimension_stages_a_unique_slug() {
        // Guards the dedup contract: no two dimensions may render the same `--slug`, or one
        // confirmed gap would silently suppress another when the human runs the staged commands.
        let model = Model::standard();
        let mut slugs = std::collections::BTreeSet::new();
        for dim in model.dimensions() {
            let cmd = stage_command(dim, "/repo");
            let slug = cmd
                .split("--slug ")
                .nth(1)
                .and_then(|s| s.split(" --label").next())
                .expect("staged command carries a --slug")
                .to_string();
            assert!(slugs.insert(slug.clone()), "duplicate staged slug: {slug}");
        }
    }

    #[test]
    fn shell_metacharacters_in_the_target_path_are_neutralized() {
        // A target dir with a shell metacharacter must not break out of the single-quoting.
        let repo = conformant_repo("meta$`;dir");
        std::fs::remove_file(repo.path.join("AGENTS.md")).unwrap();
        let report = report_for(&repo, Archetype::Cli);
        let cmd = find(&report, "base.doc-pattern")
            .staged_command
            .clone()
            .unwrap();
        // The path is wrapped in single quotes, so `$`/backtick/`;` are inert.
        assert!(cmd.contains(&shell_quote(&repo.target())));
    }

    // ---- never-act guarantees ----------------------------------------------------

    #[test]
    fn review_never_writes_to_the_target_repo() {
        // Snapshot the repo's directory entries before and after a full review; the audit must
        // add/remove nothing (no auto-fix, no scaffolding, no issue files).
        let repo = conformant_repo("nowrite");
        std::fs::remove_file(repo.path.join("AGENTS.md")).unwrap(); // force gaps + staged commands
        let before = dir_snapshot(&repo.path);
        let report = report_for(&repo, Archetype::Cli);
        assert!(report.count(FindingKind::ConfirmedGap) >= 1);
        let after = dir_snapshot(&repo.path);
        assert_eq!(before, after, "review must not mutate the target repo");
        // Specifically, it did NOT create an issues/<slug>/ tree for the staged command.
        assert!(!repo.path.join("issues").join("canon-doc-pattern").exists());
    }

    #[test]
    fn a_repo_with_no_issues_dir_still_only_stages_never_files() {
        // Even when issues/ is absent (the staged command would fail if run), review stages the
        // command as text and files nothing itself.
        let repo = TmpRepo::new("noissues");
        repo.touch("CLAUDE.md"); // AGENTS.md missing → a confirmed gap
        let report = report_for(&repo, Archetype::Cli);
        assert!(find(&report, "base.doc-pattern").staged_command.is_some());
        assert!(
            !repo.path.join("issues").exists(),
            "review created no issues/"
        );
    }

    /// A sorted snapshot of every path under `root` (relative), for before/after equality.
    fn dir_snapshot(root: &Path) -> Vec<String> {
        fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
            let mut entries: Vec<_> = std::fs::read_dir(dir)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            entries.sort();
            for p in entries {
                out.push(p.strip_prefix(base).unwrap().display().to_string());
                if p.is_dir() {
                    walk(&p, base, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(root, root, &mut out);
        out
    }

    // ---- JSON schema -------------------------------------------------------------

    #[test]
    fn json_report_carries_the_schema_advisory_and_summary() {
        let repo = conformant_repo("json");
        std::fs::remove_file(repo.path.join("AGENTS.md")).unwrap();
        let report = report_for(&repo, Archetype::Cli);
        let json = report.to_json().to_string();
        assert!(json.contains("\"schema_version\":1"));
        assert!(json.contains("\"verb\":\"review\""));
        assert!(json.contains("\"advisory\":true"));
        assert!(json.contains("\"profile\":\"cli\""));
        assert!(json.contains("\"exit_code\":0"));
        assert!(json.contains("\"surface_shape\":\"flat-verb\""));
        assert!(json.contains("\"kind\":\"confirmed-gap\""));
        assert!(json.contains("\"fix_class\":\"must-fix\""));
        assert!(json.contains("\"discovery_candidates\":[]"));
        // The flat staged_commands list mirrors the per-finding staged commands.
        assert!(json.contains("\"staged_commands\":[\"( cd "));
    }

    #[test]
    fn manual_verify_serializes_a_null_staged_command() {
        let repo = conformant_repo("jsonnull");
        let report = report_for(&repo, Archetype::Cli);
        // Every finding is pass/verify/na here (conformant repo) → no non-null staged_command.
        let json = report.to_json().to_string();
        assert!(json.contains("\"kind\":\"manual-verify\""));
        assert!(json.contains("\"staged_command\":null"));
        assert!(json.contains("\"staged_commands\":[]"));
    }

    #[test]
    fn non_cli_profile_has_null_surface_shape() {
        let repo = conformant_repo("svc");
        let report = report_for(&repo, Archetype::Service);
        assert_eq!(report.surface_shape, None);
        assert!(report
            .to_json()
            .to_string()
            .contains("\"surface_shape\":null"));
    }

    // ---- human rendering ---------------------------------------------------------

    #[test]
    fn human_terse_shows_gaps_and_staged_but_hides_verify_until_verbose() {
        let repo = conformant_repo("human");
        std::fs::remove_file(repo.path.join("AGENTS.md")).unwrap();
        let report = report_for(&repo, Archetype::Cli);
        let terse = report.render_human(false);
        assert!(terse.contains("must-fix"), "{terse}");
        assert!(terse.contains("issuectl new"), "{terse}");
        assert!(terse.contains("NOT executed"), "{terse}");
        assert!(terse.contains("advisory"), "{terse}");
        // A behavioral verify row (canon.s01) is hidden in terse mode…
        assert!(!terse.contains("canon.s01"), "{terse}");
        // …but shown under --verbose.
        let verbose = report.render_human(true);
        assert!(verbose.contains("canon.s01"), "{verbose}");
        assert!(verbose.contains("verify"), "{verbose}");
    }

    #[test]
    fn a_fully_conformant_repo_stages_nothing() {
        let repo = conformant_repo("clean");
        let report = report_for(&repo, Archetype::Cli);
        assert_eq!(report.count(FindingKind::ConfirmedGap), 0);
        assert!(report.staged_commands().is_empty());
        let human = report.render_human(false);
        assert!(human.contains("no confirmed gaps"), "{human}");
        assert!(
            human.contains("none \u{2014} no confirmed gaps to stage"),
            "{human}"
        );
    }
}
