---
created: 2026-08-15
updated: 2026-08-15
type: bug
status: open
priority: high
labels: [release-blocker]
---

# CLI crate not crates.io-publishable — canon embedded via include_str! from outside the crate

## Description

## Symptom

`cargo publish -p project-canon-cli` fails at the verify step:

```
error: couldn't read `src/../../../AGENTS-AI-FIRST-CLI.md`: No such file or directory
  --> src/new.rs:39   const CANON: &str = include_str!("../../../AGENTS-AI-FIRST-CLI.md");
  --> src/skill.rs:51 const CANON: &str = include_str!("../../../AGENTS-AI-FIRST-CLI.md");
```

## Root cause

`project-canon-cli` embeds the canon by `include_str!`-ing the repo-root master
`AGENTS-AI-FIRST-CLI.md`, which lives OUTSIDE the crate directory. crates.io packages only
files inside the crate, so the published tarball is missing the file and cannot compile. The
in-repo binary build works (root file present), which is why this was never caught — the CLI
was never validated for source publishing.

## Constraint (keep the design intent)

The single-source, no-drift intent (skill.rs: 'no drifting second copy') must be preserved,
AND the repo-root path must stay readable for external consumers / the homebase cutover.

## Suggested fix (implementer's call)

Make `project-canon-core` the physical home of the canon: move `AGENTS-AI-FIRST-CLI.md`
into `crates/project-canon-core/`, expose `pub const CANON: &str = include_str!(...)` from
core (packaged within core), have `project-canon-cli` use `project_canon_core::CANON`
instead of its own `include_str!`, and make the repo-root `AGENTS-AI-FIRST-CLI.md` a symlink
to core's copy so root stays the access point (update CLAUDE.md's 'maintained home' note
accordingly). Verify with `cargo publish -p project-canon-cli --dry-run` and
`cargo publish -p project-canon-core --dry-run`. Alternative: relocate into the cli crate.
Whatever the layout, both crates must `cargo publish --dry-run` clean and the existing 214
tests must pass.

## Context

Surfaced cutting the 0.1.0 release. project-canon-core@0.1.0 is already published to
crates.io; project-canon-cli publish is blocked on this. Release is paused pending this fix.
