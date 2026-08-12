---
created: 2026-08-12
updated: 2026-08-12
type: feature
status: open
priority: normal
epic: project-canon-v0
labels: [design]
blocked_by: ['@extract-canon-and-skill']
---

# Base-canon + archetype-profile model (author only the cli profile at v0)

## Description

Implement the two-layer model (ADR 0009 §1/§4): base project canon (repo-invariant dims) + additive archetype profiles selected by an applicability questionnaire (reuse cli-canon's characterize→applicable-sections mechanism). Author ONLY the cli profile at v0 (= §1-§22, a lift). Seed base canon from create-project's implicit scaffold + repo-general sections (§10,§15-17,§22) + discovered dims. Leave service/library/release as named-but-empty extension points. A profile = a named section-set + probe registry. Route dimension-discovery candidates to base-vs-profile.
