# TODO

Pointers to open issues. Descriptions and plans live in the linked
`issues/<slug>/item.md` — do not duplicate them here.

## 🔄 Continue here (handoff)

_Repo bootstrapped 2026-08-12 from homebase **ADR 0009** (`project-canon` = project/repo-scoped
conformance tool; base project canon + per-archetype profiles, AI-first CLI canon as the `cli`
profile). Resume: `jatketaan @TODO.md`. Live scheduling is `issuectl dag`._

_**Last round (2026-08-13): `profile-and-base-canon-model` landed and is closed `done`.** commits
`8f1fe63` (feat(core): base-canon + archetype-profile model) + `53f5816` (review FIXes) + `f52a1c0`
(close). This is the **first Rust code** in the repo: a `§22` workspace split —
`crates/project-canon-core` (the two-layer model) + `crates/project-canon-cli` (thin binary).
The model resolves `resolved(repo) = BASE CANON ∪ PROFILE[archetype]` additively; the **`cli`
profile lifts §1–§22** (base = §10/§15–§17/§22 + create-project scaffold dims; cli profile = the
other 17 sections), and `service`/`library`/`release` are **named-but-empty extension points**
(`is_empty()` contract, disjointness asserted). Design in
`issues/profile-and-base-canon-model/design.md`. **Green gate green on main** (build/test/clippy
-D warnings/fmt); **24 tests** (20 unit + 4 integration `resolution.rs`). Went through `/llm-review`
+ `/assess-findings` before merge. **No deploy** (repo policy: distributable CLI, Phase 3 skipped)._

_**Prior round (2026-08-12): `extract-canon-and-skill` landed `done`** (`cef5c90`+`9770a1e`) —
project-canon is the **declared maintained home** of `AGENTS-AI-FIRST-CLI.md` (§1–§22 verbatim) and
the `cli-canon` skill (`skills/cli-canon/`); root `AGENTS.md` provenance = "homebase copies FROM
here"._

_**Next head: `env-config-hook-layer`** — chosen first deliberately: externalize homebase-specific
paths/env into a config/hook layer **before** the three verbs are built, so `doctor`/`new`/`review`
all inherit a clean seam instead of each hardcoding defaults that must be torn open later. The model
left seams for exactly this (no homebase paths hardcoded). Then, in order:
**`doctor-conformance-gate`** (CI gate; smallest surface, closest to the model — just reads it and
emits a verdict, validates the keel first in real use) → **`new-scaffold-generator`** (generates a
repo from the model; builds on doctor's resolution) → **`review-audit-verb`** (advisory audit; last,
leans on doctor's gate logic). Still **one serial `build` lane** — the verbs may split into parallel
lanes only once their modules are provably disjoint (re-assess after each lands).
`homebase-canon-cutover` is ready but **executed in the homebase repo, not here**._

_**Rollout gate (owner decision 2026-08-13):** the cross-repo adoption — homebase + all other repos
switching to consume this canon/tool FROM here (`homebase-canon-cutover` and its siblings) — is
**gated on cutting project-canon's FIRST release**. Do NOT roll this out to other repos before then.
When that first release is cut, **tell the owner clearly and explicitly**; the go-wide across all
repos happens on his go at that point. Until the release + his go: edit the canon only here, and
leave the other repos' copies untouched (avoid drift)._

**v0 scope discipline (ADR 0009 §6): a LIFT, not a greenfield canon.** ✅ `cli` profile authored
(§1–§22 lift); ✅ base canon seeded (§10/§15–§17/§22 + create-project scaffold dims); ✅
`service`/`library`/`release` left as named-but-empty extension points. Remaining: ship
`new`/`doctor`/`review` end-to-end for the `cli` profile; externalize homebase env specifics to the
config/hook layer (`env-config-hook-layer`, next).

## Execution DAG (2026-08-14, updated)

Scheduling PLAN — source of truth for lane + order; issuectl is authoritative for STATUS
(never copied here). Live scheduling is `issuectl dag` (frontmatter `lane:` + `blocked_by`);
this block is a hand-maintained snapshot that the `/stint-*` skills parse. Merge each round
(drop landed, add active, keep existing order). `▶` = head-of-line snapshot — RE-COMPUTE from
`issuectl dag` at pick time. `after <slug> (needs …)` = logical `blocked_by` mirror.
`collision: <file>` = touches a second lane's hot file (spawn-time exclusion).

<!-- execution-dag:begin -->
```
GLOBAL HEAD-OF-LINE: doctor-conformance-gate   ← start here on resume
LANE build — the project-canon workspace: crates/project-canon-{core,cli} (epic: project-canon-v0)
  # one serial lane at v0: the model landed but the verbs share the core model/resolution
  # substrate (crates/project-canon-core). doctor/new/review MAY split into parallel lanes
  # once code lands and their modules are provably disjoint — re-assess after each lands.
  # DONE (dropped): extract-canon-and-skill (2026-08-12) · profile-and-base-canon-model (2026-08-13)
  #                 · env-config-hook-layer (2026-08-14, the shared config/hook seam the verbs inherit).
  ▶ doctor-conformance-gate        after profile-and-base-canon-model (delivered; needs the profile model) — smallest surface, reads the model → verdict
    new-scaffold-generator         after profile-and-base-canon-model (delivered; needs the profile model) — builds on doctor's resolution
    review-audit-verb              after profile-and-base-canon-model (delivered; needs the profile model) — leans on doctor's gate logic; last
UNLANED — confirmed no project-canon hot files (executed in a DIFFERENT repo):
    homebase-canon-cutover         after extract-canon-and-skill (delivered) — EXECUTED IN HOMEBASE repo, tracked here
```
<!-- execution-dag:end -->

### Adjacent backlog (active but not scheduled this round)

_(none)_
