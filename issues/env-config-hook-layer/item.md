---
created: 2026-08-12
updated: 2026-08-12
type: task
status: open
priority: normal
epic: project-canon-v0
labels: [tooling]
blocked_by: ['@extract-canon-and-skill']
---

# Externalize homebase env specifics to a config/hook layer (keep the tool portable)

## Description

From commit one, externalize the non-portable homebase env specifics to a config/hook layer: the ~/Sources family-repo map (also cli-canon's hard-coded map), gh account jarimustonen, ~/Sources/<name> location, tw/projects.conf registration, .workmux.yaml emoji prefix, and (future) hauis CI release pattern. homebase's create-project skill becomes a THIN wrapper delegating scaffold to project-canon new. Per ADR 0009 §2/§5/§6.
