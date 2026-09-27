---
created: 2026-09-27
updated: 2026-09-27
type: bug
reporter: agent
status: in-progress
priority: normal
---

# Isolate cargo-dist on self-hosted macOS release runner

_Source: .github/workflows/release.yml_

## Description

## Observation

`.github/workflows/release.yml` runs `matrix.install_dist.run` on a `self-hosted` macOS ARM64 runner (`dist-workspace.toml` github-custom-runners). cargo-dist's generated installer writes `dist` to the runner user's persistent `~/.cargo/bin`. A real 2026-09-24 Taskfleet job 107705797379 logged `installing to /Users/jari/.cargo/bin`, showing the same configuration causes persistent runner pollution. Project Canon and issuectl have not yet run this specific release under a new isolation guard; avoid claiming an observed occurrence here.

## Expected

Install the pinned cargo-dist into a unique job-scoped directory on macOS (`RUNNER_TEMP`), verify resolved `dist` is from that directory, and leave the user's persistent Cargo toolchain and all Linux/hosted runners unchanged. Preserve this override through cargo-dist workflow regeneration. Test isolation and generated workflow consistency; only cut a release after this gate is green.

## Decisions

### 2026-09-27T06:38:18Z · @agent

Design: retain cargo-dist 0.33.0 and its matrix installer; override only the self-hosted macOS local-artifact step with an atomic RUNNER_TEMP root, CARGO_DIST_INSTALL_DIR + CARGO_DIST_NO_MODIFY_PATH, and a command -v guard before publishing GITHUB_PATH. The pinned installer maps INSTALL_DIR to <root>/bin. Rejected: modifying all jobs or plan (hosted runners are ephemeral and plan cache depends on its existing path); hand-editing generated YAML without a drift gate (regeneration discards it); persistently installing a test dist in the user Cargo home. allow-dirty = ["ci"] plus disposable-workspace pinned generation and full-file comparison preserves the scoped override. Hermetic mocked installers cover concurrency, success, curl failure, wrong-path fallback and missing temp; independent CI also checks the dist plan runner topology.
