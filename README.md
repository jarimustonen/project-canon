# project-canon 📏

<!-- oss-readme:badges-start -->
[![CI](https://github.com/jarimustonen/project-canon/actions/workflows/ci.yml/badge.svg)](https://github.com/jarimustonen/project-canon/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/project-canon-cli.svg)](https://crates.io/crates/project-canon-cli)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
<!-- oss-readme:badges-end -->

A project-scoped **conformance tool** for the AI-first CLI / project family. It carries a
**base project canon** plus **per-archetype profiles** (`cli`, `service`, …) and resolves them
into one model (`resolved = base ∪ profile`) that its verbs act on.

## Contents

- [What it does](#what-it-does)
- [Installation](#installation)
- [Usage](#usage)
- [Configuration](#configuration)
- [The canon as a skill](#the-canon-as-a-skill)
- [License](#license)

## What it does

Three verbs over the resolved conformance model, for the `cli` profile today:

- **`doctor`** — machine-verify a repo against the applicable profile. A mechanical CI
  conformance gate: it exits non-zero on any MUST gap, so it drops straight into a pipeline.
- **`new`** — scaffold a new repo that conforms to the canon. Generate-only: external
  bootstrap steps are *rendered and printed*, never executed.
- **`review`** — a recommending audit against the canon: severity-triaged findings plus
  staged (printed) issue-tracker commands. It advises; it never acts.

The AI-first CLI canon (`AGENTS-AI-FIRST-CLI.md`, §1–§23) is carried as the `cli` profile, and
this repo is its maintained home.

## Installation

**Homebrew** (macOS and Linux — the primary channel):

```sh
brew install jarimustonen/project-canon/project-canon
```

**From crates.io** (needs a Rust toolchain):

```sh
cargo install project-canon-cli
```

**Shell installer** (prebuilt binary, macOS and Linux, no toolchain):

```sh
curl -LsSf https://github.com/jarimustonen/project-canon/releases/latest/download/project-canon-installer.sh | sh
```

**Prebuilt binaries** — download the archive for your platform and its checksums from the
[Releases page](https://github.com/jarimustonen/project-canon/releases/latest). Prebuilt
binaries are published for **macOS (arm64, x86_64)** and **Linux (static musl; arm64,
x86_64)**.

The library crate is published separately as `project-canon-core` (`cargo add
project-canon-core`) for embedding the conformance model.

## Usage

```sh
project-canon --help              # all verbs and flags
project-canon doctor --json       # conformance gate (non-zero exit on a MUST gap)
project-canon new <name>          # scaffold a conformant repo (prints external steps)
project-canon review              # advisory audit; severity-triaged findings
```

Every verb speaks the AI-first CLI conventions: strict input validation, `--json` output, and
informative, non-interactive errors.

## Configuration

Built-in environment defaults are deliberately neutral. Before using a command that creates a
GitHub repository, configure `gh_account` in the file printed by `project-canon config path`, or
set `PROJECT_CANON_GH_ACCOUNT`. Configure §23's exact private-name scan with
`user_specific_deny_list` or `PROJECT_CANON_USER_SPECIFIC_DENY_LIST`; the built-in list is
intentionally empty. A complete fictional family configuration is available at
[`docs/config.example.toml`](docs/config.example.toml); inspect the resolved values with
`project-canon config show --json`.

## The canon as a skill

`project-canon` can install the canon itself as a versioned, single-sourced reference skill
(for Claude and Codex):

```sh
project-canon skill install       # install the ai-first-cli-canon skill
project-canon skill list --json   # installed skills + versions
project-canon skill print         # print the canon to stdout
```

## License

Licensed under the [MIT License](./LICENSE).
