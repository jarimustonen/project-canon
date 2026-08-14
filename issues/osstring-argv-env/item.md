---
created: 2026-08-14
updated: 2026-08-14
type: improvement
status: wontfix
priority: normal
epic: project-canon-v0
labels: [tooling]
closed: 2026-08-14
---

# Robust non-UTF-8 argv/env handling (OsString) so doctor never panics

## Description

## Description

Make `project-canon` robust to non-UTF-8 process arguments and environment variables so it never panics — a CI-facing conformance gate must always honor its 0/1/2 exit-code contract.

## Why

Surfaced by the 4-model review of the `doctor` verb (`history/review-doctor-raw.md`; openai + deepseek). `main.rs` uses `std::env::args()` and `doctor::run` collects `std::env::vars()`; both **panic** on invalid Unicode. A non-UTF-8 repo path (legal on Unix) or a non-UTF-8 environment entry crashes the process before the documented exit code can be returned, so CI sees a crash rather than a clean exit 2.

## Scope / why its own issue

The fix threads `OsString`/`PathBuf` through `main` and `doctor::run`: argv becomes `&[OsString]`, the repo argument becomes a `PathBuf` rather than `String`, and env parsing uses `std::env::vars_os()` filtered to the `PROJECT_CANON_` prefix (reporting invalid Unicode only for relevant variables). That is a signature change across the binary edge and the verb entry point — its own change, not a mechanical in-place fix. Real-world likelihood in CI is low, so it did not block v0.

## Acceptance

- A non-UTF-8 repo path yields a clean exit 2 (usage error), not a panic.
- A non-UTF-8 unrelated env var does not crash the process.
- Existing UTF-8 behavior and tests unchanged.
