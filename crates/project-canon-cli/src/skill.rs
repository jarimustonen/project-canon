//! The `skill` meta-verb — install / list / print the companion AI-skills (canon §15/§16/§17).
//!
//! project-canon is the maintained home of `AGENTS-AI-FIRST-CLI.md` (ADR 0009 §6), and that canon
//! *itself* prescribes the shape of a companion-skill installer: §15 (`skill list`/`install`,
//! `show`), §16 (`skill print` — the read-only twin of install), §17 (skill↔CLI version sync via
//! the `cli_version`/`schema_version` frontmatter + a drift warning on install). This verb
//! dogfoods that surface, so the canon reaches adopting repos as a **versioned, installable
//! skill** rather than a hand-copied markdown file that drifts (issue `canon-installable-skill`).
//!
//! ## The skills it ships
//!
//! The catalog keeps two artifacts deliberately distinct: `ai-first-cli-canon` is synthetic
//! canon *content*, assembled from [`project_canon_core::CANON`], while `cli-canon` is the
//! hand-authored reviewer/generator *behavior* skill. The latter's complete resource tree is
//! packaged inside this crate and installed together for every supported runtime.
//!
//! ## Side-effect discipline
//!
//! `install` writes skill files under `--target` (default `$HOME`) in the native Claude, pi,
//! and Codex layouts required by §15; default and explicit `all` both select all three.
//! That is its only effect: it never shells out, never touches the network. `--dry-run` computes
//! the full per-file plan and writes nothing. `list`/`print` are read-only. All per-file actions
//! are resolved up front (pure); a *blocking* conflict (a foreign/non-regular file, or an on-disk
//! skill newer than the running binary) aborts the whole run before any write. Each file is then
//! written **atomically** (temp file + rename over the target — which never follows a final-
//! component symlink); this is per-file atomic, **not** cross-file transactional, so a mid-run
//! I/O failure can still leave a subset of the files installed (reported, non-zero exit). Once
//! all native writes succeed, managed legacy Codex prompts are removed; foreign files and
//! symlinks are preserved.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use project_canon_core::EnvConfigLayer;

use crate::error::{fail, json_requested, write_stdout, CliError};
use crate::json::Json;

/// The `--json` payload schema version (§10). Bump on any breaking shape change.
const SCHEMA_VERSION: i64 = 1;

/// The skill-format version (§17 `schema_version:` — the §10 contract applied to the skill
/// payload itself, so an agent can detect a breaking skill-format change independently of the
/// tool's data schema). Bump when the emitted SKILL.md/prompt *shape* changes incompatibly.
const SKILL_SCHEMA_VERSION: i64 = 2;

/// The CLI release the shipped skill bodies were written against (§17 `cli_version:`). Pinned to
/// the running binary so `skill print` is always version-consistent with `--version`.
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The canon master, bundled verbatim — the single source of truth (ADR 0009 §6). Both the
/// Claude and Codex skill bodies embed exactly these bytes, so no second copy can drift. The
/// physical master is packaged in `project-canon-core`; this imports
/// [`project_canon_core::CANON`] so the whole workspace shares one copy.
use project_canon_core::CANON;

/// A stable provenance marker written into every installed skill. It identifies a file as
/// **project-canon-managed** (so re-install upgrades it in place), but only when it appears at the
/// *anchored* position (`is_ours`) — file start (Codex) or the first body line after the
/// frontmatter (Claude) — so a user file that merely quotes the marker is never mistaken for ours.
/// The `cli_version=` field (parsed only from within the marker) drives the §17 drift decision.
const MARKER_PREFIX: &str = "<!-- Installed by `project-canon skill install`";

// Installer interface constants feed both behavior and its `skill list --json` declaration so the
// capability object cannot drift from this binary's parser/layout implementation.
const AGENT_FLAG: &str = "--agent";
const TARGET_FLAG: &str = "--target";
const DRY_RUN_FLAG: &str = "--dry-run";
const FORCE_FLAG: &str = "--force";
const ALL_AGENTS: &str = "all";

// ===== exit codes ===================================================================
/// Success — installed/upgraded/unchanged, a dry-run plan, or a list/print.
const EXIT_OK: u8 = 0;

/// One source resource in a bundled skill tree.
struct SkillResource {
    path: &'static str,
    content: &'static str,
}

#[derive(Clone, Copy)]
enum SkillKind {
    SyntheticCanon,
    ResourceTree(&'static [SkillResource]),
}

/// A skill shipped by this binary.
struct ShippedSkill {
    name: &'static str,
    description: &'static str,
    source_path: &'static str,
    kind: SkillKind,
}

const CLI_CANON_RESOURCES: &[SkillResource] = &[
    SkillResource {
        path: "SKILL.md",
        content: include_str!("../skills/cli-canon/SKILL.md"),
    },
    SkillResource {
        path: "templates/conformance-probes.md",
        content: include_str!("../skills/cli-canon/templates/conformance-probes.md"),
    },
    SkillResource {
        path: "templates/generate-plan.md",
        content: include_str!("../skills/cli-canon/templates/generate-plan.md"),
    },
    SkillResource {
        path: "templates/review-report.md",
        content: include_str!("../skills/cli-canon/templates/review-report.md"),
    },
];

/// The complete shipped-skill catalog. Every name and resource path is validated in tests before
/// it can flow into an install path.
const SHIPPED: &[ShippedSkill] = &[
    ShippedSkill {
        name: "ai-first-cli-canon",
        description: "The AI-first CLI canon v4 (AGENTS-AI-FIRST-CLI.md, \u{a7}1\u{2013}\u{a7}24): the family's binding conventions for any CLI surface, including verified deferrals, neutral public artifacts, strict input validation, --json output, JSONL logs, non-interactive operation, informative errors, meaningful exit codes, and composable commands. Reference this when designing or changing this repo's CLI surface.",
        source_path: "AGENTS-AI-FIRST-CLI.md",
        kind: SkillKind::SyntheticCanon,
    },
    ShippedSkill {
        name: "cli-canon",
        description: "Apply the AI-first CLI canon as a behavioral reviewer/generator, using the bundled conformance probes and review/generation templates.",
        source_path: "skills/cli-canon/SKILL.md",
        kind: SkillKind::ResourceTree(CLI_CANON_RESOURCES),
    },
];

/// Metadata for `version --json`'s bundled-skill drift contract (§17).
pub(crate) fn bundled_skill_metadata() -> Vec<(&'static str, &'static str, i64)> {
    SHIPPED
        .iter()
        .map(|skill| (skill.name, CLI_VERSION, SKILL_SCHEMA_VERSION))
        .collect()
}

fn lookup_skill(name: &str) -> Option<&'static ShippedSkill> {
    SHIPPED.iter().find(|s| s.name == name)
}

/// A strict path-safe / YAML-safe skill slug: ASCII lowercase letters, digits, and `-`, starting
/// with a letter. This is the boundary that keeps a catalog name out of trouble everywhere it
/// flows — it can be neither `..`/`/` (path traversal in [`Agent::path`]) nor a YAML-significant
/// token in the Claude frontmatter. Enforced over the whole `SHIPPED` table by a test (its only
/// consumer — the catalog is static, so this is a compile-time-shaped invariant, not a runtime gate).
#[cfg_attr(not(test), allow(dead_code))]
fn is_valid_skill_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

// ===== dispatch =====================================================================

/// Run `project-canon skill <sub> …` (the args *after* `skill`). Owns all of the verb's I/O.
pub fn run(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("install") => install::run(&args[1..]),
        Some("list") => list::run(&args[1..]),
        // `show` is §15's name for the read-only streamer; §16 calls it `print`. They are the same
        // operation, so `show` is a straight alias — dogfooding both canonical names.
        Some("print") | Some("show") => print::run(&args[1..]),
        None | Some("--help") => {
            print!("{HELP}");
            ExitCode::from(EXIT_OK)
        }
        Some(other) => fail(
            json_requested(args),
            CliError::actionable(
                "usage_error",
                format!(
                    "skill: unknown subcommand or flag: {other:?}; known: install, list, print (alias: show)"
                ),
            ),
        )
    }
}

const HELP: &str = "\
project-canon skill — install / list / print the companion AI-skills (canon \u{a7}15/\u{a7}16/\u{a7}17)

USAGE:
    project-canon skill install [<name>] [FLAGS]
    project-canon skill list [--json]
    project-canon skill print <name> [--agent claude|pi|codex] [--resource <path>] [--json]
        (alias: show)

Bundled skills:
    ai-first-cli-canon   Synthetic canon content, single-sourced from AGENTS-AI-FIRST-CLI.md.
    cli-canon            Behavioral reviewer/generator plus its complete templates/ tree.

`print` defaults to SKILL.md. For resource-tree skills, pass --resource <relative-path>;
`--json` lists every printable resource. Claude, pi, and Codex all receive native Agent Skills
resource trees.

INSTALL FLAGS:
    --target <dir>          Install base (default: $HOME). Pass a repo root to install into
                            that repo's native agent dirs.
    --agent <claude|pi|codex|all>   Which runtime layout(s) to write (default: all).
    --force                 Overwrite a newer on-disk skill or a non-managed file at the path.
    --dry-run               Print the per-file plan; write nothing.
    --json                  Emit the structured \u{a7}10 report on stdout.

PRINT FLAGS:
    --agent <claude|pi|codex>   Render the selected runtime form (default: claude).
    --resource <path>       Native-tree resource to print (default: SKILL.md).
    --json                  Emit metadata, selected content, and the complete resource list.

SIDE EFFECTS:
    install writes skill files under <target> and never shells out or touches the network.
    When Codex is selected, it removes same-scope managed legacy .codex/prompts files from
    this or an older version. Foreign files, symlinks, and newer managed prompts are preserved,
    including with --force. list/print are read-only. --dry-run writes nothing.

EXIT CODES:
    0   success (installed/upgraded/unchanged, dry-run plan, list, or print)
    2   usage/operational error (bad flag/--agent, unknown skill, a blocking clobber/version
        conflict without --force, an I/O fault, or malformed PROJECT_CANON_* env)
";

// ===== agents =======================================================================

/// A supported agent runtime and its on-disk skill layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Agent {
    /// The `claude` layout: `<base>/.claude/skills/<name>/<resource>`.
    Claude,
    /// The `pi` layout: `<base>/.pi/agent/skills/<name>/<resource>`.
    Pi,
    /// The `codex` layout: `<base>/.codex/skills/<name>/<resource>`.
    Codex,
}

impl Agent {
    fn slug(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Pi => "pi",
            Agent::Codex => "codex",
        }
    }

    fn root(self) -> &'static str {
        match self {
            Agent::Claude => ".claude/skills",
            Agent::Pi => ".pi/agent/skills",
            Agent::Codex => ".codex/skills",
        }
    }

    fn layout_path(self) -> &'static str {
        match self {
            Agent::Claude => ".claude/skills/<name>/...",
            Agent::Pi => ".pi/agent/skills/<name>/...",
            Agent::Codex => ".codex/skills/<name>/...",
        }
    }

    fn layout_form(self) -> &'static str {
        "agent-skill-tree"
    }

    fn path(self, base: &Path, name: &str, resource: &str) -> PathBuf {
        base.join(self.root()).join(name).join(resource)
    }

    /// Resources materialized by this runtime. Every runtime preserves the complete relative
    /// resource tree; synthetic canon content consists only of `SKILL.md`.
    fn resources(self, skill: &ShippedSkill) -> Vec<&'static str> {
        match skill.kind {
            SkillKind::SyntheticCanon => vec!["SKILL.md"],
            SkillKind::ResourceTree(resources) => {
                resources.iter().map(|resource| resource.path).collect()
            }
        }
    }

    fn render(self, skill: &ShippedSkill, resource: &str) -> Option<String> {
        match skill.kind {
            SkillKind::SyntheticCanon => (resource == "SKILL.md").then(|| {
                format!(
                    "---\nname: {}\ndescription: {}\ncli_version: \"{CLI_VERSION}\"\nschema_version: {SKILL_SCHEMA_VERSION}\n---\n\n{}\n\n{CANON}",
                    skill.name,
                    yaml_double_quote(skill.description),
                    provenance_line(skill.name),
                )
            }),
            SkillKind::ResourceTree(resources) => resources
                .iter()
                .find(|candidate| candidate.path == resource)
                .map(|found| render_tree_resource(skill, found)),
        }
    }
}

fn render_tree_resource(skill: &ShippedSkill, resource: &SkillResource) -> String {
    if resource.path == "SKILL.md" {
        let source = resource.content;
        let rest = source
            .strip_prefix("---\n")
            .and_then(|body| body.split_once("\n---\n"))
            .unwrap_or_else(|| panic!("{} must contain YAML frontmatter", skill.source_path));
        format!(
            "---\n{}\ncli_version: \"{CLI_VERSION}\"\nschema_version: {SKILL_SCHEMA_VERSION}\n---\n\n{}\n{}",
            rest.0,
            provenance_line(skill.name),
            rest.1.strip_prefix('\n').unwrap_or(rest.1),
        )
    } else {
        format!("{}\n\n{}", provenance_line(skill.name), resource.content)
    }
}

fn source_path_for(skill: &ShippedSkill, resource: &str) -> String {
    match skill.kind {
        SkillKind::SyntheticCanon => skill.source_path.to_string(),
        SkillKind::ResourceTree(_) if resource == "SKILL.md" => skill.source_path.to_string(),
        SkillKind::ResourceTree(_) => format!("skills/{}/{resource}", skill.name),
    }
}

/// Emit `s` as a YAML double-quoted scalar (escaping `\` and `"`). Double-quoted YAML tolerates
/// `: `, `#`, and other tokens that would break a plain scalar, so any description is safe.
fn yaml_double_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The provenance/marker comment embedded at the top of every installed skill body. Carries the
/// managed-by marker (identifies the file as ours) and the `cli_version` (drives §17 drift).
fn provenance_line(name: &str) -> String {
    format!(
        "{MARKER_PREFIX} \u{2014} {name} cli_version={CLI_VERSION} schema_version={SKILL_SCHEMA_VERSION}. Generated from AGENTS-AI-FIRST-CLI.md; do not hand-edit, re-run to upgrade. -->"
    )
}

/// Parse `--agent`, echoing the bad value and the valid set on failure (§4). `all` is only valid
/// where `allow_all` (install); `print` targets a single form.
fn parse_agent(s: &str, allow_all: bool) -> Result<Vec<Agent>, String> {
    match s {
        "claude" => Ok(vec![Agent::Claude]),
        "pi" => Ok(vec![Agent::Pi]),
        "codex" => Ok(vec![Agent::Codex]),
        ALL_AGENTS if allow_all => Ok(vec![Agent::Claude, Agent::Pi, Agent::Codex]),
        _ => {
            let valid = if allow_all {
                "claude/pi/codex/all"
            } else {
                "claude/pi/codex"
            };
            Err(format!("invalid --agent {s:?} (expected one of {valid})"))
        }
    }
}

// ===== shared parse helpers (uniform with new.rs/review.rs) ==========================

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

/// Pull a value for a value-taking flag: the inline `=value`, else the next argument. Rejects a
/// missing value, an empty value, and a flag-like separated value (a forgotten value) — §1 strict.
fn take_value<'a>(
    flag: &str,
    inline: Option<&str>,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<String, String> {
    let value = match inline {
        Some(v) => v.to_string(),
        None => {
            let next = iter
                .next()
                .cloned()
                .ok_or_else(|| format!("{flag} requires a value"))?;
            if next.starts_with('-') {
                return Err(format!(
                    "{flag} requires a value, got flag-like token {next:?} (use {flag}={next} for a literal dash value)"
                ));
            }
            next
        }
    };
    if value.is_empty() {
        return Err(format!("{flag} requires a non-empty value"));
    }
    Ok(value)
}

/// Split `--flag=value` into `(flag, Some(value))`, else `(arg, None)`.
fn split_flag(arg: &str) -> (&str, Option<&str>) {
    match arg.split_once('=') {
        Some((f, v)) if f.starts_with("--") => (f, Some(v)),
        _ => (arg, None),
    }
}

/// A stable absolute (lexical, un-canonicalized) path — identical whether or not the dir exists.
fn abs(root: &Path) -> String {
    let p = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|c| c.join(root))
            .unwrap_or_else(|_| root.to_path_buf())
    };
    p.display().to_string()
}

/// Strict §1 validation of the env override layer, uniform across all three subcommands. `skill`
/// consumes no env field itself, but still refuses to run under a malformed `PROJECT_CANON_*`
/// value (same posture as new/review). Callers run this *after* `--help` short-circuits.
fn validate_env() -> Result<(), String> {
    EnvConfigLayer::from_env_vars(&std::env::vars().collect())
        .map(|_| ())
        .map_err(|err| err.to_string())
}

// ===== `skill install` ==============================================================

mod install {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Args {
        skill: Option<String>,
        target: Option<String>,
        agents: Vec<Agent>,
        force: bool,
        dry_run: bool,
        json: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Command {
        Help,
        Run(Args),
    }

    pub fn run(args: &[String]) -> ExitCode {
        let parsed = match parse(args) {
            Ok(Command::Help) => {
                print!("{HELP}");
                return ExitCode::from(EXIT_OK);
            }
            Ok(Command::Run(a)) => a,
            Err(err) => {
                return fail(
                    json_requested(args),
                    CliError::actionable("usage_error", format!("skill install: {err}")),
                );
            }
        };

        if let Err(err) = validate_env() {
            return fail(
                parsed.json,
                CliError::actionable("validation_error", format!("skill install: {err}")),
            );
        }

        // Which skills: the named one (validated), else all shipped (§15 "installs all").
        let skills: Vec<&ShippedSkill> = match &parsed.skill {
            Some(name) => match lookup_skill(name) {
                Some(s) => vec![s],
                None => {
                    return fail(
                        parsed.json,
                        CliError::actionable(
                            "not_found",
                            format!(
                                "skill install: unknown skill {name:?} (available: {})",
                                SHIPPED
                                    .iter()
                                    .map(|s| s.name)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        ),
                    );
                }
            },
            None => SHIPPED.iter().collect(),
        };

        // Resolve the install base: --target, else $HOME; each selected runtime appends its
        // §15 native destination.
        let base = match resolve_base(&parsed.target) {
            Ok(b) => b,
            Err(err) => {
                return fail(
                    parsed.json,
                    CliError::actionable("validation_error", format!("skill install: {err}")),
                );
            }
        };

        // Resolve every per-file action first (pure of writes), so a blocking conflict aborts the
        // whole run before any file is touched.
        let rows = match resolve_rows(&skills, &parsed.agents, &base, parsed.force) {
            Ok(rows) => rows,
            Err(fault) => {
                return fail(
                    parsed.json,
                    CliError::system(
                        "io_error",
                        format!(
                            "skill install: cannot inspect {}: {}",
                            fault.path, fault.source
                        ),
                    ),
                );
            }
        };

        let mut report = Report {
            target: abs(&base),
            agents: parsed.agents.clone(),
            dry_run: parsed.dry_run,
            force: parsed.force,
            rows,
            actual_legacy_removed: 0,
        };

        // Blocking conflicts: refuse the whole run before any write. Under --json the caller gets
        // the central structured error envelope on stderr, with no stdout data.
        if report.rows.iter().any(|r| r.action.is_blocking()) {
            let blocked = report
                .rows
                .iter()
                .find(|r| r.action.is_blocking())
                .expect("blocking row exists after any check");
            return fail(
                parsed.json,
                CliError::actionable(
                    "already_exists",
                    format!(
                        "skill install: refusing to write {} ({})\npass --force to overwrite.",
                        blocked.path,
                        blocked.action.blocking_reason().unwrap_or("conflict")
                    ),
                ),
            );
        }

        // Apply native writes first (skipped under --dry-run). Only after every write succeeds do
        // we retire managed legacy Codex prompts, so a failed native install never removes the
        // caller's last usable Project Canon artifact.
        if !parsed.dry_run {
            for r in &report.rows {
                if let (Some(content), true) = (&r.desired, r.action.writes_file()) {
                    if let Err(source) = write_file_atomic(Path::new(&r.path), content) {
                        return fail(
                            parsed.json,
                            CliError::system(
                                "io_error",
                                format!("skill install: writing {}: {source}", r.path),
                            ),
                        );
                    }
                }
            }
            let mut actual_legacy_removed = 0;
            for r in &report.rows {
                if r.action.removes_file() {
                    match remove_managed_legacy(&base, Path::new(&r.path), r.name) {
                        Ok(RemovalOutcome::Removed) => actual_legacy_removed += 1,
                        Ok(RemovalOutcome::AlreadyAbsent) => {}
                        Err(source) => {
                            return fail(
                                parsed.json,
                                CliError::system(
                                    "io_error",
                                    format!(
                                        "skill install: removing managed legacy {}: {source}",
                                        r.path
                                    ),
                                ),
                            );
                        }
                    }
                }
            }
            report.actual_legacy_removed = actual_legacy_removed;
            // §17 drift / overwrite notes go to stderr only on a real run — advisory, never flips
            // the exit code (uniform with the canon's WARN discipline).
            for r in &report.rows {
                if let Some(note) = &r.note {
                    eprintln!("project-canon skill install: {}: {note}", r.path);
                }
            }
        }

        let output = if parsed.json {
            format!("{}\n", report.to_json(EXIT_OK, "ok"))
        } else {
            report.render_human()
        };
        write_stdout(&output, parsed.json)
    }

    /// Resolve the install base: `--target` (verbatim, relative-ok), else `$HOME`.
    fn resolve_base(target: &Option<String>) -> Result<PathBuf, String> {
        if let Some(t) = target {
            return Ok(PathBuf::from(t));
        }
        match std::env::var("HOME") {
            Ok(h) if !h.is_empty() => Ok(PathBuf::from(h)),
            _ => Err(
                "no --target given and $HOME is not set (pass --target <dir> for the install base)"
                    .to_string(),
            ),
        }
    }

    fn parse(args: &[String]) -> Result<Command, String> {
        let mut skill: Option<String> = None;
        let mut target: Option<String> = None;
        let mut agents: Option<Vec<Agent>> = None;
        let mut force = false;
        let mut dry_run = false;
        let mut json = false;

        let mut positional_only = false;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if positional_only {
                set_skill(&mut skill, arg)?;
                continue;
            }
            if arg == "--" {
                positional_only = true;
                continue;
            }
            let (flag, inline) = split_flag(arg);
            match flag {
                "--help" => {
                    reject_inline("--help", inline)?;
                    return Ok(Command::Help);
                }
                FORCE_FLAG => {
                    reject_inline(FORCE_FLAG, inline)?;
                    set_flag(&mut force, FORCE_FLAG)?;
                }
                DRY_RUN_FLAG => {
                    reject_inline(DRY_RUN_FLAG, inline)?;
                    set_flag(&mut dry_run, DRY_RUN_FLAG)?;
                }
                "--json" => {
                    reject_inline("--json", inline)?;
                    set_flag(&mut json, "--json")?;
                }
                TARGET_FLAG => {
                    if target.is_some() {
                        return Err(format!("repeated flag: {TARGET_FLAG}"));
                    }
                    target = Some(take_value(TARGET_FLAG, inline, &mut iter)?);
                }
                AGENT_FLAG => {
                    if agents.is_some() {
                        return Err(format!("repeated flag: {AGENT_FLAG}"));
                    }
                    agents = Some(parse_agent(
                        &take_value(AGENT_FLAG, inline, &mut iter)?,
                        true,
                    )?);
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown flag: {other}"));
                }
                _ => set_skill(&mut skill, arg)?,
            }
        }

        Ok(Command::Run(Args {
            skill,
            target,
            agents: agents.unwrap_or_else(|| vec![Agent::Claude, Agent::Pi, Agent::Codex]),
            force,
            dry_run,
            json,
        }))
    }

    /// Record the single optional positional `<name>`, rejecting an empty or second one (§1).
    fn set_skill(skill: &mut Option<String>, arg: &str) -> Result<(), String> {
        if arg.is_empty() {
            return Err("skill name must not be empty".to_string());
        }
        if skill.is_some() {
            return Err(format!("unexpected extra argument: {arg:?}"));
        }
        *skill = Some(arg.to_string());
        Ok(())
    }

    /// One planned native skill file or legacy Codex prompt: its target path, desired bytes, and
    /// resolved action.
    #[derive(Debug)]
    pub(super) struct Row {
        pub name: &'static str,
        pub resource: &'static str,
        pub agent: Agent,
        pub path: String,
        /// The bytes install would write — `None` for `Unchanged`/blocked rows we do not (re)write.
        pub desired: Option<String>,
        pub action: Action,
        pub note: Option<String>,
    }

    /// An I/O fault inspecting one path.
    pub(super) struct Fault {
        pub path: String,
        pub source: std::io::Error,
    }

    /// What is currently at a target path — resolved with **no-follow** metadata so a symlink or
    /// other non-regular file is never read through or written through.
    #[derive(Debug)]
    enum Existing {
        /// Nothing at the path (the install case).
        Absent,
        /// A symlink, directory, FIFO, device, … — never ours; treated as a foreign conflict
        /// (we neither read it nor follow it).
        NonRegular,
        /// A regular file, with its bytes.
        Regular(Vec<u8>),
    }

    /// Resolve the action for every (skill × agent) target against the current tree. Pure of
    /// writes; a read error other than NotFound faults. Uses no-follow `symlink_metadata` first so
    /// a planted symlink/FIFO at the destination is classified as a conflict, never followed.
    fn resolve_rows(
        skills: &[&ShippedSkill],
        agents: &[Agent],
        base: &Path,
        force: bool,
    ) -> Result<Vec<Row>, Fault> {
        let mut rows = Vec::new();
        for skill in skills {
            for &agent in agents {
                for resource in agent.resources(skill) {
                    let path = agent.path(base, skill.name, resource);
                    validate_install_ancestors(base, &path).map_err(|source| Fault {
                        path: path.display().to_string(),
                        source,
                    })?;
                    let desired = agent
                        .render(skill, resource)
                        .expect("catalog resource must render");
                    let existing = inspect_path(&path).map_err(|source| Fault {
                        path: path.display().to_string(),
                        source,
                    })?;
                    let (action, note) = decide(&existing, desired.as_bytes(), force);
                    rows.push(Row {
                        name: skill.name,
                        resource,
                        agent,
                        path: path.display().to_string(),
                        desired: action.writes_file().then_some(desired),
                        action,
                        note,
                    });
                }
            }
        }

        // Migration scope follows selection scope: only an invocation selecting Codex considers
        // legacy prompts, and a named install considers only that skill. Foreign files, symlinks,
        // and prompts written by a newer binary are reported but never removed.
        if agents.contains(&Agent::Codex) {
            for skill in skills {
                let path = base
                    .join(".codex/prompts")
                    .join(format!("{}.md", skill.name));
                let existing = match validate_install_ancestors(base, &path) {
                    Ok(()) => inspect_path(&path).map_err(|source| Fault {
                        path: path.display().to_string(),
                        source,
                    })?,
                    Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {
                        Existing::NonRegular
                    }
                    Err(source) => {
                        return Err(Fault {
                            path: path.display().to_string(),
                            source,
                        })
                    }
                };
                let action = legacy_action(&existing, skill.name);
                if action != Action::LegacyAbsent {
                    rows.push(Row {
                        name: skill.name,
                        resource: "legacy-prompt",
                        agent: Agent::Codex,
                        path: path.display().to_string(),
                        desired: None,
                        action,
                        note: (action == Action::PreserveLegacy).then(|| {
                            "preserve foreign, non-regular, or newer legacy prompt".to_string()
                        }),
                    });
                }
            }
        }
        Ok(rows)
    }

    fn inspect_path(path: &Path) -> std::io::Result<Existing> {
        match std::fs::symlink_metadata(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Existing::Absent),
            Err(error) => Err(error),
            Ok(metadata) if !metadata.file_type().is_file() => Ok(Existing::NonRegular),
            Ok(_) => match std::fs::read(path) {
                Ok(bytes) => Ok(Existing::Regular(bytes)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Existing::Absent),
                Err(error) => Err(error),
            },
        }
    }

    /// Reject an observed symlink or non-directory in the destination's parent chain. This keeps
    /// an install from following a pre-existing `<base>/.claude`/`.pi`/`.codex` redirect outside
    /// the declared target. The final component has its separate no-follow conflict check below.
    fn validate_install_ancestors(base: &Path, path: &Path) -> std::io::Result<()> {
        let parent = path.parent().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no parent")
        })?;
        let relative = parent.strip_prefix(base).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "install path is outside the declared target base",
            )
        })?;
        // `base` itself is the caller-declared boundary and may intentionally be a symlink (for
        // example a symlinked home). Only redirects introduced *below* that boundary are escapes.
        let mut current = base.to_path_buf();
        for component in relative.components() {
            current.push(component.as_os_str());
            match std::fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!(
                            "install parent is a symlink and may escape --target: {}",
                            current.display()
                        ),
                    ));
                }
                Ok(metadata) if !metadata.is_dir() => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("install parent is not a directory: {}", current.display()),
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// The resolved action for one target file.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum Action {
        /// No file present — write it.
        Install,
        /// Present and byte-identical — leave it (idempotent no-op).
        Unchanged,
        /// Present, ours, and same-or-older on-disk version — write the current bytes (upgrade).
        Upgrade,
        /// Present but blocked (foreign/non-regular file, or newer on-disk) — needs `--force`.
        Blocked(BlockReason),
        /// A blocked case the caller passed `--force` for — overwrite.
        Overwrite,
        /// A positively identified Project Canon legacy Codex prompt from this or an older version.
        RemoveLegacy,
        /// A legacy-path artifact that is foreign, non-regular, or newer; never remove it.
        PreserveLegacy,
        /// No legacy artifact exists. Used internally and omitted from reports.
        LegacyAbsent,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum BlockReason {
        /// A file exists at the path that project-canon did not write (or a non-regular file).
        Foreign,
        /// The on-disk skill's `cli_version` is newer than the running binary (§17).
        NewerOnDisk,
    }

    impl Action {
        pub(super) fn writes_file(self) -> bool {
            matches!(self, Action::Install | Action::Upgrade | Action::Overwrite)
        }
        pub(super) fn removes_file(self) -> bool {
            self == Action::RemoveLegacy
        }
        pub(super) fn mutates(self) -> bool {
            self.writes_file() || self.removes_file()
        }
        pub(super) fn is_blocking(self) -> bool {
            matches!(self, Action::Blocked(_))
        }
        pub(super) fn blocking_reason(self) -> Option<&'static str> {
            match self {
                Action::Blocked(BlockReason::Foreign) => {
                    Some("a non-managed or non-regular file already exists here")
                }
                Action::Blocked(BlockReason::NewerOnDisk) => {
                    Some("on-disk skill is newer than this binary")
                }
                _ => None,
            }
        }
        pub(super) fn as_str(self) -> &'static str {
            match self {
                Action::Install => "install",
                Action::Unchanged => "unchanged",
                Action::Upgrade => "upgrade",
                Action::Blocked(BlockReason::Foreign) => "blocked-foreign",
                Action::Blocked(BlockReason::NewerOnDisk) => "blocked-newer",
                Action::Overwrite => "overwrite",
                Action::RemoveLegacy => "remove-managed-legacy",
                Action::PreserveLegacy => "preserve-legacy",
                Action::LegacyAbsent => "legacy-absent",
            }
        }
    }

    fn legacy_action(existing: &Existing, expected_name: &str) -> Action {
        match existing {
            Existing::Absent => Action::LegacyAbsent,
            Existing::NonRegular => Action::PreserveLegacy,
            Existing::Regular(bytes) => match managed_legacy_version(bytes, expected_name) {
                Some(version)
                    if cmp_versions(&version, CLI_VERSION) != std::cmp::Ordering::Greater =>
                {
                    Action::RemoveLegacy
                }
                _ => Action::PreserveLegacy,
            },
        }
    }

    /// Parse only the exact marker shape emitted at the start of historical Codex prompts.
    /// Native frontmatter-form files, wrong-name markers, incomplete markers, and malformed
    /// versions are not positively identified and therefore remain untouched.
    fn managed_legacy_version(bytes: &[u8], expected_name: &str) -> Option<String> {
        let text = std::str::from_utf8(bytes).ok()?;
        let marker = text.lines().next()?;
        if !marker.starts_with(MARKER_PREFIX) || !marker.ends_with("-->") {
            return None;
        }
        let fields = marker
            .strip_prefix(MARKER_PREFIX)?
            .strip_prefix(" \u{2014} ")?;
        let (name, fields) = fields.split_once(' ')?;
        if name != expected_name {
            return None;
        }
        let version = marker_field(fields, "cli_version=")?;
        let schema = marker_field(fields, "schema_version=")?.trim_end_matches('.');
        if version
            .split('.')
            .any(|part| part.is_empty() || part.parse::<u64>().is_err())
            || schema
                .parse::<u64>()
                .ok()
                .filter(|value| *value > 0)
                .is_none()
        {
            return None;
        }
        Some(version.to_string())
    }

    fn marker_field<'a>(marker: &'a str, key: &str) -> Option<&'a str> {
        marker
            .split_whitespace()
            .find_map(|field| field.strip_prefix(key))
    }

    /// Decide the action for one native file from what is currently at the path and desired bytes.
    /// Notes are tense-neutral (they describe the planned action, so they read correctly under
    /// both `--dry-run` and a real run).
    fn decide(existing: &Existing, desired: &[u8], force: bool) -> (Action, Option<String>) {
        let cur = match existing {
            Existing::Absent => return (Action::Install, None),
            // A symlink/dir/FIFO/… is never ours: block, or overwrite-via-rename under --force.
            Existing::NonRegular => {
                return if force {
                    (
                        Action::Overwrite,
                        Some("overwrite a non-regular file (--force)".to_string()),
                    )
                } else {
                    (Action::Blocked(BlockReason::Foreign), None)
                }
            }
            Existing::Regular(bytes) => bytes.as_slice(),
        };
        if cur == desired {
            return (Action::Unchanged, None);
        }
        // Differs. Is it ours (marker at the anchored position)?
        if !is_ours(cur) {
            return if force {
                (
                    Action::Overwrite,
                    Some("overwrite a non-managed file (--force)".to_string()),
                )
            } else {
                (Action::Blocked(BlockReason::Foreign), None)
            };
        }
        // Ours and stale — apply the §17 drift rule off the on-disk cli_version.
        match marker_cli_version(cur) {
            Some(old) if cmp_versions(&old, CLI_VERSION) == std::cmp::Ordering::Greater => {
                if force {
                    (
                        Action::Overwrite,
                        Some(format!(
                            "downgrade on-disk cli_version {old} \u{2192} {CLI_VERSION} (--force)"
                        )),
                    )
                } else {
                    (Action::Blocked(BlockReason::NewerOnDisk), None)
                }
            }
            Some(old) if cmp_versions(&old, CLI_VERSION) == std::cmp::Ordering::Less => (
                Action::Upgrade,
                Some(format!(
                    "upgrade from cli_version {old} \u{2192} {CLI_VERSION}"
                )),
            ),
            // Equal version but different bytes (canon body changed within a version), or an
            // unparseable on-disk version on a definitely-ours (anchored-marker) file — refresh.
            _ => (Action::Upgrade, None),
        }
    }

    /// True when `cur` is a project-canon-managed artifact: the marker appears at the
    /// **anchored** position — either the file start (legacy Codex prompt) or the first body line
    /// after Agent Skills YAML frontmatter. A marker merely quoted elsewhere remains foreign.
    fn is_ours(cur: &[u8]) -> bool {
        let text = String::from_utf8_lossy(cur);
        if text.starts_with(MARKER_PREFIX) {
            return true; // Legacy Codex prompt form.
        }
        // Native form: `---\n<frontmatter>\n---\n\n<marker>…`.
        if let Some(rest) = text.strip_prefix("---\n") {
            if let Some(idx) = rest.find("\n---\n") {
                let after = &rest[idx + "\n---\n".len()..];
                let after = after.strip_prefix('\n').unwrap_or(after);
                return after.starts_with(MARKER_PREFIX);
            }
        }
        false
    }

    /// Extract `cli_version=` from **within** the provenance marker (from the marker start up to
    /// its `-->` close), so an unrelated `cli_version=` elsewhere in the file cannot spoof it.
    fn marker_cli_version(bytes: &[u8]) -> Option<String> {
        let text = String::from_utf8_lossy(bytes);
        let start = text.find(MARKER_PREFIX)?;
        let marker = &text[start..];
        let marker = marker.split_once("-->").map(|(m, _)| m).unwrap_or(marker);
        let rest = marker.split_once("cli_version=")?.1;
        let ver: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
        (!ver.is_empty()).then_some(ver)
    }

    /// A dotted-numeric version compare (`0.0.0` vs `0.1.0`). Non-numeric components (a SemVer
    /// prerelease/build suffix) fall back to a lexical compare of the whole string — correct for
    /// the v0 `0.0.0` line; a full SemVer compare is a documented follow-up for when releases
    /// (and prerelease tags) begin (assessment D2).
    fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
        let parse =
            |s: &str| -> Option<Vec<u64>> { s.split('.').map(|p| p.parse::<u64>().ok()).collect() };
        match (parse(a), parse(b)) {
            (Some(a), Some(b)) => a.cmp(&b),
            _ => a.cmp(b),
        }
    }

    /// Revalidate a planned migration immediately before deletion. The final component must still
    /// be a regular, managed legacy prompt from this or an older version. An absent file is an
    /// idempotent no-op; any changed/foreign state fails closed rather than deleting it.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum RemovalOutcome {
        Removed,
        AlreadyAbsent,
    }

    fn remove_managed_legacy(
        base: &Path,
        path: &Path,
        expected_name: &str,
    ) -> std::io::Result<RemovalOutcome> {
        validate_install_ancestors(base, path)?;
        match inspect_path(path)? {
            Existing::Absent => Ok(RemovalOutcome::AlreadyAbsent),
            existing if legacy_action(&existing, expected_name) == Action::RemoveLegacy => {
                std::fs::remove_file(path)?;
                Ok(RemovalOutcome::Removed)
            }
            _ => Err(std::io::Error::other(
                "legacy prompt changed after planning; refusing removal",
            )),
        }
    }

    /// Write one skill file **atomically**: create parents, write a sibling temp file, then rename
    /// it over the destination. `rename` replaces the final component in one step and operates on
    /// the link itself (it never follows a final-component symlink into a foreign file), giving
    /// both durability (no truncated half-write) and the symlink-safety the plan relies on. The
    /// temp file is removed on any failure so a fault leaves no `.tmp-…` litter.
    fn write_file_atomic(path: &Path, content: &str) -> std::io::Result<()> {
        let parent = path.parent().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no parent")
        })?;
        std::fs::create_dir_all(parent)?;
        let file_name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name")
        })?;
        let tmp = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
        // create_new so a stale/hostile temp path is never silently reused.
        let write_result = (|| -> std::io::Result<()> {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)?;
            f.write_all(content.as_bytes())?;
            f.sync_all()?;
            Ok(())
        })();
        if let Err(e) = write_result {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        if let Err(e) = std::fs::rename(&tmp, path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        Ok(())
    }

    /// The whole install result.
    struct Report {
        target: String,
        agents: Vec<Agent>,
        dry_run: bool,
        force: bool,
        rows: Vec<Row>,
        actual_legacy_removed: usize,
    }

    impl Report {
        fn count(&self, action: Action) -> usize {
            self.rows.iter().filter(|r| r.action == action).count()
        }

        /// The §10 payload. `exit_code`/`status` are passed in so the blocked path can emit a
        /// central structured error envelope on stderr rather than only prose.
        fn to_json(&self, exit_code: u8, status: &str) -> Json {
            let files = self
                .rows
                .iter()
                .map(|r| {
                    let mut obj = vec![
                        ("name".into(), Json::str(r.name)),
                        ("agent".into(), Json::str(r.agent.slug())),
                        ("resource".into(), Json::str(r.resource)),
                        ("path".into(), Json::str(r.path.clone())),
                        ("action".into(), Json::str(r.action.as_str())),
                        ("blocked".into(), Json::Bool(r.action.is_blocking())),
                    ];
                    if let Some(note) = &r.note {
                        obj.push(("note".into(), Json::str(note.clone())));
                    }
                    Json::Object(obj)
                })
                .collect();

            let planned_writes = self.rows.iter().filter(|r| r.action.writes_file()).count();
            let written = if self.dry_run { 0 } else { planned_writes };
            let planned_removals = self.rows.iter().filter(|r| r.action.removes_file()).count();
            let removed = self.actual_legacy_removed;
            let summary = Json::Object(vec![
                ("files".into(), Json::Int(self.rows.len() as i64)),
                (
                    "installed".into(),
                    Json::Int(self.count(Action::Install) as i64),
                ),
                (
                    "upgraded".into(),
                    Json::Int(self.count(Action::Upgrade) as i64),
                ),
                // A forced overwrite of a foreign/newer file is reported separately from an
                // upgrade — the most destructive action is never hidden inside `upgraded`.
                (
                    "overwritten".into(),
                    Json::Int(self.count(Action::Overwrite) as i64),
                ),
                (
                    "unchanged".into(),
                    Json::Int(self.count(Action::Unchanged) as i64),
                ),
                (
                    "managed_legacy_would_remove".into(),
                    Json::Int(planned_removals as i64),
                ),
                ("managed_legacy_removed".into(), Json::Int(removed as i64)),
                (
                    "legacy_preserved".into(),
                    Json::Int(self.count(Action::PreserveLegacy) as i64),
                ),
                (
                    "blocked".into(),
                    Json::Int(self.rows.iter().filter(|r| r.action.is_blocking()).count() as i64),
                ),
                // Actual changes are zero under --dry-run; planned totals remain explicit.
                ("would_write".into(), Json::Int(planned_writes as i64)),
                ("written".into(), Json::Int(written as i64)),
            ]);

            Json::Object(vec![
                ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
                ("tool".into(), Json::str("project-canon")),
                ("verb".into(), Json::str("skill install")),
                ("status".into(), Json::str(status)),
                ("cli_version".into(), Json::str(CLI_VERSION)),
                ("target".into(), Json::str(self.target.clone())),
                (
                    "agents".into(),
                    Json::Array(self.agents.iter().map(|a| Json::str(a.slug())).collect()),
                ),
                ("dry_run".into(), Json::Bool(self.dry_run)),
                ("force".into(), Json::Bool(self.force)),
                ("files".into(), Json::Array(files)),
                ("summary".into(), summary),
                ("exit_code".into(), Json::Int(exit_code as i64)),
            ])
        }

        fn render_human(&self) -> String {
            let mut out = String::new();
            let mode = if self.dry_run { "  (dry-run)" } else { "" };
            out.push_str(&format!(
                "project-canon skill install: base {}{mode}\n",
                self.target
            ));
            for r in &self.rows {
                let verb = if self.dry_run && r.action.mutates() {
                    "would-".to_string() + r.action.as_str()
                } else {
                    r.action.as_str().to_string()
                };
                out.push_str(&format!("  {:<16} {} [{}]\n", verb, r.path, r.agent.slug()));
            }
            let written = self.rows.iter().filter(|r| r.action.writes_file()).count();
            let planned_removals = self.rows.iter().filter(|r| r.action.removes_file()).count();
            let (remove_verb, removals) = if self.dry_run {
                ("would remove", planned_removals)
            } else {
                ("removed", self.actual_legacy_removed)
            };
            let verb = if self.dry_run { "would write" } else { "wrote" };
            out.push_str(&format!(
                "summary: {verb} {written} file{}, {} unchanged, {remove_verb} {removals} managed legacy, {} legacy preserved\n",
                if written == 1 { "" } else { "s" },
                self.count(Action::Unchanged),
                self.count(Action::PreserveLegacy),
            ));
            out
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn canon_bytes(agent: Agent) -> String {
            agent.render(&SHIPPED[0], "SKILL.md").unwrap()
        }

        fn reg(bytes: &[u8]) -> Existing {
            Existing::Regular(bytes.to_vec())
        }

        #[test]
        fn absent_installs() {
            let (a, note) = decide(&Existing::Absent, b"x", false);
            assert_eq!(a, Action::Install);
            assert!(note.is_none());
        }

        #[test]
        fn identical_is_unchanged() {
            let desired = canon_bytes(Agent::Claude);
            let (a, _) = decide(&reg(desired.as_bytes()), desired.as_bytes(), false);
            assert_eq!(a, Action::Unchanged);
        }

        #[test]
        fn foreign_file_is_blocked_without_force() {
            let (a, _) = decide(&reg(b"hand-written notes"), b"desired", false);
            assert_eq!(a, Action::Blocked(BlockReason::Foreign));
            let (a, note) = decide(&reg(b"hand-written notes"), b"desired", true);
            assert_eq!(a, Action::Overwrite);
            assert!(note.unwrap().contains("non-managed"));
        }

        #[test]
        fn non_regular_file_is_blocked_without_force() {
            let (a, _) = decide(&Existing::NonRegular, b"desired", false);
            assert_eq!(a, Action::Blocked(BlockReason::Foreign));
            let (a, _) = decide(&Existing::NonRegular, b"desired", true);
            assert_eq!(a, Action::Overwrite);
        }

        #[test]
        fn a_marker_only_quoted_in_prose_is_not_ours() {
            // A README that merely mentions the marker mid-file must NOT be classified as ours.
            let prose = format!("# My notes\n\nWe use `{MARKER_PREFIX}`-style tools.\n");
            let (a, _) = decide(&reg(prose.as_bytes()), b"desired", false);
            assert_eq!(
                a,
                Action::Blocked(BlockReason::Foreign),
                "an un-anchored marker must not make a foreign file 'ours'"
            );
        }

        #[test]
        fn our_stale_file_upgrades() {
            // Legacy managed form (marker at start), older/equal body → upgrade, no force.
            let old = format!(
                "{MARKER_PREFIX} \u{2014} ai-first-cli-canon cli_version={CLI_VERSION} schema_version=1. -->\n\nOLD BODY"
            );
            let (a, _) = decide(&reg(old.as_bytes()), b"new desired", false);
            assert_eq!(a, Action::Upgrade);
        }

        #[test]
        fn native_form_marker_is_anchored_after_frontmatter() {
            let claude = canon_bytes(Agent::Claude);
            assert!(is_ours(claude.as_bytes()));
            // The same body with an extra user line pushed ABOVE the frontmatter is not anchored.
            let tampered = format!("hello\n{claude}");
            assert!(!is_ours(tampered.as_bytes()));
        }

        #[test]
        fn newer_on_disk_is_blocked_without_force() {
            let newer = format!(
                "{MARKER_PREFIX} \u{2014} ai-first-cli-canon cli_version=99.0.0 schema_version=1. -->\n\nBODY"
            );
            let (a, _) = decide(&reg(newer.as_bytes()), b"desired", false);
            assert_eq!(a, Action::Blocked(BlockReason::NewerOnDisk));
            let (a, note) = decide(&reg(newer.as_bytes()), b"desired", true);
            assert_eq!(a, Action::Overwrite);
            assert!(note.unwrap().contains("downgrade"));
        }

        #[test]
        fn older_on_disk_upgrades_without_force() {
            let older = format!(
                "{MARKER_PREFIX} \u{2014} ai-first-cli-canon cli_version=0.0.0 schema_version=1. -->\n\nBODY"
            );
            let (a, _) = decide(&reg(older.as_bytes()), b"desired", false);
            assert!(a.writes_file() && !a.is_blocking());
        }

        #[test]
        fn version_compare_orders_numerically() {
            use std::cmp::Ordering::*;
            assert_eq!(cmp_versions("0.0.0", "0.1.0"), Less);
            assert_eq!(cmp_versions("1.2.3", "1.2.3"), Equal);
            assert_eq!(cmp_versions("2.0.0", "1.9.9"), Greater);
            assert_eq!(cmp_versions("0.10.0", "0.9.0"), Greater);
        }

        #[test]
        fn marker_version_parses_only_from_within_the_marker() {
            let s = format!("{MARKER_PREFIX} \u{2014} x cli_version=1.2.3 schema_version=1. -->");
            assert_eq!(marker_cli_version(s.as_bytes()).as_deref(), Some("1.2.3"));
            assert_eq!(marker_cli_version(b"no marker here"), None);
            // A cli_version= placed BEFORE the marker is ignored (only the marker's own is read).
            let spoof = format!("cli_version=9.9.9\n{MARKER_PREFIX} y cli_version=1.0.0 -->");
            assert_eq!(
                marker_cli_version(spoof.as_bytes()).as_deref(),
                Some("1.0.0")
            );
        }

        #[test]
        fn rendered_forms_embed_the_canon_frontmatter_and_marker() {
            for agent in [Agent::Claude, Agent::Pi, Agent::Codex] {
                let rendered = canon_bytes(agent);
                assert!(rendered.contains(CANON));
                assert!(rendered.starts_with("---\nname: ai-first-cli-canon"));
                assert!(rendered.contains(MARKER_PREFIX));
            }
        }

        #[test]
        fn claude_description_is_a_quoted_yaml_scalar() {
            // The description contains `: ` — it must be double-quoted so YAML frontmatter parses.
            let claude = canon_bytes(Agent::Claude);
            assert!(
                claude.contains("description: \""),
                "description must be a double-quoted YAML scalar"
            );
        }

        #[test]
        fn shipped_names_and_resource_paths_are_safe() {
            for skill in SHIPPED {
                assert!(
                    is_valid_skill_name(skill.name),
                    "shipped skill name {:?} is not a path-safe slug",
                    skill.name
                );
                if let SkillKind::ResourceTree(resources) = skill.kind {
                    for resource in resources {
                        let path = Path::new(resource.path);
                        assert!(
                            !path.is_absolute()
                                && path
                                    .components()
                                    .all(|part| matches!(part, std::path::Component::Normal(_))),
                            "unsafe resource path {:?}",
                            resource.path
                        );
                    }
                }
            }
        }

        #[test]
        fn every_bundled_native_skill_render_has_a_compliant_description() {
            for agent in [Agent::Claude, Agent::Pi, Agent::Codex] {
                for skill in SHIPPED {
                    let rendered = agent.render(skill, "SKILL.md").unwrap();
                    let length = crate::probes::skill_description_length(&rendered).unwrap_or_else(
                        |error| {
                            panic!(
                                "{} {} render has invalid frontmatter: {error}",
                                agent.slug(),
                                skill.name
                            )
                        },
                    );
                    assert!(
                        length <= crate::probes::SKILL_DESCRIPTION_MAX_CHARS,
                        "{} {} rendered description is {length} characters (maximum {})",
                        agent.slug(),
                        skill.name,
                        crate::probes::SKILL_DESCRIPTION_MAX_CHARS
                    );
                }
            }
        }

        #[test]
        fn cli_canon_native_forms_expose_every_resource() {
            let skill = lookup_skill("cli-canon").unwrap();
            for agent in [Agent::Claude, Agent::Pi, Agent::Codex] {
                assert_eq!(agent.resources(skill).len(), 4);
            }
            let native = Agent::Codex.render(skill, "SKILL.md").unwrap();
            assert!(native.contains("cli_version:"));
            assert!(native.contains("schema_version:"));
            assert!(native.contains("check shipshape/issuectl/taskfleet"));
            assert!(!native.contains("check ossctl/issuectl/taskfleet"));
        }

        #[test]
        fn shipped_skill_sources_exclude_retired_taskfleet_identities() {
            let retired_product = concat!("orchestrate", "ctl");
            let retired_env_prefix = concat!("O", "CTL_").to_ascii_lowercase();
            let assert_canonical = |source: &str, label: &str| {
                let source = source.to_ascii_lowercase();
                assert!(
                    !source.contains(retired_product),
                    "{label} contains the retired Taskfleet product identity"
                );
                assert!(
                    !source.contains(&retired_env_prefix),
                    "{label} contains the retired Taskfleet environment prefix"
                );
            };

            assert_canonical(CANON, "AI-first CLI canon");
            for resource in CLI_CANON_RESOURCES {
                assert_canonical(resource.content, resource.path);
            }
        }

        #[test]
        fn legacy_action_only_removes_strictly_identified_non_newer_prompts() {
            let marker = |name: &str, version: &str| {
                format!(
                    "{MARKER_PREFIX} — {name} cli_version={version} schema_version=1. Generated. -->\n"
                )
            };
            let equal = marker("ai-first-cli-canon", CLI_VERSION);
            let older = marker("ai-first-cli-canon", "0.0.0");
            let newer = marker("ai-first-cli-canon", "99.0.0");
            assert_eq!(
                legacy_action(&Existing::Absent, "ai-first-cli-canon"),
                Action::LegacyAbsent
            );
            assert_eq!(
                legacy_action(&Existing::NonRegular, "ai-first-cli-canon"),
                Action::PreserveLegacy
            );
            assert_eq!(
                legacy_action(&reg(b"foreign"), "ai-first-cli-canon"),
                Action::PreserveLegacy
            );
            for removable in [equal, older] {
                assert_eq!(
                    legacy_action(&reg(removable.as_bytes()), "ai-first-cli-canon"),
                    Action::RemoveLegacy
                );
            }
            for preserved in [
                newer,
                marker("cli-canon", CLI_VERSION),
                format!("{MARKER_PREFIX} — ai-first-cli-canon schema_version=1. -->"),
                format!(
                    "{MARKER_PREFIX} — ai-first-cli-canon cli_version=bad schema_version=1. -->"
                ),
                format!(
                    "{MARKER_PREFIX} — ai-first-cli-canon cli_version={CLI_VERSION} schema_version=1."
                ),
                canon_bytes(Agent::Codex),
            ] {
                assert_eq!(
                    legacy_action(&reg(preserved.as_bytes()), "ai-first-cli-canon"),
                    Action::PreserveLegacy
                );
            }
        }

        #[test]
        fn removal_reports_an_already_absent_artifact_without_counting_an_unlink() {
            let base = std::env::temp_dir().join(format!(
                "pc-legacy-absent-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let path = base.join(".codex/prompts/ai-first-cli-canon.md");
            assert_eq!(
                remove_managed_legacy(&base, &path, "ai-first-cli-canon").unwrap(),
                RemovalOutcome::AlreadyAbsent
            );
        }

        #[cfg(unix)]
        #[test]
        fn removal_rejects_an_intermediate_symlink() {
            let base = std::env::temp_dir().join(format!(
                "pc-legacy-parent-link-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let external = base.with_extension("external");
            let _ = std::fs::remove_dir_all(&base);
            let _ = std::fs::remove_dir_all(&external);
            std::fs::create_dir_all(base.join(".codex")).unwrap();
            std::fs::create_dir_all(&external).unwrap();
            let external_prompt = external.join("ai-first-cli-canon.md");
            std::fs::write(
                &external_prompt,
                format!(
                    "{MARKER_PREFIX} — ai-first-cli-canon cli_version={CLI_VERSION} schema_version=1. -->\n"
                ),
            )
            .unwrap();
            std::os::unix::fs::symlink(&external, base.join(".codex/prompts")).unwrap();

            let path = base.join(".codex/prompts/ai-first-cli-canon.md");
            assert!(remove_managed_legacy(&base, &path, "ai-first-cli-canon").is_err());
            assert!(external_prompt.is_file());
            std::fs::remove_dir_all(&base).unwrap();
            std::fs::remove_dir_all(&external).unwrap();
        }

        #[test]
        fn canon_master_has_no_leading_frontmatter_delimiter() {
            // Guards a future refactor: native forms use `---` as the frontmatter fence, so the
            // canon body itself must not begin with one.
            assert!(!CANON.starts_with("---"));
        }
    }
}

// ===== `skill list` =================================================================

mod list {
    use super::*;

    pub fn run(args: &[String]) -> ExitCode {
        let mut json = false;
        for arg in args {
            let (flag, inline) = split_flag(arg);
            match flag {
                "--help" => {
                    if let Err(err) = reject_inline("--help", inline) {
                        return fail(
                            json_requested(args),
                            CliError::actionable("usage_error", format!("skill list: {err}")),
                        );
                    }
                    print!("{HELP}");
                    return ExitCode::from(EXIT_OK);
                }
                "--json" => {
                    if let Err(err) =
                        reject_inline("--json", inline).and_then(|()| set_flag(&mut json, "--json"))
                    {
                        return fail(
                            json_requested(args),
                            CliError::actionable("usage_error", format!("skill list: {err}")),
                        );
                    }
                }
                other => {
                    return fail(
                        json_requested(args),
                        CliError::actionable(
                            "usage_error",
                            format!("skill list: unexpected argument: {other:?}"),
                        ),
                    );
                }
            }
        }

        if let Err(err) = validate_env() {
            return fail(
                json,
                CliError::actionable("validation_error", format!("skill list: {err}")),
            );
        }

        let output = if json {
            let skills = SHIPPED
                .iter()
                .map(|s| {
                    Json::Object(vec![
                        ("name".into(), Json::str(s.name)),
                        ("description".into(), Json::str(s.description)),
                        ("cli_version".into(), Json::str(CLI_VERSION)),
                        (
                            "skill_schema_version".into(),
                            Json::Int(SKILL_SCHEMA_VERSION),
                        ),
                        (
                            "resources".into(),
                            Json::Array(
                                Agent::Claude
                                    .resources(s)
                                    .iter()
                                    .map(|path| Json::str(*path))
                                    .collect(),
                            ),
                        ),
                    ])
                })
                .collect();
            format!(
                "{}\n",
                Json::Object(vec![
                    ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
                    ("tool".into(), Json::str("project-canon")),
                    ("verb".into(), Json::str("skill list")),
                    ("cli_version".into(), Json::str(CLI_VERSION)),
                    (
                        "supported_agents".into(),
                        Json::Array(
                            [Agent::Claude, Agent::Pi, Agent::Codex]
                                .into_iter()
                                .map(Agent::slug)
                                .map(Json::str)
                                .collect(),
                        ),
                    ),
                    (
                        "install".into(),
                        Json::Object(vec![
                            ("selection_flag".into(), Json::str(AGENT_FLAG)),
                            ("default".into(), Json::str(ALL_AGENTS)),
                            (
                                "accepted_values".into(),
                                Json::Array(
                                    ["claude", "pi", "codex", ALL_AGENTS]
                                        .into_iter()
                                        .map(Json::str)
                                        .collect(),
                                ),
                            ),
                            ("target_flag".into(), Json::str(TARGET_FLAG)),
                            ("dry_run_flag".into(), Json::str(DRY_RUN_FLAG)),
                            ("force_flag".into(), Json::str(FORCE_FLAG)),
                            ("interactive".into(), Json::Bool(false)),
                            ("no_clobber_default".into(), Json::Bool(true)),
                            ("overwrite_requires_force".into(), Json::Bool(true)),
                            (
                                "layouts".into(),
                                Json::Array(
                                    [Agent::Claude, Agent::Pi, Agent::Codex]
                                        .into_iter()
                                        .map(|agent| {
                                            Json::Object(vec![
                                                ("agent".into(), Json::str(agent.slug())),
                                                ("path".into(), Json::str(agent.layout_path())),
                                                ("form".into(), Json::str(agent.layout_form())),
                                            ])
                                        })
                                        .collect(),
                                ),
                            ),
                        ]),
                    ),
                    ("skills".into(), Json::Array(skills)),
                    ("exit_code".into(), Json::Int(EXIT_OK as i64)),
                ])
            )
        } else {
            SHIPPED
                .iter()
                .map(|s| {
                    format!(
                        "{}  (cli_version {CLI_VERSION})\n    {}\n",
                        s.name, s.description
                    )
                })
                .collect()
        };
        write_stdout(&output, json)
    }
}

// ===== `skill print` (alias: `skill show`) ==========================================

mod print {
    use super::*;

    pub fn run(args: &[String]) -> ExitCode {
        let mut name: Option<String> = None;
        let mut agent = Agent::Claude;
        let mut agent_set = false;
        let mut resource: Option<String> = None;
        let mut json = false;
        let mut positional_only = false;

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if positional_only {
                if name.is_some() {
                    return usage(args, &format!("unexpected extra argument: {arg:?}"));
                }
                name = Some(arg.clone());
                continue;
            }
            if arg == "--" {
                positional_only = true;
                continue;
            }
            let (flag, inline) = split_flag(arg);
            match flag {
                "--help" => {
                    if let Err(err) = reject_inline("--help", inline) {
                        return usage(args, &err);
                    }
                    print!("{HELP}");
                    return ExitCode::from(EXIT_OK);
                }
                "--json" => {
                    if let Err(err) =
                        reject_inline("--json", inline).and_then(|()| set_flag(&mut json, "--json"))
                    {
                        return usage(args, &err);
                    }
                }
                "--agent" => {
                    if agent_set {
                        return usage(args, "repeated flag: --agent");
                    }
                    let value = match take_value("--agent", inline, &mut iter) {
                        Ok(v) => v,
                        Err(err) => return usage(args, &err),
                    };
                    match parse_agent(&value, false) {
                        Ok(a) => agent = a[0],
                        Err(err) => return usage(args, &err),
                    }
                    agent_set = true;
                }
                "--resource" => {
                    if resource.is_some() {
                        return usage(args, "repeated flag: --resource");
                    }
                    resource = match take_value("--resource", inline, &mut iter) {
                        Ok(value) => Some(value),
                        Err(err) => return usage(args, &err),
                    };
                }
                other if other.starts_with('-') => {
                    return usage(args, &format!("unknown flag: {other}"));
                }
                _ => {
                    if name.is_some() {
                        return usage(args, &format!("unexpected extra argument: {arg:?}"));
                    }
                    name = Some(arg.clone());
                }
            }
        }

        if let Err(err) = validate_env() {
            return fail(
                json_requested(args),
                CliError::actionable("validation_error", format!("skill print: {err}")),
            );
        }

        let name = match name {
            Some(n) => n,
            None => return usage(args, "missing skill name (usage: skill print <name>)"),
        };
        let skill = match lookup_skill(&name) {
            Some(s) => s,
            None => {
                return usage(
                    args,
                    &format!(
                        "unknown skill {name:?} (available: {})",
                        SHIPPED
                            .iter()
                            .map(|s| s.name)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
            }
        };

        let resource = resource.as_deref().unwrap_or("SKILL.md");
        let available = agent.resources(skill);
        let content = match agent.render(skill, resource) {
            Some(content) => content,
            None => {
                return usage(
                    args,
                    &format!(
                        "unknown resource {resource:?} for skill {:?} and agent {} (available: {})",
                        skill.name,
                        agent.slug(),
                        available.join(", ")
                    ),
                )
            }
        };
        if json {
            // §16 structured payload: one selected resource plus complete resource discovery.
            let payload = Json::Object(vec![
                ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
                ("tool".into(), Json::str("project-canon")),
                ("verb".into(), Json::str("skill print")),
                ("name".into(), Json::str(skill.name)),
                ("cli_version".into(), Json::str(CLI_VERSION)),
                (
                    "skill_schema_version".into(),
                    Json::Int(SKILL_SCHEMA_VERSION),
                ),
                ("agent".into(), Json::str(agent.slug())),
                ("resource".into(), Json::str(resource)),
                (
                    "resources".into(),
                    Json::Array(available.iter().map(|path| Json::str(*path)).collect()),
                ),
                ("content".into(), Json::str(content)),
                (
                    "path_in_repo".into(),
                    Json::str(source_path_for(skill, resource)),
                ),
                ("exit_code".into(), Json::Int(EXIT_OK as i64)),
            ]);
            write_stdout(&format!("{payload}\n"), true)
        } else {
            // Byte-identical to what install would write for this agent (§16).
            write_stdout(&content, false)
        }
    }

    fn usage(args: &[String], msg: &str) -> ExitCode {
        fail(
            json_requested(args),
            CliError::actionable("usage_error", format!("skill print: {msg}")),
        )
    }
}
