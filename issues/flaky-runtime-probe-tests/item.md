---
created: 2026-11-27
updated: 2026-11-27
type: bug
status: open
priority: normal
provenance: other
provenance_detail: observed while running the green gate in a spinoff worker
source_ref: taskfleet:01m8ehfmd4d448yx6thnzhwjew/observed:flaky-runtime-probe-tests
originating_run: 01m8ehfmd4d448yx6thnzhwjew
originating_run_kind: spinoff
lane: unlaned
---

# Runtime-probe unit tests fail intermittently under cargo test --workspace

## Description

# Runtime-probe unit tests fail intermittently under `cargo test --workspace`

## Observed

On Linux, `cargo test --workspace` fails roughly one run in four or five, each time in a different runtime-probe test in `crates/project-canon-cli/src/probes.rs`, which the change being tested did not touch:

- `probes::tests::successful_probe_cleans_up_redirected_descendants`: FAILED
- `probes::tests::a_hanging_runtime_binary_is_killed_at_the_timeout`: panicked at `assertion failed: outcomes[0].message.contains("timed out")`

Repeated runs of only the `probes::` tests (12 times) did not fail, so the failure depends on the whole parallel test binary.

## Likely cause

The closed `ci-timeout-regression` analysis saw `RunFailure::Start("Text file busy (os error 26)")` on Linux for freshly written executable-script fixtures. A fixture script that is written and then exec'd while another test thread forks can inherit the still-open write descriptor (ETXTBSY). The generic assertions hide which `RunFailure` variant occurred.

## Impact

The green gate (`cargo test --workspace`) is red at random for unrelated changes, so workers and CI rerun it or start doubting real failures.

## Possible direction

Make the probe-test fixtures robust against ETXTBSY: write the script, close it, and set its permissions before any exec, or retry on ETXTBSY in the test harness. Have the assertions print the actual `RunFailure`/message so the next failure is diagnosable.
