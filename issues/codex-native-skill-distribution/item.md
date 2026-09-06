---
created: 2026-09-06
updated: 2026-09-06
type: bug
reporter: jari
status: in-progress
priority: high
lane: build
lane_seq: 10
commits:
- hash: 11367f0db34931d0f0d27aa3247ae6c3eb5e40e9
  summary: distribute native Codex skill trees
---

# Distribute Codex skills as native skill trees

## Description

Version 0.8.0 incorrectly defines and implements Codex companion-skill distribution as flattened custom prompt files under `.codex/prompts/<name>.md`. Current Codex has native Agent Skills support and discovers complete skill trees under `$CODEX_HOME/skills/<name>/` (normally `.codex/skills/<name>/` relative to an installation base).

The incorrect design was inherited from an older custom-prompt convention. Companion skills must be distributed as skills, not translated into custom prompts.

## Reproduction

1. Run `project-canon skill list --json` with version 0.8.0.
2. Observe the Codex layout `.codex/prompts/<name>.md` with form `self-contained-prompt`.
3. Run a default or `--agent codex` installation and observe flattened prompt artifacts instead of native Agent Skills trees.

## Acceptance Criteria

- [ ] Canon §15 identifies Codex's native destination as `.codex/skills/<name>/...` and requires an Agent Skills tree with `SKILL.md` plus resources.
- [ ] `project-canon skill list --json` reports the native Codex layout and `agent-skill-tree` form.
- [ ] Default/explicit `all` and `--agent codex` install complete native trees without flattening resources.
- [ ] `skill print` exposes Codex resources consistently with the native tree.
- [ ] Managed legacy `.codex/prompts/<name>.md` artifacts from earlier Project Canon versions are removed safely during installation; foreign files are never removed.
- [ ] Runtime/static conformance probes reject custom-prompt-only Codex distribution and require native skill-tree support.
- [ ] Tests, bundled guidance, and public documentation are updated from their canonical sources.
- [ ] Claude and pi behavior, `--target`, no-clobber defaults, `--force`, dry-run, and non-interactive safety remain unchanged.

## Quick Test

Run the full Rust green gate, then install into a temporary target with default, `--agent all`, and `--agent codex`. Verify `.codex/skills/<name>/SKILL.md` and all declared resources are present, no new `.codex/prompts/<name>.md` is written, managed legacy prompts are retired, and unrelated prompt files remain untouched.
