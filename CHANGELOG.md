# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- oss-changelog:unreleased-start -->
## [Unreleased]

### Added

- `doctor` and static `review` now enforce Canon §15's 1024-character maximum for
  frontmatter descriptions in repository skills they can locate.

### Changed

- Active release tooling guidance, generated ownership markers, distribution comments, and the
  bundled `cli-canon` audit example now use the Shipshape product, `shipshape` command, and
  `/shipshape-*` skill names. Historical release evidence and permanent compatibility identifiers
  remain unchanged.

### Fixed
<!-- oss-changelog:unreleased-end -->

## [0.6.2] - 2026-08-23

Terminology and canon-content release. No change to command behaviour.

### Changed

- The canon (§15) and the `skill` help/docs now describe skills by the open
  [Agent Skills](https://agentskills.io) standard and neutral `--agent` layout
  identifiers (`claude`/`pi`/`codex`) instead of naming agent products.
- Canon §15 now records the Agent Skills format limit: a skill's frontmatter
  `description` is at most 1024 characters.

## [0.6.1] - 2026-08-21

Release-infrastructure correctness. No change to the CLI's behaviour; the declared release
surface now matches what actually ships.

### Fixed

- **Homebrew is now a declared, verified release target.** Every release already published the
  Homebrew formula — cargo-dist does it from `publish-jobs = ["homebrew"]` — but the release
  contract never declared the channel, so it was published without ever being planned or
  verified. That is how the formula silently sat three versions behind earlier this month while
  every release reported success. It is declared as `binary:project-canon` (the distribution
  identity cargo-dist actually writes, `Formula/project-canon.rb`), not the Rust package name,
  so verification fetches the formula that exists rather than one that never did.
- **The release engine is now the sole crates.io publisher.** Every release had been running
  *two* publish paths: the engine publishing locally during its sealed transaction, and a
  tag-triggered workflow publishing again afterwards. The second path had been reporting success
  by explicitly matching cargo's `already exists on crates.io index` diagnostic and treating it
  as success — a deliberate no-op rather than a second registry write, so nothing was corrupted,
  but the declared surface and the real one disagreed. The duplicate workflow is removed.
  Publication stays inside the engine's transaction, where `dry-run-all` precedes it, core→CLI
  ordering is enforced, receipts are journaled, and the tag is pushed only after both crates
  publish.

## [0.6.0] - 2026-08-19

### Added

- `project-canon skill` now distributes the canonical `cli-canon` behavioral skill alongside
  `ai-first-cli-canon`, including all three probe/generation/review templates. Claude and pi get
  complete native skill trees; Codex keeps its prompt layout with the support resources embedded
  into one deterministic prompt. `skill print --resource` and JSON resource discovery expose the
  full tree without relying on downstream copies.

### Changed

- `--agent all` now includes the first-class pi layout at `.pi/agent/skills/`; version, skill-list,
  structured-help, and install metadata advertise both bundled skills and all supported agents.

### Fixed

## [0.5.0] - 2026-08-17

`review` stops asking a human to check what it can check itself, and the `--version` flag
becomes a first-class way to ask for the version payload.

### Added

- **`review --run <binary>`** — opt-in runtime probes that execute a built CLI to auto-confirm
  the canon sections that are only observable at runtime (§2 exit-code mapping, §8 `config
  path` / `config show --json`, §10 `version --json`, §14 `--help --json`, §15/§16/§17 the
  skill surface, §18 `doctor`). On this repository the probes move seven sections out of
  manual-verify: 16 manual / 6 pass becomes 9 manual / 14 pass.

  The execution path is deliberately conservative. It is opt-in only — `--assume-defaults`
  remains static and never executes anything — and it invokes the named binary directly rather
  than through a shell, with a per-call timeout, bounded output capture, and read-only argument
  vectors (`skill list` and `skill print`, never `skill install`). Outcomes distinguish `pass`,
  `gap`, and **`could-not-probe`**; a missing, non-executable, or hanging target is reported as
  unprobed and is never silently counted as either a pass or a gap. Under-reporting is what
  made the previous behaviour untrustworthy, so the report says what it could not determine.

### Changed

- **`--version` is now a full alias of the `version` verb.** Both spellings produce identical
  output and the same exit code in every mode, including `--json`, and argument order does not
  matter: `--version --json` and `--json --version` behave alike. Previously the flag was
  text-only and `--version --json` returned a usage error steering the caller to the verb.
  The verb remains the canonical form that agents should prefer; it is no longer the only form
  that works.
- **Canon §10 amended to match.** The section had mandated the old behaviour, justifying it on
  the grounds that the flag "cannot honor `--json`". That rationale was wrong: it is true of
  clap's built-in version action, but a tool can declare `--version` as an ordinary flag and
  dispatch it itself, which this one already did. The false rationale is dropped and the flag
  is now specified as a full alias. Sibling family CLIs that implement the old rule will begin
  reporting a §10 gap; that is the intended alignment signal, not a regression.

## [0.4.0] - 2026-08-17

Two canon sections that turn hard-won operating rules into machinery. Both follow the same
shape: a normative section, a mechanical `doctor` gate for the detectable subset, and a
`review` judgment row for the remainder.

### Added

- Canon **§23 — public artifacts must not embed user-specific facts**, scoped deliberately to
  *publicly distributed* artifacts: an internal-only tool may legitimately encode its own
  organization's policy, but a published package must never make a recipient inherit the
  maintainer's environment. Built-in defaults must be neutral; overridability does not launder
  a user-specific default, because unset still means whatever ships in the package. The section
  is routed through the base layer, so every profile inherits the obligation.
- A `doctor` gate for §23, driven by the new `user_specific_deny_list` configuration key
  (environment override `PROJECT_CANON_USER_SPECIFIC_DENY_LIST`). Matching is exact and
  case-insensitive with no username-shape heuristic, and the built-in list is empty — the
  markers are yours, supplied through user configuration that lives outside the distributed
  artifact. The scan derives the target's own owner and repository from its git remote or Cargo
  metadata and exempts that project's own GitHub, badge, Homebrew, and install coordinates,
  while still flagging a different private project name under the same owner. A project's own
  published address is not a leak; a check that fires on a README install line is a check that
  gets turned off.
- Canon **§24 — a stated blocker is re-verified, not inherited**. A justification for disabled,
  skipped, or deferred work carries implied authority and no evidence. Before building around
  one, check it: if it names a credential, permission, or dependency as missing, check whether
  that now exists; if it names an owning issue, check the issue exists and is open; if neither
  can be verified, say so rather than silently propagating the claim.
- A `doctor` check for §24 that rejects deferral justifications whose local owning issue is
  missing or closed, and fails closed on cross-repository owners it cannot verify.
- A `review` judgment row for each section, covering what mechanics cannot settle: hostnames,
  internal URLs, and borderline naming for §23; credential, permission, dependency, and blocker
  re-verification for §24.

### Changed

- `doctor` may now fail on repositories it previously passed, where a deferral justification
  names an owning issue that is missing, closed, or in another repository's tracker. This is
  the intended effect of §24 and the reason for the minor bump.
- Corrected this repository's own `dist-workspace.toml` deferral comment, which was itself a
  §24 specimen. It justified a gap by naming an issue in the `ossctl` tracker; that issue turned
  out to exist but be closed, and the Homebrew publisher it deferred had already been enabled.
  The comment is now a dated, verified statement of the release-ownership boundary rather than
  an inherited claim.

## [0.3.3] - 2026-08-17

### Fixed

- The Homebrew formula is published as `project-canon` again. `0.3.2` enabled the automatic
  formula publisher, but cargo-dist names the formula after the *package*, so it wrote
  `Formula/project-canon-cli.rb` while the canonical `Formula/project-canon.rb` — the name
  the tool is installed with — was left behind at `0.3.0` with nothing updating it. Users on
  `brew install <tap>/project-canon` now track releases again.
- Removed an internal environment name from the shipped crate. A planned CI-release pattern
  was referenced by its internal name in `project-canon-core` doc comments and a test
  fixture, which put an unreleased internal concept into a public package. The configuration
  seam itself is unchanged — `CiReleaseHook::pattern` still carries a configured pattern name
  and still defaults to `None` — only the hardcoded name is gone.

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
