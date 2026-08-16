//! Central fatal-error rendering and exit classification for the CLI.
//!
//! Every command must route failures through this module. It keeps `--json` errors on stderr,
//! prevents accidental stdout payloads on failure, and implements the canon §2 exit-code map.

use std::io::Write as _;
use std::process::ExitCode;

use crate::json::Json;

/// The coarse error classes used for canonical exit-code routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErrorClass {
    /// The caller can correct the invocation or current domain state.
    Actionable,
    /// The tool or its environment failed and the caller should retry or escalate.
    System,
}

impl ErrorClass {
    const fn exit_code(self) -> u8 {
        match self {
            Self::Actionable => 1,
            Self::System => 2,
        }
    }
}

/// A machine-facing fatal error, rendered consistently by [`fail`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliError {
    code: &'static str,
    message: String,
    class: ErrorClass,
}

impl CliError {
    /// A malformed invocation or domain validation failure.
    pub(crate) fn actionable(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            class: ErrorClass::Actionable,
        }
    }

    /// An I/O or invariant failure outside the caller's direct control.
    pub(crate) fn system(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            class: ErrorClass::System,
        }
    }
}

/// Whether an argv slice explicitly requests JSON. This deliberately recognizes only the
/// valueless spelling: `--json=value` is itself a usage error, not a JSON request.
pub(crate) fn json_requested(args: &[String]) -> bool {
    args.iter()
        .take_while(|arg| arg.as_str() != "--")
        .any(|arg| arg == "--json")
}

/// Render a fatal error to stderr and return its canon-defined exit code.
///
/// `json` comes from the successfully parsed command when available; for parse errors callers
/// pass [`json_requested`] over the raw argv so the error contract survives parser failure.
pub(crate) fn fail(json: bool, error: CliError) -> ExitCode {
    if json {
        eprintln!(
            "{}",
            Json::Object(vec![
                ("schema_version".into(), Json::Int(1)),
                (
                    "error".into(),
                    Json::Object(vec![
                        ("code".into(), Json::str(error.code)),
                        ("message".into(), Json::str(error.message)),
                    ]),
                ),
            ])
        );
    } else {
        eprintln!("project-canon: {}", error.message);
    }
    ExitCode::from(error.class.exit_code())
}

/// Write command data without turning a closed pipe into a panic.
///
/// A broken pipe is a normal pipeline termination. Other stdout failures are system errors and
/// honor the caller's requested output format.
pub(crate) fn write_stdout(content: &str, json: bool) -> ExitCode {
    let mut out = std::io::stdout().lock();
    match out.write_all(content.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => fail(
            json,
            CliError::system("io_error", format!("writing stdout: {error}")),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_only_the_valueless_json_flag() {
        assert!(json_requested(&["--json".into()]));
        assert!(!json_requested(&["--json=false".into()]));
    }
}
