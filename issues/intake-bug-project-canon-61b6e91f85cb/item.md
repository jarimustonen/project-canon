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
