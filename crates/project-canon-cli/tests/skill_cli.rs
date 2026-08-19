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
    fn skill_resource(&self, agent: &str, name: &str, resource: &str) -> PathBuf {
        let root = match agent {
            "claude" => ".claude/skills",
            "pi" => ".pi/agent/skills",
            other => panic!("unsupported native skill agent {other}"),
        };
        self.path.join(root).join(name).join(resource)
    }
    fn named_codex_prompt(&self, name: &str) -> PathBuf {
        self.path.join(".codex/prompts").join(format!("{name}.md"))
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
/// Read straight from `project_canon_core::CANON` (the one physical copy, packaged in core) so
/// this stays self-contained inside the published tarball and free of root-symlink fragility.
fn master_canon() -> String {
    project_canon_core::CANON.to_string()
}

#[test]
fn install_writes_all_agent_forms() {
    let t = Tmp::new("both");
    let out = run(&["install", "--target", t.str()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(t.claude_skill().is_file(), "claude SKILL.md missing");
    assert!(
        t.skill_resource("pi", "ai-first-cli-canon", "SKILL.md")
            .is_file(),
        "pi SKILL.md missing"
    );
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
    assert!(!t.path.join(".pi").exists());
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
    assert!(stdout.contains("\"agent\":\"pi\""));
    assert!(stdout.contains("\"agent\":\"codex\""));
    assert!(stdout.contains("\"name\":\"cli-canon\""));
    assert!(stdout.contains("\"resource\":\"templates/review-report.md\""));
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

    // Without --force: refused, exit 1, file untouched.
    let out = run(&["install", "--target", t.str(), "--agent", "claude"]);
    assert_eq!(
        code(&out),
        1,
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
    assert_eq!(code(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("newer"));
}

#[test]
fn unknown_skill_name_is_a_usage_error() {
    let t = Tmp::new("unknown");
    let out = run(&["install", "nope", "--target", t.str()]);
    assert_eq!(code(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("nope"));
}

#[test]
fn bad_agent_is_a_usage_error() {
    let t = Tmp::new("badagent");
    let out = run(&["install", "--target", t.str(), "--agent", "emacs"]);
    assert_eq!(code(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("emacs"));
}

#[test]
fn unknown_flag_is_a_usage_error() {
    let t = Tmp::new("badflag");
    let out = run(&["install", "--target", t.str(), "--nope"]);
    assert_eq!(code(&out), 1);
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
    assert_eq!(code(&bad), 1);
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
fn cli_canon_installs_complete_native_trees_and_a_self_contained_codex_prompt() {
    let t = Tmp::new("cli-canon-tree");
    let out = run(&["install", "cli-canon", "--target", t.str()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    for agent in ["claude", "pi"] {
        for resource in [
            "SKILL.md",
            "templates/conformance-probes.md",
            "templates/generate-plan.md",
            "templates/review-report.md",
        ] {
            assert!(
                t.skill_resource(agent, "cli-canon", resource).is_file(),
                "missing {agent} resource {resource}"
            );
        }
        let skill =
            std::fs::read_to_string(t.skill_resource(agent, "cli-canon", "SKILL.md")).unwrap();
        assert!(skill.contains("cli_version:"));
        assert!(skill.contains("schema_version:"));
    }

    let codex = std::fs::read_to_string(t.named_codex_prompt("cli-canon")).unwrap();
    assert!(!codex.starts_with("---"));
    for resource in [
        "SKILL.md",
        "templates/conformance-probes.md",
        "templates/generate-plan.md",
        "templates/review-report.md",
    ] {
        assert!(codex.contains(&format!("bundled resource: {resource}")));
    }
}

#[test]
fn cli_canon_print_discovers_and_streams_each_native_resource_exactly() {
    let t = Tmp::new("cli-canon-print");
    assert_eq!(
        code(&run(&[
            "install",
            "cli-canon",
            "--target",
            t.str(),
            "--agent",
            "pi",
        ])),
        0
    );
    for resource in [
        "SKILL.md",
        "templates/conformance-probes.md",
        "templates/generate-plan.md",
        "templates/review-report.md",
    ] {
        let printed = run(&[
            "print",
            "cli-canon",
            "--agent",
            "pi",
            "--resource",
            resource,
        ]);
        assert_eq!(code(&printed), 0);
        assert_eq!(
            printed.stdout,
            std::fs::read(t.skill_resource("pi", "cli-canon", resource)).unwrap()
        );
    }

    let json = run(&["print", "cli-canon", "--agent", "pi", "--json"]);
    let stdout = String::from_utf8(json.stdout).unwrap();
    assert!(stdout.contains("\"resource\":\"SKILL.md\""));
    assert!(stdout.contains("templates/conformance-probes.md"));
    assert!(stdout.contains("templates/generate-plan.md"));
    assert!(stdout.contains("templates/review-report.md"));
    assert!(stdout.contains("\"path_in_repo\":\"skills/cli-canon/SKILL.md\""));

    let template = run(&[
        "print",
        "cli-canon",
        "--agent",
        "pi",
        "--resource",
        "templates/review-report.md",
        "--json",
    ]);
    assert!(String::from_utf8(template.stdout)
        .unwrap()
        .contains("\"path_in_repo\":\"skills/cli-canon/templates/review-report.md\""));
}

#[test]
fn cli_canon_reinstall_is_idempotent_for_the_whole_tree() {
    let t = Tmp::new("cli-canon-idem");
    let args = ["install", "cli-canon", "--target", t.str(), "--agent", "pi"];
    assert_eq!(code(&run(&args)), 0);
    let out = run(&args);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("wrote 0 files, 4 unchanged"), "{stdout}");
}

#[test]
fn list_reports_the_shipped_skill() {
    let out = run(&["list"]);
    assert_eq!(code(&out), 0);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("ai-first-cli-canon"));
    assert!(text.contains("cli-canon"));

    let json = run(&["list", "--json"]);
    assert_eq!(code(&json), 0);
    let stdout = String::from_utf8_lossy(&json.stdout);
    assert!(stdout.contains("\"verb\":\"skill list\""));
    assert!(stdout.contains("\"name\":\"ai-first-cli-canon\""));
    assert!(stdout.contains("\"name\":\"cli-canon\""));
    assert!(stdout.contains("\"supported_agents\":[\"claude\",\"pi\",\"codex\"]"));
    assert!(stdout.contains("templates/conformance-probes.md"));
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
    assert!(stdout.contains("\"skill_schema_version\":1"));
    assert!(stdout.contains("\"path_in_repo\":\"AGENTS-AI-FIRST-CLI.md\""));
    assert!(stdout.contains("\"content\":"));
    assert!(stdout.contains("\"exit_code\":0"));
}

#[test]
fn show_is_an_alias_for_print() {
    // §15 names the read-only streamer `show`; §16 names it `print`. Both must work identically.
    let print = run(&["print", "ai-first-cli-canon", "--agent", "codex"]);
    let show = run(&["show", "ai-first-cli-canon", "--agent", "codex"]);
    assert_eq!(code(&print), 0);
    assert_eq!(code(&show), 0);
    assert_eq!(print.stdout, show.stdout);
}

#[test]
fn print_codex_is_byte_identical_to_install() {
    // §16 for the Codex form too (the render path differs from Claude — no frontmatter).
    let t = Tmp::new("print-codex");
    assert_eq!(
        code(&run(&["install", "--target", t.str(), "--agent", "codex"])),
        0
    );
    let installed = std::fs::read_to_string(t.codex_prompt()).unwrap();
    let printed = run(&["print", "ai-first-cli-canon", "--agent", "codex"]);
    assert_eq!(String::from_utf8_lossy(&printed.stdout), installed);
}

#[test]
fn list_and_print_reject_help_inline_value() {
    // Strict §1 parsing is uniform: `--help=x` is a usage error in every subcommand.
    assert_eq!(code(&run(&["list", "--help=x"])), 1);
    assert_eq!(code(&run(&["print", "--help=x"])), 1);
}

#[test]
fn list_and_print_refuse_a_malformed_env() {
    // env validation is uniform across all three subcommands, not just install.
    for sub in [
        ["list"].as_slice(),
        ["print", "ai-first-cli-canon"].as_slice(),
    ] {
        let out = base_command()
            .arg("skill")
            .args(sub)
            .env("PROJECT_CANON_TW_ENABLED", "not-a-bool")
            .output()
            .unwrap();
        assert_eq!(code(&out), 1, "sub {sub:?} should reject a malformed env");
    }
}

#[test]
fn list_json_envelope_is_consistent() {
    let out = run(&["list", "--json"]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"verb\":\"skill list\""));
    assert!(stdout.contains("\"exit_code\":0"));
    // Top-level cli_version + the per-skill skill_schema_version key (uniform with print).
    assert!(stdout.contains("\"cli_version\":"));
    assert!(stdout.contains("\"skill_schema_version\":1"));
}

#[test]
fn blocked_install_emits_a_json_error_envelope() {
    // Under --json, a blocking conflict must still yield a structured envelope, not only stderr.
    let t = Tmp::new("blocked-json");
    let p = t.claude_skill();
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "HAND WRITTEN").unwrap();
    let out = run(&[
        "install",
        "--target",
        t.str(),
        "--agent",
        "claude",
        "--json",
    ]);
    assert_eq!(code(&out), 1);
    assert!(out.stdout.is_empty(), "failure must not write stdout");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.trim().starts_with('{'),
        "expected JSON error: {stderr}"
    );
    assert!(stderr.contains("\"schema_version\":1"));
    assert!(stderr.contains("\"code\":\"already_exists\""));
    // The foreign file is untouched.
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "HAND WRITTEN");
}

#[cfg(unix)]
#[test]
fn a_symlink_at_the_target_is_refused_and_not_followed() {
    // A planted final-path symlink must NOT be written through to its target (write-through would
    // clobber a foreign file, e.g. ~/.ssh/...). It is refused as a conflict; --force replaces the
    // link itself (via rename), never the file it points at.
    let t = Tmp::new("symlink");
    let sentinel = t.path.join("sentinel.txt");
    std::fs::write(&sentinel, "DO NOT TOUCH").unwrap();
    let link = t.codex_prompt();
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&sentinel, &link).unwrap();

    let out = run(&["install", "--target", t.str(), "--agent", "codex"]);
    assert_eq!(
        code(&out),
        1,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The symlink's target was never written through.
    assert_eq!(std::fs::read_to_string(&sentinel).unwrap(), "DO NOT TOUCH");

    // --force replaces the link with a real file; the sentinel is still untouched.
    let out = run(&[
        "install",
        "--target",
        t.str(),
        "--agent",
        "codex",
        "--force",
    ]);
    assert_eq!(code(&out), 0);
    assert_eq!(std::fs::read_to_string(&sentinel).unwrap(), "DO NOT TOUCH");
    assert!(std::fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_file());
    assert!(std::fs::read_to_string(&link)
        .unwrap()
        .contains(&master_canon()));
}

#[test]
fn print_unknown_name_is_a_usage_error() {
    let out = run(&["print", "nope"]);
    assert_eq!(code(&out), 1);
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
