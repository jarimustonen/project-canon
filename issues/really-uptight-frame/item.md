---
created: 2026-09-18
updated: 2026-09-18
type: chore
status: untriaged
priority: normal
provenance: other
provenance_detail: Taskfleet implementation brief
source_ref: taskfleet:01m2sm2cvpcd9cfwxwd5210nwr/task:upgrade-cargo-dist-0.33.0
originating_run: 01m2sm2cvpcd9cfwxwd5210nwr
originating_run_kind: spinoff
---

# Upgrade cargo-dist release workflow to 0.33.0

## Description

## Context

The generated release workflow still installs cargo-dist 0.28.2. The approved upgrade target is cargo-dist 0.33.0 so release planning uses the current stable generator without changing the repository's release targets or publishing policy.

## Scope

- Set `cargo-dist-version` to `0.33.0` in `dist-workspace.toml`.
- Regenerate `.github/workflows/release.yml` with cargo-dist 0.33.0.
- Preserve the tag trigger, all three build targets, the custom macOS runner, shell and Homebrew installers, Homebrew publishing, and GitHub attestations.
- Do not cut or publish a release.

## Verification

- `dist generate --check`
- `dist plan --output-format=json`
- Parse the generated workflow as YAML.
- Run the repository green gate.

## Decisions

### 2026-09-18T06:45:00Z · @taskfleet:01m2sm2cvpcd9cfwxwd5210nwr

Upgraded only the declared cargo-dist version and regenerated the workflow with the mandated disposable cargo-dist 0.33.0 binary. Kept `dist-workspace.toml` as the source of truth because direct workflow edits would be overwritten and could diverge from the release plan. Preserved the existing targets, custom self-hosted macOS runner, shell and Homebrew installers, Homebrew publishing job, Git tag trigger, and build attestations.

Rejected alternatives: a global cargo-dist install would mutate the machine and violate the task's reproducibility boundary; hand-editing only the installer URL would leave the generated workflow stale; running `shipshape dist generate` would strip the repository-specific custom runner block; cutting or testing via a real release tag would publish the prepared release and is explicitly out of scope.
