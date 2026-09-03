---
created: 2026-09-03
updated: 2026-09-03
type: feature
reporter: jari
status: done
priority: normal
lane: build
commits:
- hash: 0f520ba
  summary: start three-runtime skill install work
- hash: f45a6eaa5cc0c62065f2a8fc533dab7e26b3d7a2
  summary: 'feat: require three-runtime skill installation'
- hash: 7f14ff2418d8f8592696a88c011b0bf37cd362da
  summary: 'issues: record implementation start'
- hash: ec16230e11f75461b6bb9161c914b6fdd4abb63e
  summary: 'fix: resolve final review findings'
- hash: b305a860340602098f4d44596462a8a4f3e4a996
  summary: 'fix: harden three-runtime conformance evidence'
- hash: 3329ed4aa3051d5322c262de3d7e28b5bf7d2cf5
  summary: 'issues: record three-runtime implementation'
- hash: 97c41bb5e026af595652110a5f208f5906f0db88
  summary: 'issues: close three-agent skill install'
- hash: d37fc0df9a8e578b7efdc8378c5be2be3e75b77e
  summary: 'issues: record three-runtime design decisions'
closed: 2026-09-03
closed_by: pi
---

# Require Claude, pi, and Codex skill installation

## Description

Every CLI that ships companion Agent Skills must support installation for all three maintained agent runtimes, each in its native location:

- Claude: `.claude/skills/<name>/...`
- pi: `.pi/agent/skills/<name>/...`
- Codex: `.codex/prompts/<name>.md`

A no-selection/default or explicit `all` installation must cover all three. Runtime-specific selection may remain available. Codex may require a self-contained prompt artifact rather than a native multi-file Agent Skills tree.

Project Canon itself already implements these three layouts. The missing product behavior is to make this a normative Canon §15 requirement and enforce/review it for other CLIs rather than merely documenting Claude plus a generic custom target.

## Acceptance Criteria

- [x] Amend the canonical §15 text with product-neutral runtime/layout requirements for Claude, pi, and Codex, including `all` semantics and native destinations.
- [x] Update Project Canon's resolved model/probe text and mechanical runtime/static checks where observable so `doctor`/`review` can distinguish full three-runtime support from a Claude-only installer.
- [x] Keep `--target` override behavior and safe non-interactive installation semantics.
- [x] Update the bundled `ai-first-cli-canon` and `cli-canon` guidance from the single source, plus tests and public docs where behavior is described.
- [x] Preserve neutral public artifacts and the existing generated/runtime-specific form differences.

## Decisions

### 2026-09-03T10:58:00Z · @pi

Design decisions and rejected alternatives:

- Canon §15 now defines `--agent claude|pi|codex|all`, default/explicit `all`, `--target`, native Claude/pi trees, and Codex's self-contained prompt as the normative family contract. `skill list --json` exposes a read-only, catalog-wide install capability object; every listed skill inherits the declared runtime set.
- Runtime review validates that declaration strictly (required values, exact native path/form, no-clobber/force semantics, and internally consistent extension agents), while the manual/source-and-test remainder verifies behavior. The runtime suite never invokes `skill install`.
- Static doctor remains limited to mechanically observable skill-description constraints. It does not infer installer behavior from checked-in `.claude`, `.pi`, or `.codex` files; arbitrary project-local runtime artifacts are not reliable installer evidence.
- Project Canon derives its emitted declaration from the same parser/layout constants used by installation, while the generic conformance probe retains independent canonical expectations.
- Install planning rejects observed descendant-parent symlinks that could redirect writes outside `--target`, but treats the explicitly supplied target base itself as the caller's boundary.

Rejected alternatives:

- Rejected three-way static artifact parity: it falsely failed repositories with unrelated runtime-local files and hybrid/generated installers.
- Rejected probing `skill install --help`: even help-mode invocation of a nominally mutating command violates the read-only runtime-probe guarantee.
- Rejected exact-set validation: required runtimes use subset semantics, with strict consistency for additional future agents.
- Rejected per-skill duplication of `supported_agents`: support is explicitly catalog-wide, avoiding contradictory duplicate metadata.
- Rejected treating declarations as behavioral proof: source/tests or a safe scratch install remain required.
- Rejected platform-specific `openat`/`openat2` confinement in this issue: the remaining concurrent directory-replacement race is rare for user-owned targets and would add major cross-platform complexity; the accepted hardening covers pre-existing descendant redirects.

Review/assessment evidence: four-model, multi-round `/llm-review`; `history/assessment-three-agent-skill-install.{json,md}` records 13 resolved FIX findings and five dropped/incorrect or disproportionate findings, with no follow-up issue warranted.

## Resolution

### 2026-09-03T11:00:40Z · @pi

Implemented and verified: Canon §15, resolved model and review probes, bundled skills, documentation, and tests now cover native Claude/pi/Codex installation with default/explicit all semantics and preserved safety. Multi-model review findings were assessed and resolved or deliberately dropped; the exact Rust green gate passes.
