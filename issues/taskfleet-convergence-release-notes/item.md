---
created: 2026-09-06
updated: 2026-09-06
type: task
status: open
priority: high
related: ['@taskfleet-zero-legacy-repository']
---

# Prepare Taskfleet convergence release notes

## Problem

The completed project-canon Taskfleet identity convergence is production-ready and exact-main CI is green, but curated `[Unreleased]` is empty. A release now would omit the generator and generated-skill change from its notes.

## Required work

Using marker-anchored changelog rules, add a concise Changed entry explaining that the canonical CLI canon skill and generated repository catalogs now use Taskfleet consistently. Do not name the retired identity, change application code, bump versions, tag, release, deploy, install, or edit external repositories. Keep the repository-wide identity scan at zero and prove the release engine can plan a patch cut.

## Acceptance Criteria

- [ ] `[Unreleased]` accurately describes the generator-visible change without retired identity text.
- [ ] Changelog markers remain valid.
- [ ] Canonical identity scan stays clean and focused checks pass.
- [ ] A patch release plan can be formed without cutting it.
