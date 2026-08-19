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

_**🔒 2026-08-16 (session 2, LATEST — start here). Two things happened: a 4-unit canon-conformance
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
re-prompting workers; it's a tooling bug. (2) **Four `audit-no-user-specifics` issues** filed in
`issuectl`, `orchestratectl`, `ossctl`, `glasspad` (the public siblings) so the leak class gets swept
family-wide; `aggountant` is private so the rule doesn't bite. Each says: **close as clean if the
audit finds nothing** — a recorded clean result is the point._

_**Homebrew leg is STILL manual** — `ossctl release plan` seals only the 2 crates.io targets, so the
tap formula (sha256 + push) was hand-updated **twice** today. Teaching cargo-dist to push the formula
in CI is the highest-value release-infra follow-up (noted since 0.1.1, still not filed)._

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

**▶ NEXT (head-of-line): `canon-no-user-specifics`** — promote the publicness rule to a canon
section (§23) + a mechanical `doctor` check, so it's enforced family-wide instead of remembered.
Its blocker (`portable-neutral-defaults`) has landed. **Read the appended note on that issue first**
— it records the carve-out the check must honor and what it needs as input (the repo's own
coordinates as known-good, derivable from the git remote; plus an operator deny-list via the §8
user-config layer, keeping the check itself free of user specifics). Land it in a state where
project-canon itself passes — a canon section the home repo violates is worse than none.

**Then (laned, `canon-rollout` seq 30): `intake-feature-project-canon-ab1e44dfaf66`** — make `review`
**execute the built binary** to auto-confirm runtime-observable canon checks (§2/§8/§10/§14/§15/§16/
§18) instead of punting ~14 of 22 sections to manual-verify. Admitted from intake at this handoff
(owner ack 2026-08-16). Filed against 0.1.1, when `review` auto-confirmed only ONE gap family-wide
(§22, a static filesystem check) and the real gaps had to be found by hand-running each binary.
**Much more actionable now:** this session built the very surfaces it wants probed, so the probes
have something to find. Suggested shape from the reporter: an opt-in `--run <path-to-binary>` (or
auto-detect a built target), keeping `--assume-defaults` static-only as the safe default for un-built
repos. **Sequence it with `canon-no-user-specifics`** — that one also needs a new `doctor` check, so
design the "actually execute/probe the target" mechanics once, for both.

**Also open:** `project-canon-v0` epic looks finishable — worth a close pass.

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
