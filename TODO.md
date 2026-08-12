# TODO

Pointers to open issues. Descriptions and plans live in the linked
`issues/<slug>/item.md` — do not duplicate them here.

## 🔄 Continue here (handoff)

_Repo bootstrapped 2026-08-12 from homebase **ADR 0009** (`project-canon` = project/repo-scoped
conformance tool; base project canon + per-archetype profiles, AI-first CLI canon as the `cli`
profile). Resume: `jatketaan @TODO.md`. Live scheduling is `issuectl dag`._

_**Last round (2026-08-12): `extract-canon-and-skill` landed and is closed `done`.** commits
`cef5c90` (extract) + `9770a1e` (close). project-canon is now the **declared maintained home** of
`AGENTS-AI-FIRST-CLI.md` (§1–§22 preserved verbatim; only a provenance/maintenance note added) and
the `cli-canon` skill (lifted to `skills/cli-canon/` + `skills/AGENTS.md`); root `AGENTS.md`
provenance flipped to "homebase copies FROM here". **No deploy** (repo policy: distributable CLI,
Phase 3 skipped). New follow-up filed: **`homebase-canon-cutover`** — the homebase SIDE is not yet
switched over (homebase still holds its own masters); that cutover requires editing the **homebase
repo**, so it's tracked here but executed there (homebase stint). Still **no Rust code** in this
repo._

_**Next head: `profile-and-base-canon-model`** (critical path — `doctor`/`new`/`review` all wait on
it). `env-config-hook-layer` is also unblocked and can run alongside once code lands and modules are
provably disjoint (still one serial `build` lane for now). `homebase-canon-cutover` is ready but
executed in homebase, not here._

**v0 scope discipline (ADR 0009 §6): a LIFT, not a greenfield canon.** Author only the `cli`
profile (it already exists as §1–§22 + the `cli-canon` skill); seed the base canon from what
homebase's `create-project` already scaffolds + the repo-general canon sections (§10, §15–§17,
§22) + already-discovered dims; ship `new`/`doctor`/`review` end-to-end for the `cli` profile;
externalize homebase env specifics to a config/hook layer from commit one; leave
`service`/`library`/`release` profiles as named-but-empty extension points.

## Execution DAG (2026-08-12, updated)

Scheduling PLAN — source of truth for lane + order; issuectl is authoritative for STATUS
(never copied here). Live scheduling is `issuectl dag` (frontmatter `lane:` + `blocked_by`);
this block is a hand-maintained snapshot that the `/stint-*` skills parse. Merge each round
(drop landed, add active, keep existing order). `▶` = head-of-line snapshot — RE-COMPUTE from
`issuectl dag` at pick time. `after <slug> (needs …)` = logical `blocked_by` mirror.
`collision: <file>` = touches a second lane's hot file (spawn-time exclusion).

<!-- execution-dag:begin -->
```
GLOBAL HEAD-OF-LINE: profile-and-base-canon-model   ← start here on resume
LANE build — the project-canon binary + canon/profile files (epic: project-canon-v0)
  # one serial lane at v0: no code exists yet, so hot files aren't known and the verbs share
  # the profile-registry/engine substrate. doctor/new/review MAY split into parallel lanes
  # once code lands and their modules are provably disjoint — re-assess then.
  # DONE (dropped from lane): extract-canon-and-skill — landed 2026-08-12, closed done.
  ▶ profile-and-base-canon-model   (extract-canon-and-skill delivered → unblocked)
    env-config-hook-layer          (extract-canon-and-skill delivered → unblocked; run alone until modules disjoint)
    doctor-conformance-gate        after profile-and-base-canon-model (needs the profile model)
    new-scaffold-generator         after profile-and-base-canon-model (needs the profile model)
    review-audit-verb              after profile-and-base-canon-model (needs the profile model)
UNLANED — confirmed no project-canon hot files (executed in a DIFFERENT repo):
    homebase-canon-cutover         after extract-canon-and-skill (delivered) — EXECUTED IN HOMEBASE repo, tracked here
```
<!-- execution-dag:end -->

### Adjacent backlog (active but not scheduled this round)

_(none)_
