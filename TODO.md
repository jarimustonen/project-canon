# TODO

Pointers to open issues. Descriptions and plans live in the linked
`issues/<slug>/item.md` — do not duplicate them here.

## 🔄 Continue here (handoff)

_Repo bootstrapped 2026-08-12 from homebase **ADR 0009** (`project-canon` = project/repo-scoped
conformance tool; base project canon + per-archetype profiles, AI-first CLI canon as the `cli`
profile). Resume with `/skill:stint-start`. Live scheduling is `issuectl dag`; TODO.md carries only handoff notes._

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

_**🚀 2026-08-19 — LATEST STATE. `0.6.0` is public and live on every channel**: crates.io
(`project-canon-core@0.6.0` + `project-canon-cli@0.6.0`), the GitHub Release with cargo-dist
binaries, and Homebrew `project-canon` at `0.6.0`. Workspace manifest matches. `0.6.0` shipped the
complete `cli-canon` behavioral skill alongside `ai-first-cli-canon` (all three probe/generation/
review templates; Claude and pi get native skill trees, Codex gets the resources embedded in one
deterministic prompt) plus the first-class pi layout under `--agent all`. Intake labels have since
been migrated to the issuectl lifecycle (`doctor --fix`)._

_**Canon is at v4, §1–§24.** `0.4.0` and `0.5.0` (2026-08-17) closed the canon-conformance arc —
see the two sections below. The `canon-rollout` lane is **empty**; every unit in it landed._

_**Direction from here.** v0's scope is complete and released, so the open question is no longer
"finish v0" but "how far does the family actually adopt this". The live thread is release-surface
correctness: this repo's own contract under-declares what it publishes, and that work is entangled
with a HIGH engine bug in `ossctl` (below). Beyond that, the natural next block is the sibling-CLI
§10 rollout, which is **deliberately unscheduled** — see the decision note below._

_**⚠️ Needs scheduling triage:** one active issue is unscheduled — `contract-declare-release-surface`
(the contract omits the Homebrew target). It is **blocked in practice** on the `ossctl` gh-releases
verify bug and says so in its own body; read that before touching the contract._

_**🔒 2026-08-16 (session 2). Two things happened: a 4-unit canon-conformance
round, and a publicness defect found + fixed + turned into a rule.**_

_**Round: `project-canon` now conforms to the canon it publishes.** Four units, strictly serial (core
model + `main.rs` are one lane), each green-gated with `/llm-review` + `/assess-findings`:
**§2/§10** central JSON error envelope + family exit-code map (new
`crates/project-canon-cli/src/error.rs`; all verbs rerouted; clap usage failures centrally remapped —
`--json --version` now exits **1** with an envelope, was prose+2) → **§10** `version --json` (schema
version, build provenance, `supported_schemas`, `supported_profiles`, `supported_surfaces`,
bundled skills) → **§14** `--help --json` for **every** command path, derived from the clap tree so it
can't drift → **§8** `config path` + `config show --json` with per-value provenance. `review
--assume-defaults .` on this repo now reports **0 confirmed gaps**; `doctor` mechanically conformant.
**238 tests green.** Note: the `cli-canon` lane duplicated `canon-rollout` (two audits filed the same
§8/§14 work under different slugs) — collapsed into one lane, each unit closed both twins._

_**🔒 PUBLICNESS DEFECT: found, fixed, made a RULE.** `0.1.1`/`0.2.0` shipped **one user's
environment as built-in defaults** to crates.io: a maintainer account, a personal repository-root
convention, and a family-tool list naming **three private repos**. Fixed in
**`portable-neutral-defaults`**: `gh_account`/`repo_root` are now
`Option<String>` and `None` by default, `family_tools` defaults empty, `tw` off; the environment
comes from `~/.config/project-canon/config.toml` **outside** the repo (`config show --json` proves it
— every value reports `"source": "file"`). `crates/` greps clean for every user-specific token.
**The lesson, now RULE #1 in `AGENTS.md` § Operating policy: overridability does NOT launder a
user-specific default — an unset default is still whatever ships in the package.** That exact
reasoning ("every value is overridable, so it's portable") is what produced the defect and it was
sitting in `env.rs`'s doc comment. **Carve-out (learned the hard way, see the note on
`canon-no-user-specifics`): the repo's OWN coordinates are not a leak** — its GitHub URL, tap, CI
badge, README install line all name the owner and are correct. A check that flags those gets
disabled. The test is *whose environment the fact describes*._

_**🚀 v0.3.0 IS PUBLIC** (crates.io `project-canon-core@0.3.0` + `project-canon-cli@0.3.0`, Homebrew
`0.2.0 → 0.3.0`, tag `v0.3.0`, cargo-dist). **0.3.0 not 0.2.1** — `EnvConfig::gh_account`/`repo_root`
`String`→`Option<String>` is breaking, and zerover puts breaking in the minor. **DECIDED (owner,
2026-08-16): 0.1.1/0.2.0 are NOT yanked** — crates.io retains published files permanently, so a yank
would break installs without removing the names; and the names are in this public repo's git history
regardless. Rationale is recorded in CHANGELOG 0.3.0 so it doesn't later read as an oversight._

_**Operating-policy change:** the agent now **owns the release DECISION**, not just its execution —
the old "owner-gated OSS cut" line contradicted the 2026-08-05 auto-release rule and made an agent
stop to ask. Do **not** surface "shall we publish?" as a decision item._

_**⚠️ Two things that need a human, both UNPUSHED (deliberate — cross-repo push is the owner's
call):** (1) **`orchestratectl`** issue **`spinoff-report-fields-null`** (priority high, commit
`defc7a1`) — **all 5** spinoffs this session submitted **null** `summary`/`discussion_items`/
`spinoff_proposals`/`wrap_up_recommendations`, while `run wait` returned a rich accurate summary for
the same runs. So the report is captured but not persisted into `nodes/<node>.json`. Every round fact
in this handoff was reconstructed from `git log` — the *reasoning* is lost. Don't waste effort
re-prompting workers; it's a tooling bug. **STILL RECURRING as of 2026-08-17** (2 of 4 workers that
round), which is why the recommended practice above routes reasoning into issue comments instead. (2) **Four `audit-no-user-specifics` issues** filed in
`issuectl`, `orchestratectl`, `ossctl`, `glasspad` (the public siblings) so the leak class gets swept
family-wide; `aggountant` is private so the rule doesn't bite. Each says: **close as clean if the
audit finds nothing** — a recorded clean result is the point._

_**[SUPERSEDED 2026-08-17 — see the release-infra note above. The Homebrew leg is now automatic.]**
~~Homebrew leg is STILL manual — `ossctl release plan` seals only the 2 crates.io targets, so the
tap formula (sha256 + push) was hand-updated twice today. Teaching cargo-dist to push the formula
in CI is the highest-value release-infra follow-up.~~_

_**🚀 2026-08-16 — FIRST RELEASE CUT: v0.1.1 is PUBLIC.** `project-canon` is released and the repo
is now **public** (`gh repo … --visibility public`). Live channels: **crates.io** —
`project-canon-core@0.1.1` + `project-canon-cli@0.1.1` (`cargo install project-canon-cli`);
**Homebrew** — `brew install jarimustonen/project-canon/project-canon` (source-build formula in the
new `jarimustonen/homebrew-project-canon` tap, sibling convention — `depends_on "rust"`, `cargo install
--path crates/project-canon-cli`), **verified working**; **tag** `v0.1.1` pushed → cargo-dist Actions
`release.yml` builds the shell installer + prebuilt binaries (macOS arm64/x86_64, Linux musl
arm64/x86_64) + the GitHub Release. Whole OSS face landed this session on `main`: `OSS-RELEASE.md`
(approved: mvp, MIT, crates.io + cargo-dist/Homebrew, zerover), CI (`ci.yml` fmt/clippy/test +
Dependabot), README + LICENSE (MIT, holder **Jari Mustonen**), CHANGELOG (0.1.1 finalized),
`dist-workspace.toml` + `release.yml`. Engine: `ossctl` 0.2.2 via the `/oss-*` family._

_**⚠️ Release incident + architecture change (2026-08-16).** Two `ossctl release cut` attempts
mis-published before the manual finish: (a) the engine publishes the **manifest** version, not
`--version` — so `project-canon-core@0.0.0` got published before the workspace was bumped
(**OWNER TODO: yank `core@0.0.0`** on crates.io via the web UI — the local token lacks yank scope;
optionally yank the superseded `0.1.0`); (b) the CLI crate was **not crates.io-publishable** — it
`include_str!`'d the repo-root canon from OUTSIDE the crate. **Fixed & closed** (`cli-canon-embed-packaging`):
the canon master `AGENTS-AI-FIRST-CLI.md` now physically lives in `crates/project-canon-core/`, exposed
as **`project_canon_core::CANON`**; the **repo-root path is now a SYMLINK** to it (single-source,
byte-identical); the CLI's `new`/`skill` verbs consume `project_canon_core::CANON`. This forced the
coordinated bump to **0.1.1** (core@0.1.0 was already public without the CANON API), cli pinned
`core = "=0.1.1"`. **Two ossctl engine bugs filed in the ossctl repo:** `release-cut-ignores-version`,
`release-resume-unimplemented`._

_**`homebase-canon-cutover` DONE 2026-08-16.** After the first release, homebase and the selected active repo set switched to consume the canon/tool FROM here: install the `ai-first-cli-canon` skill from project-canon, not copy the markdown. **Two deferred follow-ups** (noted, NOT filed): (1) wire `new` to optionally auto-install the `ai-first-cli-canon` skill on scaffold; (2) a top-level `version --json` `skills:` audit surface. **Also worth a follow-up:** teach cargo-dist to push the Homebrew formula in CI (today the formula is maintained by hand in the tap) once the ossctl `dist` phase is trusted._

_**🔒 2026-08-17 (round A) — the two publicness/verification rules became CANON + machinery. Shipped
as `0.4.0`.** Both follow the same shape: a normative section, a mechanical `doctor` gate for the
detectable subset, and a `review` judgment row for the remainder._

_**§23 — public artifacts must not embed user-specific facts.** Scoped deliberately to *publicly
distributed* artifacts (an internal-only tool may encode its own org's policy; a published package
must never make a recipient inherit the maintainer's environment), and routed through the BASE layer
so every profile inherits it. The `doctor` gate is driven by `user_specific_deny_list` (env override
`PROJECT_CANON_USER_SPECIFIC_DENY_LIST`), exact + case-insensitive, **no username-shape heuristic**,
built-in list **empty** — the markers live in user config outside the artifact. It derives the
target's own owner/repo from the git remote or Cargo metadata and **exempts that project's own
GitHub/badge/Homebrew/install coordinates**, while still flagging a different private project name
under the same owner. **Known design property:** an unconfigured deny-list is **non-gating** — it
reports the key to set rather than failing. Defensible (a check that fails every fresh repo gets
disabled) but it means the mechanical half is opt-in per operator._

_**§24 — a stated blocker is re-verified, not inherited.** The worker proved the rule on this repo:
`dist-workspace.toml` justified a gap by naming an `ossctl` owning issue — which turned out to
**exist but be closed**, for work already enabled. The comment is now a dated, verified statement.
`doctor` rejects deferral justifications whose local owning issue is missing/closed and **fails
closed** on cross-repo owners it cannot verify (no network lookup; recognized slugs need an open
local mirror; a reasoned `canon:s24-allow` annotation is reserved for historical quotations).
**Consequence, and the reason `0.4.0` is a minor:** `doctor` may now fail repos it previously passed._

_**🔒 2026-08-17 (round B) — `review` verifies by DOING; `--version` fixed. Shipped as `0.5.0`.**_

_**`review --run <binary>`** executes a built CLI to auto-confirm runtime-observable sections. On
this repo it moved **16 manual/6 pass → 9 manual/14 pass**. Deliberately conservative and
**independently re-verified at handoff, not taken on the worker's word**: opt-in only
(`--assume-defaults` stays static and never executes); no shell; per-call timeout; read-only argv
(`skill list`/`print`, never `skill install`). Outcomes distinguish pass / gap / **`could-not-probe`**
— a missing binary, a non-executable file, and a script sleeping an hour all returned
`could-not-probe` (the hang bounded to ~4s), and a non-conforming binary correctly reported gaps.
**`could-not-probe` never collapses into pass or gap** — under-reporting was the original complaint._

_**`--version` is now a full alias of the `version` verb** — identical output and exit code in every
mode including `--json`, argument order irrelevant. **Canon §10 was amended in step**, dropping its
false rationale that the flag "cannot honor `--json`" (true of clap's built-in action, not of a tool
that dispatches the flag itself, as this one already did). The §10 **runtime probe** from round A
encoded the old rule and moved with the amendment, so the tool never contradicted itself._

_**📌 DECIDED (owner, 2026-08-19): the sibling-CLI §10 rollout is deliberately LEFT TO SIT.** The
amendment makes every other family CLI report a §10 gap by design; that is the alignment signal
working, nothing is broken in those tools, and `--version` still behaves as users expect there —
only the machine-readable spelling differs. **Not filed, on purpose.** The one real cost is noise:
if every sibling audit shows a standing §10 gap, audit output starts getting skimmed and a genuine
gap hides among the known ones. **When the rollout is actually scheduled, file one tracking issue
first** so a future reader sees a recorded decision rather than drift. Do not "fix" this by
softening the canon rule — enumerate each tool's gaps via its own runtime probe and lane them._

_**📌 RECOMMENDED PRACTICE (2026-08-19, from evidence): put worker reasoning in the ISSUE, not only
in the run report.** Of the four workers in the 2026-08-17 rounds, **exactly one** appended its
design decisions as an `issuectl` comment — and that is the **only** reasoning that is durable today
(in-repo, in git, next to the work). Two returned **empty `discussion_items`** despite the brief
explicitly asking for them, and a third populated the report richly but left nothing in-repo, so its
rationale exists only in the orchestration run store. Prompting harder does not fix this (it was
already prompted). **Therefore: every brief should require "append your design decisions and
rejected alternatives as an issue comment before merging", as standard as the green gate and the
merge call.** It is transport-independent and survives the `orchestratectl`
`spinoff-report-fields-null` bug entirely. Keep the structured report for orchestrator sequencing;
stop letting it be the only copy._

_**Release-infra state (good news, supersedes older notes below):** the **Homebrew leg is now fully
automatic** — cargo-dist publishes the formula in CI via `HOMEBREW_TAP_TOKEN` (configured
2026-08-15), verified across `0.4.0`/`0.5.0`/`0.6.0`. The long-standing "teach cargo-dist to push the
formula" follow-up is **DONE**; ignore the 2026-08-16 note below that calls it manual._

_**⚠️ `ossctl` gh-releases verify reports a SUCCESSFUL release as FAILED — filed, laned, being
worked.** `release cut` exits non-zero with `release_failed` on a release where everything landed.
**It is a lookup bug, not a timing race**, and that distinction is the point: for `0.5.0` the GitHub
Release existed for ~18 of the 20-minute polling window and was never observed, and a read-only
`release verify` run long afterwards **still** reports missing. Reproduces on `ossctl 0.9.0`. Filed
as **`verify-gh-release-missing`** in the `ossctl` repo (now laned `verify-seam`, with a later note
suggesting the fault is in `release/reconcile.rs`'s generic registry query rather than the tag
lookup). **Operational rule until fixed: trust the CHANNELS, not the engine's exit code** — check
crates.io, the GitHub Release, and the tap directly before believing a reported failure. The stale
`in_flight` journal entries this caused (`0.3.3`/`0.4.0`/`0.5.0`/`0.6.0`) have since been abandoned
with accurate reasons; `in_flight_count` is 0._

**v0 scope discipline (ADR 0009 §6): a LIFT, not a greenfield canon — ✅ COMPLETE, ✅ RELEASED (0.1.1).**
✅ `cli` profile (§1–§24 lift); ✅ base canon seeded; ✅ `service`/`library`/`release` named-but-empty
extension points; ✅ env specifics externalized; ✅ `new`/`doctor`/`review` + skill-install shipped;
✅ **first release cut & public**. **Path to adoption:** ✅ release → ✅ **`homebase-canon-cutover`** (homebase repo) → selected active repos switched over.

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
