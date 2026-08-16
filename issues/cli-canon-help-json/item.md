---
created: 2026-08-16
updated: 2026-08-16
type: improvement
status: done
priority: normal
labels: [cli-canon, tooling]
lane: canon-rollout
lane_seq: 35
commits:
- hash: f62b0ae
  summary: 'feat(cli): add machine-readable help JSON'
- hash: e92f0a2
  summary: 'fix(cli): harden help JSON contract'
closed: 2026-08-16
---

# cli-canon: §14 --help --json machine-readable help

## Description


Filed by the `stack-cli-alignment` CLI-surface normalisation (homebase epic), phase 1.
Source: homebase `issues/cli-alignment-audit/analysis.md` (2026-08-10 audit) + live
re-verification 2026-08-16. Canon: `AGENTS-AI-FIRST-CLI.md`. This is a **fix** issue
(the audit + review only recommend); laned in `cli-canon` for a future `/stint-start`.

**Gap (§14) — `--help --json` not implemented.**

Text `--help` is fine (clap gives drill-down for free), but there is no machine-readable
help payload. Family-wide near-gap.

**Do:** support `--help --json` emitting a structured help document — subcommands, flags,
`examples[]`, and env-var mappings — so an agent can discover the surface without scraping
prose.

**Current state (evidence):** text help only; no `--help --json` payload.

## Comments

### 2026-08-16T14:51:14Z · @jari

Delivered together with @add-machine-readable by the same canon §14 implementation; both audit trails point to commits f62b0ae and e92f0a2.
