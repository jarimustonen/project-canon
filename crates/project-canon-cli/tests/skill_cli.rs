//! End-to-end integration tests for the `project-canon skill` verb — run the built binary against
//! throwaway temp dirs and assert install file layout, the single-source invariant, `--dry-run`
//! print-not-write, idempotent re-install, the clobber/force guards, `--json` shape, `--agent`
//! selection, and `list`/`print`.
//!
//! Hermetic and side-effect-safe by construction: every `install` passes `--target <tmp>`, so no
//! write ever reaches `$HOME` or this repo's real `.claude/`. The binary never shells out or hits
//! the network.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// A throwaway install-base dir under the OS temp root; removed on drop.
struct Tmp {
    path: PathBuf,
}

impl Tmp {
    fn new(tag: &str) -> Tmp {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("pc-skill-it-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Tmp { path }
    }
    fn str(&self) -> &str {
        self.path.to_str().unwrap()
    }
    fn claude_skill(&self) -> PathBuf {
        self.path.join(".claude/skills/ai-first-cli-canon/SKILL.md")
    }
    fn codex_prompt(&self) -> PathBuf {
        self.path.join(".codex/prompts/ai-first-cli-canon.md")
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A `project-canon` command with every `PROJECT_CANON_*` variable scrubbed (a stray override on
/// the dev/CI machine must not perturb these hermetic fixtures).
fn base_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_project-canon"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("PROJECT_CANON_") {
            cmd.env_remove(key);
        }
    }
    cmd
}

fn run(args: &[&str]) -> Output {
    base_command()
        .arg("skill")
        .args(args)
        .output()
        .expect("run project-canon skill")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("exited with a code")
}

/// The master canon this repo maintains — the single source the installed skills must embed.
fn master_canon() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../AGENTS-AI-FIRST-CLI.md"
    ))
    .unwrap()
}

#[test]
fn install_writes_both_agent_forms() {
    let t = Tmp::new("both");
    let out = run(&["install", "--target", t.str()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(t.claude_skill().is_file(), "claude SKILL.md missing");
    assert!(t.codex_prompt().is_file(), "codex prompt missing");

    let claude = std::fs::read_to_string(t.claude_skill()).unwrap();
    let codex = std::fs::read_to_string(t.codex_prompt()).unwrap();
    // Claude form declares §17 frontmatter; codex form does not.
    assert!(claude.starts_with("---\nname: ai-first-cli-canon"));
    assert!(claude.contains("cli_version:"));
    assert!(!codex.starts_with("---"));
}

#[test]
fn installed_skill_embeds_the_master_canon_verbatim() {
    // The single-source invariant: no drifting second copy — both forms embed the master bytes.
    let t = Tmp::new("single-source");
    assert_eq!(code(&run(&["install", "--target", t.str()])), 0);
    let master = master_canon();
    let claude = std::fs::read_to_string(t.claude_skill()).unwrap();
    let codex = std::fs::read_to_string(t.codex_prompt()).unwrap();
    assert!(
        claude.contains(&master),
        "claude skill must embed the master canon verbatim"
    );
    assert!(
        codex.contains(&master),
        "codex prompt must embed the master canon verbatim"
    );
}

#[test]
fn dry_run_prints_but_writes_nothing() {
    let t = Tmp::new("dry");
    let out = run(&["install", "--target", t.str(), "--dry-run"]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dry-run"), "{stdout}");
    assert!(stdout.contains("would-install"), "{stdout}");
    // Nothing written — not even the agent dirs.
    assert!(!t.path.join(".claude").exists());
    assert!(!t.path.join(".codex").exists());
}

#[test]
fn reinstall_is_idempotent() {
    let t = Tmp::new("idem");
    assert_eq!(code(&run(&["install", "--target", t.str()])), 0);
    let first = std::fs::read(t.claude_skill()).unwrap();
    let out = run(&["install", "--target", t.str()]);
    assert_eq!(code(&out), 0);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("unchanged"),
        "second install should report unchanged"
    );
    // Byte-identical after re-install.
    assert_eq!(std::fs::read(t.claude_skill()).unwrap(), first);
}

#[test]
fn json_report_is_well_formed() {
    let t = Tmp::new("json");
    let out = run(&["install", "--target", t.str(), "--json", "--dry-run"]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stdout = stdout.trim();
    assert!(stdout.starts_with('{') && stdout.ends_with('}'), "{stdout}");
    assert!(stdout.contains("\"schema_version\":1"));
    assert!(stdout.contains("\"verb\":\"skill install\""));
    assert!(stdout.contains("\"dry_run\":true"));
    assert!(stdout.contains("\"agent\":\"claude\""));
    assert!(stdout.contains("\"agent\":\"codex\""));
    assert!(stdout.contains("\"action\":\"install\""));
    assert!(stdout.contains("\"written\":0"));
    assert!(stdout.contains("\"exit_code\":0"));
}

#[test]
fn agent_flag_selects_a_single_form() {
    let t = Tmp::new("agent");
    assert_eq!(
        code(&run(&["install", "--target", t.str(), "--agent", "claude"])),
        0
    );
    assert!(t.claude_skill().is_file());
    assert!(!t.codex_prompt().exists(), "codex form must not be written");
}

#[test]
fn foreign_file_is_refused_without_force_then_overwritten_with_force() {
    let t = Tmp::new("foreign");
    let p = t.claude_skill();
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "HAND WRITTEN — KEEP").unwrap();

    // Without --force: refused, exit 2, file untouched.
    let out = run(&["install", "--target", t.str(), "--agent", "claude"]);
    assert_eq!(
        code(&out),
        2,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("non-managed"));
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "HAND WRITTEN — KEEP");

    // With --force: overwritten.
    let out = run(&[
        "install",
        "--target",
        t.str(),
        "--agent",
        "claude",
        "--force",
    ]);
    assert_eq!(code(&out), 0);
    assert!(std::fs::read_to_string(&p)
        .unwrap()
        .contains(&master_canon()));
}

#[test]
fn a_managed_stale_file_upgrades_without_force() {
    // A file carrying our provenance marker but a different body is a managed upgrade — no --force.
    let t = Tmp::new("upgrade");
    let p = t.codex_prompt();
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(
        &p,
        "<!-- Installed by `project-canon skill install` — ai-first-cli-canon cli_version=0.0.0 schema_version=1. -->\n\nSTALE BODY",
    )
    .unwrap();
    let out = run(&["install", "--target", t.str(), "--agent", "codex"]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(std::fs::read_to_string(&p)
        .unwrap()
        .contains(&master_canon()));
}

#[test]
fn newer_on_disk_is_refused_without_force() {
    let t = Tmp::new("newer");
    let p = t.codex_prompt();
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(
        &p,
        "<!-- Installed by `project-canon skill install` — ai-first-cli-canon cli_version=99.0.0 schema_version=1. -->\n\nFUTURE",
    )
    .unwrap();
    let out = run(&["install", "--target", t.str(), "--agent", "codex"]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("newer"));
}

#[test]
fn unknown_skill_name_is_a_usage_error() {
    let t = Tmp::new("unknown");
    let out = run(&["install", "nope", "--target", t.str()]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("nope"));
}

#[test]
fn bad_agent_is_a_usage_error() {
    let t = Tmp::new("badagent");
    let out = run(&["install", "--target", t.str(), "--agent", "emacs"]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("emacs"));
}

#[test]
fn unknown_flag_is_a_usage_error() {
    let t = Tmp::new("badflag");
    let out = run(&["install", "--target", t.str(), "--nope"]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("--nope"));
}

#[test]
fn malformed_env_is_a_usage_error_but_help_still_exits_zero() {
    let t = Tmp::new("env");
    let bad = base_command()
        .args(["skill", "install", "--target", t.str()])
        .env("PROJECT_CANON_TW_ENABLED", "not-a-bool")
        .output()
        .unwrap();
    assert_eq!(code(&bad), 2);
    // --help short-circuits before env validation.
    let help = base_command()
        .args(["skill", "install", "--help"])
        .env("PROJECT_CANON_TW_ENABLED", "not-a-bool")
        .output()
        .unwrap();
    assert_eq!(code(&help), 0);
    assert!(String::from_utf8_lossy(&help.stdout).contains("USAGE"));
}

#[test]
fn list_reports_the_shipped_skill() {
    let out = run(&["list"]);
    assert_eq!(code(&out), 0);
    assert!(String::from_utf8_lossy(&out.stdout).contains("ai-first-cli-canon"));

    let json = run(&["list", "--json"]);
    assert_eq!(code(&json), 0);
    let stdout = String::from_utf8_lossy(&json.stdout);
    assert!(stdout.contains("\"verb\":\"skill list\""));
    assert!(stdout.contains("\"name\":\"ai-first-cli-canon\""));
    assert!(stdout.contains("\"cli_version\""));
}

#[test]
fn print_is_byte_identical_to_what_install_writes() {
    // §16: print's output must equal the installed bytes for that agent.
    let t = Tmp::new("print");
    assert_eq!(code(&run(&["install", "--target", t.str()])), 0);
    let installed = std::fs::read_to_string(t.claude_skill()).unwrap();
    let printed = run(&["print", "ai-first-cli-canon", "--agent", "claude"]);
    assert_eq!(code(&printed), 0);
    assert_eq!(String::from_utf8_lossy(&printed.stdout), installed);
}

#[test]
fn print_json_carries_metadata_and_content() {
    let out = run(&["print", "ai-first-cli-canon", "--json"]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"name\":\"ai-first-cli-canon\""));
    assert!(stdout.contains("\"schema_version_skill\":1"));
    assert!(stdout.contains("\"path_in_repo\":\"AGENTS-AI-FIRST-CLI.md\""));
    assert!(stdout.contains("\"content\":"));
}

#[test]
fn print_unknown_name_is_a_usage_error() {
    let out = run(&["print", "nope"]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("nope"));
}

#[test]
fn install_never_touches_this_repos_real_dotclaude() {
    // Defence-in-depth: a bare install with no --target must resolve $HOME, and these tests set
    // HOME to a temp dir so even that path is hermetic. Assert the write lands under the temp HOME.
    let home = Tmp::new("home");
    let out = base_command()
        .args(["skill", "install", "--agent", "codex"])
        .env("HOME", home.str())
        .output()
        .unwrap();
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(home
        .path
        .join(".codex/prompts/ai-first-cli-canon.md")
        .is_file());
}
