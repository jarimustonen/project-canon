---
created: 2026-08-16
updated: 2026-08-17
type: feature
reporter: jari
status: done
priority: normal
labels:
- via:agent-homebase-wrapup
lane: canon-rollout
lane_seq: 30
commits:
- hash: 82324d3
  summary: add opt-in timeout-bounded runtime probes
- hash: 614c52a
  summary: harden process lifecycle and apply review findings
closed: 2026-08-17
closed_by: agent
---

# review: execute the built binary to auto-verify runtime canon checks in…

## Description

review: execute the built binary to auto-verify runtime canon checks instead of manual-verify

Observed (project-canon 0.1.1): `project-canon review --assume-defaults --json <repo>`
across the 8-CLI family auto-confirmed only ONE gap (§22 core/cli split, a static
filesystem check) and marked ~14 of the 22 sections **manual-verify** per repo — including
runtime-observable ones: §2 exit codes, §8 `config path/show`, §10 `version --json`
envelope + supported_schemas, §14 `--help --json`, §15/§16 `skill list/install/print`,
§18 `doctor`. So the automated review under-reports and can't be trusted as the gap source;
I had to fall back to a manual audit + hand-run each binary (`issuectl version --json`,
`crmctl show missing; echo $?`, `<tool> config --help`, etc.) to find the real gaps.

Expected / feature: `review` is already advisory and read-only (it never mutates the target),
so for a built/installed CLI it could **execute the binary** to auto-confirm the
runtime-observable canon checks rather than punting them to manual-verify:
- presence/shape of `config path` / `config show --json` (§8)
- `version --json` envelope: `schema_version`, `supported_schemas`, `skills[]` (§10/§17)
- exit-code mapping: user-error=1 vs usage/operational=2 (§2) via a couple of probe invocations
- `skill list/install/print` (§15/§16) and `doctor` (§18) subcommand presence + `--json` shape
- `--help --json` structured payload (§14)

Suggest an opt-in flag (e.g. `--run <path-to-binary>` or auto-detect a built target) that
turns these manual-verify rows into auto pass/gap, so `review` becomes a reliable
one-command gap source. Keep `--assume-defaults` (static-only) as the safe default for
un-built repos. Context: used during homebase `stack-cli-alignment` phase-1 rollout
(2026-08-16); the manual-audit fallback lives in homebase `issues/cli-alignment-audit/analysis.md`.
