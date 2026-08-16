---
created: 2026-08-16
updated: 2026-08-16
type: feature
status: in-progress
priority: normal
epic: project-canon-v0
labels: [canon]
lane: canon-rollout
lane_seq: 30
---

# Add machine-readable help JSON

## Description

Implement agent-readable help JSON for top-level and subcommand help per canon §14.

Comparative audit evidence:

- `project-canon review --assume-defaults /Users/jari/Sources/project-canon` left §14 as manual-verify.
- Manual probe: `project-canon doctor --help --json` exits 0 but prints prose help rather than JSON.
- Manual probe: `project-canon --help --json` likewise prints prose startup/help text.

Acceptance:

- `project-canon --help --json` and `project-canon <verb> --help --json` emit structured help with schema version, command path, summary, arguments, flags, subcommands where applicable, examples, and exit-code notes.
- Human `--help` stays readable and exits 0.
- JSON help is stable enough for agents and tested with golden fixtures.
