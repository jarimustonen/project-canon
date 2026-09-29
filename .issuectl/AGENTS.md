# Issuectl policy for project-canon

What an agent should know before writing to this repository's issue tracker
under `issues/`. issuectl wrote the first version of this file when the
repository was scaffolded and still owns the block between the
`issuectl-managed` sentinels at the bottom: `issuectl doctor --fix`
regenerates that block from `issues/.schema.yaml` and
`.issuectl/transitions.yaml` and leaves the prose alone. Nothing refreshes the
prose when the tool changes, and `issuectl agents init --force` is not a
refresh either; it replaces the prose with the stock template. So this file
holds what this repository's configuration means and leaves verbs and flags to
sources that move with the tool.

## Where to read

`issuectl <command> --help` is current for any flag, and `issuectl dag --help`
also carries the lane-design guidance. The `/issue` skill in
`.claude/skills/issue/SKILL.md` describes the workflows and the `--json`
contract. It is generated per repository by `issuectl skill install` and
stamped with the issuectl version it was installed for, so when it and the
binary disagree, the binary is right and the skill is due a reinstall rather
than an edit. `issuectl context <slug>` renders one issue with its epic,
references, commits, and the schema rules in a single read.

The top-level `AGENTS.md` holds the repository's policy on what the tracker is
for: issues are where design lives, a feature gets an issue before it is
built, plans and analyses go under the issue's directory, and a worktree
worker appends its design decisions and rejected alternatives to the issue
before merging because the run report does not survive the run. The generated
block below is the field reference for the schema: which declared frontmatter
fields exist, which are required, which values they accept, and the transition
graph in force. `lane_seq` and `commits` are built into issuectl rather than
declared, so they are absent from the block and written through `update`
(`--lane-seq`, `--add-commit`). After any change to `issues/.schema.yaml` or
`.issuectl/transitions.yaml`, run `issuectl doctor --fix` and commit the block
together with the change so the two do not diverge.

## Why frontmatter is written through issuectl

Several sessions write to `issues/` at once here: worktree workers, the
orchestrator running a stint, and intake filings from agents in other
repositories. The mutation verbs (`update`, `set`, `apply`, `note`, `check`,
`close`, `label`, `depend`, `intake …`) take the repository write lock,
validate against the schema, keep the `updated:` and `closed:` stamps and the
canonical key order, and return a version token that a later write can pass
as `--expected-version` so it fails instead of overwriting what another
session wrote in between. A hand edit to frontmatter skips all of that. It
passes silently at the time and shows up later as a value the schema rejects,
a query or DAG view that no longer matches, or a merge conflict in `item.md`.
The body below the frontmatter is ordinary markdown and can be edited
directly; `note`, `check`, and `close --comment` exist because they put
comments, decisions, agent runs, checklist toggles, and resolutions in the
sections that `issuectl context` and later readers look in.

## What the transition rules mean

This repository declares an `allowed_from` graph in
`.issuectl/transitions.yaml`, summarised in the block below, and the comments
in that file give the reasons. Three intentions are behind it. An issue in
flight never falls back to `untriaged`, because that would silently un-work
it. Work does not start from `untriaged` or `deferred`; an intake item is
accepted into `open` first. And the completion statuses `done` and `fixed`
are reachable only from the development flow, so an intake or parked item is
dispositioned as `wontfix`, `duplicate`, `obsolete`, or `cannot-reproduce`
rather than quietly marked finished. Independently of this file, issuectl
refuses `fixed` and `cannot-reproduce` for anything but a bug and `done` for a
bug; `close` picks the right default by type. A refused move comes back as
`Error: transition: …`, or under `--json` as the error code
`transition-illegal`, with the legal path spelled out. That message is the
rule talking, and the answer is a different move, not a retry.

## No backlog, and where scheduling lives

An open non-epic issue here is either in a lane or newly arrived and waiting
for a decision. `lane`, `lane_seq`, `blocked_by`, and `collision` are the
execution plan: `issuectl dag` derives each lane's head-of-line from them,
`/stint-start` spawns only what the DAG reports spawnable, and nothing else
sweeps the tracker. That is why `/wrap-up` and `/triage-unlaned-issues` keep
re-presenting anything outside the DAG, and why `deferred` is not a resting
place: something worth doing later is laned and blocked, and something not
worth doing is closed with the reason recorded. The hot-file clusters named in
the top-level `AGENTS.md` are what `collision:` tokens are for.

Labels classify content. Lifecycle and scheduling are never encoded in a
label, because a `deferred` or `blocked` label falls outside every status
query and every DAG computation, which is exactly how work went unseen under
the label-based model the intake statuses replaced.

Intake here is fed by other agents: the `untriaged` items carry `provenance`
and `source_ref` from a `/wrap-up` or review run in another repository, and
outside contributors use GitHub Issues because issuectl is committer-only
(`CONTRIBUTING.md`). The triage skills brief Jari with a lane-or-close
recommendation per item instead of deciding, which tells you where the
decision to accept, defer, or dispose of an intake item sits.
`issuectl intake --help` lists the moves.

## Commits and release notes

A commit that resolves or relates to an issue carries a
`Fixes-Issue: @<slug>` or `Refs-Issue: @<slug>` trailer. In this repository
the trailer is more than bookkeeping: `OSS-RELEASE.md` sets the changelog
source to issuectl trailers, so the release cut compiles the notes by running
`issuectl changelog` over the commits in the release range. A commit without a
trailer is absent from the release notes, and so is a commit recorded only on
the issue with `--commit` or `--add-commit`, because `changelog` reads git
history rather than the issue. `issuectl sync-commits` goes the other way and
records trailered commits on their issues, and `close --stamp` appends the
`Fixes-Issue` trailer to HEAD when the fix is committed but not yet pushed. A
hand-written fragment under `changelog/fragments/` is for the case where the
wording of an entry matters.

## Closing

A closure is a durable decision, and `issuectl context` and whoever reopens
the question reconstruct its history from the issue alone. `close --comment`
records the reason under `## Resolution` in one step, and a `wontfix`,
`duplicate`, or `obsolete` closure says what would reopen it. `--as` records
who made the call, which `close` otherwise leaves blank. No issue type here
declares required body sections, so there is nothing structural to check
before closing beyond that record. `issuectl doctor` without `--fix` is a
read-only report over the whole tracker, useful after a schema edit or a
migration rather than per closure.

The repository is public, so issue text is public. The test the top-level
`AGENTS.md` applies to published artifacts, whose environment a fact
describes, is worth applying to issue titles and bodies as well.
`create --slug-random` keeps a title's words out of the slug only; the title
itself still ships in `item.md` and in `issuectl changelog` output, so a title
that would leak is reworded instead.

<!-- issuectl-managed:start -->

<!-- issuectl-managed:format=1 -->

## Schema-derived rules (generated)

_Regenerated by `issuectl doctor --fix`. Do not hand-edit between the sentinels._

### Frontmatter fields

- `assessment_classification` (optional, scalar)
- `assessment_outcome` (optional, scalar)
- `assignee` (optional, scalar)
- `blocked_by` (optional, list)
- `closed` (optional, scalar)
- `closed_by` (optional, scalar)
- `collision` (optional, list)
- `created` (optional, scalar)
- `deferred_until` (optional, scalar)
- `disposition_note` (optional, scalar)
- `disposition_reason` (optional, scalar) — allowed: by-design, out-of-scope, wontfix, withdrawn, superseded
- `duplicate_of` (optional, scalar)
- `epic` (optional, scalar)
- `labels` (optional, list)
- `lane` (optional, scalar)
- `originating_run` (optional, scalar)
- `originating_run_kind` (optional, scalar)
- `owner` (optional, scalar)
- `priority` (required, scalar) — allowed: low, normal, high
- `provenance` (optional, scalar)
- `provenance_detail` (optional, scalar)
- `related` (optional, list)
- `reporter` (optional, scalar)
- `review_confidence` (optional, scalar)
- `review_severity` (optional, scalar)
- `review_source` (optional, scalar)
- `review_status` (optional, scalar) — allowed: requested, in-review, approved, changes-requested
- `review_target` (optional, scalar)
- `reviewer` (optional, scalar)
- `size` (optional, scalar) — allowed: S, M, L, XL
- `slug` (optional, scalar)
- `source_ref` (optional, scalar)
- `status` (required, scalar) — allowed: open, in-progress, testing, untriaged, deferred, needs-info, done, fixed, wontfix, duplicate, cannot-reproduce, obsolete
- `type` (required, scalar) — allowed: bug, task, feature, improvement, chore, epic
- `updated` (optional, scalar)

### Required body sections by issue type

_No per-type body-section requirements declared._

### Status-transition rules

- **→ cannot-reproduce**
  - allowed from: untriaged, needs-info, deferred, open
- **→ deferred**
  - allowed from: untriaged, needs-info, open
- **→ done**
  - allowed from: open, in-progress, testing
- **→ duplicate**
  - allowed from: untriaged, needs-info, deferred, open
- **→ fixed**
  - allowed from: open, in-progress, testing
- **→ in-progress**
  - allowed from: open, testing
- **→ needs-info**
  - allowed from: untriaged, deferred, open
- **→ obsolete**
  - allowed from: untriaged, needs-info, deferred, open
- **→ testing**
  - allowed from: in-progress
- **→ untriaged**
  - allowed from: needs-info, fixed, done, wontfix, duplicate, cannot-reproduce, obsolete
- **→ wontfix**
  - allowed from: untriaged, needs-info, deferred, open, in-progress, testing

<!-- issuectl-managed:end -->
