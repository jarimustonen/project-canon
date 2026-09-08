---
created: 2026-09-06
updated: 2026-09-08
type: task
status: open
priority: high
lane: build
lane_seq: 20
---

# Converge project-canon references on Taskfleet

## Goal

Converge Project Canon's active Taskfleet identity references from the retired product name to canonical `taskfleet`, while leaving stable protocol identifiers and immutable issue records untouched.

## Authorizing evidence

Taskfleet E1 owner map commit `8b8652a964a1353dc869e89fd541e8cf5b30f1e6`, P1-P3: https://github.com/jarimustonen/taskfleet/blob/8b8652a964a1353dc869e89fd541e8cf5b30f1e6/issues/taskfleet-dependent-owner-discovery/owner-map.md

## Required work

- Refresh the repo-local, Issuectl-owned issue-intake Claude/Codex dogfood copies through the released Issuectl supported path.
- Update Project Canon's canonical CLI-canon source and tests where examples refer to the current Taskfleet product by its retired name.
- Preserve stable `OCTL_*` protocol identifiers, the telemetry contract id, compatibility fixtures, and immutable issue/evidence records.
- Do not add historical migration narratives to `AGENTS.md`; keep operating guidance current and actionable.
- Run template/snapshot/integrity tests and the full repository gate. Follow Project Canon's normal release policy if its distributed catalog changes.

## Acceptance Criteria

- [ ] Repo-local Issuectl dogfood copies match the released owner template and identify Taskfleet correctly.
- [ ] Project Canon's active CLI-canon examples name canonical Taskfleet consistently.
- [ ] Stable protocol and compatibility identifiers remain unchanged.
- [ ] Full gate passes and any required release is verified.
