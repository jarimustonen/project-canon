# project-canon

Project-scoped conformance tool for the AI-first CLI / project family — **implemented and
released** (crates.io + GitHub Releases + Homebrew; `project-canon --version` for the
installed binary). Ships a **base project canon** plus **per-archetype profiles**
(`cli`, `service`, `library`, `release`), with the AI-first CLI canon
(`AGENTS-AI-FIRST-CLI.md` §1–§24) as the `cli` profile; the other archetypes resolve to the
base checks at v0. Everything acts on the resolved model (`resolved = base ∪ profile`).

Verb surface (all implemented):

- **`doctor`** — machine conformance gate for CI; non-zero exit on a mechanically-decided MUST gap
- **`new`** — scaffold a conformant repo; generate-only, bootstrap hooks are printed, never executed
- **`review`** — recommending audit; static by default, `--run <binary>` opts in to read-only
  runtime probes (see [`docs/review-runtime-probes.md`](docs/review-runtime-probes.md))
- **`skill`** — install/print the `ai-first-cli-canon` + `cli-canon` skills (agent layouts:
  `claude`, `pi`, `codex`)
- **`config`** — inspect resolved settings and provenance (`path`, `show`); precedence is
  built-in default < config file < `PROJECT_CANON_*` env

See homebase **ADR 0009** (`docs/decisions/0009-project-canon-scope.md`) for the
scope/subsumption/name decision that created this repo.

[`README.md`](README.md) is the human front door (audience: external users). Keep it in sync
when the CLI surface, install channels, or platform coverage change — its install/badges/license
regions are `shipshape-readme` marker-managed.

**Open an issue before building a feature**; do not pre-design the tool in this file.

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

**Public docs must link a symlink's physical target, never the symlink.** GitHub's web UI
renders a symlinked file as its target path, not its content, so README/CONTRIBUTING link
`crates/project-canon-core/AGENTS-AI-FIRST-CLI.md` directly. Repo-internal agent docs may
keep using the repo-root symlink (it resolves locally).

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

- **🔒 THIS REPO IS PUBLIC — no user-specific facts, anywhere.** A public artifact MUST NOT contain private repo/project names, personal
  filesystem-layout conventions, personal machine hostnames, internal URLs, or org-internal
  identifiers — not in source, **not in built-in defaults**, not in generated scaffold output,
  not in installed skill content, not in docs, not in tests or fixtures.
  **Carve-out — the project's own published coordinates are not a leak.** This repo's own
  URL, its Homebrew tap, its CI badge, its install commands, and the public coordinates of
  tools it genuinely depends on (e.g. `github.com/<owner>/issuectl` in the issue-skill docs)
  necessarily name the owner, and are correct. The line is *whose environment the fact
  describes*: this project's public address = fine; the maintainer's other projects, machine
  layout, or private repos = never. A check built from this rule must not flag the repo's own
  coordinates, or it will be turned off.
  **User-specific things belong in user config** (the §8 `defaults → file → env` layers,
  which live outside the distributed artifact). Overridability does **not** launder a
  user-specific default: unset still means whatever ships in the package. The correct
  built-in default is neutral/absent plus an actionable error naming the config key to set —
  never a guess at someone's environment. Fixtures and examples use obviously fictional
  values. **Check this before every publish**, and treat any new default, scaffold template,
  or `config`-surfaced value as a place this can regress.
- **No `CODE_OF_CONDUCT.md`.** Do not add one on a future `/shipshape-contributing` run even
  though the mvp tier proposes one.
- **The release engine is `shipshape`.** Use the canonical `shipshape` binary and
  `/shipshape-*` skill catalog. If Shipshape is not installed, stop and report the convergence gap;
  do not substitute another release binary for a new cut. Existing remote coordinates do not
  select the release engine.
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
  also OWNS THE DECISION to cut one.** Publishing `project-canon` itself (crates.io / GitHub Release /
  Homebrew) requires neither an explicit per-release go **nor a question about whether to
  release at all**: when `main` carries unreleased user-facing changes, `/stint` may bump the
  version, finalize the CHANGELOG, and run the release recipe as an owned Phase-3 act — no
  confirmation needed. **Judging that there is something worth releasing is the agent's call**,
  not a decision to escalate; do not surface "shall we publish?" as a Decisions-needed item.
  Preconditions still hold: the green gate passes, and the engine's `dry-run-all` phase succeeds.
  crates.io publishes are irreversible (yank-only), so never publish red, and report each step.
- **The ENGINE-DRIVEN cut (`shipshape release cut`) is fully autonomous — NO go/no-go checkpoint,
  ever.** Running the release *through the engine* — the full
  multi-target flow (crates.io ×2 + cargo-dist binaries + the Homebrew tap) — requires **no
  permission and no pause before the irreversible publish**, not for the first-ever engine cut,
  not for the homebrew leg (the homebrew leg is the most important target — it must be cut, not
  dropped). Do **not** stop to ask "shall I cut?" — just run the recipe end to end and report
  as you go. The safety is structural, not a human gate: `shipshape release plan` seals a
  content-addressed plan (a side-effect-free preview the agent inspects), the coordinator runs
  `dry-run-all` before any publish, `project-canon-core`→`project-canon-cli` ordering + index-wait
  guard the crates.io partial-publish case, and `shipshape release resume`/`abandon` recover an
  interrupted run. The engine is the sole crates.io writer and requires valid crates.io credentials
  on the host running the cut; there is deliberately no tag-triggered crates.io workflow. Never
  add one alongside `cargo-publish`, because that recreates a double writer. Still: green gate
  first, dry-run/plan first, never publish red, report each phase.
- **Git: `pull --rebase` → `push` is always allowed, no confirmation.** On this repo the agent may run the pull-rebase-push sequence
  (`git pull --rebase origin main` then `git push origin main`, and pushing tags) on its own
  whenever `main` is clean and green — publishing commits to the remote does not need a
  separate go. Release tags are created and pushed only by `shipshape release cut` or
  `shipshape release resume`; a manual version tag would start cargo-dist without the crates.io leg.
  Still: never force-push a shared branch, and never push a red tree.
- **Deploy command + target:** **none — this is a distributable CLI, not a hosted service.**
  There is no deploy-to-server step; `/stint-start` Phase 3 is skipped. Changes land on `main`,
  and releases are cut via the OSS pipeline: `OSS-RELEASE.md` (approved contract) +
  **`shipshape release`** + **cargo-dist** (`dist-workspace.toml` → `.github/workflows/release.yml`).
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
- **Worker briefs MUST require design reasoning in the ISSUE, not only in the run report.**
  Every brief handed to a worktree worker must include: *"append your design decisions and
  rejected alternatives as an `issuectl` comment on the issue before merging."* Treat this as
  non-optional, exactly like the green gate and the terminal `run merge` call. **Why:** issue
  comments are durable, visible in the repository, and available to later agents. Keep the
  structured report for orchestrator sequencing, but never let it be the only copy of a design
  decision.
- **Migration rules:** N/A (no schema/DB).
- **Test-account reset preference:** none.

A stint round here is: pull → merge the DAG → spawn worktree(s) for the ready head(s) →
green-gate + review-gate before merge → **skip deploy** (there is no server deploy; releases are
the OSS cut described above, which the agent both decides on and runs autonomously) → report. The canon (`AGENTS-AI-FIRST-CLI.md`) and the companion `cli-canon` skill
(`skills/cli-canon/`) are now maintained **here** — `extract-canon-and-skill` has landed, so
this repo is the source and homebase copies from here. The homebase-side cutover (homebase
actually pulling from here and retiring its own master copies) remains a documented follow-up
in the homebase repo.
