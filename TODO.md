# TODO

Pointers to open issues. Descriptions and plans live in the linked
`issues/<slug>/item.md` — do not duplicate them here.

## 🔄 Continue here (handoff)

_**2026-09-02 — `0.7.0` is live and verified on every declared channel.** Shipshape run
`01M1GBZC3ZGAGMSCQ6THT6VAR8` completed all phases: dry-run, build, crates.io publication
(`project-canon-core` + `project-canon-cli`), `v0.7.0`, cargo-dist/GitHub Release, Homebrew,
registry verification, and fast-forwarding `main`. The installed Homebrew binary also reports
`project-canon 0.7.0`. The temporary, isolated `cargo-dist 0.28.2` prefix used to resume the cut
was removed; there are no in-flight release runs._

_**What shipped.** Canon §15's 1024-character Agent Skills description limit is now enforced
mechanically by `doctor` and static `review` for repository skills in supported roots. Validation
measures decoded YAML text, covers the exact boundary and bundled rendered resources, confines
reads to the target repository, and keeps static gaps visible when runtime probing is enabled.
Active release guidance and owned resources now consistently use Shipshape terminology._

_**Release recovery note.** The first cut stopped safely before publication after the dry-run
passed but the build phase could not find the pinned cargo-dist `dist` executable. The same sealed,
journaled run was resumed with a disposable version-locked prefix and completed; no second plan or
manual publication was used. An older stale `0.6.1` journal was abandoned only after confirming it
had long been superseded by verified `0.6.2` artifacts._

_**Direction from here.** The next accepted product direction is `three-agent-skill-install`:
Canon §15 should require every companion-skill installer to support Claude, pi, and Codex in their
native locations, with default/`all` covering all three. Project Canon itself already writes all
three layouts; the missing work is the normative cross-tool requirement and conformance evidence.
Use the live issue DAG for scheduling details. The full Rust green gate passed for `0.7.0` before
release._

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
