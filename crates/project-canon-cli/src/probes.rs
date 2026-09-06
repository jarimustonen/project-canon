//! The shared **mechanical-probe substrate** — file-existence / repo-shape checks that both the
//! `doctor` gate and the `review` audit run over a target repo.
//!
//! A static mechanical probe is a *decidable* check (grep / file-existence / repo-shape); it never
//! builds or runs the target tool. This module also owns review's separate, explicitly opt-in
//! runtime probes. Doctor consumes only the static registry; `review --run` invokes the runtime
//! registry with timeout-bounded, captured, read-only calls. Extracting both here keeps probe
//! mechanics out of verb rendering and keeps `doctor`/`review` disjoint.
//!
//! A probe returns `io::Result<ProbeOutcome>`: `Ok(ProbeOutcome)` is a decidable pass/miss, an
//! `Err` is an *operational* I/O fault (permission denied, transient error) that each verb wraps
//! into its own exit-2 fault — keeping "could not evaluate" distinct from "the check missed".

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// User-configured inputs for probes whose answer depends on operator knowledge.
pub struct ProbeContext<'a> {
    /// Exact, case-insensitive markers known to identify the operator's private environment.
    pub user_specific_deny_list: &'a BTreeSet<String>,
}

/// The outcome of running one mechanical probe: a decidable pass/miss. An operational I/O error is
/// *not* an outcome — probes return `io::Result<ProbeOutcome>`, and an `Err` is the caller's to
/// route to its own operational-fault exit.
pub struct ProbeOutcome {
    /// Whether the conformance check passed.
    pub passed: bool,
    /// The human-facing evidence line (the observation that settled the row).
    pub message: String,
}

impl ProbeOutcome {
    fn pass(message: impl Into<String>) -> ProbeOutcome {
        ProbeOutcome {
            passed: true,
            message: message.into(),
        }
    }
    fn fail(message: impl Into<String>) -> ProbeOutcome {
        ProbeOutcome {
            passed: false,
            message: message.into(),
        }
    }
}

/// Runtime sections that `review --run` can decide using read-only target invocations.
pub const RUNTIME_PROBE_IDS: [&str; 8] = [
    "canon.s02",
    "canon.s08",
    "canon.s10",
    "canon.s14",
    "canon.s15",
    "canon.s16",
    "canon.s17",
    "canon.s18",
];

/// The three-state result of an opt-in runtime probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProbeStatus {
    Pass,
    Gap,
    CouldNotProbe,
}

impl RuntimeProbeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Gap => "gap",
            Self::CouldNotProbe => "could-not-probe",
        }
    }
}

/// Evidence for one runtime-observable canon section.
#[derive(Debug, Clone)]
pub struct RuntimeProbeOutcome {
    pub id: &'static str,
    pub status: RuntimeProbeStatus,
    pub message: String,
    pub(crate) blocks_suite: bool,
}

pub const RUNTIME_TIMEOUT_MS: u64 = 3000;
const DEFAULT_RUNTIME_TIMEOUT: Duration = Duration::from_millis(RUNTIME_TIMEOUT_MS);
const MAX_CAPTURE_BYTES: u64 = 1_048_576;

/// Execute the explicitly named target binary using only fixed, read-only argument vectors.
///
/// Every child has null stdin, captured and size-bounded output, and a timeout. No shell is used.
/// An infrastructure failure blocks later probes so one hanging binary costs one timeout rather
/// than one timeout per section; every unattempted row is still reported as `could-not-probe`.
pub fn runtime_probes(binary: &Path, repo: &Path) -> Vec<RuntimeProbeOutcome> {
    runtime_probes_with_timeout(binary, repo, DEFAULT_RUNTIME_TIMEOUT)
}

fn runtime_probes_with_timeout(
    binary: &Path,
    repo: &Path,
    timeout: Duration,
) -> Vec<RuntimeProbeOutcome> {
    let runner = RuntimeRunner {
        binary: binary.to_path_buf(),
        timeout,
        current_dir: repo.to_path_buf(),
    };
    let mut outcomes = Vec::with_capacity(RUNTIME_PROBE_IDS.len());
    let mut blocked: Option<String> = None;
    for id in RUNTIME_PROBE_IDS {
        let outcome = if let Some(reason) = &blocked {
            RuntimeProbeOutcome {
                id,
                status: RuntimeProbeStatus::CouldNotProbe,
                message: format!("not attempted after target execution failure: {reason}"),
                blocks_suite: true,
            }
        } else {
            probe_runtime_section(id, &runner)
        };
        if outcome.blocks_suite {
            blocked = Some(outcome.message.clone());
        }
        outcomes.push(outcome);
    }
    outcomes
}

struct RuntimeRunner {
    binary: PathBuf,
    timeout: Duration,
    current_dir: PathBuf,
}

struct ChildCapture {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    output_truncated: bool,
}

#[derive(Debug)]
enum RunFailure {
    Start(String),
    Timeout,
    Crash,
    Capture(String),
    Wait(String),
}

impl RuntimeRunner {
    fn run(&self, args: &[&str]) -> Result<ChildCapture, RunFailure> {
        let mut command = Command::new(&self.binary);
        command
            .args(args)
            .current_dir(&self.current_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // project-canon ships on Unix targets. A dedicated process group lets timeout handling
        // terminate descendants as well as the direct child, preventing continued work and pipe
        // holders after review returns. Other targets retain direct-child kill as a fallback.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            command.process_group(0);
        }
        let mut child = command
            .spawn()
            .map_err(|error| RunFailure::Start(error.to_string()))?;

        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let (stdout_sender, stdout_receiver) = std::sync::mpsc::sync_channel(1);
        let (stderr_sender, stderr_receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let _ = stdout_sender.send(read_bounded(stdout));
        });
        std::thread::spawn(move || {
            let _ = stderr_sender.send(read_bounded(stderr));
        });
        let started = Instant::now();
        // Drain both pipes before reaping the group leader. On Unix this keeps its pid/pgid
        // reserved until descendants are terminated, eliminating any pid-reuse window. The same
        // deadline covers capture and process exit.
        let remaining = || self.timeout.saturating_sub(started.elapsed());
        let (stdout, stdout_truncated) = match stdout_receiver.recv_timeout(remaining()) {
            Ok(Ok(capture)) => capture,
            Ok(Err(error)) => {
                kill_child_tree(&mut child);
                return Err(RunFailure::Capture(error.to_string()));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                kill_child_tree(&mut child);
                return Err(RunFailure::Timeout);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                kill_child_tree(&mut child);
                return Err(RunFailure::Wait("stdout capture thread failed".to_string()));
            }
        };
        let (stderr, stderr_truncated) = match stderr_receiver.recv_timeout(remaining()) {
            Ok(Ok(capture)) => capture,
            Ok(Err(error)) => {
                kill_child_tree(&mut child);
                return Err(RunFailure::Capture(error.to_string()));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                kill_child_tree(&mut child);
                return Err(RunFailure::Timeout);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                kill_child_tree(&mut child);
                return Err(RunFailure::Wait("stderr capture thread failed".to_string()));
            }
        };
        let status = wait_for_child_without_pid_reuse(&mut child, started, self.timeout)?;
        let code = status.code().ok_or(RunFailure::Crash)?;
        Ok(ChildCapture {
            code,
            stdout,
            stderr,
            output_truncated: stdout_truncated || stderr_truncated,
        })
    }
}

#[cfg(unix)]
fn wait_for_child_without_pid_reuse(
    child: &mut std::process::Child,
    started: Instant,
    timeout: Duration,
) -> Result<std::process::ExitStatus, RunFailure> {
    loop {
        let mut info = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
        // SAFETY: `info` points to writable siginfo storage. WNOWAIT observes the child state
        // without reaping it, keeping the pid/pgid reserved until descendants are terminated.
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                child.id(),
                info.as_mut_ptr(),
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result != 0 {
            kill_child_tree(child);
            return Err(RunFailure::Wait(
                std::io::Error::last_os_error().to_string(),
            ));
        }
        // SAFETY: successful waitid initialized the siginfo storage.
        let exited = unsafe { info.assume_init().si_pid() != 0 };
        if exited {
            terminate_process_group(child.id());
            return child
                .wait()
                .map_err(|error| RunFailure::Wait(error.to_string()));
        }
        if started.elapsed() >= timeout {
            kill_child_tree(child);
            return Err(RunFailure::Timeout);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(not(unix))]
fn wait_for_child_without_pid_reuse(
    child: &mut std::process::Child,
    started: Instant,
    timeout: Duration,
) -> Result<std::process::ExitStatus, RunFailure> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Ok(None) => {
                kill_child_tree(child);
                return Err(RunFailure::Timeout);
            }
            Err(error) => {
                kill_child_tree(child);
                return Err(RunFailure::Wait(error.to_string()));
            }
        }
    }
}

fn terminate_process_group(pid: u32) {
    #[cfg(unix)]
    {
        // SAFETY: the spawned child is placed in a fresh process group whose id equals its pid.
        // A negative pid addresses only that group. The group id remains reserved while any
        // descendant is in the group, even after the leader has been reaped.
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    let _ = pid;
}

fn kill_child_tree(child: &mut std::process::Child) {
    terminate_process_group(child.id());
    let _ = child.kill();
    let _ = child.wait();
}

fn read_bounded(mut reader: impl Read) -> std::io::Result<(Vec<u8>, bool)> {
    let mut bytes = Vec::new();
    let mut truncated = false;
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk)? {
            0 => break,
            read => {
                let remaining = MAX_CAPTURE_BYTES.saturating_sub(bytes.len() as u64) as usize;
                let retained = read.min(remaining);
                bytes.extend_from_slice(&chunk[..retained]);
                truncated |= retained < read;
            }
        }
    }
    Ok((bytes, truncated))
}

fn probe_runtime_section(id: &'static str, runner: &RuntimeRunner) -> RuntimeProbeOutcome {
    let result = match id {
        "canon.s02" => probe_exit_contract(runner),
        "canon.s08" => probe_config_surface(runner),
        "canon.s10" => probe_version_surface(runner),
        "canon.s14" => probe_help_surface(runner),
        "canon.s15" => probe_skill_install_surface(runner),
        "canon.s16" => probe_skill_print(runner),
        "canon.s17" => probe_skill_sync(runner),
        "canon.s18" => probe_doctor_surface(runner),
        _ => unreachable!("runtime probe id registry is closed"),
    };
    match result {
        Ok(message) => RuntimeProbeOutcome {
            id,
            status: RuntimeProbeStatus::Pass,
            message,
            blocks_suite: false,
        },
        Err(RuntimeCheckError::Gap(message)) => RuntimeProbeOutcome {
            id,
            status: RuntimeProbeStatus::Gap,
            message,
            blocks_suite: false,
        },
        Err(RuntimeCheckError::Unavailable {
            message,
            blocks_suite,
        }) => RuntimeProbeOutcome {
            id,
            status: RuntimeProbeStatus::CouldNotProbe,
            message,
            blocks_suite,
        },
    }
}

enum RuntimeCheckError {
    Gap(String),
    Unavailable { message: String, blocks_suite: bool },
}

type RuntimeCheck<T = String> = Result<T, RuntimeCheckError>;

fn invoke(runner: &RuntimeRunner, args: &[&str]) -> RuntimeCheck<ChildCapture> {
    let unavailable = |message: String, blocks_suite| RuntimeCheckError::Unavailable {
        message,
        blocks_suite,
    };
    match runner.run(args) {
        Ok(capture) if capture.output_truncated => Err(unavailable(
            format!("{} exceeded the 1 MiB capture limit", display_args(args)),
            false,
        )),
        Ok(capture) => Ok(capture),
        Err(RunFailure::Start(error)) => Err(unavailable(
            format!(
                "could not start explicitly named binary for {}: {error}",
                display_args(args)
            ),
            true,
        )),
        Err(RunFailure::Timeout) => Err(unavailable(
            format!(
                "{} timed out after {} ms and was killed",
                display_args(args),
                runner.timeout.as_millis()
            ),
            true,
        )),
        Err(RunFailure::Crash) => Err(unavailable(
            format!("{} terminated without an exit code", display_args(args)),
            false,
        )),
        Err(RunFailure::Capture(error)) => Err(unavailable(
            format!("could not capture {} output: {error}", display_args(args)),
            false,
        )),
        Err(RunFailure::Wait(error)) => Err(unavailable(
            format!("could not wait for {}: {error}", display_args(args)),
            true,
        )),
    }
}

fn display_args(args: &[&str]) -> String {
    if args.is_empty() {
        "<binary>".to_string()
    } else {
        format!("<binary> {}", args.join(" "))
    }
}

fn expect_json(
    capture: &ChildCapture,
    args: &[&str],
    allowed_codes: &[i32],
) -> RuntimeCheck<Value> {
    if !allowed_codes.contains(&capture.code) {
        return Err(RuntimeCheckError::Gap(format!(
            "{} exited {} (expected {})",
            display_args(args),
            capture.code,
            allowed_codes
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(" or ")
        )));
    }
    serde_json::from_slice(&capture.stdout).map_err(|_| {
        RuntimeCheckError::Gap(format!(
            "{} did not emit one valid JSON payload on stdout",
            display_args(args)
        ))
    })
}

fn object_has(value: &Value, key: &str, predicate: impl FnOnce(&Value) -> bool) -> bool {
    value.get(key).is_some_and(predicate)
}

fn schema_object(value: &Value) -> bool {
    value.is_object()
        && value
            .get("schema_version")
            .and_then(Value::as_i64)
            .is_some_and(|schema| schema > 0)
}

fn probe_exit_contract(runner: &RuntimeRunner) -> RuntimeCheck {
    let args = ["__project_canon_probe_unknown_subcommand__", "--json"];
    let capture = invoke(runner, &args)?;
    if capture.code != 1 || !capture.stdout.is_empty() {
        return Err(RuntimeCheckError::Gap(
            "caller-actionable unknown-command probe must exit 1 with empty stdout".to_string(),
        ));
    }
    let error: Value = serde_json::from_slice(&capture.stderr).map_err(|_| {
        RuntimeCheckError::Gap(
            "caller-actionable error did not use a JSON envelope on stderr".to_string(),
        )
    })?;
    if !schema_object(&error)
        || !object_has(&error, "error", |v| {
            v.is_object()
                && object_has(v, "code", Value::is_string)
                && object_has(v, "message", Value::is_string)
        })
    {
        return Err(RuntimeCheckError::Gap(
            "caller-actionable error envelope lacks schema_version/error.code/error.message"
                .to_string(),
        ));
    }
    let help_args = ["--help", "--json"];
    let help = expect_json(&invoke(runner, &help_args)?, &help_args, &[0])?;
    let advertised = help
        .get("exit_codes")
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            ["0", "1", "2"].iter().all(|wanted| {
                rows.iter()
                    .any(|row| row.get("code").and_then(Value::as_str) == Some(*wanted))
            })
        });
    if !advertised {
        return Err(RuntimeCheckError::Gap(
            "structured help does not advertise distinct exit codes 0, 1, and 2".to_string(),
        ));
    }
    Ok("caller error exits 1 with the central JSON envelope; help advertises distinct 0/1/2 mapping".to_string())
}

fn probe_config_surface(runner: &RuntimeRunner) -> RuntimeCheck {
    let path_args = ["config", "path", "--json"];
    let path = expect_json(&invoke(runner, &path_args)?, &path_args, &[0])?;
    if !schema_object(&path)
        || !object_has(&path, "config_path", Value::is_string)
        || !object_has(&path, "exists", Value::is_boolean)
    {
        return Err(RuntimeCheckError::Gap(
            "config path --json lacks schema_version/config_path/exists".to_string(),
        ));
    }
    let show_args = ["config", "show", "--json"];
    let show = expect_json(&invoke(runner, &show_args)?, &show_args, &[0])?;
    if !schema_object(&show) || !object_has(&show, "values", Value::is_object) {
        return Err(RuntimeCheckError::Gap(
            "config show --json lacks schema_version/values".to_string(),
        ));
    }
    Ok("config path/show --json are present with structured inspection payloads".to_string())
}

fn version_json(runner: &RuntimeRunner) -> RuntimeCheck<Value> {
    let args = ["version", "--json"];
    let value = expect_json(&invoke(runner, &args)?, &args, &[0])?;
    let schema = value.get("schema_version").and_then(Value::as_i64);
    let valid = schema_object(&value)
        && object_has(&value, "supported_schemas", |v| {
            v.as_array().is_some_and(|schemas| {
                !schemas.is_empty()
                    && schemas
                        .iter()
                        .all(|schema| schema.as_i64().is_some_and(|schema| schema > 0))
                    && schema.is_some_and(|current| {
                        schemas
                            .iter()
                            .any(|candidate| candidate.as_i64() == Some(current))
                    })
            })
        })
        && object_has(&value, "skills", Value::is_array)
        && value
            .get("version")
            .and_then(Value::as_str)
            .is_some_and(|version| !version.is_empty())
        && value.get("commit").is_some_and(|commit| {
            commit.is_null()
                || commit
                    .as_str()
                    .is_some_and(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        });
    if valid {
        Ok(value)
    } else {
        Err(RuntimeCheckError::Gap(
            "version --json lacks a valid schema_version/supported_schemas/skills/version/commit envelope"
                .to_string(),
        ))
    }
}

fn require_same_capture(
    canonical_args: &[&str],
    canonical: &ChildCapture,
    alias_args: &[&str],
    alias: &ChildCapture,
) -> RuntimeCheck<()> {
    let mismatch = if alias.code != canonical.code {
        Some(format!("exit code {} != {}", alias.code, canonical.code))
    } else if alias.stdout != canonical.stdout {
        Some(format!(
            "stdout differs ({} bytes != {} bytes)",
            alias.stdout.len(),
            canonical.stdout.len()
        ))
    } else if alias.stderr != canonical.stderr {
        Some(format!(
            "stderr differs ({} bytes != {} bytes)",
            alias.stderr.len(),
            canonical.stderr.len()
        ))
    } else {
        None
    };
    if let Some(detail) = mismatch {
        Err(RuntimeCheckError::Gap(format!(
            "{} is not a full alias of {}: {detail}",
            display_args(alias_args),
            display_args(canonical_args)
        )))
    } else {
        Ok(())
    }
}

fn probe_version_surface(runner: &RuntimeRunner) -> RuntimeCheck {
    let text_args = ["version"];
    let text = invoke(runner, &text_args)?;
    let text_alias_args = ["--version"];
    let text_alias = invoke(runner, &text_alias_args)?;
    require_same_capture(&text_args, &text, &text_alias_args, &text_alias)?;

    let json_args = ["version", "--json"];
    let json = invoke(runner, &json_args)?;
    for alias_args in [
        ["--version", "--json"].as_slice(),
        ["--json", "--version"].as_slice(),
    ] {
        let alias = invoke(runner, alias_args)?;
        require_same_capture(&json_args, &json, alias_args, &alias)?;
    }
    // Validate the canonical payload after proving all aliases emitted the same bytes and status.
    version_json(runner)?;
    Ok("version/--version are byte-identical in text and JSON modes; the JSON payload carries schema, compatibility, provenance, and skill metadata".to_string())
}

fn probe_help_surface(runner: &RuntimeRunner) -> RuntimeCheck {
    let args = ["--help", "--json"];
    let value = expect_json(&invoke(runner, &args)?, &args, &[0])?;
    let valid = schema_object(&value)
        && object_has(&value, "command_path", Value::is_array)
        && object_has(&value, "subcommands", Value::is_array)
        && object_has(&value, "examples", |v| {
            v.as_array().is_some_and(|a| !a.is_empty())
        });
    if valid {
        Ok("--help --json has command_path, subcommands, and examples".to_string())
    } else {
        Err(RuntimeCheckError::Gap(
            "--help --json lacks schema_version/command_path/subcommands/examples".to_string(),
        ))
    }
}

type SkillRows = Vec<(String, String, i64)>;

fn probe_skill_list(runner: &RuntimeRunner) -> RuntimeCheck<(SkillRows, Value)> {
    let args = ["skill", "list", "--json"];
    let value = expect_json(&invoke(runner, &args)?, &args, &[0])?;
    if !schema_object(&value) {
        return Err(RuntimeCheckError::Gap(
            "skill list --json lacks schema_version".to_string(),
        ));
    }
    let skills = value
        .get("skills")
        .and_then(Value::as_array)
        .filter(|skills| !skills.is_empty())
        .ok_or_else(|| RuntimeCheckError::Gap("skill list --json has no skills[]".to_string()))?;
    let rows = skills
        .iter()
        .map(|skill| {
            let name = skill.get("name").and_then(Value::as_str).filter(|name| {
                !name.is_empty()
                    && name.len() <= 64
                    && name.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            });
            let version = skill
                .get("cli_version")
                .and_then(Value::as_str)
                .filter(|version| !version.is_empty());
            let schema = skill
                .get("skill_schema_version")
                .and_then(Value::as_i64)
                .filter(|schema| *schema > 0);
            match (name, version, schema) {
                (Some(name), Some(version), Some(schema)) => {
                    Ok((name.to_string(), version.to_string(), schema))
                }
                _ => Err(RuntimeCheckError::Gap(
                    "skill list --json has an invalid name/version/schema row".to_string(),
                )),
            }
        })
        .collect::<RuntimeCheck<SkillRows>>()?;
    Ok((rows, value))
}

fn strict_string_set<'a>(
    value: Option<&'a Value>,
    field: &str,
) -> Result<BTreeSet<&'a str>, String> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{field} must be an array"))?;
    let mut result = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let item = value
            .as_str()
            .filter(|item| !item.is_empty())
            .ok_or_else(|| format!("{field}[{index}] must be a non-empty string"))?;
        if !result.insert(item) {
            return Err(format!("{field} contains duplicate value {item:?}"));
        }
    }
    Ok(result)
}

fn validate_skill_install_metadata(list: &Value) -> Result<(), String> {
    let required_agents = BTreeSet::from(["claude", "pi", "codex"]);
    let declared_agents = strict_string_set(list.get("supported_agents"), "supported_agents")?;
    if !required_agents.is_subset(&declared_agents) {
        return Err("supported_agents must include claude, pi, and codex".to_string());
    }
    let install = list
        .get("install")
        .and_then(Value::as_object)
        .ok_or_else(|| "install capability object is missing".to_string())?;
    for (field, expected) in [
        ("selection_flag", "--agent"),
        ("default", "all"),
        ("target_flag", "--target"),
        ("dry_run_flag", "--dry-run"),
        ("force_flag", "--force"),
    ] {
        if install.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(format!("install.{field} must be {expected:?}"));
        }
    }
    let accepted = strict_string_set(install.get("accepted_values"), "install.accepted_values")?;
    let selectable = accepted
        .iter()
        .copied()
        .filter(|value| *value != "all")
        .collect::<BTreeSet<_>>();
    if !accepted.contains("all") || selectable != declared_agents {
        return Err(
            "install.accepted_values must be supported_agents plus the explicit value all"
                .to_string(),
        );
    }
    for (field, expected) in [
        ("interactive", false),
        ("no_clobber_default", true),
        ("overwrite_requires_force", true),
    ] {
        if install.get(field).and_then(Value::as_bool) != Some(expected) {
            return Err(format!("install.{field} must be {expected}"));
        }
    }
    let layouts = install
        .get("layouts")
        .and_then(Value::as_array)
        .ok_or_else(|| "install.layouts must be an array".to_string())?;
    let mut by_agent = std::collections::BTreeMap::new();
    for (index, layout) in layouts.iter().enumerate() {
        let object = layout
            .as_object()
            .ok_or_else(|| format!("install.layouts[{index}] must be an object"))?;
        let agent = object
            .get("agent")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("install.layouts[{index}].agent must be a non-empty string"))?;
        let path = object
            .get("path")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("install.layouts[{index}].path must be a non-empty string"))?;
        let form = object
            .get("form")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("install.layouts[{index}].form must be a non-empty string"))?;
        if !declared_agents.contains(agent) {
            return Err(format!(
                "install.layouts[{index}].agent {agent:?} is absent from supported_agents"
            ));
        }
        if by_agent.insert(agent, (path, form)).is_some() {
            return Err(format!(
                "install.layouts contains duplicate agent {agent:?}"
            ));
        }
    }
    if by_agent.keys().copied().collect::<BTreeSet<_>>() != declared_agents {
        return Err(
            "install.layouts must contain exactly one row for every supported agent".into(),
        );
    }
    for (agent, path, form) in [
        ("claude", ".claude/skills/<name>/...", "agent-skill-tree"),
        ("pi", ".pi/agent/skills/<name>/...", "agent-skill-tree"),
        ("codex", ".codex/skills/<name>/...", "agent-skill-tree"),
    ] {
        if by_agent.get(agent).copied() != Some((path, form)) {
            return Err(format!(
                "install.layouts lacks {agent} path {path:?} with form {form:?}"
            ));
        }
    }
    Ok(())
}

fn probe_skill_install_surface(runner: &RuntimeRunner) -> RuntimeCheck {
    let (_, list) = probe_skill_list(runner)?;
    validate_skill_install_metadata(&list).map_err(|message| {
        RuntimeCheckError::Gap(format!("skill list --json install metadata gap: {message}"))
    })?;
    Ok("skill catalog declares Claude, pi, and Codex; install metadata declares --agent defaulting to all with single-runtime/explicit-all selection, --target, non-interactive safety flags, and each native path/form".to_string())
}

fn print_skill_json(runner: &RuntimeRunner, name: &str) -> RuntimeCheck<Value> {
    let args = ["skill", "print", name, "--json"];
    let value = expect_json(&invoke(runner, &args)?, &args, &[0])?;
    let valid = schema_object(&value)
        && value.get("name").and_then(Value::as_str) == Some(name)
        && value
            .get("cli_version")
            .and_then(Value::as_str)
            .is_some_and(|version| !version.is_empty())
        && value
            .get("skill_schema_version")
            .and_then(Value::as_i64)
            .is_some_and(|schema| schema > 0)
        && value
            .get("content")
            .and_then(Value::as_str)
            .is_some_and(|content| !content.is_empty());
    if valid {
        Ok(value)
    } else {
        Err(RuntimeCheckError::Gap(
            "skill print <name> --json lacks the required metadata/content shape".to_string(),
        ))
    }
}

fn probe_skill_print(runner: &RuntimeRunner) -> RuntimeCheck {
    let (skills, _) = probe_skill_list(runner)?;
    print_skill_json(runner, &skills[0].0)?;
    Ok("skill print <listed-name> --json is structured and read-only".to_string())
}

fn probe_skill_sync(runner: &RuntimeRunner) -> RuntimeCheck {
    let version = version_json(runner)?;
    let (listed, _) = probe_skill_list(runner)?;
    let cli_version = version
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let version_skills = version
        .get("skills")
        .and_then(Value::as_array)
        .ok_or_else(|| RuntimeCheckError::Gap("version --json lacks skills[]".to_string()))?;
    if version_skills.len() != listed.len() {
        return Err(RuntimeCheckError::Gap(
            "version --json and skill list --json expose different skill counts".to_string(),
        ));
    }
    for (name, listed_version, listed_schema) in &listed {
        let metadata_matches = version_skills.iter().any(|skill| {
            skill.get("name").and_then(Value::as_str) == Some(name.as_str())
                && skill.get("cli_version").and_then(Value::as_str) == Some(listed_version.as_str())
                && skill.get("schema_version").and_then(Value::as_i64) == Some(*listed_schema)
        });
        if !metadata_matches || listed_version != cli_version {
            return Err(RuntimeCheckError::Gap(format!(
                "skill metadata for {name:?} is not synchronized across version and skill list"
            )));
        }
    }
    // `skill print` shape is section 16's check. For §17, one catalog-selected sample is enough
    // to verify that the running binary stamps synchronized frontmatter without making runtime
    // proportional to a target-controlled catalog size.
    let sampled = &listed[0];
    let printed = print_skill_json(runner, &sampled.0)?;
    let content = printed
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let printed_schema = printed
        .get("skill_schema_version")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    if printed.get("cli_version").and_then(Value::as_str) != Some(cli_version)
        || printed_schema != sampled.2
        || frontmatter_value(content, "cli_version") != Some(cli_version)
        || frontmatter_value(content, "schema_version").and_then(|value| value.parse::<i64>().ok())
            != Some(printed_schema)
    {
        return Err(RuntimeCheckError::Gap(format!(
            "printed skill {:?} lacks synchronized version frontmatter",
            sampled.0
        )));
    }
    Ok(
        "version and skill-list metadata match; sampled printed skill frontmatter is synchronized"
            .to_string(),
    )
}

fn frontmatter_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        if let Some(value) = line
            .strip_prefix(key)
            .and_then(|line| line.strip_prefix(':'))
        {
            return Some(value.trim().trim_matches(['\'', '"']));
        }
    }
    None
}

fn probe_doctor_surface(runner: &RuntimeRunner) -> RuntimeCheck {
    // The runner has already set the audited repository as cwd. Passing `.` avoids duplicating a
    // relative target and preserves non-UTF-8 filesystem components at the process boundary.
    let args = ["doctor", "--json", "."];
    let capture = invoke(runner, &args)?;
    let code = capture.code;
    let value = expect_json(&capture, &args, &[0, 1])?;
    let conformant = value.get("conformant").and_then(Value::as_bool);
    let valid = schema_object(&value)
        && object_has(&value, "checks", Value::is_array)
        && object_has(&value, "summary", Value::is_object)
        && value.get("exit_code").and_then(Value::as_i64) == Some(i64::from(code))
        && matches!((code, conformant), (0, Some(true)) | (1, Some(false)));
    if valid {
        Ok("doctor --json is present with checks, summary, and conformance verdict".to_string())
    } else {
        Err(RuntimeCheckError::Gap(
            "doctor --json lacks schema_version/checks/summary/conformant".to_string(),
        ))
    }
}

/// Follow-symlinks metadata, treating only `NotFound` as "absent" (`Ok(None)`); any other error
/// (permission denied, transient I/O) propagates so it can become an operational fault.
fn stat(path: &Path) -> std::io::Result<Option<std::fs::Metadata>> {
    match std::fs::metadata(path) {
        Ok(m) => Ok(Some(m)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// No-follow metadata (the link entry itself), with the same `NotFound` → `Ok(None)` treatment.
fn lstat(path: &Path) -> std::io::Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(m) => Ok(Some(m)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Map a dimension id → its static mechanical probe, or `None` when the dimension has no
/// filesystem-decidable check. Runtime probes use the separate [`runtime_probes`] registry and
/// prose judgment remains intentionally absent. Every id here is asserted
/// to resolve in `Model::standard` by `probe_ids_exist_in_model`, so a core-side rename can't
/// silently turn an enforced MUST into a deferred/verify skip.
pub fn mechanical_probe(
    id: &str,
) -> Option<fn(&Path, &ProbeContext<'_>) -> std::io::Result<ProbeOutcome>> {
    match id {
        "base.doc-pattern" => Some(|repo, _| probe_doc_pattern(repo)),
        "base.issue-tracking" => Some(|repo, _| probe_issue_tracking(repo)),
        "base.git-hygiene" => Some(|repo, _| probe_git_hygiene(repo)),
        "base.readme" => Some(|repo, _| probe_readme(repo)),
        "base.gitignore" => Some(|repo, _| probe_gitignore(repo)),
        "canon.s15" => Some(|repo, _| probe_skill_description_lengths(repo)),
        "canon.s22" => Some(|repo, _| probe_core_cli_split(repo)),
        "canon.s23" => Some(probe_public_artifact_specifics),
        "canon.s24" => Some(probe_verified_deferrals),
        _ => None,
    }
}

/// The ids of every dimension that carries a mechanical probe — the registry's key set, kept in
/// lockstep with [`mechanical_probe`] and cross-checked against the model by
/// `every_mechanical_probe_id_exists_in_the_model`.
#[cfg(test)]
const MECHANICAL_PROBE_IDS: [&str; 9] = [
    "base.doc-pattern",
    "base.issue-tracking",
    "base.git-hygiene",
    "base.readme",
    "base.gitignore",
    "canon.s15",
    "canon.s22",
    "canon.s23",
    "canon.s24",
];

/// `AGENTS.md` and `CLAUDE.md` both present as files at the repo root (§ base.doc-pattern).
/// `CLAUDE.md` is normally a symlink to `AGENTS.md`; following it must land on a regular file, so a
/// dangling symlink, a directory, or a FIFO named `CLAUDE.md` is correctly a miss.
fn probe_doc_pattern(repo: &Path) -> std::io::Result<ProbeOutcome> {
    let agents = stat(&repo.join("AGENTS.md"))?.is_some_and(|m| m.is_file());
    let claude = stat(&repo.join("CLAUDE.md"))?.is_some_and(|m| m.is_file());
    Ok(match (agents, claude) {
        (true, true) => ProbeOutcome::pass("AGENTS.md and CLAUDE.md present"),
        (false, true) => ProbeOutcome::fail("AGENTS.md missing at repo root"),
        (true, false) => ProbeOutcome::fail("CLAUDE.md missing or not a file at repo root"),
        (false, false) => ProbeOutcome::fail("AGENTS.md and CLAUDE.md both missing at repo root"),
    })
}

/// `issues/` directory present (§ base.issue-tracking).
fn probe_issue_tracking(repo: &Path) -> std::io::Result<ProbeOutcome> {
    Ok(if stat(&repo.join("issues"))?.is_some_and(|m| m.is_dir()) {
        ProbeOutcome::pass("issues/ directory present")
    } else {
        ProbeOutcome::fail("issues/ directory missing")
    })
}

/// A `.git` entry present — a directory for a normal repo, or a gitfile for a worktree/submodule.
/// No-follow (`lstat`) so a symlinked `.git` counts by the link's presence; a permission error
/// faults rather than reading as "missing" (§ base.git-hygiene).
fn probe_git_hygiene(repo: &Path) -> std::io::Result<ProbeOutcome> {
    Ok(if lstat(&repo.join(".git"))?.is_some() {
        ProbeOutcome::pass(".git present")
    } else {
        ProbeOutcome::fail(".git missing — not a git repository")
    })
}

/// `README.md` front door present (§ base.readme, SHOULD).
fn probe_readme(repo: &Path) -> std::io::Result<ProbeOutcome> {
    Ok(
        if stat(&repo.join("README.md"))?.is_some_and(|m| m.is_file()) {
            ProbeOutcome::pass("README.md present")
        } else {
            ProbeOutcome::fail("README.md missing")
        },
    )
}

/// `.gitignore` present (§ base.gitignore, SHOULD).
fn probe_gitignore(repo: &Path) -> std::io::Result<ProbeOutcome> {
    Ok(
        if stat(&repo.join(".gitignore"))?.is_some_and(|m| m.is_file()) {
            ProbeOutcome::pass(".gitignore present")
        } else {
            ProbeOutcome::fail(".gitignore missing")
        },
    )
}

/// Agent Skills frontmatter description limit from canon §15.
pub(crate) const SKILL_DESCRIPTION_MAX_CHARS: usize = 1024;

/// Parse a rendered `SKILL.md` and return its YAML frontmatter description length in Unicode
/// characters. YAML parsing matters here: escaped and block scalars must be measured as the value
/// an Agent Skills consumer sees, not as source bytes.
pub(crate) fn skill_description_length(content: &str) -> Result<usize, String> {
    let frontmatter = extract_skill_frontmatter(content)?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(frontmatter)
        .map_err(|error| format!("invalid YAML frontmatter: {error}"))?;
    let description = yaml
        .get("description")
        .and_then(serde_yaml::Value::as_str)
        .ok_or_else(|| "frontmatter description is missing or not a string".to_string())?;
    Ok(description.chars().count())
}

fn extract_skill_frontmatter(content: &str) -> Result<&str, String> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let (first, mut offset) = next_line(content, 0);
    if first != Some("---") {
        return Err("missing opening YAML frontmatter fence".to_string());
    }
    let frontmatter_start = offset;
    loop {
        let line_start = offset;
        let (line, next_offset) = next_line(content, offset);
        let Some(line) = line else {
            return Err("missing closing YAML frontmatter fence".to_string());
        };
        if line == "---" {
            return Ok(&content[frontmatter_start..line_start]);
        }
        offset = next_offset;
    }
}

/// Return the next LF/CRLF-delimited line and the byte offset immediately after it. A final line
/// without a newline is still a line, allowing a closing frontmatter fence at EOF.
fn next_line(content: &str, offset: usize) -> (Option<&str>, usize) {
    if offset >= content.len() {
        return (None, offset);
    }
    let rest = &content[offset..];
    match rest.find('\n') {
        Some(index) => {
            let line = rest[..index].strip_suffix('\r').unwrap_or(&rest[..index]);
            (Some(line), offset + index + 1)
        }
        None => (Some(rest.strip_suffix('\r').unwrap_or(rest)), content.len()),
    }
}

/// Byte extent through the closing frontmatter fence. This lets repository probes decode only
/// bounded frontmatter bytes, avoiding a false UTF-8 error when a bounded read cuts through a
/// multibyte character later in a large skill body.
fn skill_frontmatter_extent(bytes: &[u8]) -> Option<usize> {
    let mut offset = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).map_or(0, |_| 3);
    let (first, next) = next_byte_line(bytes, offset)?;
    if first != b"---" {
        return None;
    }
    offset = next;
    loop {
        let (line, next) = next_byte_line(bytes, offset)?;
        if line == b"---" {
            return Some(next);
        }
        offset = next;
    }
}

fn next_byte_line(bytes: &[u8], offset: usize) -> Option<(&[u8], usize)> {
    if offset >= bytes.len() {
        return None;
    }
    let rest = &bytes[offset..];
    match rest.iter().position(|byte| *byte == b'\n') {
        Some(index) => {
            let line = rest[..index].strip_suffix(b"\r").unwrap_or(&rest[..index]);
            Some((line, offset + index + 1))
        }
        None => Some((rest.strip_suffix(b"\r").unwrap_or(rest), bytes.len())),
    }
}

/// Locate repository-native Agent Skills directories and enforce the §15 description limit over
/// every direct child `SKILL.md`. Repositories with no locatable skill files pass this scoped
/// check; the rest of §15 remains a review judgment rather than being inferred from absence.
fn probe_skill_description_lengths(repo: &Path) -> std::io::Result<ProbeOutcome> {
    const MAX_FRONTMATTER_BYTES: u64 = 1_048_576;
    const ROOTS: [&str; 5] = [
        "skills",
        ".agents/skills",
        ".claude/skills",
        ".pi/agent/skills",
        ".codex/skills",
    ];
    let canonical_repo = std::fs::canonicalize(repo)?;
    let mut skill_files = BTreeSet::new();
    for root in ROOTS {
        let directory = repo.join(root);
        let canonical_root = match std::fs::canonicalize(&directory) {
            Ok(path) if path.starts_with(&canonical_repo) && path.is_dir() => path,
            Ok(_) => {
                return Ok(ProbeOutcome::fail(format!(
                    "supported skill directory {root} resolves outside the target repository"
                )))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotADirectory => continue,
            Err(error) => return Err(error),
        };
        for entry in std::fs::read_dir(canonical_root)? {
            let entry = entry?;
            let candidate = entry.path().join("SKILL.md");
            let _metadata = match std::fs::symlink_metadata(&candidate) {
                Ok(metadata) if metadata.file_type().is_file() => metadata,
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Ok(ProbeOutcome::fail(format!(
                        "located skill {} is a symlink and cannot be safely inspected",
                        candidate
                            .strip_prefix(&canonical_repo)
                            .unwrap_or(&candidate)
                            .display()
                    )))
                }
                Ok(_) => continue,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) if error.kind() == std::io::ErrorKind::NotADirectory => continue,
                Err(error) => return Err(error),
            };
            let canonical = std::fs::canonicalize(&candidate)?;
            if !canonical.starts_with(&canonical_repo) {
                return Ok(ProbeOutcome::fail(format!(
                    "located skill {} resolves outside the target repository",
                    candidate
                        .strip_prefix(&canonical_repo)
                        .unwrap_or(&candidate)
                        .display()
                )));
            }
            skill_files.insert(canonical);
        }
    }

    for file in &skill_files {
        let rel = file.strip_prefix(&canonical_repo).unwrap_or(file);
        let mut bytes = Vec::new();
        std::fs::File::open(file)?
            .take(MAX_FRONTMATTER_BYTES + 1)
            .read_to_end(&mut bytes)?;
        let Some(frontmatter_end) = skill_frontmatter_extent(&bytes) else {
            if bytes.len() as u64 > MAX_FRONTMATTER_BYTES {
                return Ok(ProbeOutcome::fail(format!(
                    "located skill {} frontmatter exceeds the {MAX_FRONTMATTER_BYTES}-byte scan limit or has no closing fence within it",
                    rel.display()
                )));
            }
            return Ok(ProbeOutcome::fail(format!(
                "cannot measure located skill {}: missing YAML frontmatter fences",
                rel.display()
            )));
        };
        bytes.truncate(frontmatter_end);
        let content = match std::str::from_utf8(&bytes) {
            Ok(content) => content,
            Err(_) => {
                return Ok(ProbeOutcome::fail(format!(
                    "located skill {} is not UTF-8",
                    rel.display()
                )))
            }
        };
        let length = match skill_description_length(content) {
            Ok(length) => length,
            Err(error) => {
                return Ok(ProbeOutcome::fail(format!(
                    "cannot measure located skill {}: {error}",
                    rel.display()
                )))
            }
        };
        if length > SKILL_DESCRIPTION_MAX_CHARS {
            return Ok(ProbeOutcome::fail(format!(
                "located skill {} has a {length}-character frontmatter description (maximum {SKILL_DESCRIPTION_MAX_CHARS})",
                rel.display()
            )));
        }
    }

    Ok(if skill_files.is_empty() {
        ProbeOutcome::pass("no repository Agent Skills found in supported skill directories")
    } else {
        ProbeOutcome::pass(format!(
            "{} located Agent Skill description(s) are at most {SKILL_DESCRIPTION_MAX_CHARS} characters",
            skill_files.len()
        ))
    })
}

/// §22 core/cli split: a `crates/*-core` and a `crates/*-cli` directory both exist (SHOULD). A
/// missing `crates/` — or a `crates` that exists but is **not** a directory (a stray file) — is a
/// *conformance miss*, not an operational fault: it is repo shape, decidable without running the
/// tool. Only a genuine permission/transient I/O error (reading the dir, or a per-entry `metadata`
/// read) faults.
fn probe_core_cli_split(repo: &Path) -> std::io::Result<ProbeOutcome> {
    let crates = repo.join("crates");
    let entries = match std::fs::read_dir(&crates) {
        Ok(e) => e,
        // NotFound (`crates/` absent) and NotADirectory (`crates` is a file/other) are both
        // decidable repo-shape misses — never an exit-2 operational fault.
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(ProbeOutcome::fail(
                "no crates/ directory (missing or not a directory) — no core/cli split",
            ));
        }
        Err(e) => return Err(e),
    };
    let (mut has_core, mut has_cli) = (false, false);
    for entry in entries {
        let entry = entry?; // a per-entry read error faults rather than being silently dropped
                            // `entry.metadata()` (follows symlinks) surfaces a metadata I/O error as a fault,
                            // unlike `path().is_dir()`, which would swallow it as "not a directory".
        if !entry.metadata()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        has_core |= name.ends_with("-core");
        has_cli |= name.ends_with("-cli");
    }
    Ok(match (has_core, has_cli) {
        (true, true) => ProbeOutcome::pass("crates/*-core + *-cli split present"),
        _ => ProbeOutcome::fail("missing a crates/*-core and/or crates/*-cli directory"),
    })
}

/// §23's mechanically decidable subset. The operator names private markers; doctor scans the
/// distributed tree without guessing what a username looks like. The target's own public
/// coordinates are derived from its git remote and exempted.
fn probe_public_artifact_specifics(
    repo: &Path,
    context: &ProbeContext<'_>,
) -> std::io::Result<ProbeOutcome> {
    if context.user_specific_deny_list.is_empty() {
        return Ok(ProbeOutcome::pass(
            "no user-specific markers configured; set user_specific_deny_list or PROJECT_CANON_USER_SPECIFIC_DENY_LIST to enable the §23 scan",
        ));
    }

    let own = own_coordinates(repo)?;
    let markers: Vec<String> = context
        .user_specific_deny_list
        .iter()
        .map(|marker| marker.to_lowercase())
        .collect();
    let files = tracked_text_candidates(repo)?;
    for file in files {
        let rel = file.strip_prefix(repo).unwrap_or(&file);
        if std::fs::metadata(&file)?.len() > 1_048_576 {
            use std::io::Read;
            let mut prefix = [0u8; 8192];
            let mut handle = std::fs::File::open(&file)?;
            let read = handle.read(&mut prefix)?;
            if prefix[..read].contains(&0) {
                continue;
            }
            return Ok(ProbeOutcome::fail(format!(
                "text-like distributed file {} exceeds the 1 MiB §23 scan limit",
                rel.display()
            )));
        }
        let bytes = std::fs::read(&file)?;
        let Ok(text) = std::str::from_utf8(&bytes) else {
            if bytes.contains(&0) {
                continue;
            }
            return Ok(ProbeOutcome::fail(format!(
                "text-like distributed file {} is not UTF-8 and could not be scanned",
                rel.display()
            )));
        };
        for (line_index, line) in text.lines().enumerate() {
            let searchable = line.to_lowercase();
            for (marker_index, marker) in markers.iter().enumerate() {
                let leaked = searchable.match_indices(marker).any(|(start, _)| {
                    !own.is_allowed_occurrence(&searchable, marker, start, start + marker.len())
                });
                if leaked {
                    return Ok(ProbeOutcome::fail(format!(
                        "configured user-specific marker #{} found in {}:{}",
                        marker_index + 1,
                        rel.display(),
                        line_index + 1
                    )));
                }
            }
        }
    }
    Ok(ProbeOutcome::pass(format!(
        "no configured user-specific markers found ({} marker(s)); own public coordinates exempt",
        context.user_specific_deny_list.len()
    )))
}

/// §24's mechanically decidable subset. Deferral ownership is local: every recognized issue slug
/// must resolve in this target's issue tracker. A cross-repository issue may be supporting evidence,
/// but it cannot replace an open local mirror that doctor can verify offline.
fn probe_verified_deferrals(
    repo: &Path,
    _context: &ProbeContext<'_>,
) -> std::io::Result<ProbeOutcome> {
    let mut findings = BTreeSet::new();
    let mut issue_states = std::collections::BTreeMap::<String, IssueState>::new();
    let mut references_seen = 0usize;
    let mut skipped_files = 0usize;

    for file in tracked_text_candidates(repo)? {
        let rel = file.strip_prefix(repo).unwrap_or(&file);
        let metadata = std::fs::metadata(&file)?;
        if metadata.len() > 1_048_576 {
            use std::io::Read;
            let mut prefix = [0u8; 8192];
            let mut handle = std::fs::File::open(&file)?;
            let read = handle.read(&mut prefix)?;
            if prefix[..read].contains(&0) {
                skipped_files += 1;
                continue;
            }
            skipped_files += 1;
            continue;
        }
        let bytes = std::fs::read(&file)?;
        if bytes.contains(&0) {
            skipped_files += 1;
            continue;
        }
        let Ok(text) = std::str::from_utf8(&bytes) else {
            skipped_files += 1;
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        let mut seen_in_file = BTreeSet::new();
        for line_index in 0..lines.len() {
            let mut context = Vec::new();
            for (offset, line) in lines[line_index..lines.len().min(line_index + 3)]
                .iter()
                .enumerate()
            {
                if offset > 0 && line.trim().is_empty() {
                    break;
                }
                if let Some(fragment) = scannable_fragment(rel, line) {
                    context.push((line_index + offset + 1, fragment));
                } else if offset > 0 {
                    // Source code between two comments is a logical boundary, not continuation.
                    break;
                }
            }
            let suppression_start = line_index.saturating_sub(2);
            let suppressed = lines[suppression_start..=line_index]
                .iter()
                .rev()
                .take_while(|line| !line.trim().is_empty())
                .any(|line| line.contains("canon:s24-allow"));
            if suppressed
                || context
                    .iter()
                    .any(|(_, fragment)| fragment.contains("canon:s24-allow"))
            {
                continue;
            }
            let tokens = context_tokens(&context);
            if !looks_like_deferral(&tokens) {
                continue;
            }
            for reference in issue_references(&tokens) {
                if !seen_in_file.insert((reference.line, reference.slug.clone())) {
                    continue;
                }
                references_seen += 1;
                let state = match issue_states.get(&reference.slug) {
                    Some(state) => state.clone(),
                    None => {
                        let state = resolve_issue_state(repo, &reference.slug)?;
                        issue_states.insert(reference.slug.clone(), state.clone());
                        state
                    }
                };
                let location = format!("{}:{}", rel.display(), reference.line);
                match state {
                    IssueState::Open => {}
                    IssueState::Missing => {
                        findings.insert(format!(
                            "deferral at {location} names unresolved local issue {:?}",
                            reference.slug
                        ));
                    }
                    IssueState::NonOpen(status) => {
                        findings.insert(format!(
                            "deferral at {location} names local issue {:?} with non-open status {status:?}",
                            reference.slug
                        ));
                    }
                    IssueState::Malformed => {
                        findings.insert(format!(
                            "deferral at {location} names local issue {:?} with malformed or missing status frontmatter",
                            reference.slug
                        ));
                    }
                }
            }
        }
    }

    if findings.is_empty() {
        Ok(ProbeOutcome::pass(format!(
            "all {references_seen} detected deferral issue reference(s) resolve to open local issues; {skipped_files} binary/oversized/non-UTF-8 tracked file(s) skipped"
        )))
    } else {
        let total = findings.len();
        let sample = findings.into_iter().take(5).collect::<Vec<_>>().join("; ");
        Ok(ProbeOutcome::fail(format!(
            "{total} unverified deferral reference(s): {sample}"
        )))
    }
}

#[derive(Clone)]
enum IssueState {
    Open,
    NonOpen(String),
    Missing,
    Malformed,
}

struct IssueReference {
    slug: String,
    line: usize,
}

struct ContextToken {
    text: String,
    line: usize,
}

fn scannable_fragment<'a>(path: &Path, line: &'a str) -> Option<&'a str> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let full_text = matches!(
        extension,
        "md" | "mdx"
            | "rst"
            | "txt"
            | "toml"
            | "yaml"
            | "yml"
            | "json"
            | "jsonc"
            | "ini"
            | "cfg"
            | "conf"
    );
    if full_text {
        return Some(line);
    }

    let markers: &[&str] = match extension {
        "rs" | "js" | "jsx" | "ts" | "tsx" | "c" | "cc" | "cpp" | "h" | "hpp" | "java" | "go"
        | "swift" => &["//", "/*"],
        "py" | "rb" | "sh" | "bash" | "zsh" => &["#"],
        "html" | "htm" | "xml" => &["<!--"],
        "sql" | "lua" => &["-- "],
        _ => &["//", "#", "/*", "<!--", "-- "],
    };
    comment_start_outside_quotes(line, markers)
        .map(|(index, length)| &line[index + length..])
        .or_else(|| {
            line.trim_start()
                .starts_with('*')
                .then(|| line.trim_start_matches([' ', '*']))
        })
}

fn comment_start_outside_quotes(line: &str, markers: &[&str]) -> Option<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if quote.is_some() && byte == b'\\' {
            escaped = true;
            index += 1;
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            if quote == Some(byte) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(byte);
            }
            index += 1;
            continue;
        }
        if quote.is_none() {
            if let Some(marker) = markers
                .iter()
                .find(|marker| bytes[index..].starts_with(marker.as_bytes()))
            {
                return Some((index, marker.len()));
            }
        }
        index += 1;
    }
    None
}

fn context_tokens(lines: &[(usize, &str)]) -> Vec<ContextToken> {
    lines
        .iter()
        .flat_map(|(line, text)| {
            text.split(|character: char| !(character.is_ascii_alphanumeric() || character == '-'))
                .filter(|token| !token.is_empty())
                .map(|token| ContextToken {
                    text: token.to_ascii_lowercase(),
                    line: *line,
                })
        })
        .collect()
}

fn looks_like_deferral(tokens: &[ContextToken]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        matches!(
            token.text.as_str(),
            "defer" | "deferred" | "deferral" | "disabled" | "skipped" | "blocked" | "blocker"
        ) || (token.text == "owned" && tokens.get(index + 1).is_some_and(|next| next.text == "by"))
            || (token.text == "not" && tokens.get(index + 1).is_some_and(|next| next.text == "yet"))
            || token.text == "until"
            || (token.text == "tracks"
                && tokens[index + 1..tokens.len().min(index + 5)]
                    .iter()
                    .any(|next| matches!(next.text.as_str(), "closing" | "gap")))
    })
}

fn issue_references(tokens: &[ContextToken]) -> Vec<IssueReference> {
    let mut references = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.text == "issue" {
            if let Some(slug) = tokens
                .get(index + 1)
                .filter(|token| is_issue_slug(&token.text))
            {
                references.insert((slug.line, slug.text.clone()));
            }
            continue;
        }
        if !is_issue_slug(&token.text) {
            continue;
        }
        let following = &tokens[index + 1..tokens.len().min(index + 5)];
        let preceding = &tokens[index.saturating_sub(5)..index];
        let ownership = preceding.windows(2).any(|pair| {
            matches!(pair[0].text.as_str(), "owned" | "blocked") && pair[1].text == "by"
        });
        if ownership
            && following
                .first()
                .is_some_and(|candidate| candidate.text == "issue")
        {
            references.insert((token.line, token.text.clone()));
        }
    }
    references
        .into_iter()
        .map(|(line, slug)| IssueReference { slug, line })
        .collect()
}

fn is_issue_slug(token: &str) -> bool {
    token.len() <= 128
        && token.contains('-')
        && token.split('-').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn resolve_issue_state(repo: &Path, slug: &str) -> std::io::Result<IssueState> {
    // `is_issue_slug` admits no separators or dots, so this join cannot escape `issues/`.
    let issue = repo.join("issues").join(slug).join("item.md");
    let contents = match std::fs::read_to_string(issue) {
        Ok(contents) => contents,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(IssueState::Missing)
        }
        Err(error) => return Err(error),
    };
    Ok(match issue_status(&contents) {
        Some(status) if is_open_issue_status(status) => IssueState::Open,
        Some(status) => IssueState::NonOpen(status.to_string()),
        None => IssueState::Malformed,
    })
}

fn issue_status(contents: &str) -> Option<&str> {
    let mut lines = contents.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    let mut status = None;
    let mut closed = false;
    for line in lines {
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        if let Some(value) = line.strip_prefix("status:") {
            let value = value.trim().trim_matches(['"', '\'']);
            if status.is_some() || value.is_empty() {
                return None;
            }
            status = Some(value);
        }
    }
    closed.then_some(status).flatten()
}

fn is_open_issue_status(status: &str) -> bool {
    // Mirrors the active statuses in this target's issuectl-managed `issues/.schema.yaml`.
    matches!(
        status,
        "open" | "in-progress" | "testing" | "untriaged" | "deferred" | "needs-info"
    )
}

#[derive(Default)]
struct OwnCoordinates {
    owner: Option<String>,
    repo: Option<String>,
}

impl OwnCoordinates {
    fn is_allowed_occurrence(&self, line: &str, marker: &str, start: usize, end: usize) -> bool {
        let (Some(owner), Some(repo)) = (&self.owner, &self.repo) else {
            return false;
        };
        let owner = owner.to_lowercase();
        let repo = repo.to_lowercase();

        // The repository/package name is intrinsically this project's public identity, including
        // package suffixes such as `<repo>-cli`.
        if marker == repo {
            return true;
        }
        // An owner is allowed only as the owner segment of a coordinate. A separately configured
        // private repository marker on the same line remains visible and still fails.
        if marker == owner && line.as_bytes().get(end) == Some(&b'/') {
            return true;
        }

        // For markers that overlap an own coordinate, exempt only this specific occurrence. Never
        // delete text before scanning: deletion can concatenate or erase unrelated private names.
        for coordinate in [
            format!("{owner}/{repo}"),
            format!("{owner}/homebrew-{repo}"),
        ] {
            for (coordinate_start, _) in line.match_indices(&coordinate) {
                let coordinate_end = coordinate_start + coordinate.len();
                if start >= coordinate_start && end <= coordinate_end {
                    return true;
                }
            }
        }
        false
    }
}

fn own_coordinates(repo: &Path) -> std::io::Result<OwnCoordinates> {
    let dot_git = repo.join(".git");
    let config = if dot_git.is_dir() {
        Some(dot_git.join("config"))
    } else if dot_git.is_file() {
        let pointer = std::fs::read_to_string(&dot_git)?;
        pointer
            .trim()
            .strip_prefix("gitdir:")
            .map(str::trim)
            .map(|path| {
                let gitdir = PathBuf::from(path);
                let gitdir = if gitdir.is_absolute() {
                    gitdir
                } else {
                    repo.join(gitdir)
                };
                let local = gitdir.join("config");
                if local.is_file() {
                    local
                } else {
                    let common = std::fs::read_to_string(gitdir.join("commondir"))
                        .unwrap_or_else(|_| ".".to_string());
                    gitdir.join(common.trim()).join("config")
                }
            })
    } else {
        None
    };

    if let Some(config) = config {
        if let Ok(contents) = std::fs::read_to_string(config) {
            let mut in_origin = false;
            for line in contents.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') {
                    in_origin = trimmed == "[remote \"origin\"]";
                    continue;
                }
                if !in_origin {
                    continue;
                }
                let Some((key, value)) = trimmed.split_once('=') else {
                    continue;
                };
                if key.trim() == "url" {
                    if let Some((owner, name)) = parse_github_coordinate(value.trim()) {
                        return Ok(OwnCoordinates {
                            owner: Some(owner),
                            repo: Some(name),
                        });
                    }
                }
            }
        }
    }

    Ok(coordinates_from_manifest(repo).unwrap_or_default())
}

fn coordinates_from_manifest(repo: &Path) -> Option<OwnCoordinates> {
    let contents = std::fs::read_to_string(repo.join("Cargo.toml")).ok()?;
    let manifest: toml::Value = contents.parse().ok()?;
    let repository = manifest
        .get("package")
        .and_then(|package| package.get("repository"))
        .or_else(|| {
            manifest
                .get("workspace")
                .and_then(|workspace| workspace.get("package"))
                .and_then(|package| package.get("repository"))
        })?
        .as_str()?;
    let (owner, repo) = parse_github_coordinate(repository)?;
    Some(OwnCoordinates {
        owner: Some(owner),
        repo: Some(repo),
    })
}

fn parse_github_coordinate(url: &str) -> Option<(String, String)> {
    let path = url
        .strip_prefix("git@github.com:")
        .or_else(|| url.strip_prefix("https://github.com/"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
    let mut parts = path
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .split('/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    (!owner.is_empty() && !repo.is_empty()).then_some((owner, repo))
}

fn tracked_text_candidates(repo: &Path) -> std::io::Result<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .args(["-C", repo.to_string_lossy().as_ref(), "ls-files", "-z"])
        .output();
    if let Ok(output) = output {
        if output.status.success() {
            let mut files = output
                .stdout
                .split(|byte| *byte == 0)
                .filter(|path| !path.is_empty())
                .filter_map(|path| std::str::from_utf8(path).ok())
                .map(PathBuf::from)
                .filter(|path| {
                    !path.is_absolute()
                        && path
                            .components()
                            .all(|component| matches!(component, std::path::Component::Normal(_)))
                })
                .map(|path| repo.join(path))
                .filter(|path| {
                    std::fs::symlink_metadata(path)
                        .is_ok_and(|metadata| metadata.file_type().is_file())
                })
                .collect::<Vec<_>>();
            files.sort();
            return Ok(files);
        }
    }

    // Synthetic fixtures and source archives may not have a functioning git command. Fall back
    // to a bounded tree walk with component-level exclusions.
    let mut files = Vec::new();
    collect_text_candidates(repo, repo, &mut files)?;
    Ok(files)
}

fn collect_text_candidates(
    root: &Path,
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    let mut entries = std::fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(&path);
        // The source-archive fallback excludes metadata/build/scratch components at any depth.
        let excluded_component = rel.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some(".git" | "target" | "node_modules" | "history")
            )
        });
        if excluded_component {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect_text_candidates(root, &path, files)?;
        } else if kind.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use project_canon_core::Model;

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
                std::env::temp_dir().join(format!("pc-probes-{tag}-{}-{n}", std::process::id()));
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
        fn write(&self, rel: &str, content: &str) {
            let path = self.path.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, content).unwrap();
        }
        #[cfg(unix)]
        fn symlink(&self, target: &str, link: &str) {
            std::os::unix::fs::symlink(target, self.path.join(link)).unwrap();
        }
    }

    impl Drop for TmpRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// Convenience: run a probe and unwrap the (test-only, never-faulting) I/O result to `passed`.
    fn passed(outcome: std::io::Result<ProbeOutcome>) -> bool {
        outcome.expect("no I/O fault on a tmp repo").passed
    }

    #[test]
    fn doc_pattern_probe_distinguishes_missing_files() {
        let repo = TmpRepo::new("doc");
        assert!(!passed(probe_doc_pattern(&repo.path)));
        repo.touch("AGENTS.md");
        assert!(!passed(probe_doc_pattern(&repo.path))); // CLAUDE.md still missing
        repo.touch("CLAUDE.md");
        assert!(passed(probe_doc_pattern(&repo.path)));
    }

    #[cfg(unix)]
    #[test]
    fn doc_pattern_probe_rejects_a_dangling_claude_symlink() {
        let repo = TmpRepo::new("doc-symlink");
        repo.touch("AGENTS.md");
        // A valid CLAUDE.md -> AGENTS.md symlink passes (followed to a real file)…
        repo.symlink("AGENTS.md", "CLAUDE.md");
        assert!(passed(probe_doc_pattern(&repo.path)));
        // …but a dangling symlink is a miss, not a pass.
        std::fs::remove_file(repo.path.join("CLAUDE.md")).unwrap();
        repo.symlink("nowhere.md", "CLAUDE.md");
        assert!(!passed(probe_doc_pattern(&repo.path)));
    }

    #[cfg(unix)]
    #[test]
    fn doc_pattern_probe_rejects_a_directory_named_claude() {
        let repo = TmpRepo::new("doc-dir");
        repo.touch("AGENTS.md");
        repo.mkdir("CLAUDE.md"); // a directory must not satisfy the doc pattern
        assert!(!passed(probe_doc_pattern(&repo.path)));
    }

    #[test]
    fn structural_probes_detect_presence() {
        let repo = TmpRepo::new("struct");
        assert!(!passed(probe_issue_tracking(&repo.path)));
        assert!(!passed(probe_git_hygiene(&repo.path)));
        assert!(!passed(probe_readme(&repo.path)));
        assert!(!passed(probe_gitignore(&repo.path)));
        repo.mkdir("issues");
        repo.mkdir(".git");
        repo.touch("README.md");
        repo.touch(".gitignore");
        assert!(passed(probe_issue_tracking(&repo.path)));
        assert!(passed(probe_git_hygiene(&repo.path)));
        assert!(passed(probe_readme(&repo.path)));
        assert!(passed(probe_gitignore(&repo.path)));
    }

    fn rendered_skill(description: &str) -> String {
        format!("---\nname: fixture-skill\ndescription: {description}\n---\n\n# Fixture\n")
    }

    #[test]
    fn skill_description_length_accepts_compliant_and_exact_limit_values() {
        let compliant = rendered_skill(&format!("\"{}\"", "a".repeat(42)));
        assert_eq!(skill_description_length(&compliant).unwrap(), 42);

        let exact = rendered_skill(&format!("\"{}\"", "é".repeat(SKILL_DESCRIPTION_MAX_CHARS)));
        assert_eq!(
            skill_description_length(&exact).unwrap(),
            SKILL_DESCRIPTION_MAX_CHARS,
            "the limit counts Unicode characters rather than UTF-8 bytes"
        );
    }

    #[test]
    fn skill_description_length_decodes_yaml_scalars_and_frontmatter_line_endings() {
        let escaped = rendered_skill("\"four\\u0020words\"");
        assert_eq!(skill_description_length(&escaped).unwrap(), 10);

        let folded =
            "---\r\nname: fixture-skill\r\ndescription: >-\r\n  first line\r\n  second line\r\n---";
        assert_eq!(
            skill_description_length(folded).unwrap(),
            "first line second line".chars().count()
        );

        let literal = "---\nname: fixture-skill\ndescription: |-\n  first\n  second\n---\n";
        assert_eq!(
            skill_description_length(literal).unwrap(),
            "first\nsecond".chars().count()
        );
    }

    #[test]
    fn skill_description_probe_rejects_over_limit_generic_and_codex_skills() {
        for root in [".agents/skills", ".codex/skills"] {
            let repo = TmpRepo::new("skill-description-over");
            let content = rendered_skill(&format!(
                "\"{}\"",
                "x".repeat(SKILL_DESCRIPTION_MAX_CHARS + 1)
            ));
            repo.write(&format!("{root}/fixture-skill/SKILL.md"), &content);

            let outcome = probe_skill_description_lengths(&repo.path).unwrap();
            assert!(!outcome.passed);
            assert!(
                outcome.message.contains("1025-character"),
                "{}",
                outcome.message
            );
            assert!(outcome
                .message
                .contains(&format!("{root}/fixture-skill/SKILL.md")));
        }
    }

    #[test]
    fn skill_description_probe_accepts_located_compliant_skills_and_no_skills() {
        let empty = TmpRepo::new("skill-description-empty");
        assert!(passed(probe_skill_description_lengths(&empty.path)));

        let repo = TmpRepo::new("skill-description-ok");
        repo.write(
            "skills/fixture-skill/SKILL.md",
            &rendered_skill(&format!("\"{}\"", "x".repeat(SKILL_DESCRIPTION_MAX_CHARS))),
        );
        assert!(passed(probe_skill_description_lengths(&repo.path)));
    }

    #[test]
    fn skill_description_probe_bounds_frontmatter_not_the_skill_body() {
        let repo = TmpRepo::new("skill-description-large-body");
        let mut content = rendered_skill("\"short description\"");
        content.push_str(&"x".repeat(1_048_576));
        repo.write("skills/fixture-skill/SKILL.md", &content);
        assert!(passed(probe_skill_description_lengths(&repo.path)));
    }

    #[cfg(unix)]
    #[test]
    fn skill_description_probe_rejects_a_skill_root_outside_the_repo() {
        let repo = TmpRepo::new("skill-description-scope");
        let external = TmpRepo::new("skill-description-external");
        external.write(
            "fixture-skill/SKILL.md",
            &rendered_skill("\"short description\""),
        );
        repo.symlink(external.path.to_str().unwrap(), "skills");
        let outcome = probe_skill_description_lengths(&repo.path).unwrap();
        assert!(!outcome.passed);
        assert!(outcome.message.contains("outside the target repository"));
    }

    #[test]
    fn core_cli_split_probe_needs_both_crates() {
        let repo = TmpRepo::new("split");
        assert!(!passed(probe_core_cli_split(&repo.path))); // no crates/
        repo.mkdir("crates/foo-core");
        assert!(!passed(probe_core_cli_split(&repo.path))); // core only
        repo.mkdir("crates/foo-cli");
        assert!(passed(probe_core_cli_split(&repo.path)));
    }

    #[test]
    fn core_cli_split_treats_a_crates_file_as_a_miss_not_a_fault() {
        // `crates` existing as a regular file is decidable repo shape → a conformance miss
        // (Ok(false)), never an operational I/O fault (Err → exit 2).
        let repo = TmpRepo::new("crates-file");
        repo.touch("crates");
        let outcome = probe_core_cli_split(&repo.path).expect("a stray crates file is not a fault");
        assert!(!outcome.passed);
        assert!(
            outcome.message.contains("not a directory"),
            "{}",
            outcome.message
        );
    }

    #[test]
    fn every_mechanical_probe_id_exists_in_the_model() {
        // Guards against a core-side id rename silently turning an enforced MUST into a
        // deferred/verify skip (fail-open). If this fires, update MECHANICAL_PROBE_IDS + the
        // `mechanical_probe` match to the new id.
        let model = Model::standard();
        for id in MECHANICAL_PROBE_IDS {
            assert!(
                model.dimension(id).is_some(),
                "probe id {id:?} no longer exists in the model"
            );
            assert!(
                mechanical_probe(id).is_some(),
                "probe id {id:?} missing from the mechanical_probe registry"
            );
        }
        for id in RUNTIME_PROBE_IDS {
            assert!(
                model.dimension(id).is_some(),
                "runtime probe id {id:?} no longer exists in the model"
            );
        }
    }

    #[test]
    fn public_artifact_probe_flags_a_configured_private_marker() {
        let repo = TmpRepo::new("private-marker");
        repo.touch("src/defaults.rs");
        std::fs::write(
            repo.path.join("src/defaults.rs"),
            "const DEFAULT_REPO: &str = \"private-widget\";",
        )
        .unwrap();
        let deny = BTreeSet::from(["private-widget".to_string()]);
        let context = ProbeContext {
            user_specific_deny_list: &deny,
        };
        let outcome = probe_public_artifact_specifics(&repo.path, &context).unwrap();
        assert!(!outcome.passed);
        assert!(outcome.message.contains("src/defaults.rs:1"));
    }

    #[test]
    fn public_artifact_probe_exempts_the_projects_own_public_coordinates() {
        let repo = TmpRepo::new("own-coordinates");
        repo.mkdir(".git");
        std::fs::write(
            repo.path.join(".git/config"),
            "[remote \"origin\"]\n    url = git@github.com:example-owner/example-tool.git\n",
        )
        .unwrap();
        std::fs::write(
            repo.path.join("README.md"),
            "[![CI](https://github.com/example-owner/example-tool/actions/badge.svg)]\n\
             brew install example-owner/example-tool/example-tool\n\
             https://github.com/example-owner/homebrew-example-tool\n\
             https://github.com/example-owner/public-dependency\n",
        )
        .unwrap();
        let deny = BTreeSet::from(["example-owner".to_string(), "example-tool".to_string()]);
        let context = ProbeContext {
            user_specific_deny_list: &deny,
        };
        let outcome = probe_public_artifact_specifics(&repo.path, &context).unwrap();
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn public_artifact_probe_derives_own_coordinates_from_a_package_manifest_without_git() {
        let repo = TmpRepo::new("manifest-coordinates");
        std::fs::write(
            repo.path.join("Cargo.toml"),
            "[package]\nname = \"example-tool\"\nversion = \"0.1.0\"\nrepository = \"https://github.com/example-owner/example-tool\"\n",
        )
        .unwrap();
        std::fs::write(
            repo.path.join("README.md"),
            "https://github.com/example-owner/example-tool\n",
        )
        .unwrap();
        let deny = BTreeSet::from(["example-owner".to_string(), "example-tool".to_string()]);
        let context = ProbeContext {
            user_specific_deny_list: &deny,
        };
        let outcome = probe_public_artifact_specifics(&repo.path, &context).unwrap();
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn public_artifact_probe_still_flags_an_other_private_repo_under_the_owner() {
        let repo = TmpRepo::new("other-private-coordinate");
        repo.mkdir(".git");
        std::fs::write(
            repo.path.join(".git/config"),
            "[remote \"origin\"]\n    url = https://github.com/example-owner/example-tool.git\n",
        )
        .unwrap();
        std::fs::write(
            repo.path.join("README.md"),
            "https://github.com/example-owner/private-widget\n",
        )
        .unwrap();
        let deny = BTreeSet::from(["example-owner".to_string(), "private-widget".to_string()]);
        let context = ProbeContext {
            user_specific_deny_list: &deny,
        };
        let outcome = probe_public_artifact_specifics(&repo.path, &context).unwrap();
        assert!(!outcome.passed);
        assert!(outcome.message.contains("README.md:1"));
        assert!(!outcome.message.contains("private-widget"));
    }

    fn verify_deferrals(repo: &Path) -> ProbeOutcome {
        let deny = BTreeSet::new();
        let context = ProbeContext {
            user_specific_deny_list: &deny,
        };
        probe_verified_deferrals(repo, &context).unwrap()
    }

    fn write_issue(repo: &TmpRepo, slug: &str, status: &str) {
        let path = repo.path.join("issues").join(slug).join("item.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("---\nstatus: {status}\n---\n\n# Fixture\n")).unwrap();
    }

    #[test]
    fn deferral_probe_flags_an_unresolvable_reference() {
        let repo = TmpRepo::new("unresolved-deferral");
        let slug = ["missing", "widget"].join("-");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature disabled until issue {slug} is resolved.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);
        assert!(outcome.message.contains("unresolved local issue"));
    }

    #[test]
    fn deferral_probe_accepts_a_valid_open_reference() {
        let repo = TmpRepo::new("open-deferral");
        let slug = ["enable", "widget"].join("-");
        write_issue(&repo, &slug, "open");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature disabled until issue {slug} is resolved.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn deferral_probe_rejects_a_closed_reference() {
        let repo = TmpRepo::new("closed-deferral");
        let slug = ["enable", "widget"].join("-");
        write_issue(&repo, &slug, "done");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature disabled until issue {slug} is resolved.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);
        assert!(outcome.message.contains("non-open status \"done\""));
    }

    #[test]
    fn deferral_probe_fails_closed_for_cross_repository_references() {
        let repo = TmpRepo::new("cross-repo-deferral");
        let tracker = ["example", "tracker"].join("-");
        let slug = ["enable", "widget"].join("-");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature disabled until {tracker} issue {slug} is resolved.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);
        assert!(outcome.message.contains("unresolved local issue"));
    }

    #[test]
    fn cross_repository_support_passes_when_the_slug_has_an_open_local_mirror() {
        let repo = TmpRepo::new("cross-repo-mirror");
        let tracker = ["example", "tracker"].join("-");
        let slug = ["enable", "widget"].join("-");
        write_issue(&repo, &slug, "open");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature disabled until {tracker} issue {slug} is resolved.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn deferral_probe_catches_an_owner_named_before_the_issue_noun() {
        let repo = TmpRepo::new("reverse-owner");
        let slug = ["missing", "owner"].join("-");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature is owned by the separate {slug} issue until it lands.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);
        assert!(outcome.message.contains(&slug));
    }

    #[test]
    fn deferral_probe_does_not_treat_a_tool_name_as_an_issue_without_the_noun() {
        let repo = TmpRepo::new("tool-owner");
        std::fs::write(
            repo.path.join("design.md"),
            "The generated file is owned by cargo-dist as a settled design boundary.\n",
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn source_scan_checks_comments_but_not_equivalent_code_strings() {
        let repo = TmpRepo::new("source-comments");
        let slug = ["missing", "owner"].join("-");
        std::fs::write(
            repo.path.join("example.rs"),
            format!("const TEXT: &str = \"Feature disabled until issue {slug} lands.\";\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);

        std::fs::write(
            repo.path.join("example.rs"),
            format!("// Feature disabled until issue {slug} lands.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);

        std::fs::write(
            repo.path.join("example.rs"),
            format!(
                "const URL: &str = \"https://example.invalid/#issue\"; // Feature disabled until issue {slug} lands.\n"
            ),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(
            !outcome.passed,
            "a real comment after a URL string must be scanned"
        );
    }

    #[test]
    fn blocked_by_reverse_owner_is_detected() {
        let repo = TmpRepo::new("blocked-owner");
        let slug = ["missing", "owner"].join("-");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature is blocked by the separate {slug} issue.\n"),
        )
        .unwrap();
        assert!(!verify_deferrals(&repo.path).passed);
    }

    #[test]
    fn an_explicit_historical_suppression_skips_the_logical_block() {
        let repo = TmpRepo::new("historical-allow");
        let slug = ["old", "owner"].join("-");
        std::fs::write(
            repo.path.join("CHANGELOG.md"),
            format!(
                "<!-- canon:s24-allow: historical quotation -->\nFeature was disabled until issue {slug} landed.\n"
            ),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn issue_status_requires_closed_frontmatter_and_accepts_a_quoted_value() {
        assert_eq!(
            issue_status("---\nstatus: \"open\"\n---\n# body"),
            Some("open")
        );
        assert_eq!(issue_status("---\nstatus: open\n# body status: done"), None);
    }

    #[test]
    fn tracked_file_enumeration_ignores_an_untracked_deferral() {
        let repo = TmpRepo::new("tracked-only");
        assert!(std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo.path)
            .status()
            .unwrap()
            .success());
        let slug = ["missing", "owner"].join("-");
        std::fs::write(
            repo.path.join("untracked.md"),
            format!("Feature disabled until issue {slug} lands.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);

        assert!(std::process::Command::new("git")
            .args(["add", "untracked.md"])
            .current_dir(&repo.path)
            .status()
            .unwrap()
            .success());
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);
    }

    #[test]
    fn the_deferral_probe_passes_on_this_repository() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let outcome = verify_deferrals(&workspace);
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[cfg(unix)]
    fn executable_script(tag: &str, body: &str) -> TmpRepo {
        use std::os::unix::fs::PermissionsExt;
        let repo = TmpRepo::new(tag);
        repo.touch("probe-target");
        std::fs::write(
            repo.path.join("probe-target"),
            format!("#!/bin/sh\n{body}\n"),
        )
        .unwrap();
        std::fs::set_permissions(
            repo.path.join("probe-target"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        repo
    }

    #[cfg(unix)]
    #[test]
    fn every_runtime_section_rejects_an_absent_or_unstructured_surface() {
        let target = executable_script("runtime-gaps", "printf '{}\\n'");
        let runner = RuntimeRunner {
            binary: target.path.join("probe-target"),
            timeout: Duration::from_secs(2),
            current_dir: target.path.clone(),
        };
        for id in RUNTIME_PROBE_IDS {
            let outcome = probe_runtime_section(id, &runner);
            assert_eq!(
                outcome.status,
                RuntimeProbeStatus::Gap,
                "{id}: {}",
                outcome.message
            );
        }
    }

    #[test]
    fn a_missing_runtime_binary_is_reported_not_panicked() {
        let repo = TmpRepo::new("runtime-missing");
        let outcomes = runtime_probes_with_timeout(
            &repo.path.join("missing-binary"),
            &repo.path,
            Duration::from_millis(50),
        );
        assert!(outcomes
            .iter()
            .all(|outcome| outcome.status == RuntimeProbeStatus::CouldNotProbe));
        assert!(outcomes[0].message.contains("could not start"));
    }

    #[cfg(unix)]
    #[test]
    fn a_non_executable_runtime_binary_is_reported() {
        let repo = TmpRepo::new("runtime-nonexec");
        repo.touch("binary");
        let outcomes = runtime_probes_with_timeout(
            &repo.path.join("binary"),
            &repo.path,
            Duration::from_millis(50),
        );
        assert_eq!(outcomes[0].status, RuntimeProbeStatus::CouldNotProbe);
        assert!(outcomes[0].message.contains("could not start"));
    }

    #[cfg(unix)]
    #[test]
    fn a_hanging_runtime_binary_is_killed_at_the_timeout() {
        let target = executable_script("runtime-timeout", "while :; do :; done");
        let started = Instant::now();
        let outcomes = runtime_probes_with_timeout(
            &target.path.join("probe-target"),
            &target.path,
            Duration::from_millis(50),
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(outcomes[0].status, RuntimeProbeStatus::CouldNotProbe);
        assert!(outcomes[0].message.contains("timed out"));
    }

    #[cfg(unix)]
    #[test]
    fn descendants_holding_capture_pipes_cannot_bypass_the_timeout() {
        let state = TmpRepo::new("runtime-descendant-state");
        let pid_file = state.path.join("descendant.pid");
        let target = executable_script(
            "runtime-descendant",
            &format!("sleep 10 &\necho $! > {:?}\nexit 0", pid_file),
        );
        let runner = RuntimeRunner {
            binary: target.path.join("probe-target"),
            timeout: Duration::from_secs(2),
            current_dir: target.path.clone(),
        };
        let started = Instant::now();
        assert!(matches!(runner.run(&[]), Err(RunFailure::Timeout)));
        assert!(started.elapsed() < Duration::from_secs(3));
        let pid: i32 = std::fs::read_to_string(pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let state = std::process::Command::new("ps")
            .args(["-o", "state=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        let state = String::from_utf8_lossy(&state.stdout);
        assert!(
            state.trim().is_empty() || state.trim_start().starts_with('Z'),
            "timed-out descendant is still running with state {state:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn successful_probe_cleans_up_redirected_descendants() {
        let state = TmpRepo::new("runtime-redirected-state");
        let pid_file = state.path.join("descendant.pid");
        let target = executable_script(
            "runtime-redirected",
            &format!(
                "sleep 10 >/dev/null 2>&1 &\necho $! > {:?}\nprintf '{{}}\\n'",
                pid_file
            ),
        );
        let runner = RuntimeRunner {
            binary: target.path.join("probe-target"),
            timeout: Duration::from_secs(2),
            current_dir: target.path.clone(),
        };
        assert!(runner.run(&[]).is_ok());
        let pid: i32 = std::fs::read_to_string(pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let state = std::process::Command::new("ps")
            .args(["-o", "state=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        let state = String::from_utf8_lossy(&state.stdout);
        assert!(
            state.trim().is_empty() || state.trim_start().starts_with('Z'),
            "probe descendant is still running with state {state:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_section_local_crash_does_not_suppress_later_probes() {
        let body = r#"
if [ "$1" = "--help" ]; then
  printf '%s\n' '{"schema_version":1,"exit_codes":[{"code":"0"},{"code":"1"},{"code":"2"}]}'
  exit 0
fi
if [ "$1" = "__project_canon_probe_unknown_subcommand__" ]; then
  printf '%s\n' '{"schema_version":1,"error":{"code":"usage_error","message":"unknown"}}' >&2
  exit 1
fi
if [ "$1" = "config" ]; then
  kill -9 $$
fi
printf '{}\n'
"#;
        let target = executable_script("runtime-local-crash", body);
        let outcomes = runtime_probes_with_timeout(
            &target.path.join("probe-target"),
            &target.path,
            Duration::from_secs(2),
        );
        assert_eq!(outcomes[0].status, RuntimeProbeStatus::Pass);
        assert_eq!(outcomes[1].status, RuntimeProbeStatus::CouldNotProbe);
        assert_eq!(outcomes[2].status, RuntimeProbeStatus::Gap);
        assert!(!outcomes[2].message.contains("not attempted"));
    }

    fn complete_skill_install_metadata() -> Value {
        serde_json::json!({
            "supported_agents": ["claude", "pi", "codex", "future-agent"],
            "install": {
                "selection_flag": "--agent",
                "default": "all",
                "accepted_values": ["claude", "pi", "codex", "all", "future-agent"],
                "target_flag": "--target",
                "dry_run_flag": "--dry-run",
                "force_flag": "--force",
                "interactive": false,
                "no_clobber_default": true,
                "overwrite_requires_force": true,
                "layouts": [
                    {"agent": "claude", "path": ".claude/skills/<name>/...", "form": "agent-skill-tree"},
                    {"agent": "pi", "path": ".pi/agent/skills/<name>/...", "form": "agent-skill-tree"},
                    {"agent": "codex", "path": ".codex/skills/<name>/...", "form": "agent-skill-tree"},
                    {"agent": "future-agent", "path": ".future/skills/<name>", "form": "agent-skill-tree"}
                ]
            }
        })
    }

    #[test]
    fn skill_install_metadata_validates_each_required_capability_and_native_form() {
        assert!(validate_skill_install_metadata(&complete_skill_install_metadata()).is_ok());

        let mut cases = Vec::new();
        let mut missing_install = complete_skill_install_metadata();
        missing_install.as_object_mut().unwrap().remove("install");
        cases.push((missing_install, "install capability object"));
        let mut wrong_default = complete_skill_install_metadata();
        wrong_default["install"]["default"] = Value::String("claude".to_string());
        cases.push((wrong_default, "install.default"));
        let mut interactive = complete_skill_install_metadata();
        interactive["install"]["interactive"] = Value::Bool(true);
        cases.push((interactive, "install.interactive"));
        let mut wrong_codex_form = complete_skill_install_metadata();
        wrong_codex_form["install"]["layouts"][2]["form"] =
            Value::String("self-contained-prompt".to_string());
        cases.push((wrong_codex_form, "agent-skill-tree"));
        let mut missing_target = complete_skill_install_metadata();
        missing_target["install"]
            .as_object_mut()
            .unwrap()
            .remove("target_flag");
        cases.push((missing_target, "install.target_flag"));
        let mut unsafe_default = complete_skill_install_metadata();
        unsafe_default["install"]["no_clobber_default"] = Value::Bool(false);
        cases.push((unsafe_default, "install.no_clobber_default"));
        let mut malformed_agent = complete_skill_install_metadata();
        malformed_agent["supported_agents"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"bad": true}));
        cases.push((malformed_agent, "supported_agents[4]"));
        let mut missing_future_layout = complete_skill_install_metadata();
        missing_future_layout["install"]["layouts"]
            .as_array_mut()
            .unwrap()
            .pop();
        cases.push((missing_future_layout, "exactly one row"));
        let mut duplicate_codex = complete_skill_install_metadata();
        let duplicate = duplicate_codex["install"]["layouts"][2].clone();
        duplicate_codex["install"]["layouts"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        cases.push((duplicate_codex, "duplicate agent"));

        for (value, expected) in cases {
            let error = validate_skill_install_metadata(&value).unwrap_err();
            assert!(
                error.contains(expected),
                "{error:?} should name {expected:?}"
            );
        }
    }

    #[test]
    fn prompt_only_codex_skill_metadata_is_rejected() {
        let mut prompt_only = complete_skill_install_metadata();
        prompt_only["install"]["layouts"][2] = serde_json::json!({
            "agent": "codex",
            "path": ".codex/prompts/<name>.md",
            "form": "self-contained-prompt"
        });
        let error = validate_skill_install_metadata(&prompt_only).unwrap_err();
        assert!(error.contains(".codex/skills/<name>/..."), "{error}");
        assert!(error.contains("agent-skill-tree"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn runtime_skill_probe_distinguishes_claude_only_from_all_three() {
        let claude_only = executable_script(
            "runtime-skill-claude-only",
            r#"
if [ "$1 $2" = "skill list" ]; then
  printf '%s\n' '{"schema_version":1,"supported_agents":["claude"],"install":{"selection_flag":"--agent","default":"all","accepted_values":["claude","all"],"target_flag":"--target","dry_run_flag":"--dry-run","force_flag":"--force","interactive":false,"no_clobber_default":true,"overwrite_requires_force":true,"layouts":[{"agent":"claude","path":".claude/skills/<name>/...","form":"agent-skill-tree"}]},"skills":[{"name":"fixture-skill","cli_version":"1.0.0","skill_schema_version":1}]}'
  exit 0
fi
exit 1
"#,
        );
        let runner = RuntimeRunner {
            binary: claude_only.path.join("probe-target"),
            timeout: Duration::from_secs(2),
            current_dir: claude_only.path.clone(),
        };
        let outcome = probe_runtime_section("canon.s15", &runner);
        assert_eq!(outcome.status, RuntimeProbeStatus::Gap);
        assert!(outcome.message.contains("claude, pi, and codex"));

        let complete = executable_script(
            "runtime-skill-all",
            r#"
if [ "$1 $2" = "skill list" ]; then
  printf '%s\n' '{"schema_version":1,"supported_agents":["claude","pi","codex","future-agent"],"install":{"selection_flag":"--agent","default":"all","accepted_values":["claude","pi","codex","all","future-agent"],"target_flag":"--target","dry_run_flag":"--dry-run","force_flag":"--force","interactive":false,"no_clobber_default":true,"overwrite_requires_force":true,"layouts":[{"agent":"claude","path":".claude/skills/<name>/...","form":"agent-skill-tree"},{"agent":"pi","path":".pi/agent/skills/<name>/...","form":"agent-skill-tree"},{"agent":"codex","path":".codex/skills/<name>/...","form":"agent-skill-tree"},{"agent":"future-agent","path":".future/skills/<name>/...","form":"agent-skill-tree"}]},"skills":[{"name":"fixture-skill","cli_version":"1.0.0","skill_schema_version":1}]}'
  exit 0
fi
exit 1
"#,
        );
        let runner = RuntimeRunner {
            binary: complete.path.join("probe-target"),
            timeout: Duration::from_secs(2),
            current_dir: complete.path.clone(),
        };
        let outcome = probe_runtime_section("canon.s15", &runner);
        assert_eq!(
            outcome.status,
            RuntimeProbeStatus::Pass,
            "{}",
            outcome.message
        );
    }

    #[cfg(unix)]
    #[test]
    fn runtime_invocation_passes_literal_arguments_without_a_shell() {
        let target = executable_script("runtime-argv", "printf '%s\\n' \"$1\"");
        let runner = RuntimeRunner {
            binary: target.path.join("probe-target"),
            timeout: Duration::from_secs(2),
            current_dir: target.path.clone(),
        };
        let sentinel = target.path.join("shell-injection");
        let argument = format!("; touch {}", sentinel.display());
        let capture = runner.run(&[&argument]).unwrap();
        assert_eq!(capture.stdout, format!("{argument}\n").as_bytes());
        assert!(!sentinel.exists(), "argument was interpreted by a shell");
    }

    #[cfg(unix)]
    #[test]
    fn runtime_suite_selects_only_read_only_verbs() {
        let target = TmpRepo::new("runtime-readonly");
        let log = target.path.join("argv.log");
        let body = format!("printf '%s\\n' \"$*\" >> {:?}\nprintf '{{}}\\n'", log);
        let script = executable_script("runtime-readonly-bin", &body);
        let _ = runtime_probes_with_timeout(
            &script.path.join("probe-target"),
            &target.path,
            Duration::from_secs(2),
        );
        let calls = std::fs::read_to_string(log).unwrap();
        assert!(!calls.contains("skill install"), "{calls}");
        assert!(
            !calls.lines().any(|line| line.starts_with("new ")),
            "{calls}"
        );
        assert!(!calls.contains("--fix"), "{calls}");
    }

    #[test]
    fn a_probe_io_fault_propagates_as_err() {
        // A permission-denied read is an operational fault, not a conformance miss.
        // Unix-only: a `chmod 000` directory is the portable way to force EACCES on read.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let repo = TmpRepo::new("fault");
            repo.mkdir("crates");
            std::fs::set_permissions(
                repo.path.join("crates"),
                std::fs::Permissions::from_mode(0o000),
            )
            .unwrap();
            let result = probe_core_cli_split(&repo.path);
            // Restore perms so Drop can clean up regardless of the assertion outcome.
            let _ = std::fs::set_permissions(
                repo.path.join("crates"),
                std::fs::Permissions::from_mode(0o755),
            );
            // Running as root bypasses permission bits; only assert when the fault actually occurs.
            if let Err(e) = &result {
                assert_ne!(e.kind(), std::io::ErrorKind::NotFound);
            }
        }
    }
}
