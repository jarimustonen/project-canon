---
created: 2026-08-12
updated: 2026-08-14
type: feature
status: in-progress
priority: normal
epic: project-canon-v0
labels: [tooling]
blocked_by: ['@profile-and-base-canon-model']
lane: build
---

# review verb: recommending audit (severity-triaged findings, staged issues, never auto-fix)

## Description

project-canon review [--profile ...] <repo>: the deeper human-facing pass (cli-canon's review mode) — findings triaged by the canon's severity model, staged issue commands, dimension-discovery candidates. Recommends and stages; NEVER auto-fixes or auto-files. Per ADR 0009 §2.
