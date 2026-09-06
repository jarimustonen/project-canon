---
created: 2026-09-06
updated: 2026-09-06
type: task
status: open
priority: high
lane: build
lane_seq: 20
---

# Converge project-canon references on Taskfleet

## Goal

Converge project-canon's distributed and dogfooded skills from active old Taskfleet identity references to canonical `taskfleet` while preserving historical references and public-neutrality.

## Authorizing evidence

Taskfleet E1 owner map commit `8b8652a964a1353dc869e89fd541e8cf5b30f1e6`, P1-P3: https://github.com/jarimustonen/taskfleet/blob/8b8652a964a1353dc869e89fd541e8cf5b30f1e6/issues/taskfleet-dependent-owner-discovery/owner-map.md

## Preconditions

Start only after issuectl's canonical Taskfleet issue-intake template is released and intakectl accepts the canonical `taskfleet` key.

## Required work

- Refresh generated issue-intake Claude/Codex copies through the released issuectl supported path.
- Update project-canon's own canonical CLI-canon skill source, tests, examples, and current tool-bug routing to public-neutral Taskfleet coordinates.
- Preserve explicitly historical bug references, stable `OCTL_*`, telemetry contract id, immutable evidence/history, and compatibility fixtures.
- Run template/snapshot/integrity tests and the full repository gate. Follow project-canon's normal release/convergence policy if its distributed catalog changes.

## Acceptance Criteria

- [ ] Distributed/dogfood current skills name canonical Taskfleet consistently.
- [ ] Generated copies match their owner and project-canon's own skill tests pass.
- [ ] Historical/protocol/compatibility references remain intentional.
- [ ] Full gate passes and any required release is verified before downstream installation.
