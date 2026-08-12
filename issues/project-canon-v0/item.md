---
created: 2026-08-12
updated: 2026-08-12
type: epic
owner: jari
status: open
priority: normal
labels: [design]
---

# project-canon v0: extract the canon + ship new/doctor/review with the cli profile

## Description

Build project-canon v0 per homebase ADR 0009 (docs/decisions/0009-project-canon-scope.md). Project/repo-scoped conformance tool: a base project canon + per-archetype profiles, with the AI-first CLI canon as the `cli` profile. v0 is a LIFT, not a greenfield canon (ADR §6): author ONLY the cli profile, seed the base canon from what create-project already scaffolds + repo-general canon sections (§10,§15-17,§22) + already-discovered dims, ship new/doctor/review end-to-end for the cli profile, and externalize homebase env specifics to a config/hook layer from commit one. service/library/release profiles are named-but-empty extension points.
