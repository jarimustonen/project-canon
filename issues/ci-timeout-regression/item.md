---
created: 2026-08-21
updated: 2026-08-21
type: bug
reporter: mail-triage
status: open
priority: high
---

# CI timeout regression for descendant-held capture pipes

_Source: GitHub Actions CI run 32336751643_

## Description

The `CI` workflow is red on current `main` (run 32336751643). The Rust test job fails at `crates/project-canon-cli/src/probes.rs:2141`:

`descendants_holding_capture_pipes_cannot_bypass_the_timeout` expected `Err(RunFailure::Timeout)` but received another result. The test starts `sleep 10` in the background, lets the parent shell exit, and expects the runner's two-second timeout to fire because the descendant still holds the captured stdout/stderr pipes.

Failing run: https://github.com/jarimustonen/project-canon/actions/runs/32336751643

## Root cause reading

The runner currently treats parent-process exit as completion before proving that captured pipes have reached EOF, or its timeout/descendant cleanup path is race-sensitive on GitHub's Linux runner. That breaks the invariant the test documents: descendants must not keep capture pipes open beyond the configured deadline or let the command appear successfully complete.

## Concrete fix

Make process completion wait on both the child status and captured-pipe drainage under the same deadline. On timeout, terminate the process group/descendants and join capture readers deterministically. Keep the regression test, but synchronize descendant startup robustly (the PID file already provides a suitable handshake) so it does not depend on shell scheduling.
