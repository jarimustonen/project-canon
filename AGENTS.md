# project-canon

Project-scoped conformance tool for the AI-first CLI / project family. Ships a **base
project canon** plus **per-archetype profiles** (`cli`, `service`, …), with the AI-first CLI
canon (`AGENTS-AI-FIRST-CLI.md` §1–§22) as the `cli` profile. Intended verb surface:
`new` (scaffold a conformant repo), `doctor` (machine conformance gate for CI), `review`
(recommending audit). Consumes an expanding dimension-discovery registry. See homebase
**ADR 0009** (`docs/decisions/0009-project-canon-scope.md`) for the scope/subsumption/name
decision that created this repo.

> **Status: bootstrap only.** This repo currently holds the scaffold + the shared canon copy.
> The canon/skill extraction from homebase and the `new` / `doctor` / `review` implementation
> are **not built yet** — they are tracked as issues here. **Open an issue before building a
> feature**; do not pre-design the tool in this file.

## CLI Design Principles

This project follows the AI-first CLI conventions in [`AGENTS-AI-FIRST-CLI.md`](AGENTS-AI-FIRST-CLI.md) — strict input validation, `--json` output, JSONL logs, no interactive prompts, informative errors, composable commands. Read that file before designing or changing CLI surface. **This repo is now the maintained home of that canon** (per ADR 0009 §2/§6): edit it *here*, and homebase / other consumers copy it FROM here. The physical master lives in [`crates/project-canon-core/AGENTS-AI-FIRST-CLI.md`](crates/project-canon-core/AGENTS-AI-FIRST-CLI.md) so it packages inside `project-canon-core` and ships on crates.io (exposed as `project_canon_core::CANON`, the single copy both the `new` scaffolder and `skill` installer embed); the repo-root `AGENTS-AI-FIRST-CLI.md` is a **symlink** to it, kept for external consumers and the homebase cutover. Edit the file through either path — they are the same bytes. The companion `cli-canon` skill is likewise maintained here under [`skills/cli-canon/`](skills/) (see [`skills/AGENTS.md`](skills/AGENTS.md)). **Follow-up (out of scope for this repo):** the homebase-side cutover — making homebase actually copy from here and retiring its own master copy of the canon + skill — is not yet done and must be done in the homebase repo; until it lands, do not edit the homebase copies, so the two do not diverge.

## Gitignored directories

- `history/` — agent scratchpad and ephemeral planning docs (not tracked)
- `/target` — Rust build artifacts

## Documentation Pattern

Every directory follows this structure:

- `CLAUDE.md` — symlink to `AGENTS.md`
- `AGENTS.md` — all AI-relevant info (consolidated)
- `AGENTS-<TOPIC>.md` — complex topics split out (optional)

## Issues & Planning

Issue tracking is managed by [`issuectl`](https://github.com/jarimustonen/issuectl). Use the `/issue` skill (installed by `issuectl init`) to create, search, update, and close issues.

- `issues/<slug>/item.md` — every issue and epic (flat layout — no numeric prefix, no `open/closed/` split)
- Status lives in the `status:` frontmatter field, not in the path
- `issues/AGENTS.md` — issue schema, types, workflow (owned by issuectl)
- `.issuectl/AGENTS.md` — repo-local policy for AI agents (owned by issuectl)

All planning documents (plans, analyses, validations, designs, breakdowns, todos) belong under their parent issue directory — not as standalone files. If work needs a planning document, it also needs an issue.

- `issues/<slug>/plan.md` — architecture, implementation plans
- `issues/<slug>/analysis.md` — research and analysis
- `issues/<slug>/design.md` — design documents
- `issues/<slug>/breakdown.md` — epic → child-issue breakdown with dependencies and critical path
- `issues/<slug>/todo.md` — task checklists

## Operating policy (for `/stint-start` and `/stint-handoff`)

This repo supports the stint work-session skills. `/stint-start` (round engine) and
`/stint-handoff` (terminal wrap) read the facts below plus the `TODO.md` handoff block +
Execution-DAG section. Live scheduling is **`issuectl dag`** (frontmatter `lane:` + `blocked_by`);
the `TODO.md` Execution-DAG block is a hand-maintained dual-run snapshot.

- **Green gate** (must pass before anything lands): `cargo build`, `cargo test`,
  `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`. (Standard for the AI-first
  CLI family. No code exists yet — the gate applies from the first Rust code that lands.)
- **Deploy command + target:** **none — this is a distributable CLI, not a hosted service.**
  There is no deploy-to-server step; `/stint-start` **Phase 3 (Deploy) is skipped**. Changes land
  on `main`, and **releases are cut via the OSS pipeline** (live since **v0.1.1**, 2026-08-16):
  `OSS-RELEASE.md` (approved contract) + **`ossctl release`** + **cargo-dist** (`dist-workspace.toml`
  → `.github/workflows/release.yml`). Publish targets: **crates.io** (`project-canon-core` +
  `project-canon-cli`, released in lockstep — the CLI exact-pins `core = "=<ver>"`) and **Homebrew**
  (tap **`jarimustonen/homebrew-project-canon`**, a **source-build** formula per the family
  convention — `depends_on "rust"`, `cargo install --path crates/project-canon-cli`; **maintained by
  hand in the tap** today, not auto-pushed). A release is a deliberate owner-gated action, not a
  stint deploy.
  - **⚠️ Release-cut gotcha (ossctl 0.2.2):** `ossctl release cut --version X` publishes the
    **manifest** version, *not* `X` — it does not bump `Cargo.toml`. **Bump
    `workspace.package.version` to the target version and commit it *before* cutting**, or you
    permanently mis-publish (this burned `project-canon-core@0.0.0`). Tracked as ossctl bug
    `release-cut-ignores-version`; also `release-resume-unimplemented` (a partial cut can't be
    resumed in 0.2.2 — finish by hand: `cargo publish` core → cli, tag `v<ver>`, push formula).
- **Deploy autonomy:** N/A (no deploy step). A **release** is owner-gated — never cut without an
  explicit go (it's an irreversible crates.io publish).
- **Live-version check:** `project-canon --version` (binary shipped as of v0.1.1); for the published
  crates, `curl -s https://crates.io/api/v1/crates/project-canon-cli | jq .crate.max_version`.
- **Hot files (define the DAG's lanes):** the `crates/project-canon-core` model/resolution
  substrate (`profile.rs`, `resolve.rs`, `canon.rs`, `questionnaire.rs`, `dimension.rs`,
  `routing.rs`, `scaffold.rs`, `lib.rs`) + the workspace `Cargo.toml`. This is the single serial
  `build` lane at v0 — every verb (`doctor`/`new`/`review`) reads the core model, so they collide
  here. `crates/project-canon-cli/src/main.rs` is the thin binary. Split `doctor`/`new`/`review`
  into parallel lanes only once their modules are provably disjoint (re-assess after each lands).
- **Migration rules:** N/A (no schema/DB).
- **Test-account reset preference:** none.

A stint round here is: pull → merge the DAG → spawn worktree(s) for the ready head(s) →
green-gate + review-gate before merge → **skip deploy** (releases are a separate owner-gated OSS
cut, above) → report. The canon (`AGENTS-AI-FIRST-CLI.md`) and the companion `cli-canon` skill
(`skills/cli-canon/`) are now maintained **here** — `extract-canon-and-skill` has landed, so
this repo is the source and homebase copies from here. The homebase-side cutover (homebase
actually pulling from here and retiring its own master copies) remains a documented follow-up
in the homebase repo.
