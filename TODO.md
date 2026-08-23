# TODO

Pointers to open issues. Descriptions and plans live in the linked
`issues/<slug>/item.md` — do not duplicate them here.

## 🔄 Continue here (handoff)

_Repo bootstrapped 2026-08-12 from homebase **ADR 0009** (`project-canon` = project/repo-scoped
conformance tool; base project canon + per-archetype profiles, AI-first CLI canon as the `cli`
profile). Resume with `/skill:stint-start`. Live scheduling is `issuectl dag`; TODO.md carries only handoff notes._

_**🚀 2026-08-23 — LATEST STATE. `0.6.2` is live and verified on all four channels** (crates.io ×2,
GitHub Release, Homebrew; engine `verify` phase: all targets `matches`; release commit `4693463`
fast-forwarded to main by hand — see the ossctl bug below). This supersedes the "backlog empty"
note further down: **one open, unscheduled issue now exists** (`skill-description-length-check`,
awaiting human lane-or-close triage — context only, not scheduled or accepted)._

_**🌍 2026-08-22/23 session — the PUBLICIZE pass: stealth-public → external-user-facing.** Interactive
session (no worktrees). (1) **README rewritten for an external audience** — value-prop lead, Why,
worked quickstart from a real `doctor` run, canon-highlights table, Status (ZeroVer). Every claim
fact-checked against the binary + dist config; **fixed a real error: the README promised macOS
x86_64 prebuilt binaries that are not in the dist target list** (coverage is macOS arm64 + Linux
musl arm64/x86_64). (2) **AGENTS.md re-grounded** — it still said "bootstrap only, verbs not built"
four releases after they shipped. (3) **GitHub metadata set** (description + six topics, was empty).
(4) **CONTRIBUTING.md + PR template** landed; issue-channel split is explicit (externals → GitHub
issues; the in-repo issuectl tracker is committer-only). **CODE_OF_CONDUCT.md was added then
REMOVED (owner decision — do not re-add).** GitHub issue forms deliberately skipped (issuectl is
canonical). (5) **SECURITY.md** — threat gate fired (subprocess probes, untrusted probe output,
shipped binaries) → full mvp-scale policy; **Private Vulnerability Reporting enabled** in repo
settings on owner's go. (6) **Canon links now target the physical master**
(`crates/project-canon-core/AGENTS-AI-FIRST-CLI.md`) because GitHub's web UI renders a symlink as
its target path, not content. (7) **Product-name neutrality (owner rule: no agent product names —
pi.dev is used too):** canon §15 now defines skills by the open **Agent Skills** standard
(agentskills.io); help text + docs use `--agent` layout ids (`claude`/`pi`/`codex`). **Canon §15
also gained the format limit: skill frontmatter `description` ≤ 1024 chars** (both bundled skills
measured well under: 390/867). All shipped in `0.6.2` (terminology/canon content, no behaviour
change)._

_**⚠️ Two NEW ossctl engine bugs found by the `0.6.2` cut, both filed in ossctl (and the changelog
one already has a worker running there):** (1) **`changelog-finalize-markers` (high)** — finalize
put the dated `## [0.6.2]` header INSIDE the Unreleased marker block, compiled neither the pending
fragment nor issue trailers (empty sections), left the fragment unconsumed, and the broken block
propagated into the public cargo-dist GitHub release body. Repaired here by hand: CHANGELOG fixed
(`19f3bf8`, fragment consumed) + `gh release edit`. (2) **`cut-main-not-advanced` (normal)** — the
engine tagged the bump commit and published everything but never advanced main, leaving origin/main
at 0.6.1 while 0.6.2 was live (next `plan --bump patch` would have collided); fixed by ff-merge +
push. Both are also evidence for post-cut mechanical checks (see the skill thread below)._

_**📌 De-stealth → skill thread: collection issue `oss-publicize-skill` filed in ossctl.** The whole
publicize pass + the five gaps it exposed in the /oss-* family (metadata apply, PVR check,
claims-vs-binary audit, symlink-link check, neutrality sweep) are recorded there, with an A/B/C
design question (own member vs. /oss-release mode vs. checklist). **Deliberately NOT a skill yet —
one project is one data point.** Next: run the same pass on a second stealth-public repo, append
observations as a comment on that issue, then decide and extract._

_**Direction from here.** The "how far does the family adopt this" question from the last handoff has
a concrete first answer: project-canon itself is now presentable to external users (front door, 
onboarding, security policy, neutral naming). Plausible next threads, none scheduled: lane-or-close
the description-limit check, a second publicize pass elsewhere (feeds the skill), family
adoption/rollout, or new intake._

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

_**🚀 2026-08-21 [superseded by 0.6.2 above]. `0.6.1` public and verified on all four channels**: crates.io
(`project-canon-core@0.6.1` + `project-canon-cli@0.6.1`), the GitHub Release (12 assets), and
Homebrew `project-canon` at `0.6.1`. Workspace manifest matches. `0.6.0` (08-19) shipped the complete
`cli-canon` behavioral skill alongside `ai-first-cli-canon` plus the first-class pi layout under
`--agent all`; `0.6.1` is release-infrastructure correctness only, no CLI behaviour change._

_**Canon is at v4, §1–§24.** `0.4.0`/`0.5.0` (08-17) closed the canon-conformance arc — see the
round-A/round-B sections below._

_**🎯 THE BACKLOG IS EMPTY.** Zero lanes, zero unscheduled, zero open issues (verified at handoff).
A next round therefore needs **new work defined** — this is a genuine cold start for planning, not a
prepared agenda. Do not manufacture units from history. The two standing decisions below (§10
rollout deliberately sitting; hybrid-metadata refactor dropped) are recorded so nobody re-opens them
by accident._

_**Direction from here.** v0's scope is complete and released; the open question is no longer "finish
v0" but **how far the family actually adopts this**. The release-surface thread that dominated the
last two rounds is now closed. Plausible next threads, none scheduled: family adoption/rollout, the
sibling-CLI §10 alignment (see the sit-decision), or whatever new intake arrives._

_**🔒 2026-08-21 (round) — the release surface now matches reality. Shipped as `0.6.1`.** Two units,
run in parallel (disjoint lanes), both landed green (279 tests)._

_**`contract-declare-release-surface` → fixed.** (1) **Homebrew is now a declared, verified target.**
Every release already published the formula via cargo-dist, but the contract never declared the
channel, so it shipped un-planned and un-verified — which is exactly how it silently sat three
versions behind earlier in the month. It is declared as **`binary:project-canon`**, the distribution
identity cargo-dist actually writes (`Formula/project-canon.rb`), **not** the Rust package name: the
`/llm-review` pass caught that a `project-canon-cli` key would seal a target verifying a formula that
never existed. (2) **The engine is now the sole crates.io publisher.** Releases had been running
*two* publish paths; the tag-triggered `publish-crates.yml` was **deleted**. The mystery of why the
duplicate always "succeeded" is answered: it explicitly regex-matched cargo's `already exists on
crates.io index` diagnostic and returned zero — a deliberate no-op after ossctl had already
published, not a second registry write, so nothing was ever corrupted. Full reasoning + the rejected
alternative are in the issue's comments._

_**`ci-timeout-regression` → wontfix, NO code change** (owner scoped it as an investigation, with
"leave it alone" pre-authorised as a valid outcome). The worker measured instead of guessing: reran
the same SHA 4× on Linux — **all passed** (1 failure in 5, 20%) — and found the failing attempt ran
the whole 140-test binary in **0.30s**, so the test never waited for its 2s deadline at all. Reading
the completion path showed the runner **already** waits on capture-pipe EOF *and* child status under
one deadline, i.e. the proposed "fix" was behaviour it already had. Likely real cause: an
intermittent Linux `Text file busy` (ETXTBSY) on the freshly-written executable script fixture —
another test in the same run failed exactly that way — and the assertion `matches!(…, Err(Timeout))`
**hides which variant actually occurred**. It deliberately changed nothing: weakening the timeout
would damage a real safety property. **Follow-up worth filing only if ETXTBSY failures become
frequent:** a test-infrastructure issue whose diagnostics preserve the actual `RunFailure` variant._

_**✅ `ossctl`'s gh-releases verify bug is FIXED — and the workaround self-expired as designed.**
`ossctl 0.10.0` resolves `verify-gh-release-missing`. **Verified, not inherited:** re-verifying the
exact v0.5.0 run that used to report `missing` returned 3/3 matches, and in the `0.6.1` cut
`gh-releases` verified `matches`. The temporary "trust the channels, not the exit code" bullet in
`AGENTS.md` was therefore **deleted**, as its own expiry clause required. That loop worked — a
workaround was written in with its removal condition and died in two days instead of hardening into
architecture (§24's whole point)._

_**⚠️ BUT: a NEW verify defect appeared on the newly-declared Homebrew target — `0.6.1`'s cut still
exited non-zero.** The release is completely fine (all four channels verified by hand); only the
engine misreports. Evidence: the formula is public, `Formula/project-canon.rb` returns **200** with
`version "0.6.1"`, and `package: project-canon` is the only path that exists — yet verify said
**`missing`** during the cut (after ~20 min polling) and **`unknown` / "could not be observed
(network or command failure)"** on a fresh read afterwards. Two contradictory verdicts about one
settled, publicly reachable artifact ⇒ the **observation path** is broken, not the lookup key, and an
observation failure is being coerced into `missing` in at least one code path. That coercion is the
dangerous part: `missing` asserts a fact about the registry and invites an irreversible re-publish.
**Filed via intake as `intake-bug-ossctl-51f9c1ce4cfd`** (ossctl). **Perverse incentive to be aware
of: declaring the Homebrew channel correctly is what triggers this**, so the bug currently punishes
correct configuration. **Until it is fixed, verify release channels by hand and do NOT act on the
cut's exit code** — but do not re-add a standing workaround bullet to `AGENTS.md` without an expiry
condition._

_**📌 DECIDED (owner, 2026-08-21): the hybrid mechanical+judgment metadata refactor is DROPPED
(wontfix).** §23/§24 each need a mechanical `doctor` row plus a review-only remainder, and that
hybrid-ness is currently hardcoded in the CLI (`review.rs::judgment_remainder`, matching literal
`canon.s23`/`canon.s24`) rather than declared in the core dimension model. A worker proposed moving
it into core. **Not filed, deliberately** — it is readable at two arms and the cost only appears if a
third hybrid section arrives. Revisit then, not before._

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

_**[RESOLVED 2026-08-21 — `verify-gh-release-missing` is fixed in `ossctl 0.10.0`; see the ✅ note
above. Kept for the diagnostic pattern only.]** The gh-releases verify bug reported a SUCCESSFUL
release as FAILED. The method that cracked it is the reusable part: it was a **lookup bug, not a
timing race**, proven by re-running the read-only `release verify` long after everything settled and
still getting `missing`. Apply that same test to any future "verify says missing" report — it
separates "looked too early" from "looked in the wrong place" in one command. (The stale `in_flight`
journal entries this caused have been abandoned with accurate reasons; `in_flight_count` is 0.)_

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
