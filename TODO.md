# TODO

Pointers to open issues. Descriptions and plans live in the linked
`issues/<slug>/item.md` — do not duplicate them here.

## 🔄 Continue here (handoff)

_Repo bootstrapped 2026-08-12 from homebase **ADR 0009** (`project-canon` = project/repo-scoped
conformance tool; base project canon + per-archetype profiles, AI-first CLI canon as the `cli`
profile). Resume: `jatketaan @TODO.md`. Live scheduling is `issuectl dag`._

_**Session 2026-08-14 (five rounds, all landed `done`, green on main, no deploy — repo policy):**
`env-config-hook-layer` (`2d9f8ab`) → `doctor-conformance-gate` (`50f55e9`) →
`new-scaffold-generator` (`5f3fcfd`) → `review-audit-verb` (`12c91ed`) → `canon-installable-skill`
(`a8d69e3`), each with review FIXes + close commits. Prior: `profile-and-base-canon-model`
(2026-08-13, the two-layer model), `extract-canon-and-skill` (2026-08-12, project-canon = maintained
home of `AGENTS-AI-FIRST-CLI.md` + `cli-canon`). **Green gate green; 214 tests; clippy -D warnings +
fmt clean.**_

_**🎯 v0 is FEATURE-COMPLETE.** All three verbs shipped end-to-end for the `cli` profile —
**`doctor`** (mechanical CI gate, non-zero on MUST gap), **`new`** (generate-only scaffold; external
bootstrap steps rendered from the EnvConfig hook layer and PRINTED, never executed), **`review`**
(advisory audit; severity-triaged findings + staged/printed `issuectl` commands; never acts) — on
the two-layer model (`resolved = BASE ∪ PROFILE[cli]`) with the homebase env specifics externalized
to the `env` config/hook layer (defaults → file → env). **Plus** the distribution mechanism:
**`project-canon skill install|list|print`** installs the canon as the versioned, single-sourced
(`include_str!` of the master, no drifting copy) **`ai-first-cli-canon`** reference skill (Claude +
Codex). Design decisions in `issues/canon-installable-skill/design.md` — kept as **two** skills
(`ai-first-cli-canon` = content, `cli-canon` = behavior); `skill` meta-verb dogfoods canon §14–§17;
one recorded canon deviation (unknown-name in `print` exits **2** for binary-wide consistency, not
§16's literal 1 — owner may amend the canon)._

_**Next: cut the FIRST release.** There is NO local build-lane work left to schedule in this repo.
The only remaining tracked item, `homebase-canon-cutover`, is **executed in the homebase repo** and
is **release-gated** (below). So the immediate next action is the release itself — no tracked issue
by design; run the `/oss-*` family (`/oss-release`) when starting it. **When the first release is
cut, tell the owner clearly and explicitly** (rollout gate below). **Two deferred follow-ups**
(noted, NOT filed — owner has not asked to file): (1) wire `new` to optionally auto-install the
`ai-first-cli-canon` skill on scaffold; (2) a top-level `version --json` `skills:` audit surface
(§17 hook; `skill list --json` already exposes versions)._

_**Rollout gate (owner decision 2026-08-13):** the cross-repo adoption — homebase + all other repos
switching to consume this canon/tool FROM here (`homebase-canon-cutover` and its siblings) — is
**gated on cutting project-canon's FIRST release**. Do NOT roll this out to other repos before then.
When that first release is cut, **tell the owner clearly and explicitly**; the go-wide across all
repos happens on his go at that point. Until the release + his go: edit the canon only here, and
leave the other repos' copies untouched (avoid drift)._

**v0 scope discipline (ADR 0009 §6): a LIFT, not a greenfield canon — ✅ COMPLETE.** ✅ `cli`
profile authored (§1–§22 lift); ✅ base canon seeded (§10/§15–§17/§22 + create-project scaffold
dims); ✅ `service`/`library`/`release` left as named-but-empty extension points; ✅ homebase env
specifics externalized to the config/hook layer; ✅ `new`/`doctor`/`review` shipped end-to-end for
the `cli` profile; ✅ canon installable as a skill. **Remaining path to adoption:** cut the first
release → `homebase-canon-cutover` (homebase repo; now "install the canon skill", not "copy the
markdown") → go-wide across all repos, on the owner's go — all gated on the release.

## Execution DAG (2026-08-15, updated)

Scheduling PLAN — source of truth for lane + order; issuectl is authoritative for STATUS
(never copied here). Live scheduling is `issuectl dag` (frontmatter `lane:` + `blocked_by`);
this block is a hand-maintained snapshot that the `/stint-*` skills parse. Merge each round
(drop landed, add active, keep existing order). `▶` = head-of-line snapshot — RE-COMPUTE from
`issuectl dag` at pick time. `after <slug> (needs …)` = logical `blocked_by` mirror.
`collision: <file>` = touches a second lane's hot file (spawn-time exclusion).

<!-- execution-dag:begin -->
```
GLOBAL HEAD-OF-LINE: (none in-repo) → cut the FIRST release via /oss-* (no tracked issue by design)
LANE build — the project-canon workspace: crates/project-canon-{core,cli} (epic: project-canon-v0)
  # EMPTY — v0 is feature-complete; no local build-lane work remains to schedule.
  # DONE (dropped): extract-canon-and-skill (2026-08-12) · profile-and-base-canon-model (2026-08-13)
  #                 · env-config-hook-layer (2026-08-14, the shared config/hook seam the verbs inherit)
  #                 · doctor-conformance-gate (2026-08-14, the mechanical CI gate; reads the model → verdict)
  #                 · new-scaffold-generator (2026-08-14, generate-only scaffold; external steps printed via EnvConfig hooks)
  #                 · review-audit-verb (2026-08-14, advisory audit; severity-triaged findings + staged/printed commands; never acts)
  #                 · canon-installable-skill (2026-08-14, `skill install|list|print`; canon → versioned single-sourced ai-first-cli-canon skill).
  #   🎯 v0 FEATURE-COMPLETE: doctor + new + review + skill-install all landed for the cli profile (214 tests).
  # WONTFIX (owner call 2026-08-14, dropped): osstring-argv-env (one-in-a-blue-moon non-UTF-8 argv/env)
  #                 · typed-dimension-id (already guarded by every_mechanical_probe_id_exists_in_the_model test).
  # NEXT (no issue by design): cut the first release with /oss-release; when cut, tell the owner explicitly (rollout gate).
UNLANED — confirmed no project-canon hot files (executed in a DIFFERENT repo, release-gated):
    homebase-canon-cutover         after FIRST RELEASE (gated) — EXECUTED IN HOMEBASE repo; now "install the ai-first-cli-canon skill from project-canon", not "copy the markdown"
```
<!-- execution-dag:end -->

### Adjacent backlog (active but not scheduled this round)

_(none)_
