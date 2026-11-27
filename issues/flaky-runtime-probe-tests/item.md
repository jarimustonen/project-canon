---
created: 2026-11-27
updated: 2026-11-27
type: bug
status: fixed
priority: normal
provenance: other
provenance_detail: observed while running the green gate in a spinoff worker
source_ref: taskfleet:01m8ehfmd4d448yx6thnzhwjew/observed:flaky-runtime-probe-tests
originating_run: 01m8ehfmd4d448yx6thnzhwjew
originating_run_kind: spinoff
lane: unlaned
closed: 2026-11-27
commits:
- hash: 2669ab5
  summary: make runtime-probe fixtures deterministic (ETXTBSY-free fixture writer, polled SIGKILL check)
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

## Resolution

### 2026-11-27T14:03:56Z · @issuectl

Root causes reproduced and fixed in tests only: (1) ETXTBSY from the fixture write fd inherited by a concurrent fork in another test thread (captured: Start("Text file busy (os error 26)")); fixtures are now written by a helper process. (2) a second race: descendant-cleanup tests sampled ps once right after an asynchronous SIGKILL (captured state "R"); now polled with a bounded deadline. Production launcher unchanged. Verified: 300/300 runs of the cli unit-test binary and 25 consecutive full cargo test --workspace runs green.

## Comments

### 2026-11-27T14:04:01Z · @worker

Design decisions. Rejected: (a) temp-name + rename: the inherited fd names the inode, so rename does not clear ETXTBSY; (b) retrying exec on ETXTBSY in the production launcher: production never execs a file it has just written, so this would add behaviour for a race only tests create; (c) widening runtime_process_test_guard: it only serialises probe tests, while the forking thread can be any test (e.g. ones spawning git), so serialising cannot close the window. The two integration-test shims in tests/review_cli.rs are written the same way but are asserted never to be executed, so ETXTBSY cannot fail them; left unchanged.
