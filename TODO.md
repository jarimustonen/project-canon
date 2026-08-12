# TODO

Pointers to open issues. Descriptions and plans live in the linked
`issues/<slug>/item.md` — do not duplicate them here.

## 🔄 Continue here (handoff)

_Repo bootstrapped 2026-08-12 from homebase **ADR 0009** (`project-canon` = project/repo-scoped
conformance tool; base project canon + per-archetype profiles, AI-first CLI canon as the `cli`
profile). **No code yet** — the v0 build backlog is filed as epic `project-canon-v0` + 6
children. Resume: `jatketaan @TODO.md`. Live scheduling is `issuectl dag`. Start at the ready
head **`extract-canon-and-skill`** (lift `AGENTS-AI-FIRST-CLI.md` + the `cli-canon` skill in
from homebase — homebase stays the canon source until this lands), then
`profile-and-base-canon-model` → {`doctor-conformance-gate`, `new-scaffold-generator`,
`review-audit-verb`}; `env-config-hook-layer` after `extract`._

**v0 scope discipline (ADR 0009 §6): a LIFT, not a greenfield canon.** Author only the `cli`
profile (it already exists as §1–§22 + the `cli-canon` skill); seed the base canon from what
homebase's `create-project` already scaffolds + the repo-general canon sections (§10, §15–§17,
§22) + already-discovered dims; ship `new`/`doctor`/`review` end-to-end for the `cli` profile;
externalize homebase env specifics to a config/hook layer from commit one; leave
`service`/`library`/`release` profiles as named-but-empty extension points.

## Execution DAG (2026-08-12)

Scheduling PLAN — source of truth for lane + order; issuectl is authoritative for STATUS
(never copied here). Live scheduling is `issuectl dag` (frontmatter `lane:` + `blocked_by`);
this block is a hand-maintained snapshot that the `/stint-*` skills parse. Merge each round
(drop landed, add active, keep existing order). `▶` = head-of-line snapshot — RE-COMPUTE from
`issuectl dag` at pick time. `after <slug> (needs …)` = logical `blocked_by` mirror.
`collision: <file>` = touches a second lane's hot file (spawn-time exclusion).

<!-- execution-dag:begin -->
```
GLOBAL HEAD-OF-LINE: extract-canon-and-skill   ← start here on resume
LANE build — the project-canon binary + canon/profile files (epic: project-canon-v0)
  # one serial lane at v0: no code exists yet, so hot files aren't known and the verbs share
  # the profile-registry/engine substrate. doctor/new/review MAY split into parallel lanes
  # once code lands and their modules are provably disjoint — re-assess then.
  ▶ extract-canon-and-skill
    profile-and-base-canon-model   after extract-canon-and-skill (needs canon+skill in-repo)
    env-config-hook-layer          after extract-canon-and-skill (needs the base to externalize)
    doctor-conformance-gate        after profile-and-base-canon-model (needs the profile model)
    new-scaffold-generator         after profile-and-base-canon-model (needs the profile model)
    review-audit-verb              after profile-and-base-canon-model (needs the profile model)
```
<!-- execution-dag:end -->

### Adjacent backlog (active but not scheduled this round)

_(none yet)_
