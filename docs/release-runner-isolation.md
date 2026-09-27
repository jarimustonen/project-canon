# Release runner isolation

The `aarch64-apple-darwin` local-artifact build uses a self-hosted macOS runner.
Cargo-dist 0.33.0 normally installs the matrix-selected `dist` into the runner's
persistent Cargo home. The release workflow therefore overrides **only** that
job's local install step: an atomic `mktemp` directory beneath `RUNNER_TEMP` is
passed as `CARGO_DIST_INSTALL_DIR` with `CARGO_DIST_NO_MODIFY_PATH=1`. The pinned
v0.33.0 installer interprets this as a Cargo-home layout (`<root>/bin/dist`).
The job fails unless `command -v dist` resolves to that exact binary, then adds
only that bin directory to `GITHUB_PATH`. Linux local builds and the hosted plan
job retain cargo-dist's generated steps. No cleanup of the user's Cargo home is
required or permitted. The guard detects a fallback install, but cannot undo a
misbehaving installer's writes; keep the installer pinned and review any upgrade.

Cargo-dist has no per-job pre-install hook. `allow-dirty = ["ci"]` permits this
intentional override, but also exempts it from cargo-dist's native CI drift
check. To regenerate, run:

```sh
python3 scripts/release_workflow.py --write --dist /path/to/pinned/dist
python3 tests/release_workflow.py
python3 scripts/release_workflow.py --check --dist /path/to/pinned/dist
```

The script generates in a disposable copy, requires the exact original install
step once, applies a single anchored replacement, and compares **the entire**
result to the checked-in workflow. It refuses a different cargo-dist version.
The independent `release-guard.yml` CI runs the hermetic tests and generation
comparison with a checksum-verified, temp-only cargo-dist binary; it catches
both unexpected generated changes and lost overrides. Review and update the
checksum and anchor when intentionally upgrading cargo-dist. Do not run raw
`dist generate` in this checkout: it would overwrite the override.
