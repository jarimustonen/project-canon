---
created: 2026-08-21
updated: 2026-08-21
type: bug
reporter: mail-triage
status: wontfix
priority: high
lane: ci-health
commits:
- hash: a0981fe
  summary: mark investigation in progress
closed: 2026-08-21
closed_by: agent
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

### 2026-08-21T09:40:03Z · @agent

#### Investigation verdict: (B), not a runner timeout race

I re-verified the stale blocker and measured the reported signal rather than inheriting the issue's original conclusion.

##### Measurements

- GitHub Linux run `32336751643`, attempt 1, failed this assertion on SHA `0c40d359945dda6941d17048d15a26c3db0f3203`.
- I reran that same workflow/SHA four times (attempts 2–5). All four passed. Measured rate for that exact run is therefore 1 failure in 5 attempts (20%), with no reproduction in the four fresh attempts.
- Later Linux runs also contradict a persistent regression: `32457390725` passed; in `32459064256` this descendant test passed (that run failed elsewhere); and `32468414223` passed.
- On local macOS, the exact test passed 20/20 times, each returning at the expected roughly two-second deadline. This is not Linux evidence; the repeated GitHub attempts above are the Linux sample.
- The original failing attempt completed the entire 140-test binary in 0.30 seconds. Thus this test returned almost immediately; it did not wait for the two-second timeout or for `sleep 10` to finish.

##### Completion-path reading

`RuntimeRunner::run` does not treat direct-child exit as command completion. After spawn it starts bounded stdout/stderr readers, records one `started` instant, waits for stdout drainage with `recv_timeout(remaining())`, then stderr drainage with the same remaining deadline, and only then calls `wait_for_child_without_pid_reuse` with that same start/deadline. If a descendant really holds either capture pipe open, the corresponding receive times out, `kill_child_tree` kills the process group, and the result is `RunFailure::Timeout`. Parent exit alone cannot produce `ChildCapture` while a capture pipe remains open.

There is a stronger explanation for the sub-second non-timeout result. Run `32459064256` failed another test using the same freshly written executable-script fixture with `RunFailure::Start("Text file busy (os error 26)")` on Linux, while this descendant test passed in that run. The descendant test's generic `matches!(..., Err(RunFailure::Timeout))` assertion hides which alternate variant occurred; a transient `Start`/fixture-execution failure would produce exactly the observed immediate assertion failure. Its PID-file check happens only after that assertion, so it cannot distinguish or prevent this startup failure.

##### Decision

The evidence supports (B), more specifically an intermittent Linux test-fixture/startup failure rather than a two-second scheduling miss. It does not support (A): the runner already waits for capture EOF and child status under one deadline, and the four same-SHA Linux reruns did not expose a bypass. I deliberately made no production or test change: weakening the timeout would damage a real safety property, and changing the runner to implement behavior it already has would manufacture a fix. Closing as `wontfix` for the alleged runner regression. If Linux `ETXTBSY` fixture failures become frequent enough to address, that should be a separately scoped test-infrastructure issue with diagnostics that preserve the actual `RunFailure` variant.
