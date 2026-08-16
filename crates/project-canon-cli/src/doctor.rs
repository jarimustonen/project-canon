//! The `doctor` verb — a read-only, non-interactive, CI-shaped **mechanical conformance gate**.
//!
//! It reads the two-layer model from [`project_canon_core`], resolves the target repo's profile
//! with a conservative (non-interactive) questionnaire, runs the **mechanically-decidable**
//! probes over the target repo's files, emits a pass/fail matrix (human + `--json`), and exits
//! non-zero on a mechanically-decided MUST gap. Mirrors canon §18 (doctor discipline) and §2
//! (exit-code discipline). No LLM/human judgment — that is `review`'s job (ADR 0009 §2).
//!
//! Verb logic lives at the CLI edge (not in the I/O-free core) because mechanical probing reads
//! the filesystem. The core model is consumed unchanged.

use std::path::Path;
use std::process::ExitCode;

use project_canon_core::{
    AppStatus, Archetype, Dimension, EnvConfigLayer, Layer, Model, Questionnaire, Resolution,
    Severity, SurfaceShape,
};

use crate::error::{fail, json_requested, CliError};
use crate::json::Json;
use crate::probes::mechanical_probe;

/// The `--json` payload schema version (§10). Bump on any breaking shape change.
const SCHEMA_VERSION: i64 = 1;

// ===== exit codes (see design.md "Exit-code contract") ==============================
/// Conformant — every mechanically-decided MUST passed.
const EXIT_CONFORMANT: u8 = 0;
/// Non-conformant — ≥1 mechanically-decided MUST gap (the gate's designed non-zero).
const EXIT_GAP: u8 = 1;

/// Run `project-canon doctor <args…>` (the args *after* the `doctor` subcommand). Owns all of
/// doctor's I/O and returns the process exit code.
pub fn run(args: &[String]) -> ExitCode {
    // Parse first so `--help` is always an exit-0 event (§2), even under a malformed environment.
    let parsed = match parse_args(args) {
        Ok(Command::Help) => {
            print!("{HELP}");
            return ExitCode::from(EXIT_CONFORMANT); // §2: help is an exit-0 event.
        }
        Ok(Command::Run(a)) => a,
        Err(err) => {
            return fail(
                json_requested(args),
                CliError::actionable("usage_error", format!("doctor: {err}")),
            );
        }
    };

    // Strict §1 validation of the env override layer, uniform with the rest of the family: a
    // malformed `PROJECT_CANON_*` value is a usage error, never a silent coerce. Doctor consumes
    // no env field itself (it probes an explicit target path), but it still refuses to run on a
    // broken environment override rather than ignoring it. Runs *after* `--help` so help never
    // requires a clean environment.
    if let Err(err) = EnvConfigLayer::from_env_vars(&std::env::vars().collect()) {
        return fail(
            parsed.json,
            CliError::actionable("validation_error", format!("doctor: {err}")),
        );
    }

    // Read-only target validation (§1): the repo must be an existing directory.
    let repo = Path::new(&parsed.repo);
    if !repo.is_dir() {
        return fail(
            parsed.json,
            CliError::actionable(
                "invalid_target",
                format!("doctor: target repo is not a directory: {:?}", parsed.repo),
            ),
        );
    }
    // Absolute, symlink-resolved path for the report's `target` field. A failure here (a race or
    // permission error after the `is_dir` check) is operational, not a conformance gap → exit 2.
    let target = match std::fs::canonicalize(repo) {
        Ok(p) => p.display().to_string(),
        Err(err) => {
            return fail(
                parsed.json,
                CliError::system(
                    "io_error",
                    format!("doctor: cannot resolve target {:?}: {err}", parsed.repo),
                ),
            );
        }
    };

    // Resolve the model with the conservative questionnaire (all conditionals off — §3, doctor
    // never prompts). `--assume-defaults` names this explicitly; it is the only mode at v0.
    let model = Model::standard();
    let questionnaire = Questionnaire::builder(parsed.profile).build();
    let resolution = model.resolve(&questionnaire);

    // An operational I/O fault while probing (permission denied, transient error, target vanished)
    // is not a conformance gap: it means doctor could not evaluate the repo → exit 2, never exit 1.
    let report = match build_report(&model, &resolution, parsed.profile, &target, repo) {
        Ok(r) => r,
        Err(fault) => {
            return fail(
                parsed.json,
                CliError::system(
                    "io_error",
                    format!(
                        "doctor: cannot probe {} ({}): {}",
                        fault.dim_id, target, fault.source
                    ),
                ),
            );
        }
    };

    if parsed.json {
        println!("{}", report.to_json());
    } else {
        print!("{}", report.render_human(parsed.verbose));
    }
    ExitCode::from(report.exit_code())
}

// ===== argument parsing =============================================================

/// A parsed doctor invocation.
#[derive(Debug, PartialEq, Eq)]
struct DoctorArgs {
    repo: String,
    profile: Archetype,
    json: bool,
    verbose: bool,
    #[allow(dead_code)] // Accepted & validated; a no-op affirmation of the v0 default mode.
    assume_defaults: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Run(DoctorArgs),
}

/// Strict argument parsing (§1): unknown flags, bad `--profile`, missing values, repeated flags,
/// and extra positionals are all errors echoing the offending token — never a silent fixup.
fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut repo: Option<String> = None;
    let mut profile: Option<Archetype> = None;
    let mut json = false;
    let mut verbose = false;
    let mut assume_defaults = false;

    // `--` stops flag parsing so a repo path beginning with `-` can still be addressed (standard
    // CLI convention). Everything after it is positional.
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
        // Support both `--profile v` and `--profile=v`; `inline` is the value after `=` (if any).
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v)),
            _ => (arg.as_str(), None),
        };
        match flag {
            // Valueless flags reject an inline value (§1: `--json=false` is a usage error, not a
            // silent truthy parse that discards the value).
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
            "--profile" => {
                if profile.is_some() {
                    return Err("repeated flag: --profile".to_string());
                }
                let value = match inline {
                    Some(v) => v.to_string(),
                    None => iter.next().cloned().ok_or_else(|| {
                        "--profile requires a value (cli/service/library/release)".to_string()
                    })?,
                };
                profile = Some(parse_archetype(&value)?);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag: {other}"));
            }
            _ => set_positional(&mut repo, arg)?,
        }
    }

    Ok(Command::Run(DoctorArgs {
        repo: repo.unwrap_or_else(|| ".".to_string()),
        profile: profile.unwrap_or(Archetype::Cli),
        json,
        verbose,
        assume_defaults,
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
project-canon doctor — mechanical conformance gate (read-only, non-interactive)

USAGE:
    project-canon doctor [--profile <archetype>] [--assume-defaults] [--json] [--verbose] [<repo>]

ARGS:
    <repo>                  Target repo to probe (default: current directory). Read-only.

FLAGS:
    --profile <archetype>   cli | service | library | release  (default: cli)
    --assume-defaults       Characterize non-interactively with conservative defaults (v0 default).
    --json                  Emit the structured §10 report on stdout.
    --verbose               Also list skipped (n/a + deferred) checks in the human matrix.
    --help                  Show this help.

EXIT CODES:
    0   conformant — every mechanically-decided MUST passed
    1   non-conformant — a mechanically-decided MUST gap (the gate tripped)
    2   usage/operational error (bad flag, bad --profile, missing target, malformed env)
";

// ===== the report ===================================================================

/// A check's outcome, mirroring §18's `OK/WARN/FAIL` plus `skipped` for n/a + deferred rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckStatus {
    Ok,
    Warn,
    Fail,
    Skipped,
}

impl CheckStatus {
    fn json(self) -> &'static str {
        match self {
            CheckStatus::Ok => "ok",
            CheckStatus::Warn => "warn",
            CheckStatus::Fail => "fail",
            CheckStatus::Skipped => "skipped",
        }
    }
    fn tag(self) -> &'static str {
        match self {
            CheckStatus::Ok => "OK",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
            CheckStatus::Skipped => "SKIP",
        }
    }
}

/// Why a check was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkipReason {
    /// A conditional whose gating question was not answered `true` — never a fail.
    NotApplicable { gated_by: &'static str },
    /// Applies, but has no mechanical probe at v0 — deferred to `review`.
    DeferredToReview,
}

/// One resolved dimension's row in the pass/fail matrix.
#[derive(Debug, Clone)]
struct Check {
    id: &'static str,
    title: &'static str,
    severity: Severity,
    layer: Layer,
    canon_section: Option<u8>,
    status: CheckStatus,
    /// Whether a `fail` on this row flips the exit code.
    gates: bool,
    message: String,
    skip_reason: Option<SkipReason>,
}

/// The whole doctor report.
#[derive(Debug, Clone)]
struct Report {
    tool: &'static str,
    target: String,
    profile: Archetype,
    surface_shape: Option<SurfaceShape>,
    checks: Vec<Check>,
}

impl Report {
    fn count(&self, status: CheckStatus) -> usize {
        self.checks.iter().filter(|c| c.status == status).count()
    }

    /// Mechanically-decided MUST gaps: a `fail` on a row that `gates`. Today every `fail` gates by
    /// construction (a non-gating miss is graded `warn`), but reading the authoritative `gates`
    /// field — rather than assuming the invariant — keeps the gate correct if a future status path
    /// produces a non-gating `fail`. Asserted by `every_fail_gates`.
    fn gaps(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail && c.gates)
            .count()
    }

    fn is_conformant(&self) -> bool {
        self.gaps() == 0
    }

    fn exit_code(&self) -> u8 {
        if self.is_conformant() {
            EXIT_CONFORMANT
        } else {
            EXIT_GAP
        }
    }

    /// The §10 structured payload.
    fn to_json(&self) -> Json {
        let checks = self
            .checks
            .iter()
            .map(|c| {
                let (reason, gated_by) = match c.skip_reason {
                    Some(SkipReason::NotApplicable { gated_by }) => {
                        (Some("not-applicable"), Some(gated_by))
                    }
                    Some(SkipReason::DeferredToReview) => (Some("deferred-to-review"), None),
                    None => (None, None),
                };
                Json::Object(vec![
                    ("id".into(), Json::str(c.id)),
                    ("title".into(), Json::str(c.title)),
                    ("severity".into(), Json::str(severity_str(c.severity))),
                    ("layer".into(), Json::str(layer_str(c.layer))),
                    (
                        "canon_section".into(),
                        c.canon_section.map_or(Json::Null, |n| Json::Int(n as i64)),
                    ),
                    ("status".into(), Json::str(c.status.json())),
                    ("gates".into(), Json::Bool(c.gates)),
                    ("message".into(), Json::str(c.message.clone())),
                    ("reason".into(), Json::opt_str(reason)),
                    ("gated_by".into(), Json::opt_str(gated_by)),
                ])
            })
            .collect();

        let summary = Json::Object(vec![
            ("ok".into(), Json::Int(self.count(CheckStatus::Ok) as i64)),
            (
                "warn".into(),
                Json::Int(self.count(CheckStatus::Warn) as i64),
            ),
            (
                "fail".into(),
                Json::Int(self.count(CheckStatus::Fail) as i64),
            ),
            (
                "skipped".into(),
                Json::Int(self.count(CheckStatus::Skipped) as i64),
            ),
            ("gaps".into(), Json::Int(self.gaps() as i64)),
        ]);

        Json::Object(vec![
            ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
            ("tool".into(), Json::str(self.tool)),
            ("verb".into(), Json::str("doctor")),
            ("target".into(), Json::str(self.target.clone())),
            ("profile".into(), Json::str(self.profile.slug())),
            (
                "surface_shape".into(),
                Json::opt_str(self.surface_shape.map(surface_shape_str)),
            ),
            ("checks".into(), Json::Array(checks)),
            ("summary".into(), summary),
            ("conformant".into(), Json::Bool(self.is_conformant())),
            ("exit_code".into(), Json::Int(self.exit_code() as i64)),
        ])
    }

    /// The human pass/fail matrix (§18). Graded rows always; `skipped` rows only when `verbose`.
    fn render_human(&self, verbose: bool) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "project-canon doctor: {} (profile: {})\n",
            self.target,
            self.profile.slug()
        ));
        for c in &self.checks {
            if c.status == CheckStatus::Skipped && !verbose {
                continue;
            }
            out.push_str(&format!(
                "{:<5} {:<20} {}\n",
                c.status.tag(),
                c.id,
                c.message
            ));
        }
        let verdict = if self.is_conformant() {
            // Scoped honesty: doctor decides only the *mechanically* decidable MUSTs; behavioral
            // sections are deferred to `review`, so this is not a claim of full conformance.
            "mechanically conformant".to_string()
        } else {
            format!(
                "{} mechanical MUST gap{}  (non-conformant)",
                self.gaps(),
                if self.gaps() == 1 { "" } else { "s" }
            )
        };
        out.push_str(&format!(
            "summary: {} ok, {} warn, {} fail, {} skipped  \u{2192}  {}\n",
            self.count(CheckStatus::Ok),
            self.count(CheckStatus::Warn),
            self.count(CheckStatus::Fail),
            self.count(CheckStatus::Skipped),
            verdict,
        ));
        out
    }
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

fn surface_shape_str(shape: SurfaceShape) -> &'static str {
    match shape {
        SurfaceShape::NounVerb => "noun-verb",
        SurfaceShape::FlatVerb => "flat-verb",
    }
}

/// True when a mechanical `fail` on a dimension of this severity flips the exit code. `Should`
/// never gates (cli-canon's "SHOULD is never a hard gate"); both MUST tiers gate when applicable.
fn is_gating_severity(severity: Severity) -> bool {
    matches!(severity, Severity::Must | Severity::MustWhenApplies)
}

// ===== building the report ==========================================================

/// An operational I/O fault while probing a dimension. Carries the dimension id (for a precise
/// error message) and the underlying error. Routed to exit `2` — it is doctor failing to
/// *evaluate* the repo, never a conformance gap.
#[derive(Debug)]
struct ProbeFault {
    dim_id: &'static str,
    source: std::io::Error,
}

/// Turn a [`Resolution`] into a [`Report`]: iterate the resolved dimensions in id order, run the
/// mechanical probe where one exists, and grade each row. A probe's operational I/O fault
/// short-circuits to `Err` (→ exit 2); a genuine conformance miss is a `fail`/`warn` row.
fn build_report(
    model: &Model,
    resolution: &Resolution,
    profile: Archetype,
    target: &str,
    repo: &Path,
) -> Result<Report, ProbeFault> {
    let mut checks = Vec::with_capacity(resolution.entries().len());
    for rd in resolution.entries() {
        let dim = model
            .dimension(rd.id)
            .expect("resolution ids resolve in the model");
        checks.push(grade(dim, rd.status, repo)?);
    }

    Ok(Report {
        tool: "project-canon",
        target: target.to_string(),
        profile,
        surface_shape: resolution.surface_shape(),
        checks,
    })
}

/// Grade one resolved dimension into a [`Check`], propagating an operational probe fault as `Err`.
fn grade(dim: &Dimension, status: AppStatus, repo: &Path) -> Result<Check, ProbeFault> {
    let base = |status, gates, message, skip_reason| Check {
        id: dim.id,
        title: dim.title,
        severity: dim.severity,
        layer: dim.layer,
        canon_section: dim.canon_section(),
        status,
        gates,
        message,
        skip_reason,
    };

    // Out of scope for this repo (a conditional gated off) — never a fail.
    if let AppStatus::NotApplicable { gated_by } = status {
        let label = gated_by.label();
        return Ok(base(
            CheckStatus::Skipped,
            false,
            format!("n/a — conditional trigger off ({label} = no)"),
            Some(SkipReason::NotApplicable { gated_by: label }),
        ));
    }

    // Applies — run its mechanical probe if one exists.
    match mechanical_probe(dim.id) {
        None => Ok(base(
            CheckStatus::Skipped,
            false,
            "no mechanical probe — deferred to review".to_string(),
            Some(SkipReason::DeferredToReview),
        )),
        Some(probe) => {
            let gates = is_gating_severity(dim.severity);
            // A miss (`Ok(false)`) is a conformance result; an I/O error is an operational fault.
            let outcome = probe(repo).map_err(|source| ProbeFault {
                dim_id: dim.id,
                source,
            })?;
            let status = if outcome.passed {
                CheckStatus::Ok
            } else if gates {
                CheckStatus::Fail
            } else {
                // A SHOULD miss is a warning, never a gate.
                CheckStatus::Warn
            };
            Ok(base(status, gates, outcome.message, None))
        }
    }
}

// ===== the mechanical-probe registry ===============================================
//
// The probes themselves live in the shared `crate::probes` module (consumed by both `doctor`
// and `review`); doctor only maps a resolved dimension id to a probe via `mechanical_probe`
// and wraps a probe's operational I/O error into a [`ProbeFault`] here.

#[cfg(test)]
mod tests {
    use super::*;

    // ---- arg parsing -------------------------------------------------------------

    fn parse(args: &[&str]) -> Result<Command, String> {
        parse_args(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn defaults_are_cli_profile_and_cwd() {
        let cmd = parse(&[]).unwrap();
        assert_eq!(
            cmd,
            Command::Run(DoctorArgs {
                repo: ".".to_string(),
                profile: Archetype::Cli,
                json: false,
                verbose: false,
                assume_defaults: false,
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
            Command::Run(DoctorArgs {
                repo: "/some/repo".to_string(),
                profile: Archetype::Service,
                json: true,
                verbose: true,
                assume_defaults: true,
            })
        );
    }

    #[test]
    fn profile_accepts_equals_form() {
        let Command::Run(a) = parse(&["--profile=library"]).unwrap() else {
            panic!("expected Run");
        };
        assert_eq!(a.profile, Archetype::Library);
    }

    #[test]
    fn help_short_circuits() {
        assert_eq!(parse(&["--help"]).unwrap(), Command::Help);
        // Even alongside other args, --help wins.
        assert_eq!(parse(&["--json", "--help"]).unwrap(), Command::Help);
    }

    #[test]
    fn unknown_flag_is_rejected_and_echoed() {
        let err = parse(&["--nope"]).unwrap_err();
        assert!(err.contains("--nope"), "{err}");
    }

    #[test]
    fn bad_profile_echoes_value_and_valid_set() {
        let err = parse(&["--profile", "webapp"]).unwrap_err();
        assert!(err.contains("webapp"), "{err}");
        assert!(err.contains("cli"), "{err}");
    }

    #[test]
    fn missing_profile_value_is_rejected() {
        assert!(parse(&["--profile"]).is_err());
    }

    #[test]
    fn repeated_flag_is_rejected() {
        assert!(parse(&["--json", "--json"]).is_err());
        assert!(parse(&["--profile", "cli", "--profile", "cli"]).is_err());
    }

    #[test]
    fn extra_positional_is_rejected() {
        assert!(parse(&["repo-a", "repo-b"]).is_err());
    }

    // ---- fixtures ----------------------------------------------------------------
    // (The mechanical probes themselves are unit-tested in `crate::probes`; here TmpRepo builds
    // fixture repos for the grading/report tests.)

    /// A throwaway temp dir under the OS temp root; removed on drop. Avoids a tempfile dep.
    struct TmpRepo {
        path: std::path::PathBuf,
    }

    impl TmpRepo {
        fn new(tag: &str) -> TmpRepo {
            use std::sync::atomic::{AtomicU32, Ordering};
            static N: AtomicU32 = AtomicU32::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("pc-doctor-{tag}-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            TmpRepo { path }
        }
        fn touch(&self, rel: &str) {
            let p = self.path.join(rel);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&p, b"x").unwrap();
        }
        fn mkdir(&self, rel: &str) {
            std::fs::create_dir_all(self.path.join(rel)).unwrap();
        }
    }

    impl Drop for TmpRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    // ---- grading & report --------------------------------------------------------

    fn conformant_repo(tag: &str) -> TmpRepo {
        let repo = TmpRepo::new(tag);
        repo.touch("AGENTS.md");
        repo.touch("CLAUDE.md");
        repo.mkdir("issues");
        repo.mkdir(".git");
        repo.touch("README.md");
        repo.touch(".gitignore");
        repo.mkdir("crates/pc-core");
        repo.mkdir("crates/pc-cli");
        repo
    }

    fn report_for(repo: &Path, profile: Archetype) -> Report {
        let model = Model::standard();
        let resolution = model.resolve(&Questionnaire::builder(profile).build());
        build_report(&model, &resolution, profile, "target", repo).expect("no I/O fault")
    }

    #[test]
    fn conformant_repo_has_no_gaps_and_exits_zero() {
        let repo = conformant_repo("ok");
        let report = report_for(&repo.path, Archetype::Cli);
        assert_eq!(report.gaps(), 0);
        assert!(report.is_conformant());
        assert_eq!(report.exit_code(), EXIT_CONFORMANT);
        // No graded row is a FAIL.
        assert!(report.checks.iter().all(|c| c.status != CheckStatus::Fail));
    }

    #[test]
    fn a_missing_must_scaffold_is_a_gap_and_exits_one() {
        let repo = conformant_repo("gap");
        std::fs::remove_file(repo.path.join("AGENTS.md")).unwrap();
        let report = report_for(&repo.path, Archetype::Cli);
        assert_eq!(report.gaps(), 1);
        assert!(!report.is_conformant());
        assert_eq!(report.exit_code(), EXIT_GAP);
        let doc = report
            .checks
            .iter()
            .find(|c| c.id == "base.doc-pattern")
            .unwrap();
        assert_eq!(doc.status, CheckStatus::Fail);
        assert!(doc.gates);
    }

    #[test]
    fn a_missing_should_is_a_warn_not_a_gap() {
        let repo = conformant_repo("warn");
        std::fs::remove_file(repo.path.join("README.md")).unwrap();
        let report = report_for(&repo.path, Archetype::Cli);
        // README is a SHOULD → warn, never a gap.
        assert_eq!(report.gaps(), 0);
        assert_eq!(report.exit_code(), EXIT_CONFORMANT);
        let readme = report
            .checks
            .iter()
            .find(|c| c.id == "base.readme")
            .unwrap();
        assert_eq!(readme.status, CheckStatus::Warn);
        assert!(!readme.gates);
    }

    #[test]
    fn behavioral_canon_sections_are_deferred_not_failed() {
        let repo = conformant_repo("defer");
        let report = report_for(&repo.path, Archetype::Cli);
        // §1 (strict input validation) is behavioral — no mechanical probe → deferred, gates:false.
        let s1 = report.checks.iter().find(|c| c.id == "canon.s01").unwrap();
        assert_eq!(s1.status, CheckStatus::Skipped);
        assert_eq!(s1.skip_reason, Some(SkipReason::DeferredToReview));
        assert!(!s1.gates);
    }

    #[test]
    fn conditional_sections_are_na_under_conservative_defaults() {
        let repo = conformant_repo("na");
        let report = report_for(&repo.path, Archetype::Cli);
        // §11 (dry-run) is gated by Q3, off by default → n/a, never a fail.
        let s11 = report.checks.iter().find(|c| c.id == "canon.s11").unwrap();
        assert_eq!(s11.status, CheckStatus::Skipped);
        assert_eq!(
            s11.skip_reason,
            Some(SkipReason::NotApplicable { gated_by: "Q3" })
        );
    }

    #[test]
    fn json_report_carries_the_schema_and_summary() {
        let repo = conformant_repo("json");
        let report = report_for(&repo.path, Archetype::Cli);
        let json = report.to_json().to_string();
        assert!(json.contains("\"schema_version\":1"));
        assert!(json.contains("\"verb\":\"doctor\""));
        assert!(json.contains("\"profile\":\"cli\""));
        assert!(json.contains("\"conformant\":true"));
        assert!(json.contains("\"exit_code\":0"));
        assert!(json.contains("\"surface_shape\":\"flat-verb\""));
    }

    #[test]
    fn human_matrix_hides_skipped_unless_verbose() {
        let repo = conformant_repo("human");
        let report = report_for(&repo.path, Archetype::Cli);
        let terse = report.render_human(false);
        assert!(!terse.contains("SKIP"), "{terse}");
        assert!(terse.contains("conformant"));
        let verbose = report.render_human(true);
        assert!(verbose.contains("SKIP"), "{verbose}");
    }

    #[test]
    fn non_cli_profile_has_null_surface_shape() {
        let repo = conformant_repo("svc");
        let report = report_for(&repo.path, Archetype::Service);
        assert_eq!(report.surface_shape, None);
        assert!(report
            .to_json()
            .to_string()
            .contains("\"surface_shape\":null"));
    }

    // ---- strict-validation holes & invariants ------------------------------------

    #[test]
    fn valueless_flags_reject_an_inline_value() {
        // §1: `--json=false` (etc.) is a usage error, never a silently-discarded value.
        for arg in [
            "--json=false",
            "--verbose=0",
            "--assume-defaults=no",
            "--help=x",
        ] {
            let err = parse(&[arg]).unwrap_err();
            assert!(
                err.contains("does not take a value"),
                "{arg} should be rejected, got: {err}"
            );
        }
    }

    #[test]
    fn double_dash_stops_flag_parsing() {
        // A repo path beginning with `-` is addressable after `--`.
        let Command::Run(a) = parse(&["--", "-weird-repo"]).unwrap() else {
            panic!("expected Run");
        };
        assert_eq!(a.repo, "-weird-repo");
        // A second positional after `--` is still rejected.
        assert!(parse(&["--", "a", "b"]).is_err());
    }

    #[test]
    fn empty_profile_value_is_rejected() {
        // `--profile=` yields an empty archetype token → invalid, not a silent default.
        assert!(parse(&["--profile="]).is_err());
    }

    #[test]
    fn every_fail_gates_and_deferred_never_gates() {
        // The invariant `gaps()` relies on: a graded FAIL always gates; a deferred/n-a skip never
        // does. If a future probe produces a non-gating FAIL, this catches it.
        let repo = conformant_repo("inv");
        std::fs::remove_file(repo.path.join("AGENTS.md")).unwrap();
        let report = report_for(&repo.path, Archetype::Cli);
        for c in &report.checks {
            if c.status == CheckStatus::Fail {
                assert!(c.gates, "a FAIL row must gate: {}", c.id);
            }
            if c.skip_reason == Some(SkipReason::DeferredToReview) {
                assert!(!c.gates, "a deferred row must not gate: {}", c.id);
            }
        }
    }
}
