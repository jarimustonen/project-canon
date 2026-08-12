---
created: 2026-08-12
updated: 2026-08-12
type: feature
status: done
priority: normal
epic: project-canon-v0
labels: [design]
blocked_by: ['@extract-canon-and-skill']
lane: build
commits:
- hash: 8f1fe63
  summary: 'core: two-layer model + cli profile lift + crate/workspace'
- hash: 53f5816
  summary: apply 8 review FIXes; defer 5 architectural findings as spinoffs
closed: 2026-08-12
---

# Base-canon + archetype-profile model (author only the cli profile at v0)

## Description

Implement the two-layer model (ADR 0009 §1/§4): base project canon (repo-invariant dims) + additive archetype profiles selected by an applicability questionnaire (reuse cli-canon's characterize→applicable-sections mechanism). Author ONLY the cli profile at v0 (= §1-§22, a lift). Seed base canon from create-project's implicit scaffold + repo-general sections (§10,§15-17,§22) + discovered dims. Leave service/library/release as named-but-empty extension points. A profile = a named section-set + probe registry. Route dimension-discovery candidates to base-vs-profile.
