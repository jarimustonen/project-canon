---
created: 2026-09-28
updated: 2026-09-28
type: bug
status: open
priority: normal
---

# CI cli-canon native skill resource test fails

## Description

CI on main at `ce7b78b` fails in the Rust test job: https://github.com/jarimustonen/project-canon/actions/runs/36365309058. The successful release-workflow guard on the same commit does not make CI green; the last visible successful CI is on `0ec3130`.

## Reproduction

The GitHub Actions `test (rust)` job reports:

```
test skill::install::tests::cli_canon_native_forms_expose_every_resource ... FAILED
thread 'skill::install::tests::cli_canon_native_forms_expose_every_resource' panicked at crates/project-canon-cli/src/skill.rs:1492:13:
assertion failed: native.contains("check shipshape/issuectl/taskfleet")
test result: FAILED. 166 passed; 1 failed
```

The generated native `cli-canon` skill content no longer contains the exact phrase expected by this test. The most likely cause is wording drift between bundled skill source and the hardcoded test; verify whether the missing phrase represents a lost use case or merely a wording change before altering either side.

## Quick Test

Reproduce `cargo test --locked -p project-canon-cli cli_canon_native_forms_expose_every_resource`, reconcile the shipped native skill output and test with the intended set of CLI canon use cases, then run the full workspace tests and verify CI on main.
