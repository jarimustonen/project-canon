# skills/ — canonical skill sources

This directory holds the **maintained, canonical source** of the agent skills that ship
alongside the AI-first CLI canon. Per homebase **ADR 0009 §2/§6**, `project-canon` is now the
maintained home of these skills; homebase and other consumers copy them **FROM here** rather
than the reverse.

## Contents

- `cli-canon/` — the companion skill for the `cli` profile's canon. This repo-root path is a
  symlink to the physical package master under `crates/project-canon-cli/skills/cli-canon/`, so
  the complete resource tree is included in crates.io source archives without a second copy.
  ([`AGENTS-AI-FIRST-CLI.md`](../AGENTS-AI-FIRST-CLI.md), §1–§24). It is the reviewer/generator
  that *applies* the canon: **review** an existing family CLI against the canon (conformance
  matrix + prioritized recommendations) or **generate** canon-conformant surface scaffolding
  inside an existing repo. This is the §15 companion skill, version-synced with the canon per
  §17. Contents: `SKILL.md` + `templates/` (`conformance-probes.md`, `generate-plan.md`,
  `review-report.md`).

- `ai-first-cli-canon` — the canon **content** as an installable reference skill. **This one has
  no directory here on purpose:** it is *synthetic*, assembled by the binary from a small
  frontmatter/description const + the master canon via `include_str!` (see
  `crates/project-canon-cli/src/skill.rs`). Keeping it un-checked-in is the single-source rule in
  action — a physical `SKILL.md` with a pasted canon body would be the drifting second copy this
  design removes. It reaches adopting repos through `project-canon skill install` (§15/§16), not
  by hand-copying `AGENTS-AI-FIRST-CLI.md`. Distinct from `cli-canon` (which is *behavior* — the
  auditor; this is *content* — the rules). See `issues/canon-installable-skill/design.md`.

## Maintenance

- Edit the skill through **this path**. It resolves to the packaged physical master and remains
  the source of truth; downstream copies are derived.
- Installed copies are **consumers**, not sources. Keep environment-specific repository maps out
  of skill text: `project-canon-core`'s `EnvConfig` config/hook layer is the single source for
  each operator's settings.
