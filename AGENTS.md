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

This project follows the AI-first CLI conventions in [`AGENTS-AI-FIRST-CLI.md`](AGENTS-AI-FIRST-CLI.md) — strict input validation, `--json` output, JSONL logs, no interactive prompts, informative errors, composable commands. Read that file before designing or changing CLI surface. **This repo is the maintained home of that canon** (per ADR 0009 §2/§6): edit it here. The physical master lives in [`crates/project-canon-core/AGENTS-AI-FIRST-CLI.md`](crates/project-canon-core/AGENTS-AI-FIRST-CLI.md) so it packages inside `project-canon-core` and ships on crates.io (exposed as `project_canon_core::CANON`, the single copy both the `new` scaffolder and `skill` installer embed); the repo-root `AGENTS-AI-FIRST-CLI.md` is a symlink to it. The released `project-canon skill install` / `skill print` surface is now the distribution path for homebase and other consumers, which consume the `/ai-first-cli-canon` skill instead of keeping repo-local canon markdown copies. The companion `cli-canon` skill is likewise maintained here under [`skills/cli-canon/`](skills/) (see [`skills/AGENTS.md`](skills/AGENTS.md)).

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

## Operating policy (for `/stint`)

`/stint` reads this section for how to run a work-session in this repo.

- **🔒 THIS REPO IS PUBLIC — no user-specific facts, anywhere** (maintainer rule,
  2026-08-16). A public artifact MUST NOT contain personal account handles, private
  repo/project names, personal filesystem-layout conventions, hostnames, internal URLs, or
  org-internal identifiers — not in source, **not in built-in defaults**, not in generated
  scaffold output, not in installed skill content, not in docs, not in tests or fixtures.
  **User-specific things belong in user config** (the §8 `defaults → file → env` layers,
  which live outside the distributed artifact). Overridability does **not** launder a
  user-specific default: unset still means whatever ships in the package. The correct
  built-in default is neutral/absent plus an actionable error naming the config key to set —
  never a guess at someone's environment. Fixtures and examples use obviously fictional
  values. **Check this before every publish**, and treat any new default, scaffold template,
  or `config`-surfaced value as a place this can regress. This rule was written after
  `0.1.1`/`0.2.0` shipped `gh_account: "jarimustonen"`, `repo_root: "~/Sources"`, and a
  family-tool list naming three *private* repos to crates.io — see `portable-neutral-defaults`
  (the cleanup) and `canon-no-user-specifics` (making it a `doctor`-enforced canon section).
- **Green gate** (must pass before a unit counts as landed):
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo build --workspace` (release build not required per-unit)
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` — **CI runs this and it
    is easy to miss locally**: broken intra-doc links (`[`Foo`]` to a moved/renamed/private
    item, redundant explicit link targets) fail the `docs` job even when tests pass. Run it
    before landing any unit that touches doc comments (`//!` / `///`).
- **Releases MAY be cut automatically whenever there is something to release, and the agent
  also OWNS THE DECISION to cut one** (maintainer decision, 2026-08-05; decision-ownership
  clarified 2026-08-16). Publishing `project-canon` itself (crates.io / GitHub Release /
  Homebrew) requires neither an explicit per-release go **nor a question about whether to
  release at all**: when `main` carries unreleased user-facing changes, `/stint` may bump the
  version, finalize the CHANGELOG, and run the release recipe as an owned Phase-3 act — no
  confirmation needed. **Judging that there is something worth releasing is the agent's call**,
  not a decision to escalate; do not surface "shall we publish?" as a Decisions-needed item.
  Preconditions still hold: the green gate passes, and `cargo publish` runs `--dry-run` first.
  crates.io publishes are irreversible (yank-only), so never publish red, and report each step.
- **The ENGINE-DRIVEN cut (`ossctl release cut`) is fully autonomous — NO go/no-go checkpoint,
  ever** (maintainer decision, 2026-08-06). Running the release *through the engine* — the full
  multi-target flow (crates.io ×2 + cargo-dist binaries + the Homebrew tap) — requires **no
  permission and no pause before the irreversible publish**, not for the first-ever engine cut,
  not for the homebrew leg (the homebrew leg is the most important target — it must be cut, not
  dropped). Do **not** stop to ask "shall I cut?" — just run the recipe end to end and report
  as you go. The safety is structural, not a human gate: `ossctl release plan` seals a
  content-addressed plan (a side-effect-free preview the agent inspects), the coordinator runs
  `dry-run-all` before any publish, `project-canon-core`→`project-canon-cli` ordering + index-wait
  guard the crates.io partial-publish case, and `ossctl release resume`/`abandon` recover an
  interrupted run. Still: green gate first, dry-run/plan first, never publish red, report each
  phase.
- **Git: `pull --rebase` → `push` is always allowed, no confirmation** (maintainer
  decision, 2026-08-05). On this repo the agent may run the pull-rebase-push sequence
  (`git pull --rebase origin main` then `git push origin main`, and pushing tags) on its own
  whenever `main` is clean and green — publishing commits to the remote does not need a
  separate go. Still: never force-push a shared branch, and never push a red tree.
- **Deploy command + target:** **none — this is a distributable CLI, not a hosted service.**
  There is no deploy-to-server step; `/stint-start` Phase 3 is skipped. Changes land on `main`,
  and releases are cut via the OSS pipeline: `OSS-RELEASE.md` (approved contract) +
  **`ossctl release`** + **cargo-dist** (`dist-workspace.toml` → `.github/workflows/release.yml`).
  Publish targets: **crates.io** (`project-canon-core` + `project-canon-cli`, released in
  lockstep — the CLI exact-pins `core = "=<ver>"`) and **Homebrew** (tap
  **`jarimustonen/homebrew-project-canon`**).
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
green-gate + review-gate before merge → **skip deploy** (there is no server deploy; releases are
the OSS cut described above, which the agent both decides on and runs autonomously) → report. The canon (`AGENTS-AI-FIRST-CLI.md`) and the companion `cli-canon` skill
(`skills/cli-canon/`) are now maintained **here** — `extract-canon-and-skill` has landed, so
this repo is the source and homebase copies from here. The homebase-side cutover (homebase
actually pulling from here and retiring its own master copies) remains a documented follow-up
in the homebase repo.
