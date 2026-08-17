---
created: 2026-08-16
updated: 2026-08-16
type: feature
status: done
priority: normal
epic: project-canon-v0
labels: [canon]
lane: canon-rollout
lane_seq: 30
commits:
- hash: f62b0ae
  summary: 'feat(cli): add machine-readable help JSON'
- hash: e92f0a2
  summary: 'fix(cli): harden help JSON contract'
closed: 2026-08-16
---

# Add machine-readable help JSON

## Description

Implement agent-readable help JSON for top-level and subcommand help per canon §14.

Comparative audit evidence:

- `project-canon review --assume-defaults <personal-repo-root>/project-canon` left §14 as manual-verify.
- Manual probe: `project-canon doctor --help --json` exits 0 but prints prose help rather than JSON.
- Manual probe: `project-canon --help --json` likewise prints prose startup/help text.

Acceptance:

- `project-canon --help --json` and `project-canon <verb> --help --json` emit structured help with schema version, command path, summary, arguments, flags, subcommands where applicable, examples, and exit-code notes.
- Human `--help` stays readable and exits 0.
- JSON help is stable enough for agents and tested with golden fixtures.
