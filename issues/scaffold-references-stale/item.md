---
created: 2026-09-18
updated: 2026-09-18
type: bug
reporter: jari
status: fixed
priority: normal
lane: build
collision: [crates/project-canon-cli/src/new.rs]
commits:
- hash: e6dba1589281e2056d502e2170c7bd428c59bcaa
  summary: 'fix: reference installed canon skill from scaffolds'
closed: 2026-09-18
---

# Scaffold references a stale repo-local canon copy

## Description

`project-canon new` generates `AGENTS-AI-FIRST-CLI.md` inside every new repository even though the maintained distribution path is the versioned `ai-first-cli-canon` skill installed by `project-canon skill install`. This leaves each generated project with a duplicate canon copy that can become stale after the installed skill is upgraded.

Generated `AGENTS.md` also directs agents to read that duplicate file. It should instead direct them to use the installed `/ai-first-cli-canon` skill. Other generated references must not point at a file that the scaffold no longer creates.

Do not change environment-specific wrapper skills. The generic scaffold remains owned by `project-canon new`.

## Acceptance

- New scaffolds do not contain `AGENTS-AI-FIRST-CLI.md`.
- Generated `AGENTS.md` directs CLI-surface work to `/ai-first-cli-canon`.
- Generated conformance guidance has no dangling repo-local canon reference.
- The canonical document remains bundled for `project-canon skill install` and `skill print`.
- Scaffold tests cover the absence and the skill-based guidance.

## Resolution

### 2026-09-18T06:34:18Z · @issuectl

Fresh scaffolds now consume the installed canon skill and no longer generate a duplicate canon document; targeted and full workspace gates passed.
