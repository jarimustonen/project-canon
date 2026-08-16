# TODO

Pointers to open issues. Descriptions and plans live in the linked
`issues/<slug>/item.md` — do not duplicate them here.

## 🔄 Continue here (handoff)

_Repo bootstrapped 2026-08-12 from homebase **ADR 0009** (`project-canon` = project/repo-scoped
conformance tool; base project canon + per-archetype profiles, AI-first CLI canon as the `cli`
profile). Resume: `jatketaan @TODO.md`. Live scheduling is `issuectl dag`; TODO.md carries only handoff notes._

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

**v0 scope discipline (ADR 0009 §6): a LIFT, not a greenfield canon — ✅ COMPLETE, ✅ RELEASED (0.1.1).**
✅ `cli` profile (§1–§22 lift); ✅ base canon seeded; ✅ `service`/`library`/`release` named-but-empty
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

