# TODO

Pointers to open issues. Descriptions and plans live in the linked
`issues/<slug>/item.md` — do not duplicate them here.

## 🔄 Continue here (handoff)

_**2026-09-08 — Taskfleet reference convergence is complete.** Project Canon `0.8.2` is the
current published release; its GitHub release and the installed `project-canon 0.8.2` binary were
verified. That release already ships the canonical CLI-canon and generated catalog wording with
the Taskfleet product name._

_**What landed after the release.** `taskfleet-project-canon-reference-convergence` refreshed the
repo-local Claude and Codex Issuectl dogfood artifacts byte-for-byte from released Issuectl
`0.18.2`, so their analysis-worker prerequisite now identifies Taskfleet. A focused integrity test
was corrected to preserve stable `OCTL_*` protocol identifiers rather than treating them as retired
product branding. Multi-model review and assessment completed, and the full Rust green gate passed:
fmt, clippy with warnings denied, workspace tests, workspace build, and rustdoc with warnings
denied. These operational dogfood and test changes require no additional Project Canon release.
The issue is closed and no Project Canon worker or release run remains active._

_**Direction from here.** There is no accepted follow-up agenda prepared. Use the live issue DAG
for scheduling state and perform a fresh planning pass before starting new product work._

## Scheduling

Canonical scheduling lives in `issuectl` frontmatter (`lane:`, `lane_seq:`, `blocked_by:`, `collision:`). Do not maintain a markdown DAG or adjacent backlog in this file.

Use these views instead:

```bash
issuectl dag
issuectl dag --json
issuectl ls --status open
issuectl ls --status in-progress
```

`TODO.md` is only the session handoff and project notes; issue bodies and `issuectl dag` are the source of truth.
