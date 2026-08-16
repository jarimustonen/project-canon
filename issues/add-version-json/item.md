---
created: 2026-08-16
updated: 2026-08-16
type: feature
status: in-progress
priority: normal
epic: project-canon-v0
labels: [canon]
lane: canon-rollout
lane_seq: 20
---

# Add version JSON drift contract

## Description

Add the canon §10 `version --json` drift contract for `project-canon`.

Comparative audit evidence:

- `project-canon review --assume-defaults /Users/jari/Sources/project-canon` left §10 as manual-verify.
- Manual probe: `project-canon --version --json` prints human prose and ignores the requested JSON shape.
- Manual probe: `project-canon version --json` is not implemented.

Acceptance:

- `project-canon version --json` emits a structured payload with `schema_version`, tool name, CLI version, build commit or explicit null plus build provenance, supported output schema versions, supported profiles/surfaces, and bundled skills with their schema/version metadata.
- `--version` remains human-readable and exits 0.
- The JSON shape is covered by a golden test so downstream agents can rely on it.
