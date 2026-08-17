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

## [0.3.2] - 2026-08-17

Completes the distribution repair started in `0.3.1`. That release fixed the build profile —
both Linux musl targets compiled successfully — but still published no artifacts, because both
macOS jobs queued indefinitely and a release only attaches assets once every build job
finishes. `0.3.1` therefore has no binaries either; use `0.3.2`.

### Fixed

- macOS builds are pinned back to the repository's self-hosted Apple-silicon runner.
  GitHub-hosted macOS jobs are not picked up for this repository — they queue indefinitely
  (22h on `v0.1.1`; `v0.3.1` was cancelled after 90 minutes with both macOS jobs still
  queued while both Linux jobs had already succeeded). The override was removed in `0.3.1`
  on the mistaken assumption that it was incidental personal configuration; it is load-bearing,
  and `dist-workspace.toml` now says so.
- `x86_64-apple-darwin` is removed from the target matrix again. The self-hosted runner uses
  Homebrew Rust and cannot cross-compile it, and an unbuildable target does not fail the
  release — it queues forever and blocks every other artifact from publishing. Intel macOS
  installs via `cargo install project-canon-cli`.

### Added

- `homepage` package metadata, required by the Homebrew publish job.

## [0.3.1] - 2026-08-17

Repairs binary distribution. No library or CLI behaviour changes.

### Fixed

- The workspace manifest was missing the `[profile.dist]` Cargo profile that cargo-dist
  builds with, so the tag-triggered release workflow failed on every target with
  `error: profile 'dist' is not defined` → `failed to find bin project-canon`. **No release
  since `0.2.0` attached any prebuilt binary**, which meant the shell installer URL
  (`.../releases/download/v<ver>/project-canon-installer.sh`) 404'd and Linux installs could
  not work at all. macOS was unaffected: crates.io publishing succeeded throughout, and the
  Homebrew formula builds from source.

### Changed

- The release target matrix now covers all four required platforms — `x86_64-apple-darwin`
  was previously absent, so Intel macOS had no prebuilt binary.
- macOS builds run on hosted runners. A self-hosted runner override for
  `aarch64-apple-darwin` was removed: it tied releases to one personal machine being online,
  and it was the reason Intel macOS could not be cross-compiled.

## [0.3.0] - 2026-08-16

Removes user-specific facts from the shipped artifact. `0.1.1` and `0.2.0` carried one
maintainer's environment as built-in defaults — a GitHub account, a personal repo-root
convention, and a seven-tool family list that named three private repositories. A public
package must not describe someone's environment; that belongs in user configuration.

Those versions remain available and are **not** yanked: crates.io retains published files
permanently, so a yank would not remove the names while it would break existing installs.

### Changed

- **BREAKING** — `EnvConfig::gh_account` and `EnvConfig::repo_root` are now
  `Option<String>` (previously `String`). Both are `None` by default, and
  `EnvConfig::family_tools` defaults to an empty set. A value the tool genuinely needs but
  that is not configured now produces an actionable error naming the config key to set,
  instead of silently assuming a layout.
- Built-in defaults are neutral. The environment is supplied entirely through the existing
  `defaults → file → env` layers — i.e. from a user config file outside the distributed
  artifact. `config path` reports where that file lives; `config show --json` reports each
  value's provenance, so a configured value is visibly `"source": "file"` rather than a
  default.
- Optional `tw` registration is off by default and no longer assumes a personal registry.
- Tests and fixtures use fictional values.

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
