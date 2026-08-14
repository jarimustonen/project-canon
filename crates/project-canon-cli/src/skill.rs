//! The `skill` meta-verb — install / list / print the companion AI-skills (canon §15/§16/§17).
//!
//! project-canon is the maintained home of `AGENTS-AI-FIRST-CLI.md` (ADR 0009 §6), and that canon
//! *itself* prescribes the shape of a companion-skill installer: §15 (`skill list`/`install`),
//! §16 (`skill print` — the read-only twin of install), §17 (skill↔CLI version sync via the
//! `cli_version`/`schema_version` frontmatter + a drift warning on install). This verb dogfoods
//! that surface, so the canon reaches adopting repos as a **versioned, installable skill** rather
//! than a hand-copied markdown file that drifts (issue `canon-installable-skill`).
//!
//! ## The skill it ships
//!
//! v0 ships exactly one skill, `ai-first-cli-canon`: the canon *content* as a reference skill,
//! distinct from the `cli-canon` *behavior* skill (the reviewer/generator). Its body is not a
//! second checked-in copy of the canon — it is **assembled from the single master** via
//! [`include_str!`], the same master `new` bundles. There is therefore no drifting second copy;
//! the single-source invariant is asserted in the integration tests.
//!
//! ## Side-effect discipline
//!
//! `install` writes skill files under `--target` (default `$HOME` → §15's `~/.claude/skills/`).
//! That is its only effect: it never shells out, never touches the network. `--dry-run` computes
//! the full per-file plan and writes nothing. `list`/`print` are read-only. All per-file actions
//! are resolved up front (pure), and a *blocking* conflict (a foreign file, or an on-disk skill
//! newer than the running binary) aborts the whole run before any write — atomic and predictable.

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
/// Claude and Codex skill bodies embed exactly these bytes, so no second copy can drift.
const CANON: &str = include_str!("../../../AGENTS-AI-FIRST-CLI.md");

/// A stable provenance marker written into every installed skill. Its presence identifies a file
/// as **project-canon-managed** (so re-install upgrades it in place rather than refusing to
/// clobber a user's file); the `cli_version=` field drives the §17 drift decision. The
/// version-independent prefix is matched to detect "ours"; the version is parsed from the field.
const MARKER_PREFIX: &str = "<!-- Installed by `project-canon skill install`";

// ===== exit codes ===================================================================
/// Success — installed/upgraded/unchanged, a dry-run plan, or a list/print.
const EXIT_OK: u8 = 0;
/// Usage / operational error — bad flag, bad `--agent`, unknown skill, a blocking clobber/version
/// conflict without `--force`, an I/O fault, or malformed `PROJECT_CANON_*` env. (Uniform with
/// new/review: exit `1` is reserved for a gate outcome, which `skill` has none of. §16 literally
/// says unknown-name in `print` exits 1; binary-wide consistency wins — see design.md.)
const EXIT_USAGE: u8 = 2;

/// A skill shipped by this binary. The body is generated (canon via [`include_str!`]), never a
/// stored file — see the module docs.
struct ShippedSkill {
    name: &'static str,
    description: &'static str,
}

/// The shipped-skill catalog. v0 ships the canon reference skill only; add rows as more land.
const SHIPPED: &[ShippedSkill] = &[ShippedSkill {
    name: "ai-first-cli-canon",
    description: "The AI-first CLI canon (AGENTS-AI-FIRST-CLI.md, \u{a7}1\u{2013}\u{a7}22): the family's binding conventions for any CLI surface \u{2014} strict input validation, --json output, JSONL logs, non-interactive operation, informative errors, meaningful exit codes, composable commands. Reference this when designing or changing this repo's CLI surface.",
}];

fn lookup_skill(name: &str) -> Option<&'static ShippedSkill> {
    SHIPPED.iter().find(|s| s.name == name)
}

// ===== dispatch =====================================================================

/// Run `project-canon skill <sub> …` (the args *after* `skill`). Owns all of the verb's I/O.
pub fn run(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("install") => install::run(&args[1..]),
        Some("list") => list::run(&args[1..]),
        Some("print") => print::run(&args[1..]),
        None | Some("--help") => {
            print!("{HELP}");
            ExitCode::from(EXIT_OK)
        }
        Some(other) => {
            eprintln!("project-canon skill: unknown subcommand or flag: {other:?}");
            eprintln!("known: install, list, print (try `project-canon skill --help`)");
            ExitCode::from(EXIT_USAGE)
        }
    }
}

const HELP: &str = "\
project-canon skill — install / list / print the companion AI-skills (canon \u{a7}15/\u{a7}16/\u{a7}17)

USAGE:
    project-canon skill install [<name>] [FLAGS]
    project-canon skill list [--json]
    project-canon skill print <name> [--agent claude|codex] [--json]

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

    /// The install path for `name` under `base`, per this agent's layout.
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
            // §17: the Claude form declares both version fields in frontmatter.
            Agent::Claude => format!(
                "---\nname: {}\ndescription: {}\ncli_version: \"{CLI_VERSION}\"\nschema_version: {SKILL_SCHEMA_VERSION}\n---\n\n{provenance}\n\n{CANON}",
                skill.name, skill.description,
            ),
            // The Codex form carries no frontmatter; the provenance comment still records the
            // version so drift detection works uniformly across both forms.
            Agent::Codex => format!("{provenance}\n\n{CANON}"),
        }
    }
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

/// Strict §1 validation of the env override layer, uniform with the family. `skill` consumes no
/// env field itself, but still refuses to run under a malformed `PROJECT_CANON_*` value.
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
        // whole run before any file is touched (atomic — no half-installed state).
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
        let blockers: Vec<&Row> = rows.iter().filter(|r| r.action.is_blocking()).collect();
        if !blockers.is_empty() {
            for b in &blockers {
                eprintln!(
                    "project-canon skill install: refusing to write {} ({})",
                    b.path,
                    b.action.blocking_reason().unwrap_or("conflict")
                );
            }
            eprintln!("pass --force to overwrite.");
            return ExitCode::from(EXIT_USAGE);
        }

        // Apply the writes (skipped under --dry-run).
        if !parsed.dry_run {
            for r in &rows {
                if let Some(content) = &r.desired {
                    if r.action.writes() {
                        if let Err(source) = write_file(Path::new(&r.path), content) {
                            eprintln!("project-canon skill install: writing {}: {source}", r.path);
                            return ExitCode::from(EXIT_USAGE);
                        }
                    }
                }
            }
        }

        // A §17 drift warning (an older on-disk skill was upgraded) goes to stderr, never flips
        // the exit code — advisory, uniform with the canon's WARN discipline.
        for r in &rows {
            if let Some(note) = &r.note {
                eprintln!("project-canon skill install: {}: {note}", r.path);
            }
        }

        let report = Report {
            target: abs(&base),
            agents: parsed.agents.clone(),
            dry_run: parsed.dry_run,
            force: parsed.force,
            rows,
        };
        if parsed.json {
            println!("{}", report.to_json());
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
        /// The bytes install would write — `None` only for `Unchanged`/blocked rows where we do
        /// not (re)write.
        pub desired: Option<String>,
        pub action: Action,
        pub note: Option<String>,
    }

    /// An I/O fault inspecting one path.
    pub(super) struct Fault {
        pub path: String,
        pub source: std::io::Error,
    }

    /// Resolve the action for every (skill × agent) target against the current tree. Pure of
    /// writes; a read error other than NotFound faults.
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
                let existing = match std::fs::read(&path) {
                    Ok(bytes) => Some(bytes),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                    Err(source) => {
                        return Err(Fault {
                            path: path.display().to_string(),
                            source,
                        })
                    }
                };
                let (action, note) = decide(existing.as_deref(), desired.as_bytes(), force);
                let write_bytes = matches!(
                    action,
                    Action::Install | Action::Upgrade | Action::Overwrite
                );
                rows.push(Row {
                    name: skill.name,
                    agent,
                    path: path.display().to_string(),
                    desired: write_bytes.then_some(desired),
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
        /// Present but blocked (foreign file, or newer on-disk) — needs `--force`.
        Blocked(BlockReason),
        /// A blocked case the caller passed `--force` for — overwrite.
        Overwrite,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum BlockReason {
        /// A file exists at the path that project-canon did not write.
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
                    Some("a non-managed file already exists here")
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

    /// Decide the action for one file from its current bytes (if any) and the desired bytes.
    pub(super) fn decide(
        existing: Option<&[u8]>,
        desired: &[u8],
        force: bool,
    ) -> (Action, Option<String>) {
        let Some(cur) = existing else {
            return (Action::Install, None);
        };
        if cur == desired {
            return (Action::Unchanged, None);
        }
        // Differs. Is it ours?
        let ours = contains(cur, MARKER_PREFIX.as_bytes());
        if !ours {
            return if force {
                (
                    Action::Overwrite,
                    Some("overwrote a non-managed file (--force)".to_string()),
                )
            } else {
                (Action::Blocked(BlockReason::Foreign), None)
            };
        }
        // Ours and stale — apply the §17 drift rule off the on-disk cli_version.
        match on_disk_cli_version(cur) {
            Some(old) if cmp_versions(&old, CLI_VERSION) == std::cmp::Ordering::Greater => {
                if force {
                    (
                        Action::Overwrite,
                        Some(format!(
                            "downgraded on-disk cli_version {old} to {CLI_VERSION} (--force)"
                        )),
                    )
                } else {
                    (Action::Blocked(BlockReason::NewerOnDisk), None)
                }
            }
            Some(old) if cmp_versions(&old, CLI_VERSION) == std::cmp::Ordering::Less => (
                Action::Upgrade,
                Some(format!(
                    "upgraded skill from cli_version {old} to {CLI_VERSION}"
                )),
            ),
            // Equal version but different bytes (canon body changed within a version), or an
            // unparseable on-disk version — refresh in place.
            _ => (Action::Upgrade, None),
        }
    }

    /// Extract the `cli_version=` field from an installed file's provenance marker.
    fn on_disk_cli_version(bytes: &[u8]) -> Option<String> {
        let text = String::from_utf8_lossy(bytes);
        let rest = text.split_once("cli_version=")?.1;
        let ver: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
        (!ver.is_empty()).then_some(ver)
    }

    /// A dotted-numeric version compare (`0.0.0` vs `0.1.0`). Non-numeric components fall back to a
    /// lexical compare of the whole string — enough for the v0 `0.0.0` line while giving the §17
    /// drift logic real ordering once versions move.
    fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
        let parse =
            |s: &str| -> Option<Vec<u64>> { s.split('.').map(|p| p.parse::<u64>().ok()).collect() };
        match (parse(a), parse(b)) {
            (Some(a), Some(b)) => a.cmp(&b),
            _ => a.cmp(b),
        }
    }

    /// Substring search over bytes (no UTF-8 requirement on the on-disk file).
    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        if needle.is_empty() {
            return true;
        }
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    /// Write one skill file, creating parents. Overwrites (this is the upgrade path).
    fn write_file(path: &Path, content: &str) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
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

        fn to_json(&self) -> Json {
            let files = self
                .rows
                .iter()
                .map(|r| {
                    let mut obj = vec![
                        ("name".into(), Json::str(r.name)),
                        ("agent".into(), Json::str(r.agent.slug())),
                        ("path".into(), Json::str(r.path.clone())),
                        ("action".into(), Json::str(r.action.as_str())),
                    ];
                    if let Some(note) = &r.note {
                        obj.push(("note".into(), Json::str(note.clone())));
                    }
                    Json::Object(obj)
                })
                .collect();

            let summary = Json::Object(vec![
                ("files".into(), Json::Int(self.rows.len() as i64)),
                (
                    "installed".into(),
                    Json::Int(self.count(Action::Install) as i64),
                ),
                (
                    "upgraded".into(),
                    Json::Int((self.count(Action::Upgrade) + self.count(Action::Overwrite)) as i64),
                ),
                (
                    "unchanged".into(),
                    Json::Int(self.count(Action::Unchanged) as i64),
                ),
                // `written` = rows actually written this run (0 under --dry-run).
                (
                    "written".into(),
                    Json::Int(if self.dry_run {
                        0
                    } else {
                        self.rows.iter().filter(|r| r.action.writes()).count() as i64
                    }),
                ),
            ]);

            Json::Object(vec![
                ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
                ("tool".into(), Json::str("project-canon")),
                ("verb".into(), Json::str("skill install")),
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
                ("exit_code".into(), Json::Int(EXIT_OK as i64)),
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

        #[test]
        fn absent_installs() {
            let (a, note) = decide(None, b"x", false);
            assert_eq!(a, Action::Install);
            assert!(note.is_none());
        }

        #[test]
        fn identical_is_unchanged() {
            let desired = canon_bytes(Agent::Claude);
            let (a, _) = decide(Some(desired.as_bytes()), desired.as_bytes(), false);
            assert_eq!(a, Action::Unchanged);
        }

        #[test]
        fn foreign_file_is_blocked_without_force() {
            let (a, _) = decide(Some(b"hand-written notes"), b"desired", false);
            assert_eq!(a, Action::Blocked(BlockReason::Foreign));
            // --force overwrites it.
            let (a, note) = decide(Some(b"hand-written notes"), b"desired", true);
            assert_eq!(a, Action::Overwrite);
            assert!(note.unwrap().contains("non-managed"));
        }

        #[test]
        fn our_stale_file_upgrades() {
            // A managed file whose canon body differs → upgrade in place, no force needed.
            let old = format!(
                "{MARKER_PREFIX} \u{2014} ai-first-cli-canon cli_version={CLI_VERSION} schema_version=1. -->\n\nOLD BODY"
            );
            let (a, _) = decide(Some(old.as_bytes()), b"new desired", false);
            assert_eq!(a, Action::Upgrade);
        }

        #[test]
        fn newer_on_disk_is_blocked_without_force() {
            let newer = format!(
                "{MARKER_PREFIX} \u{2014} ai-first-cli-canon cli_version=99.0.0 schema_version=1. -->\n\nBODY"
            );
            let (a, _) = decide(Some(newer.as_bytes()), b"desired", false);
            assert_eq!(a, Action::Blocked(BlockReason::NewerOnDisk));
            let (a, note) = decide(Some(newer.as_bytes()), b"desired", true);
            assert_eq!(a, Action::Overwrite);
            assert!(note.unwrap().contains("downgraded"));
        }

        #[test]
        fn older_on_disk_upgrades_with_a_note() {
            let older = format!(
                "{MARKER_PREFIX} \u{2014} ai-first-cli-canon cli_version=0.0.0 schema_version=1. -->\n\nBODY"
            );
            // Only meaningful when the binary version is > 0.0.0; otherwise this equals CLI_VERSION
            // and refreshes without a note. Either way it must be a non-blocking write.
            let (a, _) = decide(Some(older.as_bytes()), b"desired", false);
            assert!(a.writes() && !a.is_blocking());
        }

        #[test]
        fn version_compare_orders_numerically() {
            use std::cmp::Ordering::*;
            assert_eq!(cmp_versions("0.0.0", "0.1.0"), Less);
            assert_eq!(cmp_versions("1.2.3", "1.2.3"), Equal);
            assert_eq!(cmp_versions("2.0.0", "1.9.9"), Greater);
            // 10 > 9 numerically (a lexical compare would get this wrong).
            assert_eq!(cmp_versions("0.10.0", "0.9.0"), Greater);
        }

        #[test]
        fn on_disk_version_parses_from_the_marker() {
            let s = format!("{MARKER_PREFIX} \u{2014} x cli_version=1.2.3 schema_version=1. -->");
            assert_eq!(on_disk_cli_version(s.as_bytes()).as_deref(), Some("1.2.3"));
            assert_eq!(on_disk_cli_version(b"no marker here"), None);
        }

        #[test]
        fn rendered_forms_embed_the_canon_and_marker() {
            let claude = canon_bytes(Agent::Claude);
            let codex = canon_bytes(Agent::Codex);
            // Single source: both forms embed the master canon verbatim.
            assert!(claude.contains(CANON));
            assert!(codex.contains(CANON));
            // Claude carries frontmatter; Codex does not.
            assert!(claude.starts_with("---\nname: ai-first-cli-canon"));
            assert!(!codex.starts_with("---"));
            // Both carry the provenance marker.
            assert!(claude.contains(MARKER_PREFIX));
            assert!(codex.contains(MARKER_PREFIX));
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

        if json {
            let skills = SHIPPED
                .iter()
                .map(|s| {
                    Json::Object(vec![
                        ("name".into(), Json::str(s.name)),
                        ("description".into(), Json::str(s.description)),
                        ("cli_version".into(), Json::str(CLI_VERSION)),
                        ("schema_version".into(), Json::Int(SKILL_SCHEMA_VERSION)),
                    ])
                })
                .collect();
            println!(
                "{}",
                Json::Object(vec![
                    ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
                    ("tool".into(), Json::str("project-canon")),
                    ("verb".into(), Json::str("skill list")),
                    ("skills".into(), Json::Array(skills)),
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

// ===== `skill print` ================================================================

mod print {
    use super::*;

    pub fn run(args: &[String]) -> ExitCode {
        let mut name: Option<String> = None;
        let mut agent = Agent::Claude;
        let mut agent_set = false;
        let mut json = false;

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            let (flag, inline) = split_flag(arg);
            match flag {
                "--help" => {
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
            println!(
                "{}",
                Json::Object(vec![
                    ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
                    ("name".into(), Json::str(skill.name)),
                    ("cli_version".into(), Json::str(CLI_VERSION)),
                    (
                        "schema_version_skill".into(),
                        Json::Int(SKILL_SCHEMA_VERSION)
                    ),
                    ("agent".into(), Json::str(agent.slug())),
                    ("content".into(), Json::str(content)),
                    // The synthetic skill's source of truth (§16 path_in_repo).
                    ("path_in_repo".into(), Json::str("AGENTS-AI-FIRST-CLI.md")),
                ])
            );
        } else {
            // Byte-identical to what install would write for this agent (§16).
            print!("{content}");
        }
        ExitCode::from(EXIT_OK)
    }

    fn usage(sub: &str, msg: &str) -> ExitCode {
        eprintln!("project-canon skill {sub}: {msg}");
        ExitCode::from(EXIT_USAGE)
    }
}
