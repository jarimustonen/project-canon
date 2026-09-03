---
created: 2026-09-03
updated: 2026-09-03
type: feature
reporter: jari
status: in-progress
priority: normal
lane: build
---

# Require Claude, pi, and Codex skill installation

## Description

Every CLI that ships companion Agent Skills must support installation for all three maintained agent runtimes, each in its native location:

- Claude: `.claude/skills/<name>/...`
- pi: `.pi/agent/skills/<name>/...`
- Codex: `.codex/prompts/<name>.md`

A no-selection/default or explicit `all` installation must cover all three. Runtime-specific selection may remain available. Codex may require a self-contained prompt artifact rather than a native multi-file Agent Skills tree.

Project Canon itself already implements these three layouts. The missing product behavior is to make this a normative Canon §15 requirement and enforce/review it for other CLIs rather than merely documenting Claude plus a generic custom target.

Acceptance scope:

1. Amend the canonical §15 text with product-neutral runtime/layout requirements for Claude, pi, and Codex, including `all` semantics and native destinations.
2. Update Project Canon's resolved model/probe text and mechanical runtime/static checks where observable so `doctor`/`review` can distinguish full three-runtime support from a Claude-only installer.
3. Keep `--target` override behavior and safe non-interactive installation semantics.
4. Update the bundled `ai-first-cli-canon` and `cli-canon` guidance from the single source, plus tests and public docs where behavior is described.
5. Preserve neutral public artifacts and the existing generated/runtime-specific form differences.
