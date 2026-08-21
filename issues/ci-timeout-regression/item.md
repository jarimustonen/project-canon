---
created: 2026-08-21
updated: 2026-08-21
type: bug
reporter: mail-triage
status: in-progress
priority: high
lane: ci-health
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

## Comments

### 2026-08-21T09:32:30Z · @agent

**Scoped as an INVESTIGATION first, not a fix (owner decision, 2026-08-21).**

It is genuinely uncertain whether there is a defect here at all. Establish that before writing any fix.

**"Leave it alone" is an explicitly acceptable outcome** — if the investigation concludes the runner behaviour is correct and only the test is over-specified for a shared CI machine, say so and close accordingly (`wontfix`/`obsolete` as appropriate). Do not manufacture a fix to justify the work, and do not weaken the runner's timeout guarantee just to make a flaky test green.

**What is actually known (verified 2026-08-21, do not inherit the issue title's claim):**

- The issue says "the CI workflow is red on current `main`". That is **no longer true**: run `32336751643` (2026-08-20) failed, but the next run `32457390725` (2026-08-21) **passed** on `main`.
- So the observed signal is an **intermittent/flaky failure**, not a persistent regression. One failure, one pass, same test.
- The test passes consistently on local macOS; the single failure was on GitHub's Linux runner.

**The real question to answer first:** is `descendants_holding_capture_pipes_cannot_bypass_the_timeout` detecting a real race in the runner's completion path (parent exit treated as completion before captured pipes reach EOF), or is the *test* timing-fragile on a loaded shared runner (2-second deadline, background `sleep`, shell scheduling)? Those have opposite remedies and the evidence so far does not distinguish them.

Useful next step: re-run the job several times to measure the actual flake rate before theorising. A one-in-N failure on a 2s deadline on shared CI hardware points at the test; a reproducible failure points at the runner.

Context: this code landed 2026-08-17 as part of `review --run` (opt-in runtime probes). The timeout guarantee it encodes is a real safety property of that feature — a hanging probe target must not wedge `review` — so if the runner *is* wrong, it matters. Related canon: §24 (verify a stated blocker rather than inheriting it) applies to this issue's own claims.
