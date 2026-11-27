---
created: 2026-11-27
updated: 2026-11-27
type: bug
reporter: jari
status: fixed
priority: normal
lane: unlaned
closed: 2026-11-27
commits:
- hash: 39e7229
  summary: review resolves the declared profile like doctor
---

# review ignores the profile a repository declares

## Description

`project-canon doctor` now reads the profile a repository declares in its `CONFORMANCE.md` when `--profile` is omitted (fix for `intake-bug-project-canon-61b6e91f85cb`). `project-canon review` still defaults to the `cli` profile and ignores that declaration, so in a repository generated with `--profile service|library|release` the two commands disagree about which canon sections apply, and `review` produces CLI-surface rows for a repository that has no CLI.

## Expected

`review` resolves the profile the same way `doctor` does: an explicit `--profile` wins, otherwise the declared profile, otherwise `cli`; its JSON names the source the same way (`profile_source`). Reuse the shared declaration reader rather than a second parser. Tests for a declared service repo, an explicit override, and an undeclared repo.

## Decisions

### 2026-11-27T13:04:27Z · @agent

Moved ProfileSource + resolve() from doctor.rs into declared_profile.rs (verb name only prefixes error messages), so doctor and review share one resolver instead of review getting a copy. To keep build_report under clippy's 7-argument limit, review passes (Archetype, ProfileSource) as one parameter. An explicit non-cli --profile with --run is still rejected while parsing. A declared non-cli profile with --run is rejected after resolution as a usage error that asks for --profile cli. Rejected: letting --run silently force cli (it would hide the declaration), and letting --run with a non-cli profile skip the runtime probes (it would hide them).
