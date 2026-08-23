# project-canon 📏

<!-- shipshape-readme:badges-start -->
[![CI](https://github.com/jarimustonen/project-canon/actions/workflows/ci.yml/badge.svg)](https://github.com/jarimustonen/project-canon/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/project-canon-cli.svg)](https://crates.io/crates/project-canon-cli)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
<!-- shipshape-readme:badges-end -->

**Conformance tooling for AI-first CLI projects.**

When the primary operator of a command-line tool is an AI agent rather than a human,
the design rules change: strict input validation instead of silent fixups, `--json`
output everywhere, no interactive prompts, errors that explain how to recover, JSONL
logs, composable non-interactive commands. project-canon maintains those rules as a
versioned, citable canon — [`AGENTS-AI-FIRST-CLI.md`](crates/project-canon-core/AGENTS-AI-FIRST-CLI.md),
sections §1–§24 — and ships the tooling to apply it:

- **`doctor`** — a mechanical conformance gate for CI. Exits non-zero on any
  machine-checkable MUST gap, so it drops straight into a pipeline.
- **`new`** — scaffold a new repo that starts conformant. Generate-only: external
  bootstrap steps (git init, repo creation) are *printed* as a plan, never executed.
- **`review`** — a recommending audit: severity-triaged findings plus staged
  (printed, never executed) issue-tracker commands. Static by default; an explicit
  `--run <binary>` opts in to timeout-bounded, read-only runtime probes.
- **`skill`** — install the canon and its reviewer workflow as skills for coding
  agents, version-synchronized with the binary.

## Contents

- [Why](#why)
- [Quickstart](#quickstart)
- [Installation](#installation)
- [Usage](#usage)
- [The canon](#the-canon)
- [Agent skills](#agent-skills)
- [Configuration](#configuration)
- [Status](#status)
- [License](#license)

## Why

Human-oriented CLI conventions actively hurt agent-driven workflows: interactive
prompts hang an autonomous run, "did you mean?" fixups mask bugs, TTY-detected output
breaks parsers, and terse errors force expensive retry loops. The canon collects the
conventions that make a CLI reliable to drive programmatically, numbered so tools and
reviews can cite them precisely (e.g. *§3 no interactive prompts*, *§10 schema
versioning*, *§23 no user-specific facts in public artifacts*).

project-canon resolves a **base project canon** (documentation layout, issue
tracking, git hygiene) together with a **per-archetype profile** (`cli`, `service`,
`library`, `release`) into one model, and every verb acts on that resolved model. The
`cli` profile — the full 24-section AI-first CLI canon — is the deepest one today;
the other archetypes currently resolve to the base checks.

## Quickstart

Gate a repo in CI (exit code 1 on any mechanically-decided MUST gap):

```sh
$ project-canon doctor .
project-canon doctor: /work/my-tool (profile: cli)
OK    base.doc-pattern     AGENTS.md and CLAUDE.md present
OK    base.git-hygiene     .git present
OK    base.gitignore       .gitignore present
OK    base.issue-tracking  issues/ directory present
OK    base.readme          README.md present
OK    canon.s22            crates/*-core + *-cli split present
OK    canon.s23            no user-specific markers configured; set user_specific_deny_list or PROJECT_CANON_USER_SPECIFIC_DENY_LIST to enable the §23 scan
OK    canon.s24            all 0 detected deferral issue reference(s) resolve to open local issues; 0 binary/oversized/non-UTF-8 tracked file(s) skipped
summary: 8 ok, 0 warn, 0 fail, 21 skipped  →  mechanically conformant
```

Audit the same repo with recommendations instead of a gate — including read-only
runtime probes of your built binary:

```sh
project-canon review --run ./target/debug/my-tool --json .
```

Scaffold a new conformant project:

```sh
project-canon new my-tool --description "One-line description"
```

Add `--json` to any verb for the structured report.

## Installation

<!-- shipshape-readme:install-start -->
**Homebrew** (macOS and Linux):

```sh
brew install jarimustonen/project-canon/project-canon
```

**From crates.io** (needs a Rust toolchain):

```sh
cargo install project-canon-cli
```

**Shell installer** (prebuilt binary, no toolchain needed):

```sh
curl -LsSf https://github.com/jarimustonen/project-canon/releases/latest/download/project-canon-installer.sh | sh
```

**Prebuilt binaries** — download the archive for your platform and its checksums from
the [Releases page](https://github.com/jarimustonen/project-canon/releases/latest).
Prebuilt binaries cover **macOS (arm64)** and **Linux (static musl; arm64, x86_64)**;
on other platforms, install via Homebrew or `cargo install`.

The library crate is published separately as `project-canon-core` (`cargo add
project-canon-core`) for embedding the conformance model in your own tooling.
<!-- shipshape-readme:install-end -->

## Usage

```sh
project-canon --help                    # all verbs and flags
project-canon doctor [--profile <p>] [--json] [<repo>]
project-canon new [--profile <p>] [--dry-run] <dir>
project-canon review [--run <binary>] [--json] [<repo>]
project-canon skill install [<name>] [--agent <agent>] [--dry-run]
project-canon config show --json
```

Every verb follows the canon it enforces: strict input validation, fixed
(non-TTY-detected) output, `--json` reports with a schema version, meaningful exit
codes, and no interactive prompts. `doctor` and `review` are read-only against the
target repo; `new` writes only into its target directory and never shells out.

Runtime review (`review --run`) executes only the explicitly named binary, directly
(no shell), with read-only probe arguments and a per-invocation timeout. A missing,
crashing, or timed-out target is reported as `could-not-probe` — never as a pass or a
gap. See [`docs/review-runtime-probes.md`](docs/review-runtime-probes.md) for the
safety and JSON contracts.

## The canon

[`AGENTS-AI-FIRST-CLI.md`](crates/project-canon-core/AGENTS-AI-FIRST-CLI.md) is the canonical document, and
this repository is its maintained home. Sections §1–§18 are a stable citation
surface (never renumbered); new principles append as §19+. Highlights:

| § | Principle |
|---|---|
| §1 | Strict input validation — no silent fixups |
| §2 | Structured, parseable output; JSONL logs |
| §3 | No interactive prompts |
| §4 | Informative error messages |
| §8 | Configuration precedence: flag > env > file > built-in default |
| §10 | Schema versioning, errors, warnings, deprecation |
| §11 | Dry-run, idempotency, retry safety |
| §14 | `--help` is agent-first, structured, drill-down |
| §18 | `doctor`: read-only self-diagnostic |
| §23 | Public artifacts contain no user-specific facts |

It reads standalone — you can adopt the document without the tool, and the tool
without the whole family workflow.

## Agent skills

The canon also ships as installable skills for coding agents, so an agent can read
and apply it without a repo-local copy:

- **`ai-first-cli-canon`** — the canon itself as reference content.
- **`cli-canon`** — a reviewer/generator workflow: audit an existing CLI against the
  canon, or scaffold canon-conformant surface in an existing repo.

```sh
project-canon skill install --dry-run       # see what would be installed, for which agents
project-canon skill install                 # install both skills for every supported agent
project-canon skill install cli-canon --agent pi
project-canon skill list --json
project-canon skill print ai-first-cli-canon   # stream content without installing
```

Supported agent layouts (`--agent`, default: all) are `claude`, `pi`, and `codex`:
the `claude` and `pi` layouts receive native skill directories; `codex` receives a
self-contained prompt. Skills are version-synchronized with the binary (§17).

## Configuration

Built-in defaults are deliberately neutral — the tool assumes nothing about your
accounts or machine layout. Settings resolve as
`built-in default < config file < PROJECT_CANON_* environment variable` (§8):

- `project-canon config path` — print the effective config-file location.
- `project-canon config show --json` — print resolved settings and their provenance.
- Before scaffolding a repo that references a GitHub account, set `gh_account` (or
  `PROJECT_CANON_GH_ACCOUNT`).
- The §23 private-name scan is opt-in: configure `user_specific_deny_list` (or
  `PROJECT_CANON_USER_SPECIFIC_DENY_LIST`) with the names that must never appear in
  your public artifacts.

A complete, fictional example configuration is in
[`docs/config.example.toml`](docs/config.example.toml).

## Status

Pre-1.0 ([ZeroVer](https://0ver.org/)): the CLI surface is functional and released,
but minor versions may still break compatibility. Changes are tracked in
[`CHANGELOG.md`](CHANGELOG.md). Bug reports and feature requests are welcome via
[GitHub issues](https://github.com/jarimustonen/project-canon/issues).

## License

<!-- shipshape-readme:license-start -->
Licensed under the [MIT License](./LICENSE).
<!-- shipshape-readme:license-end -->
