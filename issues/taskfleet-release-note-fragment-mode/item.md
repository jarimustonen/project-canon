---
created: 2026-09-06
updated: 2026-09-06
type: bug
status: open
priority: high
related: ['@taskfleet-convergence-release-notes']
---

# Place Taskfleet release note in configured fragment mode

## Problem

The Taskfleet convergence release note was written directly into marker-owned `CHANGELOG.md`, but the approved contract declares `changelog.mode: fragment`. In fragment mode, per-change wording belongs in `changelog/fragments/` and the release engine compiles it at finalize time. The sealed plan `7f3b9723...` exposed this mismatch before any cut and must not be used.

## Required work

Remove only the newly added direct Taskfleet bullet from `[Unreleased]` and add the equivalent explicitly worded fragment under the configured fragment directory using the repository's naming/content convention. Preserve markers and unrelated changelog content. Do not include retired identity text. Validate contract-driven fragment compilation in dry-run/plan form.

Do not modify application code, bump, tag, publish, deploy, install, or edit external repositories.

## Acceptance Criteria

- [ ] The direct per-change bullet is absent from marker-owned `[Unreleased]`.
- [ ] An equivalent collision-safe Taskfleet convergence fragment exists under the configured directory.
- [ ] Contract/changelog validation and a fresh patch plan pass.
- [ ] Zero-reference scan remains clean.
