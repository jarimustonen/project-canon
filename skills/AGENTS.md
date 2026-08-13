# skills/ — canonical skill sources

This directory holds the **maintained, canonical source** of the Claude Code skills that ship
alongside the AI-first CLI canon. Per homebase **ADR 0009 §2/§6**, `project-canon` is now the
maintained home of these skills; homebase and other consumers copy them **FROM here** rather
than the reverse.

## Contents

- `cli-canon/` — the companion skill for the `cli` profile's canon
  ([`AGENTS-AI-FIRST-CLI.md`](../AGENTS-AI-FIRST-CLI.md), §1–§22). It is the reviewer/generator
  that *applies* the canon: **review** an existing family CLI against the canon (conformance
  matrix + prioritized recommendations) or **generate** canon-conformant surface scaffolding
  inside an existing repo. This is the §15 companion skill, version-synced with the canon per
  §17. Contents: `SKILL.md` + `templates/` (`conformance-probes.md`, `generate-plan.md`,
  `review-report.md`).

## Maintenance

- Edit the skill **here**. This is the source of truth; downstream copies are derived.
- The `~/.claude/skills/cli-canon/` install and the homebase copy are **consumers**, not
  sources. The homebase-side cutover (making homebase copy from here and retiring its own
  master copy) is a documented **follow-up**, tracked separately — until it lands, do not edit
  the homebase copy, so the two do not diverge.
- The skill was lifted **verbatim** from its homebase origin (issue `extract-canon-and-skill`).
  It still carries homebase-environment specifics (e.g. the `~/Sources/...` family repo map in
  `SKILL.md`). Those specifics are now externalized in `project-canon-core`'s `env` config/hook
  layer (`EnvConfig` — the single source); the skill reading the map *from* that layer is the
  remaining homebase-side cutover, not part of the lift.
