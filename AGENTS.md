# project-canon

`project-canon` is the conformance tool for the AI-first CLI project family: a base project
canon plus per-archetype profiles (`cli`, `service`, `library`, `release`), where the `cli`
profile is the AI-first CLI canon itself and the other archetypes resolve to the base checks
at v0. It is released on crates.io, GitHub Releases, and Homebrew. [`README.md`](README.md)
describes the verb surface (`doctor`, `new`, `review`, `skill`, `config`) for external users,
`project-canon <verb> --help` is authoritative for flags, and
[`docs/review-runtime-probes.md`](docs/review-runtime-probes.md) covers the one verb that can
execute a target. The scope, subsumption, and naming decision that created this repo is ADR
0009 in the homebase repository (`docs/decisions/0009-project-canon-scope.md` there).

The tool enforces on itself what it checks in others, so any change to the CLI surface follows
the canon in [`AGENTS-AI-FIRST-CLI.md`](AGENTS-AI-FIRST-CLI.md). Read it before designing a
surface change. It is long, its sections are cited by number, and reviews hold this repo to it.

## Where the canon lives

This repo is the maintained home of the canon and of the companion `cli-canon` skill. The
physical master is `crates/project-canon-core/AGENTS-AI-FIRST-CLI.md`, embedded at build time
as `project_canon_core::CANON`; the repo-root file is a symlink to it. Everything downstream
is derived from that one constant: the `skill install` / `skill print` surface, the synthetic
`ai-first-cli-canon` skill, and the scaffolds `project-canon new` generates, which point
adopting repos at the skill instead of carrying a markdown copy. That is why there is no
second checked-in copy anywhere, and why adding one would reintroduce the drift this design
removed. [`skills/AGENTS.md`](skills/AGENTS.md) explains the skill layout and its reasoning.

GitHub's web UI renders a symlink as its target path rather than its content, so anything
external readers see (README, CONTRIBUTING) links the physical file. Repo-internal agent docs
may link the root symlink; it resolves locally.

## Public repo: nothing user-specific ships

Versions 0.1.1 and 0.2.0 shipped a maintainer account, a personal repository-root convention,
and three private repository names to crates.io as built-in defaults. Making them overridable
had not helped, because unset still means whatever the package ships. The cleanup became canon
§23, which `doctor` now checks, and §23 is where the rule and its carve-out for the project's
own published coordinates are stated. The test to apply is whose environment a fact
describes: this project's public address is fine, the maintainer's other projects and machines
are not. Where no neutral default exists, the value stays absent and the error names the
config key to set. Fixtures use obviously fictional values. New defaults, scaffold templates,
skill text, and `config`-surfaced values are where this regresses, so they deserve a look
before any publish.

## Documentation and issues

Every directory carries an `AGENTS.md` holding all agent-relevant knowledge, with `CLAUDE.md`
symlinked to it, and optionally `AGENTS-<TOPIC>.md` for a topic worth splitting out. The README
is the human front door; keep it in sync when the CLI surface, install channels, or platform
coverage change. Its badge, install, and license regions are marker-managed by
`/shipshape-readme`, so refresh those through the skill rather than by hand.

Issues live in `issues/<slug>/item.md` under `issuectl`; use the `/issue` skill, and see
`issues/AGENTS.md` and `.issuectl/AGENTS.md` for the schema and agent policy. Plans, analyses,
designs, breakdowns, and todo lists belong in the issue's directory, not as standalone files
and not in this file. An issue is the durable, findable record of a decision, and design
written anywhere else gets lost. Open an issue before building a feature for the same reason.

Two maintainer decisions to know about: there is deliberately no `CODE_OF_CONDUCT.md` (removed
2026-08-22, and `/shipshape-contributing` will keep proposing one for the mvp tier), and the
release engine is `shipshape` with its `/shipshape-*` skills. If shipshape is not installed on
the host, that is a convergence gap to report, not a reason to cut with something else.

## Operating policy

`/stint-start` and `/stint-handoff` read this section for the facts they need to run a round
here.

**Green gate.** A unit has landed when these pass:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

The CI workflow runs only the first three. The doc build reports broken intra-doc links and
redundant link targets that nothing else catches, so run it for any unit that touches `///`
or `//!` comments. A release build is not needed per unit.

**Deploy.** None. This is a distributable CLI, not a hosted service, so there is no server
step and `/stint-start` has nothing to deploy. Changes land on `main` and reach users through a
release. Migration rules and test-account reset: not applicable.

**Releases.** The contract is `OSS-RELEASE.md`. Publish targets are crates.io
(`project-canon-core`, then `project-canon-cli`, which exact-pins core so the two release in
lockstep), GitHub Releases through cargo-dist, and the Homebrew tap
`jarimustonen/homebrew-project-canon`. Homebrew is the primary install channel, so a cut that
drops it has not shipped. Judging that `main` carries something worth releasing is your call,
and so is running the cut: bump the version, finalize the changelog, `shipshape release plan`,
then `shipshape release cut`, reporting each phase as you go. Nobody needs to be asked, and
"shall we publish?" is not a decision to escalate. The safety is in the engine rather than in
a human gate: the plan is a sealed, side-effect-free preview you can inspect, `dry-run-all`
runs before any publish, core-before-cli ordering with an index wait covers the
partial-publish case, and `shipshape release resume` / `abandon` recover an interrupted run.
What cannot be recovered is a bad crates.io publish (yank only), so a red gate or a failed
dry run ends the attempt.

Two facts about the pipeline that are easy to trip over. The engine is the sole crates.io
writer and needs crates.io credentials on the host running the cut; a tag-triggered crates.io
workflow would be a second writer, which is why there is none. Release tags come only from
`shipshape release cut` or `resume`, because a hand-made version tag starts cargo-dist without
the crates.io leg. `release.yml` also carries a hand-maintained override for the self-hosted
macOS runner; raw `dist generate` would overwrite it, so regenerate the workflow the way
[`docs/release-runner-isolation.md`](docs/release-runner-isolation.md) describes.

Live-version check: `project-canon --version` for the installed binary, and
`curl -s -A project-canon https://crates.io/api/v1/crates/project-canon-cli | jq .crate.max_version`
for the registry (crates.io answers 403 to curl's default user agent, so the `-A` is needed).

**Git.** `main` is shared with CI and the release engine. Pulling with rebase and pushing a
clean, green `main`, plus engine-made tags, needs no go. A red push or a force-push of a shared
branch costs someone else their work.

**Hot files.** `crates/project-canon-core` (`profile.rs`, `resolve.rs`, `canon.rs`,
`questionnaire.rs`, `dimension.rs`, `env.rs`, `routing.rs`, `scaffold.rs`, `lib.rs`) plus the
workspace `Cargo.toml` form one serial lane: every verb reads the core model, so worktrees
touching it collide. `crates/project-canon-cli/src/main.rs` is the thin binary. Give `doctor`,
`new`, and `review` their own lanes only once their modules are provably disjoint, and re-check
after each lands.

**Worker briefs.** Every brief handed to a worktree asks the worker to append its design
decisions and rejected alternatives as an `issuectl` comment on the issue before merging. The
run report exists for the orchestrator's sequencing and is gone afterwards; the issue comment
is what the next agent finds.

**Autonomy.** The user's attention is the scarce resource. Ask when the outcomes differ in a
way they would care about and you cannot tell which they would pick; otherwise choose, say
what you chose, and continue. The two things at stake in this repo are named above: what goes
to crates.io, and what a public artifact says about someone's environment.
