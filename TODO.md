# TODO

Pointers to open issues. Descriptions and plans live in the linked
`issues/<slug>/item.md` — do not duplicate them here.

## 🔄 Continue here (handoff)

_**2026-09-06 — `0.8.1` is live and verified on every declared channel.** Shipshape run
`01M1VKGRGG2XXCAPYYWQ264ECH` completed dry-run, build, crates.io publication
(`project-canon-core` + `project-canon-cli`), `v0.8.1`, cargo-dist/GitHub Release, Homebrew,
registry verification, and fast-forwarding `main`. The installed binary also reports
`project-canon 0.8.1`. The isolated, version-locked cargo-dist prefix used for the cut was removed,
and no Project Canon worker or release run remains active._

_**What shipped.** Canon §15 and Project Canon now require companion-skill installers to support
Claude, pi, and Codex as native Agent Skills trees, with default/`all` covering all three. The
initial `0.8.0` implementation incorrectly treated Codex custom prompts as its native skill form;
`0.8.1` corrects the destination to `.codex/skills/<name>/...`, preserves complete resource trees,
and makes runtime conformance reject prompt-only Codex distribution. Installation migrates only
positively identified Project Canon-managed legacy prompts and preserves foreign, malformed,
symlinked, or newer artifacts. The full Rust green gate passed after process-heavy runtime tests
were isolated from one another to keep their bounded deadlines deterministic under concurrent
test execution._

_**Direction from here.** The next accepted item is
`taskfleet-project-canon-reference-convergence`: update active distributed and dogfooded guidance
to Taskfleet while preserving historical and compatibility references. Do not begin that work
until its stated upstream preconditions are verified: the canonical issue-intake template must be
released and the canonical `taskfleet` intake key must be accepted. Use the live issue DAG for all
scheduling state; this narrative records intent only._

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
