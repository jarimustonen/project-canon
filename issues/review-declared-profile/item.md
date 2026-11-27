---
created: 2026-11-27
updated: 2026-11-27
type: bug
reporter: jari
status: open
priority: normal
lane: unlaned
---

# review ignores the profile a repository declares

## Description

`project-canon doctor` now reads the profile a repository declares in its `CONFORMANCE.md` when `--profile` is omitted (fix for `intake-bug-project-canon-61b6e91f85cb`). `project-canon review` still defaults to the `cli` profile and ignores that declaration, so in a repository generated with `--profile service|library|release` the two commands disagree about which canon sections apply, and `review` produces CLI-surface rows for a repository that has no CLI.

## Expected

`review` resolves the profile the same way `doctor` does: an explicit `--profile` wins, otherwise the declared profile, otherwise `cli`; its JSON names the source the same way (`profile_source`). Reuse the shared declaration reader rather than a second parser. Tests for a declared service repo, an explicit override, and an undeclared repo.
