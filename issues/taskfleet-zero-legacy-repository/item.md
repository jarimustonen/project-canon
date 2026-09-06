---
created: 2026-09-06
updated: 2026-09-06
type: task
status: open
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
- [ ] Tracked path/content scans contain zero retired orchestration identities.
- [ ] Canon source and all generated copies use canonical Taskfleet wording and are hash/snapshot coherent.
- [ ] Fresh disposable generation/install output remains canonical.
- [ ] Full repository gate passes and the change is ready for normal project-canon release.
