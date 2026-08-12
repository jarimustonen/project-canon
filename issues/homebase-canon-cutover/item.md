---
created: 2026-08-12
updated: 2026-08-12
type: task
status: open
priority: normal
epic: project-canon-v0
labels: [tooling]
lane: build
blocked_by: [extract-canon-and-skill]
---

# Homebase-side canon cutover: homebase copies canon + cli-canon skill FROM project-canon; retire its masters

## Description

Follow-up to `extract-canon-and-skill` (now done). `project-canon` is now the DECLARED
maintained home of `AGENTS-AI-FIRST-CLI.md` (§1–§22) + the `cli-canon` skill, but the homebase
side has NOT been switched over: homebase still holds its own master copies. This task makes
homebase copy those artifacts FROM `project-canon` and retires its own masters, so the two
cannot diverge.

Requires edits in the HOMEBASE repo — out of scope for a `project-canon` worktree, so it is
executed there (or as a homebase stint), only tracked here. Blocked_by `extract-canon-and-skill`
(delivered). Per ADR 0009 §2/§6.
