//! The `new` verb — scaffold a repo that **starts conformant** (ADR 0009 §2/§6).
//!
//! It reads the two-layer model from [`project_canon_core`], resolves the target's profile with
//! the conservative (non-interactive) questionnaire, and writes the base-canon files + the
//! selected profile's surface scaffold into a target directory. The external/irreversible
//! bootstrap steps (git init, private GitHub repo, `issuectl init`, `tw` registration) are
//! **emitted as a hook plan, never executed** — see the design note's side-effect boundary.
//!
//! ## Side-effect discipline
//!
//! At v0 `new` makes exactly one kind of side effect: it writes files into the caller's target
//! directory. It never spawns a subprocess — not `gh`, `tw`, `git`, or `issuectl`. Every action
//! beyond writing files is rendered (filled from the [`EnvConfig`] hook layer) into a printed
//! plan the caller runs deliberately. `--dry-run` suppresses the file writes too, so the whole
//! run is side-effect-free. Plan building is pure and unit-tested; I/O is confined to [`apply`].

use std::path::Path;
use std::process::ExitCode;

use project_canon_core::{
    Archetype, Dimension, EnvConfig, EnvConfigLayer, Model, Questionnaire, Resolution, Severity,
    SurfaceShape,
};

use crate::json::Json;

/// The `--json` payload schema version (§10). Bump on any breaking shape change.
const SCHEMA_VERSION: i64 = 1;

// ===== exit codes (see design.md "Exit-code contract") ==============================
/// Success — the plan was printed (`--dry-run`) or the files were written.
const EXIT_OK: u8 = 0;
/// Usage / operational error — bad flag/profile, missing target, clobber guard, I/O, bad env.
/// `new` has no gate semantics, so exit `1` is reserved/unused (kept distinct from doctor's gate).
const EXIT_USAGE: u8 = 2;

/// The canon, bundled verbatim: project-canon is the maintained home of the canon (ADR 0009 §6),
/// so the binary carries it and every scaffolded repo gets a byte-identical copy.
const CANON: &str = include_str!("../../../AGENTS-AI-FIRST-CLI.md");

/// Run `project-canon new <args…>` (the args *after* the `new` subcommand). Owns all of new's
/// I/O and returns the process exit code.
pub fn run(args: &[String]) -> ExitCode {
    // Parse first so `--help` is always an exit-0 event (§2), even under a malformed environment.
    let parsed = match parse_args(args) {
        Ok(Command::Help) => {
            print!("{HELP}");
            return ExitCode::from(EXIT_OK);
        }
        Ok(Command::Run(a)) => a,
        Err(err) => {
            eprintln!("project-canon new: {err}");
            eprintln!("try `project-canon new --help`");
            return ExitCode::from(EXIT_USAGE);
        }
    };

    // Strict §1 validation of the env override layer, uniform with the family: a malformed
    // `PROJECT_CANON_*` value is a usage error, never a silent coerce. Runs *after* `--help`.
    let env_layer = match EnvConfigLayer::from_env_vars(&std::env::vars().collect()) {
        Ok(layer) => layer,
        Err(err) => {
            eprintln!("project-canon new: {err}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    let cfg = EnvConfig::resolve(&[&EnvConfigLayer::empty(), &env_layer]);
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());

    // Derive AND validate the project name: explicit `--name`, else the target dir's final
    // component. Validation is a security boundary, not cosmetics — the name is interpolated into
    // relative paths, Cargo/Rust source, and the printed hook commands, so an unconstrained name
    // is a path-traversal / broken-manifest / shell-injection vector. A strict slug closes all
    // three at the source (a valid crate identifier can be neither `..` nor a flag nor a shell
    // metacharacter).
    let name = match resolve_name(&parsed.name, &parsed.dir).and_then(|n| {
        validate_name(&n)?;
        Ok(n)
    }) {
        Ok(n) => n,
        Err(err) => {
            eprintln!("project-canon new: {err}");
            return ExitCode::from(EXIT_USAGE);
        }
    };

    // A stable, absolute (lexical, un-canonicalized) target path — independent of whether the dir
    // exists yet, so the `--json` `target` and the hook `cwd`/`tw` line are the same in dry-run and
    // real runs. The hooks are rendered against the *actual* target the caller named, never a
    // configured default location.
    let root = Path::new(&parsed.dir);
    let target = abs_target(root);

    // Resolve the model with the conservative questionnaire (all conditionals off — §3, `new`
    // never prompts). `--assume-defaults` names this explicitly; it is the only mode at v0.
    let model = Model::standard();
    let resolution = model.resolve(&Questionnaire::builder(parsed.profile).build());

    let emoji = parsed
        .emoji
        .clone()
        .or_else(|| cfg.workmux_emoji_prefix.clone());
    let plan = build_plan(
        &model,
        &resolution,
        &name,
        parsed.description.as_deref(),
        emoji.as_deref(),
        &cfg,
        &home,
        &target,
    );

    // Clobber guard (§1, no-clobber like §21 init): a non-empty target is refused unless --force.
    // Missing / empty is fine (we create it). A dry-run inspects but must not create the dir. A
    // symlinked target root is refused outright — following it would let writes escape the named
    // directory (the side-effect boundary).
    match dir_state(root) {
        Err(err) => {
            eprintln!(
                "project-canon new: cannot inspect target {:?}: {err}",
                parsed.dir
            );
            return ExitCode::from(EXIT_USAGE);
        }
        Ok(DirState::NotADir) => {
            eprintln!(
                "project-canon new: target {:?} exists but is a symlink or non-directory; refusing (would escape the target)",
                parsed.dir
            );
            return ExitCode::from(EXIT_USAGE);
        }
        Ok(DirState::NonEmpty) if !parsed.force => {
            eprintln!(
                "project-canon new: target {:?} is not empty (use --force to fill gaps; existing files are never overwritten)",
                parsed.dir
            );
            return ExitCode::from(EXIT_USAGE);
        }
        Ok(_) => {}
    }

    // Compute per-file actions (create vs skip-exists) against the current tree, then — only on a
    // real run — write the `create` rows. Existing files are never overwritten (fill-gaps only).
    let file_rows = match resolve_file_actions(&plan, root) {
        Ok(rows) => rows,
        Err(err) => {
            eprintln!("project-canon new: {}: {}", err.rel, err.source);
            return ExitCode::from(EXIT_USAGE);
        }
    };
    if !parsed.dry_run {
        if let Err(err) = apply(&plan, &file_rows, root) {
            eprintln!("project-canon new: writing {}: {}", err.rel, err.source);
            return ExitCode::from(EXIT_USAGE);
        }
    }

    let report = Report {
        target,
        name,
        profile: parsed.profile,
        surface_shape: resolution.surface_shape(),
        dry_run: parsed.dry_run,
        force: parsed.force,
        plan,
        file_rows,
    };

    if parsed.json {
        println!("{}", report.to_json());
    } else {
        print!("{}", report.render_human(parsed.verbose));
    }
    ExitCode::from(EXIT_OK)
}

/// A stable absolute path for the report + hook rendering. Lexical only (no `canonicalize`), so it
/// is identical whether or not the dir exists yet — a relative target is joined onto the process
/// cwd; an absolute one is used verbatim. It is intentionally not normalized (`..` is left as-is);
/// name validation already forbids the traversal vectors that would matter.
fn abs_target(root: &Path) -> String {
    let p = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|c| c.join(root))
            .unwrap_or_else(|_| root.to_path_buf())
    };
    p.display().to_string()
}

/// The project name: `--name` if given, else the final path component of `<dir>`. The result is
/// validated by [`validate_name`] at the call site before it reaches any template or path.
fn resolve_name(explicit: &Option<String>, dir: &str) -> Result<String, String> {
    if let Some(n) = explicit {
        return Ok(n.clone());
    }
    Path::new(dir)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("cannot derive a project name from {dir:?}; pass --name <name>"))
}

/// Reject any name that is not a strict project slug: a leading ASCII letter, then ASCII
/// alphanumerics, `-`, or `_`, at most 64 chars. This is the security boundary that keeps the name
/// out of trouble everywhere it flows — it cannot be `..`/`/` (path traversal), a leading `-`
/// (flag injection into a printed hook), a shell metacharacter (`$`, backtick, `;`), or an invalid
/// Cargo package / Rust identifier. Derived-from-dir names are held to the same bar so a directory
/// like `123` or `my project` fails loudly instead of scaffolding a broken repo.
fn validate_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(format!(
            "invalid project name {name:?}: use ≤64 chars of ASCII letters/digits/'-'/'_', starting with a letter (pass --name to override a bad directory name)"
        ))
    }
}

use crate::shell::shell_quote;

// ===== argument parsing =============================================================

/// A parsed `new` invocation.
#[derive(Debug, PartialEq, Eq)]
struct NewArgs {
    dir: String,
    name: Option<String>,
    description: Option<String>,
    emoji: Option<String>,
    profile: Archetype,
    dry_run: bool,
    force: bool,
    json: bool,
    verbose: bool,
    #[allow(dead_code)] // Accepted & validated; a no-op affirmation of the v0 default mode.
    assume_defaults: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Run(NewArgs),
}

/// Strict argument parsing (§1): unknown flags, a bad `--profile`, a missing/empty value, a
/// repeated flag, an inline value on a valueless flag, and extra positionals are all errors
/// echoing the offending token — never a silent fixup.
fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut dir: Option<String> = None;
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut emoji: Option<String> = None;
    let mut profile: Option<Archetype> = None;
    let mut dry_run = false;
    let mut force = false;
    let mut json = false;
    let mut verbose = false;
    let mut assume_defaults = false;

    let mut positional_only = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if positional_only {
            set_positional(&mut dir, arg)?;
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
            "--dry-run" => {
                reject_inline("--dry-run", inline)?;
                set_flag(&mut dry_run, "--dry-run")?;
            }
            "--force" => {
                reject_inline("--force", inline)?;
                set_flag(&mut force, "--force")?;
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
                profile = Some(parse_archetype(&take_value(
                    "--profile",
                    inline,
                    &mut iter,
                )?)?);
            }
            "--name" => {
                set_value(&mut name, "--name", inline, &mut iter)?;
            }
            "--description" => {
                set_value(&mut description, "--description", inline, &mut iter)?;
            }
            "--emoji" => {
                set_value(&mut emoji, "--emoji", inline, &mut iter)?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag: {other}"));
            }
            _ => set_positional(&mut dir, arg)?,
        }
    }

    let dir = dir.ok_or_else(|| {
        "missing target directory (usage: project-canon new [flags] <dir>)".to_string()
    })?;

    Ok(Command::Run(NewArgs {
        dir,
        name,
        description,
        emoji,
        profile: profile.unwrap_or(Archetype::Cli),
        dry_run,
        force,
        json,
        verbose,
        assume_defaults,
    }))
}

/// Record the single positional `<dir>` argument, rejecting an empty one (the design forbids a cwd
/// default — an empty target would silently scaffold into the process cwd) and a second one (§1).
fn set_positional(dir: &mut Option<String>, arg: &str) -> Result<(), String> {
    if arg.is_empty() {
        return Err("target directory must not be empty".to_string());
    }
    if dir.is_some() {
        return Err(format!("unexpected extra argument: {arg:?}"));
    }
    *dir = Some(arg.to_string());
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

/// Pull a value for a value-taking flag: the inline `=value`, else the next argument. Rejects a
/// missing value and an empty value (§1: an empty `--name`/`--profile`/… is a usage error).
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
            // A separated value that looks like a flag (`--name --json`) is almost always a
            // forgotten value, not an intended dash-prefixed string — reject it rather than
            // silently swallowing the next flag (§1 strict). A genuine dash value uses `=`.
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

/// Set an optional string flag once, rejecting a repeat and an empty value (§1).
fn set_value<'a>(
    slot: &mut Option<String>,
    flag: &str,
    inline: Option<&str>,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("repeated flag: {flag}"));
    }
    *slot = Some(take_value(flag, inline, iter)?);
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
project-canon new — scaffold a repo that starts conformant (generate-only; hooks are printed)

USAGE:
    project-canon new [FLAGS] <dir>

ARGS:
    <dir>                    Target directory to scaffold into (created if absent).

FLAGS:
    --profile <archetype>   cli | service | library | release  (default: cli)
    --name <name>           Project name (default: final component of <dir>).
    --description <text>    One-line description for AGENTS.md / README.md.
    --emoji <glyph>         .workmux.yaml / tw window prefix (default: from env config).
    --assume-defaults       Characterize non-interactively with conservative defaults (v0 default).
    --dry-run               Print the file + hook plan; write nothing.
    --force                 Generate into a non-empty dir (fills gaps; never overwrites a file).
    --json                  Emit the structured §10 plan on stdout.
    --verbose               Also show skip-exists rows + the full resolved conformance list.
    --help                  Show this help.

SIDE EFFECTS:
    new only ever writes files into <dir>. The bootstrap steps (git init, gh repo create,
    issuectl init, tw registration) are PRINTED as a hook plan for you to run — new never
    executes them, and never shells out.

EXIT CODES:
    0   success — plan printed (--dry-run) or files written
    2   usage/operational error (bad flag, bad --profile, missing <dir>, non-empty target
        without --force, an I/O write fault, or malformed PROJECT_CANON_* env)
";

// ===== the scaffold plan (pure) =====================================================

/// A file the scaffold will materialize.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedFile {
    /// Repo-relative path (forward slashes; joined onto the target root at the I/O edge).
    rel: String,
    kind: FileKind,
    /// For [`FileKind::File`], the file body. For [`FileKind::Symlink`], the link target.
    content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileKind {
    File,
    Symlink,
}

impl FileKind {
    fn as_str(self) -> &'static str {
        match self {
            FileKind::File => "file",
            FileKind::Symlink => "symlink",
        }
    }
}

/// Whether a bootstrap hook is a local, reversible step or an external/irreversible one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookClass {
    /// Local & reversible (git init, issuectl init).
    Local,
    /// External / irreversible — creates a GitHub repo, registers with tw, etc.
    External,
}

impl HookClass {
    fn as_str(self) -> &'static str {
        match self {
            HookClass::Local => "local",
            HookClass::External => "external",
        }
    }
}

/// One bootstrap action, rendered from the model + [`EnvConfig`] but **never executed** at v0.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedHook {
    id: &'static str,
    class: HookClass,
    description: String,
    /// The directory the command must be run from — the scaffolded repo. Surfaced so a caller
    /// running the plan doesn't accidentally run `git init` / `gh repo create --source=.` in the
    /// wrong repository.
    cwd: String,
    command: String,
}

/// The full scaffold plan: the files to write, the hook plan to print, and the resolved canon
/// section numbers (for the `--json` report + the `CONFORMANCE.md` skeleton).
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScaffoldPlan {
    files: Vec<PlannedFile>,
    hooks: Vec<PlannedHook>,
    conformance_sections: Vec<u8>,
}

/// Build the scaffold plan. Pure: no I/O, deterministic in its inputs (so it is unit-tested).
#[allow(clippy::too_many_arguments)] // A pure plan builder: every arg is a distinct plan input.
fn build_plan(
    model: &Model,
    resolution: &Resolution,
    name: &str,
    description: Option<&str>,
    emoji: Option<&str>,
    cfg: &EnvConfig,
    home: &str,
    target: &str,
) -> ScaffoldPlan {
    let profile = resolution.archetype();
    let is_cli = profile == Archetype::Cli;
    let desc = description.unwrap_or(DEFAULT_DESCRIPTION);

    let mut files = base_files(name, desc, emoji, is_cli);
    if is_cli {
        files.extend(cli_surface_files(name));
    }
    files.push(PlannedFile {
        rel: "CONFORMANCE.md".to_string(),
        kind: FileKind::File,
        content: conformance_todo(model, resolution, name),
    });

    ScaffoldPlan {
        files,
        hooks: bootstrap_hooks(name, emoji, cfg, home, target),
        conformance_sections: resolution.canon_section_set(model),
    }
}

/// The one-line description placeholder when `--description` is omitted.
const DEFAULT_DESCRIPTION: &str =
    "<!-- one-line description: what it is, what it does — pass --description to fill -->";

/// The archetype-invariant base scaffold (every `new`, every profile).
fn base_files(name: &str, desc: &str, emoji: Option<&str>, is_cli: bool) -> Vec<PlannedFile> {
    let file = |rel: &str, content: String| PlannedFile {
        rel: rel.to_string(),
        kind: FileKind::File,
        content,
    };
    vec![
        // AGENTS.md must precede CLAUDE.md: on a non-unix apply, CLAUDE.md copies from it.
        file("AGENTS.md", agents_md(name, desc)),
        PlannedFile {
            rel: "CLAUDE.md".to_string(),
            kind: FileKind::Symlink,
            content: "AGENTS.md".to_string(),
        },
        file("AGENTS-AI-FIRST-CLI.md", CANON.to_string()),
        file("README.md", readme_md(name, desc, emoji)),
        file(".gitignore", gitignore(is_cli)),
        file(".workmux.yaml", workmux_yaml(emoji)),
    ]
}

/// The `cli`-profile surface scaffold — the §22 core/cli split, so a cli repo starts in shape.
fn cli_surface_files(name: &str) -> Vec<PlannedFile> {
    let file = |rel: String, content: String| PlannedFile {
        rel,
        kind: FileKind::File,
        content,
    };
    vec![
        file("Cargo.toml".to_string(), workspace_cargo_toml(name)),
        file(
            format!("crates/{name}-core/Cargo.toml"),
            core_cargo_toml(name),
        ),
        file(format!("crates/{name}-core/src/lib.rs"), core_lib_rs(name)),
        file(
            format!("crates/{name}-cli/Cargo.toml"),
            cli_cargo_toml(name),
        ),
        file(format!("crates/{name}-cli/src/main.rs"), cli_main_rs(name)),
    ]
}

/// The bootstrap hook plan, filled from the env layer. Emitted, never executed (see module docs).
/// Every interpolated value is POSIX-`shell_quote`d; `name` is a validated slug and `target` is the
/// directory actually generated into (not a configured default location). Each hook records its
/// `cwd` (the scaffolded repo) so a caller running the plan does so in the right directory.
fn bootstrap_hooks(
    name: &str,
    emoji: Option<&str>,
    cfg: &EnvConfig,
    home: &str,
    target: &str,
) -> Vec<PlannedHook> {
    let account = &cfg.gh_account;
    let ssh_url = format!("git@github.com:{account}/{name}.git");
    // The tw registry records where the repo actually lives — the caller's target — not
    // `cfg.repo_location(name)` (that convention only supplies the *default* location).
    let repo_location = target;
    let repo_slug = shell_quote(&format!("{account}/{name}"));

    let mut hooks = vec![
        PlannedHook {
            id: "git-init",
            class: HookClass::Local,
            description: "Initialize the local git repository with `main` as the default branch."
                .to_string(),
            cwd: target.to_string(),
            command: "git init -b main".to_string(),
        },
        PlannedHook {
            id: "issuectl-init",
            class: HookClass::Local,
            description: "Scaffold issue tracking (owns issues/, .issuectl/, the /issue skill)."
                .to_string(),
            cwd: target.to_string(),
            command: "issuectl init".to_string(),
        },
        PlannedHook {
            id: "git-commit",
            class: HookClass::Local,
            description: "Stage and commit the scaffold so the GitHub push has something to send."
                .to_string(),
            cwd: target.to_string(),
            command: format!(
                "git add -A && git commit -m {}",
                shell_quote(&format!("chore: scaffold {name}"))
            ),
        },
        PlannedHook {
            id: "github-create",
            class: HookClass::External,
            // Fully-qualify the repo with the configured account so the repo is created *there*,
            // not under whatever `gh` happens to default to.
            description: format!("Create the private GitHub repo {account}/{name} and push."),
            cwd: target.to_string(),
            command: format!(
                "gh repo create {repo_slug} --private --source=. --remote=origin --push"
            ),
        },
        PlannedHook {
            id: "git-remote-ssh",
            class: HookClass::External,
            description: "Force the origin remote to the family SSH URL.".to_string(),
            cwd: target.to_string(),
            command: format!("git remote set-url origin {}", shell_quote(&ssh_url)),
        },
    ];

    // tw registration is emitted only when the env layer has it enabled (portable-off otherwise).
    if cfg.tw.enabled {
        let projects_conf = EnvConfig::expand_home(&cfg.tw.projects_conf, home);
        let emoji_field = emoji.map(|e| format!("  emoji:{e}")).unwrap_or_default();
        // Append the full registry line (name  directory  clone_url  [emoji:X]); the command is
        // runnable as-is. `>>` never clobbers the file.
        let line = format!("{name}  {repo_location}  {ssh_url}{emoji_field}");
        hooks.push(PlannedHook {
            id: "tw-register",
            class: HookClass::External,
            description: format!("Register the repo in the tw project registry ({projects_conf})."),
            cwd: target.to_string(),
            command: format!(
                "printf '%s\\n' {} >> {}",
                shell_quote(&line),
                shell_quote(&projects_conf)
            ),
        });
    }

    hooks
}

// ===== file bodies (templates) ======================================================

fn agents_md(name: &str, desc: &str) -> String {
    format!(
        "# {name}\n\n\
{desc}\n\n\
## CLI Design Principles\n\n\
This project follows the AI-first CLI conventions in [`AGENTS-AI-FIRST-CLI.md`](AGENTS-AI-FIRST-CLI.md) \
— strict input validation, `--json` output, JSONL logs, no interactive prompts, informative errors, \
composable commands. Read that file before designing or changing CLI surface. It is a verbatim copy \
of the shared canon; treat it as canon, not a project-local doc to edit.\n\n\
## Gitignored directories\n\n\
- `history/` — agent scratchpad and ephemeral planning docs (not tracked)\n\
- `/target` — Rust build artifacts\n\n\
## Documentation Pattern\n\n\
Every directory follows this structure:\n\n\
- `CLAUDE.md` — symlink to `AGENTS.md`\n\
- `AGENTS.md` — all AI-relevant info (consolidated)\n\
- `AGENTS-<TOPIC>.md` — complex topics split out (optional)\n\n\
## Issues & Planning\n\n\
Issue tracking is managed by [`issuectl`](https://github.com/jarimustonen/issuectl). Use the \
`/issue` skill (installed by `issuectl init`) to create, search, update, and close issues.\n\n\
- `issues/<slug>/item.md` — every issue and epic (flat layout — no numeric prefix, no `open/closed/` split)\n\
- Status lives in the `status:` frontmatter field, not in the path\n\n\
All planning documents belong under their parent issue directory — not as standalone files. \
**Open an issue before building a feature**; keep this file describing the repo, not pre-designing the tool.\n\n\
## Conformance\n\n\
`CONFORMANCE.md` tracks this repo against the AI-first CLI canon — flip each row to `pass` as the \
surface lands. Verify mechanically with `project-canon doctor`.\n"
    )
}

fn readme_md(name: &str, desc: &str, emoji: Option<&str>) -> String {
    let title = match emoji {
        Some(e) => format!("# {name} {e}"),
        None => format!("# {name}"),
    };
    // Strip an HTML-comment placeholder down to a neutral human line for the README.
    let human_desc = if desc.trim_start().starts_with("<!--") {
        "A project in the AI-first CLI / project family."
    } else {
        desc
    };
    format!(
        "{title}\n\n\
{human_desc}\n\n\
**Status: Private, early.** Bootstrap scaffold — work is tracked as issues in this repo.\n\n\
## License\n\n\
MIT.\n"
    )
}

fn gitignore(is_cli: bool) -> String {
    let mut s = String::from("# agent scratchpad and ephemeral planning docs\nhistory/\n");
    if is_cli {
        s.push_str("\n# Rust build artifacts\n/target\n");
    }
    s
}

fn workmux_yaml(emoji: Option<&str>) -> String {
    let header = "# workmux project configuration\n\
# For global settings, edit ~/.config/workmux/config.yaml\n\n";
    match emoji {
        Some(e) => format!(
            "{header}\
# Match the tw-command emoji prefix (projects.conf) so workmux windows\n\
# blend in with `tw dev` windows.\n\
window_prefix: \"{e} \"\n"
        ),
        None => format!(
            "{header}\
# No emoji prefix set — add a window-prefix line to match the tw registry, e.g.\n\
#   window_prefix: \"📏 \"\n"
        ),
    }
}

fn workspace_cargo_toml(name: &str) -> String {
    format!(
        "# {name} workspace — §22 core/cli split (the AI-first CLI family's canonical shape).\n\
[workspace]\n\
resolver = \"2\"\n\
members = [\"crates/{name}-core\", \"crates/{name}-cli\"]\n\n\
[workspace.package]\n\
version = \"0.0.0\"\n\
edition = \"2021\"\n\
license = \"MIT\"\n"
    )
}

fn core_cargo_toml(name: &str) -> String {
    format!(
        "# Pure-domain crate: no clap, no I/O (§22). The CLI edge depends on this.\n\
[package]\n\
name = \"{name}-core\"\n\
version.workspace = true\n\
edition.workspace = true\n\
license.workspace = true\n"
    )
}

fn core_lib_rs(name: &str) -> String {
    format!(
        "//! `{name}-core` — the pure domain library (§22). No clap, no I/O.\n\n\
/// Placeholder so the crate builds. Replace with the real domain model.\n\
pub fn placeholder() -> &'static str {{\n    \"{name}-core\"\n}}\n"
    )
}

fn cli_cargo_toml(name: &str) -> String {
    format!(
        "# Thin binary crate over `{name}-core` (§22).\n\
[package]\n\
name = \"{name}-cli\"\n\
version.workspace = true\n\
edition.workspace = true\n\
license.workspace = true\n\n\
[[bin]]\n\
name = \"{name}\"\n\
path = \"src/main.rs\"\n\n\
[dependencies]\n\
{name}-core = {{ path = \"../{name}-core\" }}\n"
    )
}

fn cli_main_rs(name: &str) -> String {
    format!(
        "//! The `{name}` binary — the thin CLI over `{name}-core`.\n\n\
fn main() {{\n    \
println!(\"{{}}\", {}_core::placeholder());\n}}\n",
        name.replace('-', "_")
    )
}

/// The conformance-TODO matrix skeleton (generate-plan Step 4): one row per resolved canon §N,
/// every `Status` starting `todo`, so the new tool flips each to `pass` and the file is a direct
/// `doctor`/`review` input. Conditional sections carry their gating `Qn` and start `n/a`.
fn conformance_todo(model: &Model, resolution: &Resolution, name: &str) -> String {
    use project_canon_core::AppStatus;

    let mut out = format!(
        "# {name} — canon conformance TODO\n\n\
Generated by `project-canon new`. Each row is a resolved section of `AGENTS-AI-FIRST-CLI.md` \
for the `{}` profile. Flip `todo` → `pass` (with evidence) as the surface lands; `n/a` rows \
switch on when their gating question becomes yes. Verify mechanically with `project-canon doctor`.\n\n\
| § | Dimension | Severity | Status |\n\
|---|---|---|---|\n",
        resolution.archetype().slug()
    );

    // Canon sections, in section order (zero-padded ids already sort that way).
    let mut rows: Vec<(&Dimension, AppStatus)> = resolution
        .entries()
        .iter()
        .filter_map(|e| model.dimension(e.id).map(|d| (d, e.status)))
        .filter(|(d, _)| d.canon_section().is_some())
        .collect();
    rows.sort_by_key(|(d, _)| d.canon_section());

    for (dim, status) in rows {
        let section = dim.canon_section().expect("filtered to canon dims");
        let status_cell = match status {
            AppStatus::Applies => "todo".to_string(),
            AppStatus::NotApplicable { gated_by } => format!("n/a ({})", gated_by.label()),
        };
        out.push_str(&format!(
            "| §{section} | {} | {} | {status_cell} |\n",
            dim.title,
            severity_str(dim.severity),
        ));
    }
    out
}

fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Must => "MUST",
        Severity::MustWhenApplies => "MUST-when-applies",
        Severity::Should => "SHOULD",
    }
}

fn surface_shape_str(shape: SurfaceShape) -> &'static str {
    match shape {
        SurfaceShape::NounVerb => "noun-verb",
        SurfaceShape::FlatVerb => "flat-verb",
    }
}

// ===== clobber guard & file-action resolution =======================================

/// The relevant state of the target directory for the clobber guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirState {
    /// Does not exist (we will create it).
    Missing,
    /// Exists and has no entries.
    Empty,
    /// Exists and has ≥1 entry.
    NonEmpty,
    /// The target path itself is a symlink — refused outright (following it would let writes
    /// escape the named directory), or a non-directory file at the path.
    NotADir,
}

/// Classify the target, distinguishing "does not exist" (fine) from a genuine I/O error
/// (propagated → exit 2). A symlink at the target path — or any non-directory file — is `NotADir`
/// and refused even with `--force`: following it would break the side-effect boundary (writes
/// could land outside the named directory). Checked with no-follow `symlink_metadata` first.
fn dir_state(root: &Path) -> std::io::Result<DirState> {
    match std::fs::symlink_metadata(root) {
        Ok(md) if md.file_type().is_symlink() => return Ok(DirState::NotADir),
        Ok(md) if !md.is_dir() => return Ok(DirState::NotADir),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(DirState::Missing),
        Err(e) => return Err(e),
    }
    match std::fs::read_dir(root) {
        Ok(mut entries) => Ok(if entries.next().is_some() {
            DirState::NonEmpty
        } else {
            DirState::Empty
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DirState::Missing),
        Err(e) => Err(e),
    }
}

/// What will happen to one planned file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileAction {
    /// Will be written (does not exist yet).
    Create,
    /// Already present — left untouched (fill-gaps only; never overwrite).
    SkipExists,
}

impl FileAction {
    fn as_str(self) -> &'static str {
        match self {
            FileAction::Create => "create",
            FileAction::SkipExists => "skip-exists",
        }
    }
}

/// An I/O fault touching one path, carrying the repo-relative path for a precise message.
#[derive(Debug)]
struct FileFault {
    rel: String,
    source: std::io::Error,
}

/// Decide each file's action against the current tree — pure of writes. `SkipExists` when the
/// path already exists (by no-follow `symlink_metadata`, so a symlink counts as present); an I/O
/// error other than `NotFound` faults.
fn resolve_file_actions(
    plan: &ScaffoldPlan,
    root: &Path,
) -> Result<Vec<(usize, FileAction)>, FileFault> {
    let mut rows = Vec::with_capacity(plan.files.len());
    for (i, f) in plan.files.iter().enumerate() {
        let full = root.join(&f.rel);
        let action = match std::fs::symlink_metadata(&full) {
            Ok(_) => FileAction::SkipExists,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => FileAction::Create,
            Err(source) => {
                return Err(FileFault {
                    rel: f.rel.clone(),
                    source,
                })
            }
        };
        rows.push((i, action));
    }
    Ok(rows)
}

/// Materialize the `Create` rows into `root`. Existing files (`SkipExists`) are never touched.
/// Parent directories are created as needed. The only side-effecting step in the verb.
fn apply(
    plan: &ScaffoldPlan,
    file_rows: &[(usize, FileAction)],
    root: &Path,
) -> Result<(), FileFault> {
    // Create the root itself (idempotent); a pre-existing dir is fine.
    std::fs::create_dir_all(root).map_err(|source| FileFault {
        rel: ".".to_string(),
        source,
    })?;

    for &(i, action) in file_rows {
        if action != FileAction::Create {
            continue;
        }
        let f = &plan.files[i];
        let full = root.join(&f.rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|source| FileFault {
                rel: f.rel.clone(),
                source,
            })?;
        }
        write_one(f, root, &full).map_err(|source| FileFault {
            rel: f.rel.clone(),
            source,
        })?;
    }
    Ok(())
}

/// Write a single planned file: a regular file (created with no-clobber, no-follow semantics), or
/// a symlink (unix) / verbatim copy (non-unix).
///
/// Regular files are opened with `create_new(true)` so the write is atomic against the earlier
/// `symlink_metadata` check: it refuses to open an existing file OR follow a symlink that appeared
/// in the race window, making "never overwrite / never escape via a final-component symlink" an
/// OS-level guarantee rather than a check. (A parent-directory symlink that pre-exists in a
/// `--force`d tree is a separate, deeper concern — see the design note's deferred items; name
/// validation + the symlink-root refusal cover the reachable vectors.)
fn write_one(f: &PlannedFile, root: &Path, full: &Path) -> std::io::Result<()> {
    match f.kind {
        FileKind::File => create_new_file(full, f.content.as_bytes()),
        FileKind::Symlink => write_symlink(&f.content, root, full),
    }
}

/// Create a file exclusively (fails if it already exists or is a symlink) and write `content`.
fn create_new_file(full: &Path, content: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(full)?;
    file.write_all(content)
}

#[cfg(unix)]
fn write_symlink(target: &str, _root: &Path, full: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, full)
}

#[cfg(not(unix))]
fn write_symlink(target: &str, root: &Path, full: &Path) -> std::io::Result<()> {
    // No portable unprivileged symlink off-unix: fall back to a verbatim copy of the target
    // (doctor's doc-pattern probe accepts CLAUDE.md as any regular file). The target is written
    // earlier in plan order, so it exists here. Guard: the link target must be a bare filename —
    // never a path that could reach outside the scaffold (defence-in-depth; today only
    // `CLAUDE.md → AGENTS.md` is ever emitted).
    if target.contains('/') || target.contains('\\') || target.contains("..") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("refusing non-local symlink target {target:?}"),
        ));
    }
    std::fs::copy(root.join(target), full).map(|_| ())
}

// ===== the report ===================================================================

/// The whole `new` result — the plan plus the resolved per-file actions and run mode.
struct Report {
    target: String,
    name: String,
    profile: Archetype,
    surface_shape: Option<SurfaceShape>,
    dry_run: bool,
    force: bool,
    plan: ScaffoldPlan,
    file_rows: Vec<(usize, FileAction)>,
}

impl Report {
    fn count(&self, action: FileAction) -> usize {
        self.file_rows.iter().filter(|(_, a)| *a == action).count()
    }

    /// The §10 structured payload.
    fn to_json(&self) -> Json {
        let files = self
            .file_rows
            .iter()
            .map(|&(i, action)| {
                let f = &self.plan.files[i];
                Json::Object(vec![
                    ("path".into(), Json::str(f.rel.clone())),
                    ("kind".into(), Json::str(f.kind.as_str())),
                    ("action".into(), Json::str(action.as_str())),
                ])
            })
            .collect();

        let hooks = self
            .plan
            .hooks
            .iter()
            .map(|h| {
                Json::Object(vec![
                    ("id".into(), Json::str(h.id)),
                    ("class".into(), Json::str(h.class.as_str())),
                    ("description".into(), Json::str(h.description.clone())),
                    ("cwd".into(), Json::str(h.cwd.clone())),
                    ("command".into(), Json::str(h.command.clone())),
                ])
            })
            .collect();

        let sections = self
            .plan
            .conformance_sections
            .iter()
            .map(|n| Json::Int(*n as i64))
            .collect();

        // Stable keys regardless of mode (a machine consumer must not branch on key names):
        //  - `create`  = rows that would be created (mode-independent plan size),
        //  - `written` = rows actually written this run (0 under --dry-run),
        //  - `skipped` = pre-existing rows left untouched.
        let created = self.count(FileAction::Create);
        let written = if self.dry_run { 0 } else { created };
        let summary = Json::Object(vec![
            ("files".into(), Json::Int(self.plan.files.len() as i64)),
            ("create".into(), Json::Int(created as i64)),
            ("written".into(), Json::Int(written as i64)),
            (
                "skipped".into(),
                Json::Int(self.count(FileAction::SkipExists) as i64),
            ),
            ("hooks".into(), Json::Int(self.plan.hooks.len() as i64)),
        ]);

        Json::Object(vec![
            ("schema_version".into(), Json::Int(SCHEMA_VERSION)),
            ("tool".into(), Json::str("project-canon")),
            ("verb".into(), Json::str("new")),
            ("target".into(), Json::str(self.target.clone())),
            ("name".into(), Json::str(self.name.clone())),
            ("profile".into(), Json::str(self.profile.slug())),
            (
                "surface_shape".into(),
                Json::opt_str(self.surface_shape.map(surface_shape_str)),
            ),
            ("dry_run".into(), Json::Bool(self.dry_run)),
            ("force".into(), Json::Bool(self.force)),
            ("files".into(), Json::Array(files)),
            ("hooks".into(), Json::Array(hooks)),
            ("conformance_sections".into(), Json::Array(sections)),
            ("summary".into(), summary),
            ("exit_code".into(), Json::Int(EXIT_OK as i64)),
        ])
    }

    /// The human view: the file plan, then the hook plan, then a summary line.
    fn render_human(&self, verbose: bool) -> String {
        let mut out = String::new();
        let mode = if self.dry_run { "  (dry-run)" } else { "" };
        out.push_str(&format!(
            "project-canon new: {} (name: {}, profile: {}){mode}\n",
            self.target,
            self.name,
            self.profile.slug()
        ));

        out.push_str("files:\n");
        for &(i, action) in &self.file_rows {
            if action == FileAction::SkipExists && !verbose {
                continue;
            }
            let f = &self.plan.files[i];
            let verb = if self.dry_run && action == FileAction::Create {
                "would-create"
            } else {
                action.as_str()
            };
            out.push_str(&format!("  {:<12} {} ({})\n", verb, f.rel, f.kind.as_str()));
        }

        out.push_str(&format!(
            "hooks (printed, not run — run these from inside {}):\n",
            self.target
        ));
        for h in &self.plan.hooks {
            out.push_str(&format!(
                "  [{}] {}\n      $ {}\n",
                h.class.as_str(),
                h.description,
                h.command
            ));
        }

        if verbose {
            out.push_str(&format!(
                "resolved conformance sections: {}\n",
                self.plan
                    .conformance_sections
                    .iter()
                    .map(|n| format!("§{n}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }

        let created = self.count(FileAction::Create);
        let skipped = self.count(FileAction::SkipExists);
        let verb = if self.dry_run { "would write" } else { "wrote" };
        out.push_str(&format!(
            "summary: {verb} {created} file{}, {skipped} skipped, {} hook{} to run\n",
            if created == 1 { "" } else { "s" },
            self.plan.hooks.len(),
            if self.plan.hooks.len() == 1 { "" } else { "s" },
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use project_canon_core::AppStatus;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    // ---- arg parsing -------------------------------------------------------------

    fn parse(args: &[&str]) -> Result<Command, String> {
        parse_args(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    fn run_args(args: &[&str]) -> NewArgs {
        match parse(args).unwrap() {
            Command::Run(a) => a,
            Command::Help => panic!("expected Run"),
        }
    }

    #[test]
    fn defaults_are_cli_profile_and_flags_off() {
        let a = run_args(&["mydir"]);
        assert_eq!(a.dir, "mydir");
        assert_eq!(a.profile, Archetype::Cli);
        assert!(!a.dry_run && !a.force && !a.json && !a.verbose);
        assert_eq!(a.name, None);
    }

    #[test]
    fn parses_all_flags_and_positional() {
        let a = run_args(&[
            "--profile",
            "service",
            "--name",
            "foo",
            "--description",
            "does things",
            "--emoji",
            "📏",
            "--dry-run",
            "--force",
            "--json",
            "--verbose",
            "--assume-defaults",
            "/some/dir",
        ]);
        assert_eq!(a.dir, "/some/dir");
        assert_eq!(a.profile, Archetype::Service);
        assert_eq!(a.name.as_deref(), Some("foo"));
        assert_eq!(a.description.as_deref(), Some("does things"));
        assert_eq!(a.emoji.as_deref(), Some("📏"));
        assert!(a.dry_run && a.force && a.json && a.verbose && a.assume_defaults);
    }

    #[test]
    fn value_flags_accept_equals_form() {
        let a = run_args(&["--profile=library", "--name=bar", "d"]);
        assert_eq!(a.profile, Archetype::Library);
        assert_eq!(a.name.as_deref(), Some("bar"));
    }

    #[test]
    fn missing_dir_is_rejected() {
        assert!(parse(&["--json"]).is_err());
    }

    #[test]
    fn help_short_circuits() {
        assert_eq!(parse(&["--help"]).unwrap(), Command::Help);
        assert_eq!(parse(&["--json", "--help", "d"]).unwrap(), Command::Help);
    }

    #[test]
    fn unknown_flag_is_rejected_and_echoed() {
        let err = parse(&["--nope", "d"]).unwrap_err();
        assert!(err.contains("--nope"), "{err}");
    }

    #[test]
    fn bad_profile_echoes_value_and_valid_set() {
        let err = parse(&["--profile", "webapp", "d"]).unwrap_err();
        assert!(err.contains("webapp") && err.contains("cli"), "{err}");
    }

    #[test]
    fn empty_value_flags_are_rejected() {
        assert!(parse(&["--name=", "d"]).is_err());
        assert!(parse(&["--profile=", "d"]).is_err());
        assert!(parse(&["--description", "", "d"]).is_err());
    }

    #[test]
    fn missing_flag_value_is_rejected() {
        assert!(parse(&["--profile"]).is_err());
        assert!(parse(&["--name"]).is_err());
    }

    #[test]
    fn repeated_flags_are_rejected() {
        assert!(parse(&["--json", "--json", "d"]).is_err());
        assert!(parse(&["--name", "a", "--name", "b", "d"]).is_err());
    }

    #[test]
    fn extra_positional_is_rejected() {
        assert!(parse(&["a", "b"]).is_err());
    }

    #[test]
    fn valueless_flags_reject_an_inline_value() {
        for arg in ["--json=false", "--force=0", "--dry-run=no", "--help=x"] {
            let err = parse(&[arg, "d"]).unwrap_err();
            assert!(err.contains("does not take a value"), "{arg}: {err}");
        }
    }

    #[test]
    fn double_dash_lets_a_dashy_dir_through() {
        let a = run_args(&["--", "-weird-dir"]);
        assert_eq!(a.dir, "-weird-dir");
        assert!(parse(&["--", "a", "b"]).is_err());
    }

    // ---- name derivation ---------------------------------------------------------

    #[test]
    fn name_derives_from_dir_final_component() {
        assert_eq!(resolve_name(&None, "/a/b/foo").unwrap(), "foo");
        assert_eq!(resolve_name(&None, "foo").unwrap(), "foo");
        assert_eq!(
            resolve_name(&Some("explicit".to_string()), "/a/b/foo").unwrap(),
            "explicit"
        );
    }

    // ---- plan building -----------------------------------------------------------

    fn plan_for(profile: Archetype, name: &str, emoji: Option<&str>) -> ScaffoldPlan {
        let model = Model::standard();
        let resolution = model.resolve(&Questionnaire::builder(profile).build());
        let cfg = EnvConfig::builtin_defaults();
        build_plan(
            &model,
            &resolution,
            name,
            Some("test desc"),
            emoji,
            &cfg,
            "/home/j",
            "/tmp/target",
        )
    }

    fn has_file<'a>(plan: &'a ScaffoldPlan, rel: &str) -> Option<&'a PlannedFile> {
        plan.files.iter().find(|f| f.rel == rel)
    }

    #[test]
    fn base_scaffold_has_the_invariant_files() {
        let plan = plan_for(Archetype::Cli, "foo", Some("📏"));
        for rel in [
            "AGENTS.md",
            "CLAUDE.md",
            "AGENTS-AI-FIRST-CLI.md",
            "README.md",
            ".gitignore",
            ".workmux.yaml",
            "CONFORMANCE.md",
        ] {
            assert!(has_file(&plan, rel).is_some(), "missing {rel}");
        }
        // CLAUDE.md is a symlink to AGENTS.md, and AGENTS.md precedes it (non-unix copy order).
        let claude = has_file(&plan, "CLAUDE.md").unwrap();
        assert_eq!(claude.kind, FileKind::Symlink);
        assert_eq!(claude.content, "AGENTS.md");
        let agents_i = plan
            .files
            .iter()
            .position(|f| f.rel == "AGENTS.md")
            .unwrap();
        let claude_i = plan
            .files
            .iter()
            .position(|f| f.rel == "CLAUDE.md")
            .unwrap();
        assert!(agents_i < claude_i, "AGENTS.md must precede CLAUDE.md");
        // The canon is bundled verbatim.
        assert_eq!(
            has_file(&plan, "AGENTS-AI-FIRST-CLI.md").unwrap().content,
            CANON
        );
    }

    #[test]
    fn cli_profile_adds_the_core_cli_split() {
        let plan = plan_for(Archetype::Cli, "foo", None);
        for rel in [
            "Cargo.toml",
            "crates/foo-core/Cargo.toml",
            "crates/foo-core/src/lib.rs",
            "crates/foo-cli/Cargo.toml",
            "crates/foo-cli/src/main.rs",
        ] {
            assert!(has_file(&plan, rel).is_some(), "missing {rel}");
        }
        // The workspace lists both crates.
        let ws = &has_file(&plan, "Cargo.toml").unwrap().content;
        assert!(ws.contains("crates/foo-core"));
        assert!(ws.contains("crates/foo-cli"));
        // main.rs references the core crate with a valid (underscored) crate identifier.
        assert!(has_file(&plan, "crates/foo-cli/src/main.rs")
            .unwrap()
            .content
            .contains("foo_core::placeholder"));
    }

    #[test]
    fn hyphenated_name_underscores_the_crate_identifier() {
        let plan = plan_for(Archetype::Cli, "my-tool", None);
        let main = &has_file(&plan, "crates/my-tool-cli/src/main.rs")
            .unwrap()
            .content;
        // The crate path in `use`/call position must be a valid ident, not `my-tool_core`.
        assert!(main.contains("my_tool_core::placeholder"), "{main}");
    }

    #[test]
    fn non_cli_profile_emits_base_only() {
        let plan = plan_for(Archetype::Service, "svc", None);
        assert!(has_file(&plan, "Cargo.toml").is_none());
        assert!(has_file(&plan, "AGENTS.md").is_some());
        assert!(has_file(&plan, "CONFORMANCE.md").is_some());
        // /target is a Rust-only ignore; a non-cli scaffold omits it.
        assert!(!has_file(&plan, ".gitignore")
            .unwrap()
            .content
            .contains("/target"));
    }

    #[test]
    fn emoji_flows_into_workmux_and_readme() {
        let plan = plan_for(Archetype::Cli, "foo", Some("📏"));
        assert!(has_file(&plan, ".workmux.yaml")
            .unwrap()
            .content
            .contains("window_prefix: \"📏 \""));
        assert!(has_file(&plan, "README.md")
            .unwrap()
            .content
            .contains("# foo 📏"));
        // Without an emoji, the workmux file has no *active* window_prefix line (only a commented
        // example) and the title has no glyph.
        let plain = plan_for(Archetype::Cli, "foo", None);
        assert!(!has_file(&plain, ".workmux.yaml")
            .unwrap()
            .content
            .lines()
            .any(|l| l.starts_with("window_prefix:")));
        assert!(has_file(&plain, "README.md")
            .unwrap()
            .content
            .contains("# foo\n"));
    }

    #[test]
    fn description_fills_agents_md() {
        let model = Model::standard();
        let resolution = model.resolve(&Questionnaire::builder(Archetype::Cli).build());
        let cfg = EnvConfig::builtin_defaults();
        let plan = build_plan(
            &model,
            &resolution,
            "foo",
            Some("a neat tool"),
            None,
            &cfg,
            "/home/j",
            "/tmp/target",
        );
        assert!(has_file(&plan, "AGENTS.md")
            .unwrap()
            .content
            .contains("a neat tool"));
    }

    // ---- hook rendering (pulls from EnvConfig) -----------------------------------

    #[test]
    fn hooks_pull_gh_account_and_repo_url_from_env_layer() {
        let plan = plan_for(Archetype::Cli, "foo", Some("📏"));
        let gh = plan.hooks.iter().find(|h| h.id == "github-create").unwrap();
        assert_eq!(gh.class, HookClass::External);
        // Default gh account is jarimustonen (from EnvConfig::builtin_defaults); the repo is
        // account-qualified so it is created there, not under gh's default.
        assert!(gh.command.contains("jarimustonen/foo"), "{}", gh.command);
        let remote = plan
            .hooks
            .iter()
            .find(|h| h.id == "git-remote-ssh")
            .unwrap();
        assert!(
            remote
                .command
                .contains("git@github.com:jarimustonen/foo.git"),
            "{}",
            remote.command
        );
        // The local hooks exist and are classed local; every hook carries the target as its cwd.
        for id in ["git-init", "issuectl-init", "git-commit"] {
            let h = plan.hooks.iter().find(|h| h.id == id).unwrap();
            assert_eq!(h.class, HookClass::Local);
            assert_eq!(h.cwd, "/tmp/target");
        }
    }

    #[test]
    fn hook_commands_are_shell_quoted_from_env_values() {
        // A gh account with shell metacharacters is single-quoted in the emitted command, never
        // left active (§: the printed plan must be safe to copy-paste).
        let model = Model::standard();
        let resolution = model.resolve(&Questionnaire::builder(Archetype::Cli).build());
        let layer = EnvConfigLayer {
            gh_account: Some("ev$il".to_string()),
            ..EnvConfigLayer::empty()
        };
        let cfg = EnvConfig::resolve(&[&layer]);
        let plan = build_plan(
            &model,
            &resolution,
            "foo",
            None,
            None,
            &cfg,
            "/home/j",
            "/tmp/target",
        );
        let gh = plan.hooks.iter().find(|h| h.id == "github-create").unwrap();
        // The value is wrapped in single quotes, so the `$` never expands.
        assert!(gh.command.contains("'ev$il/foo'"), "{}", gh.command);
    }

    #[test]
    fn validate_name_rejects_unsafe_slugs() {
        for bad in [
            "",
            "../evil",
            "a/b",
            "-flag",
            "1tool",
            "foo bar",
            "foo;rm",
            "foo$(x)",
            "MiXeD_ok?no",
        ] {
            assert!(validate_name(bad).is_err(), "should reject {bad:?}");
        }
        for good in ["foo", "my-tool", "a", "x1_y-2", "Tool"] {
            assert!(validate_name(good).is_ok(), "should accept {good:?}");
        }
    }

    #[test]
    fn shell_quote_neutralizes_metacharacters() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("a$b`c"), "'a$b`c'");
        // An embedded single quote is closed/escaped/reopened.
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn tw_register_is_emitted_only_when_enabled() {
        // Default: tw enabled → the hook is present.
        let plan = plan_for(Archetype::Cli, "foo", Some("📏"));
        assert!(plan.hooks.iter().any(|h| h.id == "tw-register"));

        // Disable tw via a config layer → the hook is dropped.
        let model = Model::standard();
        let resolution = model.resolve(&Questionnaire::builder(Archetype::Cli).build());
        let layer = EnvConfigLayer {
            tw_enabled: Some(false),
            ..EnvConfigLayer::empty()
        };
        let cfg = EnvConfig::resolve(&[&layer]);
        let plan = build_plan(
            &model,
            &resolution,
            "foo",
            None,
            None,
            &cfg,
            "/home/j",
            "/tmp/target",
        );
        assert!(!plan.hooks.iter().any(|h| h.id == "tw-register"));
    }

    #[test]
    fn gh_account_override_flows_into_hooks() {
        let model = Model::standard();
        let resolution = model.resolve(&Questionnaire::builder(Archetype::Cli).build());
        let layer = EnvConfigLayer {
            gh_account: Some("octocat".to_string()),
            ..EnvConfigLayer::empty()
        };
        let cfg = EnvConfig::resolve(&[&layer]);
        let plan = build_plan(
            &model,
            &resolution,
            "foo",
            None,
            None,
            &cfg,
            "/home/j",
            "/tmp/target",
        );
        let remote = plan
            .hooks
            .iter()
            .find(|h| h.id == "git-remote-ssh")
            .unwrap();
        assert!(
            remote.command.contains("git@github.com:octocat/foo.git"),
            "{}",
            remote.command
        );
        // The github-create hook fully-qualifies the repo with the configured account.
        let gh = plan.hooks.iter().find(|h| h.id == "github-create").unwrap();
        assert!(gh.command.contains("octocat/foo"), "{}", gh.command);
    }

    // ---- conformance TODO --------------------------------------------------------

    #[test]
    fn conformance_todo_lists_resolved_sections_with_status() {
        let model = Model::standard();
        let resolution = model.resolve(&Questionnaire::builder(Archetype::Cli).build());
        let md = conformance_todo(&model, &resolution, "foo");
        // Always-on §1 is `todo`; conditional §11 (Q3 off) is `n/a (Q3)`.
        assert!(md.contains("| §1 |"));
        assert!(md.contains("todo"));
        assert!(md.contains("n/a (Q3)"), "{md}");
        // Sanity: it is a markdown table with a header.
        assert!(md.contains("| § | Dimension | Severity | Status |"));
    }

    #[test]
    fn conformance_status_reflects_applicability() {
        // A resolved-but-n/a conditional never reads `todo`.
        let model = Model::standard();
        let resolution = model.resolve(&Questionnaire::builder(Archetype::Cli).build());
        let entries: Vec<_> = resolution
            .entries()
            .iter()
            .filter(|e| matches!(e.status, AppStatus::NotApplicable { .. }))
            .collect();
        assert!(
            !entries.is_empty(),
            "expected some n/a conditionals under defaults"
        );
    }

    // ---- clobber guard & file actions (temp dirs) --------------------------------

    struct TmpDir {
        path: PathBuf,
    }

    impl TmpDir {
        fn new(tag: &str) -> TmpDir {
            static N: AtomicU32 = AtomicU32::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("pc-new-{tag}-{}-{n}", std::process::id()));
            TmpDir { path }
        }
    }

    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn dir_state_classifies_missing_empty_nonempty() {
        let t = TmpDir::new("state");
        assert_eq!(dir_state(&t.path).unwrap(), DirState::Missing);
        std::fs::create_dir_all(&t.path).unwrap();
        assert_eq!(dir_state(&t.path).unwrap(), DirState::Empty);
        std::fs::write(t.path.join("x"), b"x").unwrap();
        assert_eq!(dir_state(&t.path).unwrap(), DirState::NonEmpty);
    }

    #[test]
    fn apply_writes_the_plan_then_skips_existing() {
        let t = TmpDir::new("apply");
        let plan = plan_for(Archetype::Cli, "foo", Some("📏"));

        // First apply: every file is a create.
        let rows = resolve_file_actions(&plan, &t.path).unwrap();
        assert!(rows.iter().all(|(_, a)| *a == FileAction::Create));
        apply(&plan, &rows, &t.path).unwrap();
        assert!(t.path.join("AGENTS.md").is_file());
        assert!(t.path.join("crates/foo-core/src/lib.rs").is_file());
        // The canon landed byte-identical.
        assert_eq!(
            std::fs::read_to_string(t.path.join("AGENTS-AI-FIRST-CLI.md")).unwrap(),
            CANON
        );
        #[cfg(unix)]
        {
            let md = std::fs::symlink_metadata(t.path.join("CLAUDE.md")).unwrap();
            assert!(md.file_type().is_symlink());
        }

        // Second apply: everything already exists → all skip-exists, no overwrite.
        let rows2 = resolve_file_actions(&plan, &t.path).unwrap();
        assert!(rows2.iter().all(|(_, a)| *a == FileAction::SkipExists));
    }

    #[test]
    fn apply_never_overwrites_an_existing_file() {
        let t = TmpDir::new("noclobber");
        std::fs::create_dir_all(&t.path).unwrap();
        std::fs::write(t.path.join("README.md"), b"KEEP ME").unwrap();
        let plan = plan_for(Archetype::Cli, "foo", None);
        let rows = resolve_file_actions(&plan, &t.path).unwrap();
        apply(&plan, &rows, &t.path).unwrap();
        // The pre-existing README is untouched; a gap (AGENTS.md) is filled.
        assert_eq!(
            std::fs::read_to_string(t.path.join("README.md")).unwrap(),
            "KEEP ME"
        );
        assert!(t.path.join("AGENTS.md").is_file());
    }
}
