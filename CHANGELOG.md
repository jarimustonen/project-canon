# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- oss-changelog:unreleased-start -->
## [Unreleased]

### Added
### Changed
### Fixed
<!-- oss-changelog:unreleased-end -->

## [0.2.0] - 2026-08-16

Closes the tool's own conformance gaps against the canon it publishes: §2/§10 error
envelopes and exit codes, §10 `version --json`, §14 machine-readable help, and §8 config
inspection. `project-canon review` now reports zero confirmed gaps against this repo.

### Added
- `version --json` (canon §10) — schema-versioned drift payload carrying the tool name, CLI
  version, build commit (or an explicit null) with build provenance, supported output schema
  versions, supported profiles and surfaces, and bundled skills with their schema/version
  metadata. Covered by golden tests. `--version` remains human-readable and exits 0.
- `--help --json` (canon §14) — structured help for every command path, derived from the
  command definition so it cannot drift as flags change: schema version, command path,
  summary, arguments, flags (with value names, defaults, accepted values, env-var mappings
  and deprecation), subcommands, examples, and exit-code notes. Human `--help` is unchanged.
- `config path` and `config show --json` (canon §8) — inspect the resolved `defaults → file →
  env` configuration layer. Every value reports its provenance (which layer, and which file
  or variable specifically), with redaction support for secret-bearing values. Non-mutating.

### Changed
- All machine-facing failures now emit the canon error envelope on stderr with no data on
  stdout, routed through a single central error layer rather than per-callsite formatting.
- Exit codes follow the family map: 0 success, 1 caller/domain-actionable, 2 system/internal.
  Clap usage and parse failures are centrally remapped instead of leaking clap's default
  usage exit 2 with prose — `project-canon --json --version` now exits 1 with a JSON envelope
  where it previously exited 2 with a plain-text message.

## [0.1.1] - 2026-08-16

First public release.

### Added
- Two-layer conformance model — `resolved = base canon ∪ archetype profile` — with the
  AI-first CLI canon (`AGENTS-AI-FIRST-CLI.md`, §1–§22) carried as the `cli` profile, and
  `service`/`library`/`release` as named extension points.
- `doctor` — mechanical CI conformance gate; exits non-zero on any MUST gap.
- `new` — generate-only scaffold for a conformant repo; external bootstrap steps are rendered
  and printed, never executed.
- `review` — advisory conformance audit; severity-triaged findings plus staged (printed)
  issue-tracker commands. Never acts.
- `skill install|list|print` — install the canon as the versioned, single-sourced
  `ai-first-cli-canon` reference skill (Claude and Codex).
- Cross-platform binary distribution (cargo-dist): prebuilt binaries for macOS (arm64,
  x86_64) and Linux (static musl; arm64, x86_64), a Homebrew formula, and a shell installer.

### Changed
- The canon master (`AGENTS-AI-FIRST-CLI.md`) now physically lives in
  `crates/project-canon-core/` and is exposed as `project_canon_core::CANON`; the repo-root
  path is a symlink to it. Single-source, byte-identical — the CLI's `new`/`skill` verbs embed
  the core copy instead of their own out-of-crate `include_str!`.

### Fixed
- `project-canon-cli` is now crates.io-publishable: it no longer reaches outside its crate for
  the canon (`include_str!("../../../…")`), which left the packaged tarball unable to compile.
