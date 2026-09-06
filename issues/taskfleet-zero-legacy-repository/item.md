---
created: 2026-09-06
updated: 2026-09-06
type: task
status: in-progress
priority: high
related: ['@taskfleet-project-canon-reference-convergence']
---

# Converge project-canon generator on Taskfleet identity

## Goal
Converge the entire maintained project-canon repository and its generated AI-first CLI canon skill on Taskfleet as the sole orchestration identity.

## Required work
Review every tracked source, generator template, emitted skill, workflow, config, test, fixture, snapshot, document, and issue. Remove retired product, command, environment-prefix, package, protocol, repository, state/config, and filesystem identities from maintained HEAD. Treat project-canon as a generator owner: update canonical source first, regenerate all owned outputs through supported commands, and verify fresh installation output. Refresh issuectl-owned repository skills through released issuectl.

Do not release, deploy, install globally, mutate user state, perform physical checkout/worktree renames, or edit other repositories from the worker.

## Acceptance Criteria
- [x] Tracked path/content scans contain zero retired orchestration identities.
- [x] Canon source and all generated copies use canonical Taskfleet wording and are hash/snapshot coherent.
- [x] Fresh disposable generation/install output remains canonical.
- [x] Full repository gate passes and the change is ready for normal project-canon release.

## Decisions

### 2026-09-06T17:10:49Z · @agent

Implemented repository-wide identity convergence rather than limiting the change to active prose: maintained historical issue text and protocol namespace labels now use canonical Taskfleet wording while immutable run IDs remain intact. Updated the packaged cli-canon source before validating generated output, and added a regression test over every shipped canon/skill source. Refreshed all Claude, pi, and Codex issuectl-owned repository skills through issuectl 0.18.3; the newly supported pi copies are tracked so all emitted layouts stay coherent. Rejected preserving legacy compatibility spellings in maintained fixtures because this issue requires a literal zero scan, and rejected global installation, release, checkout renaming, and external-repository edits per the worker boundary.
