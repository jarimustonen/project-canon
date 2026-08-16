//! The `skill` meta-verb — install / list / print the companion AI-skills (canon §15/§16/§17).
//!
//! project-canon is the maintained home of `AGENTS-AI-FIRST-CLI.md` (ADR 0009 §6), and that canon
//! *itself* prescribes the shape of a companion-skill installer: §15 (`skill list`/`install`,
//! `show`), §16 (`skill print` — the read-only twin of install), §17 (skill↔CLI version sync via
//! the `cli_version`/`schema_version` frontmatter + a drift warning on install). This verb
//! dogfoods that surface, so the canon reaches adopting repos as a **versioned, installable
//! skill** rather than a hand-copied markdown file that drifts (issue `canon-installable-skill`).
//!
//! ## The skill it ships
//!
//! v0 ships exactly one skill, `ai-first-cli-canon`: the canon *content* as a reference skill,
//! distinct from the `cli-canon` *behavior* skill (the reviewer/generator). Its body is not a
//! second checked-in copy of the canon — it is **assembled from the single master**
//! ([`project_canon_core::CANON`]), the same master `new` bundles. There is therefore no drifting
//! second copy;
//! the single-source invariant is asserted in the integration tests.
//!
//! ## Side-effect discipline
//!
//! `install` writes skill files under `--target` (default `$HOME` → §15's `~/.claude/skills/`).
//! That is its only effect: it never shells out, never touches the network. `--dry-run` computes
//! the full per-file plan and writes nothing. `list`/`print` are read-only. All per-file actions
//! are resolved up front (pure); a *blocking* conflict (a foreign/non-regular file, or an on-disk
//! skill newer than the running binary) aborts the whole run before any write. Each file is then
//! written **atomically** (temp file + rename over the target — which never follows a final-
//! component symlink); this is per-file atomic, **not** cross-file transactional, so a mid-run
//! I/O failure can still leave a subset of the files installed (reported, non-zero exit).

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use project_canon_core::EnvConfigLayer;

use crate::json::Json;

/// The `--json` payload schema version (§10). Bump on any breaking shape change.
const SCHEMA_VERSION: i64 = 1;

/// The skill-format version (§17 `schema_version:` — the §10 contract applied to the skill
/// payload itself, so an agent can detect a breaking skill-format change independently of the
/// tool's data schema). Bump when the emitted SKILL.md/prompt *shape* changes incompatibly.
const SKILL_SCHEMA_VERSION: i64 = 1;

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

// ===== exit codes ===================================================================
/// Success — installed/upgraded/unchanged, a dry-run plan, or a list/print.
const EXIT_OK: u8 = 0;
/// Usage / operational error — bad flag, bad `--agent`, unknown skill, a blocking clobber/version
/// conflict without `--force`, an I/O fault, or malformed `PROJECT_CANON_*` env. (Uniform with
/// new/review: exit `1` is reserved for a gate outcome, which `skill` has none of. §16 literally
/// says unknown-name in `print` exits 1; binary-wide consistency wins — see design.md.)
const EXIT_USAGE: u8 = 2;

/// A skill shipped by this binary. The body is generated (canon via [`project_canon_core::CANON`]),
/// never a stored file — see the module docs.
struct ShippedSkill {
    name: &'static str,
    description: &'static str,
}

/// The shipped-skill catalog. v0 ships the canon reference skill only; add rows as more land.
/// Every `name` must be a strict path-safe slug — asserted for the whole table in the tests, since
/// it is interpolated into filesystem paths ([`Agent::path`]) and YAML frontmatter.
const SHIPPED: &[ShippedSkill] = &[ShippedSkill {
    name: "ai-first-cli-canon",
    description: "The AI-first CLI canon (AGENTS-AI-FIRST-CLI.md, \u{a7}1\u{2013}\u{a7}22): the family's binding conventions for any CLI surface \u{2014} strict input validation, --json output, JSONL logs, non-interactive operation, informative errors, meaningful exit codes, composable commands. Reference this when designing or changing this repo's CLI surface.",
}];

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
        Some(other) => {
            eprintln!("project-canon skill: unknown subcommand or flag: {other:?}");
            eprintln!(
                "known: install, list, print (alias: show) (try `project-canon skill --help`)"
            );
            ExitCode::from(EXIT_USAGE)
        }
    }
}

const HELP: &str = "\
project-canon skill — install / list / print the companion AI-skills (canon \u{a7}15/\u{a7}16/\u{a7}17)

USAGE:
    project-canon skill install [<name>] [FLAGS]
    project-canon skill list [--json]
    project-canon skill print <name> [--agent claude|codex] [--json]   (alias: show)

The one shipped skill, `ai-first-cli-canon`, is the AI-first CLI canon as a versioned,
installable reference skill (single-sourced from AGENTS-AI-FIRST-CLI.md).

INSTALL FLAGS:
    --target <dir>          Install base (default: $HOME \u{2192} ~/.claude/skills/). Pass a repo
                            root to install into that repo's agent dirs.
    --agent <claude|codex|all>   Which runtime layout(s) to write (default: all).
    --force                 Overwrite a newer on-disk skill or a non-managed file at the path.
    --dry-run               Print the per-file plan; write nothing.
    --json                  Emit the structured \u{a7}10 report on stdout.

SIDE EFFECTS:
    install writes skill files under <target> and nothing else \u{2014} it never shells out or
    touches the network. list/print are read-only. --dry-run writes nothing.

EXIT CODES:
    0   success (installed/upgraded/unchanged, dry-run plan, list, or print)
    2   usage/operational error (bad flag/--agent, unknown skill, a blocking clobber/version
        conflict without --force, an I/O fault, or malformed PROJECT_CANON_* env)
";

// ===== agents =======================================================================

/// A supported agent runtime and its on-disk skill layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Agent {
    /// Claude Code: `<base>/.claude/skills/<name>/SKILL.md` (YAML frontmatter + body).
    Claude,
    /// Codex: `<base>/.codex/prompts/<name>.md` (no frontmatter — matches the shipped /issue form).
    Codex,
}

impl Agent {
    fn slug(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
        }
    }

    /// The install path for `name` under `base`, per this agent's layout. `name` is a validated
    /// slug (no separators — see [`is_valid_skill_name`]), so the `join` cannot escape `base`.
    fn path(self, base: &Path, name: &str) -> PathBuf {
        match self {
            Agent::Claude => base.join(".claude/skills").join(name).join("SKILL.md"),
            Agent::Codex => base.join(".codex/prompts").join(format!("{name}.md")),
        }
    }

    /// The exact file bytes to install for `skill` under this agent's layout.
    fn render(self, skill: &ShippedSkill) -> String {
        let provenance = provenance_line(skill.name);
        match self {
            // §17: the Claude form declares both version fields in frontmatter. The description is
            // emitted as a YAML double-quoted scalar so a `: ` (or any YAML-significant char) in it
            // cannot break frontmatter parsing.
            Agent::Claude => format!(
                "---\nname: {}\ndescription: {}\ncli_version: \"{CLI_VERSION}\"\nschema_version: {SKILL_SCHEMA_VERSION}\n---\n\n{provenance}\n\n{CANON}",
                skill.name,
                yaml_double_quote(skill.description),
            ),
            // The Codex form carries no frontmatter; the provenance comment still records the
            // version so drift detection works uniformly across both forms.
            Agent::Codex => format!("{provenance}\n\n{CANON}"),
        }
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
        "codex" => Ok(vec![Agent::Codex]),
        "all" if allow_all => Ok(vec![Agent::Claude, Agent::Codex]),
        _ => {
            let valid = if allow_all {
                "claude/codex/all"
            } else {
                "claude/codex"
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

/// Print `content` to stdout, treating a broken pipe (`skill print … | head`) as success rather
/// than the panic Rust's `print!` macro raises on a closed stdout. Used by the streaming paths.
fn write_stdout(content: &str) -> ExitCode {
    let mut out = std::io::stdout().lock();
    match out.write_all(content.as_bytes()) {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::from(EXIT_OK),
        Err(e) => {
            eprintln!("project-canon skill: writing stdout: {e}");
            ExitCode::from(EXIT_USAGE)
        }
    }
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
                eprintln!("project-canon skill install: {err}");
                eprintln!("try `project-canon skill --help`");
                return ExitCode::from(EXIT_USAGE);
            }
        };

        if let Err(err) = validate_env() {
            eprintln!("project-canon skill install: {err}");
            return ExitCode::from(EXIT_USAGE);
        }

        // Which skills: the named one (validated), else all shipped (§15 "installs all").
        let skills: Vec<&ShippedSkill> = match &parsed.skill {
            Some(name) => match lookup_skill(name) {
                Some(s) => vec![s],
                None => {
                    eprintln!(
                        "project-canon skill install: unknown skill {name:?} (available: {})",
                        SHIPPED
                            .iter()
                            .map(|s| s.name)
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    return ExitCode::from(EXIT_USAGE);
                }
            },
            None => SHIPPED.iter().collect(),
        };

        // Resolve the install base: --target, else $HOME (→ §15's ~/.claude/skills/).
        let base = match resolve_base(&parsed.target) {
            Ok(b) => b,
            Err(err) => {
                eprintln!("project-canon skill install: {err}");
                return ExitCode::from(EXIT_USAGE);
            }
        };

        // Resolve every per-file action first (pure of writes), so a blocking conflict aborts the
        // whole run before any file is touched.
        let rows = match resolve_rows(&skills, &parsed.agents, &base, parsed.force) {
            Ok(rows) => rows,
            Err(fault) => {
                eprintln!(
                    "project-canon skill install: cannot inspect {}: {}",
                    fault.path, fault.source
                );
                return ExitCode::from(EXIT_USAGE);
            }
        };

        let report = Report {
            target: abs(&base),
            agents: parsed.agents.clone(),
            dry_run: parsed.dry_run,
            force: parsed.force,
            rows,
        };

        // Blocking conflicts: refuse the whole run before any write. Under --json the caller still
        // gets a structured envelope (status "blocked", exit_code 2) rather than only stderr text.
        if report.rows.iter().any(|r| r.action.is_blocking()) {
            if parsed.json {
                println!("{}", report.to_json(EXIT_USAGE, "blocked"));
            } else {
                for b in report.rows.iter().filter(|r| r.action.is_blocking()) {
                    eprintln!(
                        "project-canon skill install: refusing to write {} ({})",
                        b.path,
                        b.action.blocking_reason().unwrap_or("conflict")
                    );
                }
                eprintln!("pass --force to overwrite.");
            }
            return ExitCode::from(EXIT_USAGE);
        }

        // Apply the writes (skipped under --dry-run). Each file is written atomically (temp file +
        // rename); a mid-run failure leaves the earlier files installed and is reported.
        if !parsed.dry_run {
            for r in &report.rows {
                if let (Some(content), true) = (&r.desired, r.action.writes()) {
                    if let Err(source) = write_file_atomic(Path::new(&r.path), content) {
                        eprintln!("project-canon skill install: writing {}: {source}", r.path);
                        return ExitCode::from(EXIT_USAGE);
                    }
                }
            }
            // §17 drift / overwrite notes go to stderr only on a real run — advisory, never flips
            // the exit code (uniform with the canon's WARN discipline).
            for r in &report.rows {
                if let Some(note) = &r.note {
                    eprintln!("project-canon skill install: {}: {note}", r.path);
                }
            }
        }

        if parsed.json {
            println!("{}", report.to_json(EXIT_OK, "ok"));
        } else {
            print!("{}", report.render_human());
        }
        ExitCode::from(EXIT_OK)
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
                "--force" => {
                    reject_inline("--force", inline)?;
                    set_flag(&mut force, "--force")?;
                }
                "--dry-run" => {
                    reject_inline("--dry-run", inline)?;
                    set_flag(&mut dry_run, "--dry-run")?;
                }
                "--json" => {
                    reject_inline("--json", inline)?;
                    set_flag(&mut json, "--json")?;
                }
                "--target" => {
                    if target.is_some() {
                        return Err("repeated flag: --target".to_string());
                    }
                    target = Some(take_value("--target", inline, &mut iter)?);
                }
                "--agent" => {
                    if agents.is_some() {
                        return Err("repeated flag: --agent".to_string());
                    }
                    agents = Some(parse_agent(
                        &take_value("--agent", inline, &mut iter)?,
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
            agents: agents.unwrap_or_else(|| vec![Agent::Claude, Agent::Codex]),
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

    /// One planned skill file: its target path, the bytes to write, and the resolved action.
    #[derive(Debug)]
    pub(super) struct Row {
        pub name: &'static str,
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
                let path = agent.path(base, skill.name);
                let desired = agent.render(skill);
                let existing = match std::fs::symlink_metadata(&path) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Existing::Absent,
                    Err(source) => {
                        return Err(Fault {
                            path: path.display().to_string(),
                            source,
                        })
                    }
                    Ok(md) if !md.file_type().is_file() => Existing::NonRegular,
                    Ok(_) => match std::fs::read(&path) {
                        Ok(bytes) => Existing::Regular(bytes),
                        // A file that vanished between the stat and the read: treat as absent.
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Existing::Absent,
                        Err(source) => {
                            return Err(Fault {
                                path: path.display().to_string(),
                                source,
                            })
                        }
                    },
                };
                let (action, note) = decide(&existing, desired.as_bytes(), force);
                rows.push(Row {
                    name: skill.name,
                    agent,
                    path: path.display().to_string(),
                    desired: action.writes().then_some(desired),
                    action,
                    note,
                });
            }
        }
        Ok(rows)
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
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum BlockReason {
        /// A file exists at the path that project-canon did not write (or a non-regular file).
        Foreign,
        /// The on-disk skill's `cli_version` is newer than the running binary (§17).
        NewerOnDisk,
    }

    impl Action {
        pub(super) fn writes(self) -> bool {
            matches!(self, Action::Install | Action::Upgrade | Action::Overwrite)
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
            }
        }
    }

    /// Decide the action for one file from what is currently at the path and the desired bytes.
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

    /// True when `cur` is a project-canon-managed skill: the marker appears at the **anchored**
    /// position — the file start (Codex) or the first body line right after the YAML frontmatter
    /// (Claude). This deliberately does NOT match a marker floating anywhere in the bytes, so a
    /// user file that merely quotes the marker is treated as foreign, not silently upgraded.
    fn is_ours(cur: &[u8]) -> bool {
        let text = String::from_utf8_lossy(cur);
        if text.starts_with(MARKER_PREFIX) {
            return true; // Codex form.
        }
        // Claude form: `---\n<frontmatter>\n---\n\n<marker>…`.
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
    }

    impl Report {
        fn count(&self, action: Action) -> usize {
            self.rows.iter().filter(|r| r.action == action).count()
        }

        /// The §10 payload. `exit_code`/`status` are passed in so the blocked path can emit a
        /// structured error envelope (status "blocked", exit 2) rather than only stderr text.
        fn to_json(&self, exit_code: u8, status: &str) -> Json {
            let files = self
                .rows
                .iter()
                .map(|r| {
                    let mut obj = vec![
                        ("name".into(), Json::str(r.name)),
                        ("agent".into(), Json::str(r.agent.slug())),
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

            let written = if self.dry_run {
                0
            } else {
                self.rows.iter().filter(|r| r.action.writes()).count()
            };
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
                    "blocked".into(),
                    Json::Int(self.rows.iter().filter(|r| r.action.is_blocking()).count() as i64),
                ),
                // `written` = rows actually written this run (0 under --dry-run or a blocked run).
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
                let verb = if self.dry_run && r.action.writes() {
                    "would-".to_string() + r.action.as_str()
                } else {
                    r.action.as_str().to_string()
                };
                out.push_str(&format!("  {:<16} {} [{}]\n", verb, r.path, r.agent.slug()));
            }
            let written = if self.dry_run {
                0
            } else {
                self.rows.iter().filter(|r| r.action.writes()).count()
            };
            let verb = if self.dry_run { "would write" } else { "wrote" };
            out.push_str(&format!(
                "summary: {verb} {written} file{}, {} unchanged\n",
                if written == 1 { "" } else { "s" },
                self.count(Action::Unchanged),
            ));
            out
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn canon_bytes(agent: Agent) -> String {
            agent.render(&SHIPPED[0])
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
            // Codex-form managed file (marker at start), older/equal body → upgrade, no force.
            let old = format!(
                "{MARKER_PREFIX} \u{2014} ai-first-cli-canon cli_version={CLI_VERSION} schema_version=1. -->\n\nOLD BODY"
            );
            let (a, _) = decide(&reg(old.as_bytes()), b"new desired", false);
            assert_eq!(a, Action::Upgrade);
        }

        #[test]
        fn claude_form_marker_is_anchored_after_frontmatter() {
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
            assert!(a.writes() && !a.is_blocking());
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
        fn rendered_forms_embed_the_canon_and_marker() {
            let claude = canon_bytes(Agent::Claude);
            let codex = canon_bytes(Agent::Codex);
            assert!(claude.contains(CANON));
            assert!(codex.contains(CANON));
            assert!(claude.starts_with("---\nname: ai-first-cli-canon"));
            assert!(!codex.starts_with("---"));
            assert!(claude.contains(MARKER_PREFIX));
            assert!(codex.contains(MARKER_PREFIX));
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
        fn shipped_names_are_path_safe_slugs() {
            for s in SHIPPED {
                assert!(
                    is_valid_skill_name(s.name),
                    "shipped skill name {:?} is not a path-safe slug",
                    s.name
                );
            }
        }

        #[test]
        fn canon_master_has_no_leading_frontmatter_delimiter() {
            // Guards a future refactor: the Claude form uses `---` as the frontmatter fence, so the
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
                        eprintln!("project-canon skill list: {err}");
                        return ExitCode::from(EXIT_USAGE);
                    }
                    print!("{HELP}");
                    return ExitCode::from(EXIT_OK);
                }
                "--json" => {
                    if let Err(err) =
                        reject_inline("--json", inline).and_then(|()| set_flag(&mut json, "--json"))
                    {
                        eprintln!("project-canon skill list: {err}");
                        return ExitCode::from(EXIT_USAGE);
                    }
                }
                other => {
                    eprintln!("project-canon skill list: unexpected argument: {other:?}");
                    return ExitCode::from(EXIT_USAGE);
                }
            }
        }

        if let Err(err) = validate_env() {
            eprintln!("project-canon skill list: {err}");
            return ExitCode::from(EXIT_USAGE);
        }

        if json {
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
                    ])
                })
                .collect();
            println!(
                "{}",
                Json::Object(vec![
                    ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
                    ("tool".into(), Json::str("project-canon")),
                    ("verb".into(), Json::str("skill list")),
                    ("cli_version".into(), Json::str(CLI_VERSION)),
                    ("skills".into(), Json::Array(skills)),
                    ("exit_code".into(), Json::Int(EXIT_OK as i64)),
                ])
            );
        } else {
            for s in SHIPPED {
                println!(
                    "{}  (cli_version {CLI_VERSION})\n    {}",
                    s.name, s.description
                );
            }
        }
        ExitCode::from(EXIT_OK)
    }
}

// ===== `skill print` (alias: `skill show`) ==========================================

mod print {
    use super::*;

    pub fn run(args: &[String]) -> ExitCode {
        let mut name: Option<String> = None;
        let mut agent = Agent::Claude;
        let mut agent_set = false;
        let mut json = false;
        let mut positional_only = false;

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if positional_only {
                if name.is_some() {
                    return usage("print", &format!("unexpected extra argument: {arg:?}"));
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
                        return usage("print", &err);
                    }
                    print!("{HELP}");
                    return ExitCode::from(EXIT_OK);
                }
                "--json" => {
                    if let Err(err) =
                        reject_inline("--json", inline).and_then(|()| set_flag(&mut json, "--json"))
                    {
                        return usage("print", &err);
                    }
                }
                "--agent" => {
                    if agent_set {
                        return usage("print", "repeated flag: --agent");
                    }
                    let value = match take_value("--agent", inline, &mut iter) {
                        Ok(v) => v,
                        Err(err) => return usage("print", &err),
                    };
                    match parse_agent(&value, false) {
                        Ok(a) => agent = a[0],
                        Err(err) => return usage("print", &err),
                    }
                    agent_set = true;
                }
                other if other.starts_with('-') => {
                    return usage("print", &format!("unknown flag: {other}"));
                }
                _ => {
                    if name.is_some() {
                        return usage("print", &format!("unexpected extra argument: {arg:?}"));
                    }
                    name = Some(arg.clone());
                }
            }
        }

        if let Err(err) = validate_env() {
            return usage("print", &err);
        }

        let name = match name {
            Some(n) => n,
            None => return usage("print", "missing skill name (usage: skill print <name>)"),
        };
        let skill = match lookup_skill(&name) {
            Some(s) => s,
            None => {
                return usage(
                    "print",
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

        let content = agent.render(skill);
        if json {
            // §16 structured payload: metadata + content, routed separately from the body.
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
                ("content".into(), Json::str(content)),
                // The synthetic skill's source of truth (§16 path_in_repo).
                ("path_in_repo".into(), Json::str("AGENTS-AI-FIRST-CLI.md")),
                ("exit_code".into(), Json::Int(EXIT_OK as i64)),
            ]);
            write_stdout(&format!("{payload}\n"))
        } else {
            // Byte-identical to what install would write for this agent (§16).
            write_stdout(&content)
        }
    }

    fn usage(sub: &str, msg: &str) -> ExitCode {
        eprintln!("project-canon skill {sub}: {msg}");
        ExitCode::from(EXIT_USAGE)
    }
}
