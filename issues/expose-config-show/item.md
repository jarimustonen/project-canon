---
created: 2026-08-16
updated: 2026-08-16
type: feature
status: done
priority: normal
epic: project-canon-v0
labels: [canon]
lane: canon-rollout
lane_seq: 40
commits:
- hash: 3d26358
  summary: expose resolved configuration
- hash: d7da2c0
  summary: harden config inspection
closed: 2026-08-16
---

# Expose config show and path inspection

## Description

Expose an inspectable `config` surface for the environment/project defaults that `project-canon` already resolves and prints in human startup text.

Comparative audit evidence:

- The shipped canon §8 requires `config show` / path inspection when a tool has configuration, environment resolution, or data roots.
- `project-canon --version` reports an env layer (`a configured account, personal repository root, and family-repo map`), so the tool does have resolved configuration-like state.
- Manual probe: `project-canon config show --json` is not implemented.

Acceptance:

- `project-canon config show --json` reports the resolved values that affect behavior, including repo root/family repo discovery and any environment-derived settings, with provenance for each value and secret redaction if secrets are ever added.
- A path-oriented inspection command exists for any config/data root the tool reads or writes.
- Human mode remains concise, while JSON mode is complete and stable.
