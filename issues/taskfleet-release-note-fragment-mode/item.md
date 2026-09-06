---
created: 2026-09-06
updated: 2026-09-06
type: bug
status: fixed
priority: high
related: ['@taskfleet-convergence-release-notes']
commits:
- hash: 858941e131eebfccc525e2cdec959798f1614afd
  summary: 'fix(changelog): honor fragment-mode release notes'
closed: 2026-09-06
---

# Place Taskfleet release note in configured fragment mode

## Problem

The Taskfleet convergence release note was written directly into marker-owned `CHANGELOG.md`, but the approved contract declares `changelog.mode: fragment`. In fragment mode, per-change wording belongs in `changelog/fragments/` and the release engine compiles it at finalize time. The sealed plan `7f3b9723...` exposed this mismatch before any cut and must not be used.

## Required work

Remove only the newly added direct Taskfleet bullet from `[Unreleased]` and add the equivalent explicitly worded fragment under the configured fragment directory using the repository's naming/content convention. Preserve markers and unrelated changelog content. Do not include retired identity text. Validate contract-driven fragment compilation in dry-run/plan form.

Do not modify application code, bump, tag, publish, deploy, install, or edit external repositories.

## Acceptance Criteria

- [x] The direct per-change bullet is absent from marker-owned `[Unreleased]`.
- [x] An equivalent collision-safe Taskfleet convergence fragment exists under the configured directory.
- [x] Contract/changelog validation and a fresh patch plan pass.
- [x] Zero-reference scan remains clean.

## Decisions

### 2026-09-06T17:22:28Z · @agent

Moved the explicit wording into the contract-configured fragment directory with an issue-derived category filename, leaving the marker-bounded Unreleased skeleton empty. Rejected direct CHANGELOG editing because fragment mode reserves per-change content for collision-safe fragments; rejected generic or timestamp-only naming because the issue slug and category are clearer and remain collision-safe.

## Resolution

### 2026-09-06T17:22:35Z · @issuectl

Moved the explicit Taskfleet convergence note from the marker-owned Unreleased section into the configured collision-safe fragment, validated the approved fragment-mode contract and zero-reference scan, and sealed patch plan 5d652cf2426c3f52e4d24afb3993b3a39921d7045d3de109adfbdb37ba0240ae without cutting it.
