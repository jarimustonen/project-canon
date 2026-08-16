---
created: 2026-08-16
updated: 2026-08-16
type: improvement
status: done
priority: normal
epic: project-canon-v0
labels: [canon]
lane: canon-rollout
lane_seq: 10
commits:
- hash: ba6fc3a
  summary: normalize JSON error envelopes and exit codes
- hash: c32622f
  summary: harden JSON error output
closed: 2026-08-16
---

# Normalize JSON error envelopes and exit codes

## Description

Normalize `project-canon`'s machine-facing error contract to match the shipped AI-first CLI canon §2/§10.

Comparative audit evidence:

- `project-canon review --assume-defaults /Users/jari/Sources/project-canon` reported no confirmed gaps, but left §2/§10 as manual-verify.
- Manual probe: `project-canon --json --version` exits 2 with prose stderr (`project-canon: unknown subcommand or flag: "--json"`), not a JSON error envelope.
- Manual probe: `project-canon config show --json` exits with prose stderr, not `{"error":{"code":...,"message":...}}`.

Acceptance:

- All failures reachable when the caller requested JSON emit the standard error envelope on stderr and no stdout data.
- Exit codes follow the family map: success 0, caller/domain-actionable 1, system/internal 2, signal exits preserved.
- Clap usage/parse failures are centrally remapped instead of leaking clap's default usage exit 2.
- Tests cover an unknown subcommand/flag and one domain validation error.
