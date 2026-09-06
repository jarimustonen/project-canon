---
created: 2026-09-06
updated: 2026-09-06
type: bug
reporter: jari
status: fixed
priority: high
lane: build
lane_seq: 10
commits:
- hash: 11367f0db34931d0f0d27aa3247ae6c3eb5e40e9
  summary: distribute native Codex skill trees
- hash: 63a16636ab47da7274546f014960c9505afda7e7
  summary: address review findings and harden migration
- hash: 8814fef
  summary: record Codex migration decisions
- hash: 73f9eba058b078ee1a52dcc1df83c90af70b36a4
  summary: update skill schema snapshot
closed: 2026-09-06
closed_by: pi
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

- [x] Canon §15 identifies Codex's native destination as `.codex/skills/<name>/...` and requires an Agent Skills tree with `SKILL.md` plus resources.
- [x] `project-canon skill list --json` reports the native Codex layout and `agent-skill-tree` form.
- [x] Default/explicit `all` and `--agent codex` install complete native trees without flattening resources.
- [x] `skill print` exposes Codex resources consistently with the native tree.
- [x] Managed legacy `.codex/prompts/<name>.md` artifacts from earlier Project Canon versions are removed safely during installation; foreign files are never removed.
- [x] Runtime/static conformance probes reject custom-prompt-only Codex distribution and require native skill-tree support.
- [x] Tests, bundled guidance, and public documentation are updated from their canonical sources.
- [x] Claude and pi behavior, `--target`, no-clobber defaults, `--force`, dry-run, and non-interactive safety remain unchanged.

## Quick Test

Run the full Rust green gate, then install into a temporary target with default, `--agent all`, and `--agent codex`. Verify `.codex/skills/<name>/SKILL.md` and all declared resources are present, no new `.codex/prompts/<name>.md` is written, managed legacy prompts are retired, and unrelated prompt files remain untouched.

## Decisions

### 2026-09-06T14:04:38Z · @pi

Design decisions and rejected alternatives:

- Codex now uses the same native Agent Skills tree model as Claude and pi: `.codex/skills/<name>/SKILL.md` plus every bundled resource at its relative path. Runtime-specific rendering remains an agent seam, but no Codex-only flattening remains because current Codex requires no divergent form.
- Default and explicit `all` still select Claude, pi, and Codex; a single `--agent` remains confined to that runtime. `skill print --agent codex --resource <path>` is byte-identical to the corresponding installed resource.
- Legacy migration follows invocation scope: it runs only when Codex is selected and only for the named skill, or all bundled skills when no name is given. Native writes complete before cleanup begins.
- A legacy prompt is removable only when its first line exactly identifies the selected skill as a Project Canon-managed legacy artifact with a complete marker, valid positive schema, and numeric CLI version no newer than the running binary. Foreign, malformed, native-form, symlink, non-regular, and newer artifacts are preserved, including under `--force`. Ownership and ancestor confinement are checked again immediately before unlinking.
- Dry-run reports planned writes/removals separately from actual counts. Real-run removal counts come from successful unlink outcomes. The skill artifact schema advances to 2 because Codex's shipped artifact changed incompatibly from one prompt to a frontmatter-bearing resource tree.
- Static skill-description discovery now includes `.codex/skills`; the read-only runtime capability probe independently requires `.codex/skills/<name>/...` with `agent-skill-tree` and rejects the old prompt path/form. Behavioral installation proof remains in source/tests or a safe sandbox rather than inferred from arbitrary checked-in runtime files.

Rejected alternatives:

- Rejected preserving `.codex/prompts` as a supported output or compatibility mode: it would continue distributing a non-native, flattened artifact and allow conformance metadata to bless the known-bad form.
- Rejected deleting every old-path file or letting `--force` broaden cleanup ownership: safe migration requires positive ownership, and a newer managed artifact is outside this binary's prior-version migration authority.
- Rejected deleting the legacy prompts directory: directory ownership cannot be proven and it may contain foreign prompts.
- Rejected a static rule that treats any `.codex/prompts` file as failed skill distribution: custom prompts may legitimately coexist and foreign files are explicitly preserved; installer metadata plus source/test/sandbox evidence is the reliable distinction.
- Rejected platform-specific `openat`/`openat2` deletion and fixing the pre-existing write-time race in this bug: apply-time ancestor/final-path revalidation closes the practical confinement gap for user-owned targets without introducing a cross-platform filesystem subsystem.
- Rejected changing Project Canon's repo-specific `issues/AGENTS.md` pointer ahead of issuectl's own release: Canon guidance now rejects prompt-only distribution, but operational documentation must still identify the artifact the currently released producer installs.

Review evidence: four-model, two-cross-round `/llm-review`; `history/assessment-codex-native-skill-distribution.{json,md}` assessed 13 findings. Seven required fixes were applied; six incorrect, disproportionate, or latent findings were dropped, with no follow-up issue meeting the filing bar.

## Resolution

### 2026-09-06T14:07:21Z · @pi

Implemented native Codex Agent Skills trees and conservative managed-prompt migration. Four-model review and assessment completed; all confirmed required findings resolved. Exact Rust green gate and manual default/all/Codex-only installation plus runtime-probe checks passed.
