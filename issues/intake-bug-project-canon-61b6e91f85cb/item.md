---
created: 2026-09-18
updated: 2026-09-18
type: bug
reporter: jari
status: untriaged
priority: normal
provenance: agent:homebase-wrapup
source_ref: agent:homebase-wrapup/reporter:jari/id:wrapup-native-agent-host-doctor-profile
---

# Doctor silently checks CLI profile in generated service repository

## Description

Doctor silently checks CLI profile in generated service repository

## Observed

A repository generated with `project-canon new --profile service` has a `CONFORMANCE.md` that identifies the service profile, and its generated `AGENTS.md` tells agents to verify with:

```sh
project-canon doctor
```

Running `project-canon doctor --json` inside that repository reports:

```json
{"profile":"cli","surface_shape":"flat-verb"}
```

It therefore evaluates the wrong profile unless the caller remembers to add `--profile service`. The correct invocation was:

```sh
project-canon doctor --profile service --assume-defaults --json
```

## Expected

A generated repository's documented doctor command should check the profile used to generate it. Either doctor should discover a persisted machine-readable profile, or generated instructions should include the required explicit `--profile service` argument. It should not silently check an unrelated default profile.

## Environment

- project-canon 0.9.1 (`c50976c7a4c27a71bb5d8180d4badcbbce0c358f`)
- generated profile: `service`

<!-- intakectl:analysis:start job:c6b33dce-9f63-438c-95f4-6ef30311a3e0 generation:0 -->
## Triage analysis

### Root cause

Two distinct code paths produce the mismatch:

1. **`doctor` defaults to CLI profile** — in `crates/project-canon-cli/src/doctor.rs`, `parse_args()`
   sets `profile: profile.unwrap_or(Archetype::Cli)`. Without `--profile service`, doctor
   silently evaluates the wrong canon section-set (all §1–§24 instead of the base-canon-only
   sections §10, §15–§17, §22–§24) and reports a `surface_shape` of `flat-verb` — a lie for a
   service repo that has no CLI surface.

2. **Generated instructions omit the profile flag** — in `crates/project-canon-cli/src/new.rs`:
   - `agents_md()` (line ~730) always emits `Verify mechanically with \`project-canon doctor\``
     regardless of `--profile`. The function is profile-unaware.
   - `conformance_todo()` (line ~890) correctly interpolates `resolution.archetype().slug()` into
     the intro paragraph (e.g., "for the `service` profile"), but the doctor command line still
     reads `project-canon doctor` without `--profile service`.

### Scope

- CLI profile is unaffected (matches the default).
- All non-CLI profiles (`service`, `library`, `release`) are affected — every `project-canon new
  --profile service|library|release` generates instructions that silently check the wrong profile.

### Fix approaches

**Option A — doctor auto-discovers the profile** from a persisted marker in the generated repo
(e.g., `.project-canon-profile` written by `new`, or a frontmatter field in `CONFORMANCE.md`).
Doctor would read the marker when `--profile` is omitted, making the bare `project-canon doctor`
correct for every generated repo.

**Option B — generated instructions include `--profile`** so the user always runs
`project-canon doctor --profile service` (etc.). Simpler but requires the caller to remember the
flag; no fix to doctor's defaulting logic.

**Option C — both A and B** for defence in depth.

### Recommendation

Option A is the most robust: a marker file is machine-readable, survives human instructions, and
makes doctor correct for any generated repo without caller discipline. Option B is a necessary
backstop (the AGENTS.md and CONFORMANCE.md should reference the correct invocation regardless of
whether auto-discovery lands). Implement both.

### Severity

Medium. The generated instructions are wrong, so an agent following them gets a false-positive
conformance report. An experienced human might catch the mismatch, but the whole point of the
generated scaffolding is that it *starts conformant* and the instructions are the authoritative
guidance.
<!-- intakectl:analysis:end job:c6b33dce-9f63-438c-95f4-6ef30311a3e0 generation:0 -->
