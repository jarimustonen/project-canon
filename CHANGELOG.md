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
