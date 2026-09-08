---
created: 2026-09-08
updated: 2026-09-08
type: improvement
reporter: jari
status: done
priority: high
lane: build
collision: [crates/project-canon-core]
closed: 2026-09-08
---

# Validate Pi-compatible Agent Skills

## Description

Project Canon currently checks only companion-skill description length. It can therefore report a repository conformant even when a shipped `SKILL.md` cannot be parsed or discovered by pi.

Observed failure: pi 0.84.4 reports a skill conflict and skips `worktree-bug-analysis/SKILL.md` because an unquoted colon in the `description` value makes its YAML frontmatter a nested mapping (`Nested mappings are not allowed in compact mappings`). Project Canon should detect this before installation or release.

![Pi reports malformed skill frontmatter](pi-malformed-skill-frontmatter.avif)

The fix must be source-grounded and broader than this one YAML syntax error. Reconcile the current Agent Skills specification with pi's documented and implemented discovery/validation behavior, record intentional pi deviations, and turn the mechanically decidable portable requirements into Canon §15 checks.

## Source baseline

- pi `docs/skills.md` and the installed pi 0.84.4 skill loader/frontmatter parser
- Agent Skills specification and linked skill-authoring guidance at `agentskills.io`
- Existing Project Canon §15 text, static doctor/review probe, bundled skills, installer layouts, and tests

## Acceptance Criteria

- [x] A concise requirements matrix records standard requirements, pi-specific behavior/deviations, and whether each rule is mechanically enforceable by Project Canon.
- [x] Canon §15 requires valid YAML frontmatter and a portable Agent Skills tree, including required typed fields and published length/name constraints; pi-only extensions are classified without making portable skills invalid.
- [x] `doctor` and static `review` inspect every located shipped `SKILL.md`, reject malformed/non-mapping frontmatter and missing or incorrectly typed required fields, and enforce all mechanically decidable portable constraints rather than only description length.
- [x] The observed unquoted-colon reproduction fails with an actionable diagnostic naming the skill path and YAML parse problem.
- [x] Checks cover pi's relevant discovery locations/forms and do not misclassify unrelated Markdown files as declared skills.
- [x] Multiple invalid skills produce useful bounded evidence instead of hiding all but an arbitrary first problem.
- [x] Bundled/project-installed skills and generated output satisfy the strengthened checks.
- [x] Tests cover YAML syntax errors, type errors, missing fields, boundary values, name syntax/directory consistency policy, valid optional fields, pi extensions, discovery/collision edge cases where applicable, and symlink/path confinement.
- [x] Public docs and bundled `ai-first-cli-canon` / `cli-canon` guidance are regenerated or updated from canonical sources.
- [x] The full repository green gate passes.

## Quick Test

Create a temporary repo containing a discovered `SKILL.md` whose unquoted `description` contains `: `, run `project-canon doctor --json`, and verify `canon.s15` fails with the path and parse error. Repeat with a valid quoted or block-scalar description and verify it passes the mechanical checks.

## Decisions

### 2026-09-08T19:42:41Z · @agent

Implemented a portable Agent Skills core under §15 while allowing unknown runtime extensions and type-checking pi's disable-model-invocation extension. Conventional paths are collection roots with recursive stop-at-skill-root discovery. The scanner retains logical paths for alias/name semantics and canonical paths for confinement/cycle detection, uses bounded iterative traversal and bounded diagnostics, and treats repository I/O failures as operational faults rather than false conformance verdicts. Rejected: duplicating pi's lenient loader exactly, fail-fast diagnostics, unbounded recursive traversal, treating allowed-tools separator syntax as settled, silently skipping non-UTF-8 or symlinked paths, and claiming race-free confinement on a mutable repository. A four-model review converged on READY after these revisions.

## Resolution

### 2026-09-08T19:43:39Z · @issuectl

Implemented comprehensive portable Agent Skills validation for Canon §15, including pi discovery locations, typed YAML frontmatter, name and length rules, bounded recursive discovery, confinement, actionable aggregation, bundled renders, and doctor/review integration tests. The full green gate and a four-model critical review pass.
