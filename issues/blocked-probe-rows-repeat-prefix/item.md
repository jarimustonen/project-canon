---
created: 2026-11-27
updated: 2026-11-27
type: bug
status: fixed
priority: normal
provenance: other
provenance_detail: observed by maintainer while fixing probe tests
source_ref: taskfleet:01m8f62xq43zrrjp3k621ap6zg/blocked-probe-prefix
originating_run: 01m8f62xq43zrrjp3k621ap6zg
originating_run_kind: spinoff
closed: 2026-11-27
commits:
- hash: c1ab161
  summary: state the not-attempted prefix once on blocked probe rows
---

# Blocked runtime-probe rows repeat the not-attempted prefix

## Description

## Observed

When `project-canon review --runtime-probes` hits a target execution failure (for example the
binary cannot be started), every later runtime-probe row is reported as `could-not-probe`
without being attempted. The message of each such row repeats the prefix
"not attempted after target execution failure:" once per earlier blocked row, so the last row
(`canon.s18`) carries it many times before the original failure reason.

## Cause

`runtime_probes_with_timeout` stores each blocking outcome's whole message as the reason for
the next row, including the already-prefixed message of a blocked row.

## Expected

Every blocked row carries the prefix exactly once, followed by the original failure reason.
