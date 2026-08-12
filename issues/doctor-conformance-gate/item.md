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

# doctor verb: mechanical conformance gate (CI, non-zero on MUST gap)

## Description

project-canon doctor [--profile ...] [<repo>]: characterize the repo, run the profile's MECHANICAL probes (grep/probe-decidable only, mirrors canon §18 doctor discipline), emit a pass/fail matrix, exit non-zero on a mechanically-decided MUST gap. Read-only, non-interactive, --assume-defaults, CI-shaped. Distinct from review (no LLM judgment). Per ADR 0009 §2.
