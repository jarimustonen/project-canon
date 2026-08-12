---
created: 2026-08-12
updated: 2026-08-12
type: feature
status: open
priority: normal
epic: project-canon-v0
labels: [tooling]
blocked_by: ['@profile-and-base-canon-model']
lane: build
---

# new verb: scaffold a conformant repo (subsumes create-project generator)

## Description

project-canon new: scaffold a repo that starts conformant — git, private GitHub, doc structure, issuectl init, base-canon files, and the selected profile's surface scaffolding (cli-canon's generate mode folded in). Subsumes create-project's generator half. Env specifics (tw/projects.conf, gh account, ~/Sources, .workmux emoji) live in the config/hook layer, NOT here. Per ADR 0009 §2/§6.
