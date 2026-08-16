---
created: 2026-08-12
updated: 2026-08-16
type: task
status: done
priority: normal
epic: project-canon-v0
labels: [tooling]
lane: build
blocked_by: [extract-canon-and-skill]
closed: 2026-08-16
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

## Rollout gate (owner decision 2026-08-13)

This cutover — and the wider cross-repo adoption (all other repos consuming the canon/tool FROM
`project-canon`) — is **gated on cutting project-canon's FIRST release**. Do NOT switch homebase or
any other repo before that release exists. When the first release is cut, **tell the owner clearly
and explicitly**; the go-wide across all repos happens on his go at that point. Until then: edit the
canon only in `project-canon`, leave other repos' copies untouched (avoid drift).

## Comments

### 2026-08-16T08:44:34Z · @pi

2026-08-16: gate opened by v0.1.1 release. Homebase-side cutover landed in homebase: repos now consume the released project-canon /ai-first-cli-canon skill, the old homebase master AGENTS-AI-FIRST-CLI.md copy is gone, and global Claude/Codex/pi installs are present.
