---
created: 2026-08-14
updated: 2026-08-14
type: improvement
status: wontfix
priority: normal
epic: project-canon-v0
labels: [tooling]
closed: 2026-08-14
---

# Typed DimensionId so the doctor probe registry is compile-checked against core

## Description

## Description

Replace the raw `&str` dimension ids in `project-canon-core` with a typed identifier (an enum, or a probe-kind attached to `Dimension`) so the CLI-side probe registry is checked against the model at **compile time**.

## Why

Surfaced by the 4-model review of the `doctor` verb (`history/review-doctor-raw.md`, all 4 reviewers). `doctor`'s `mechanical_probe(id: &str)` matches literal id strings. A core-side id rename (e.g. `base.readme` → `base.readme-md`) does **not** cause a compile error — the probe silently disappears and the affected MUST degrades from "enforced" to "deferred-to-review", so the gate can pass on a repo that should fail. A conformance gate that fails open on a typo is broken.

Short-term mitigation already landed in the doctor issue: `every_mechanical_probe_id_exists_in_the_model` asserts the registry ids resolve in `Model::standard()`. That catches drift in tests but does not make it a compile error, and it does not help future verbs (`new`/`review`) that also key off ids.

## Scope / why its own issue

The ids are core's public surface, read by `doctor` today and by `new`/`review` later. Introducing a typed `DimensionId` (or a `probe_kind` field) touches `dimension.rs`, `canon.rs`, `scaffold.rs`, `profile.rs`, and every id call site across the verbs — a cross-cutting core API change that needs its own design (enum vs. probe-kind, how canon `§N` ids and scaffold ids coexist, migration of the string keys).

## Acceptance

- A core-side dimension rename is a compile error at the doctor probe registry.
- No behavior change to the resolved model or the doctor output.
