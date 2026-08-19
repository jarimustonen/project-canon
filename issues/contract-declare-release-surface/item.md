---
created: 2026-08-19
updated: 2026-08-19
type: bug
reporter: agent-ossctl-stint-23
status: open
priority: high
---

# contract under-declares the release surface (no homebrew target); blocked on the gh-releases verify lookup bug

## Description

## Description

This repo's contract **under-declares its release surface**, and it is additionally the
repo currently hit by a HIGH engine bug that makes a successful cut report failure. Read
the blocker section before changing anything.

Current state:

```yaml
targets:
  - {ecosystem: rust, package: project-canon-core, registry: crates.io,   adapter: cargo-publish}
  - {ecosystem: rust, package: project-canon-cli,  registry: crates.io,   adapter: cargo-publish}
  - {ecosystem: rust, package: project-canon-cli,  registry: gh-releases, adapter: cargo-dist}
```

**The Homebrew channel is missing.** `dist-workspace.toml` carries
`publish-jobs = ["homebrew"]`, so cargo-dist writes `jarimustonen/homebrew-project-canon`
on every tag, unseen by the engine. Declaring it as `adapter: homebrew-tap` was not an
option before ossctl 0.8.0, because that makes the engine write the same formula cargo-dist
already writes — see issuectl's `homebrew-double-writer-contract` for the cost.

## ⚠️ Blocker: verify reports this repo's GitHub Release as missing

ossctl `verify-gh-release-missing` (HIGH, confirmed present in ossctl **0.9.0**) was
observed on **both** of this repo's 2026-08-17 releases, v0.4.0 and v0.5.0, and reproduces
on demand afterwards. A fully published GitHub Release with all assets verifies as
`missing`, so `release cut` exits non-zero and tells the operator to reconcile manually —
for a release where everything actually landed. It is a lookup bug, not a timing race: the
release existed for ~18 minutes of the 20-minute polling window and was never seen.

**This repo is very likely the reason the bug is visible.** The suspected cause is a
package-vs-project naming mismatch in the release lookup, and this is the one fleet repo
that has one: the binary package is `project-canon-cli` while the project, the tag, and the
tap are all `project-canon`. issuectl, glasspad, and orchestratectl all have a binary
package name equal to the project name, and none of them shows the fault.

So: fix the contract by all means, but **expect a false-red on the gh-releases target until
`verify-gh-release-missing` is fixed**, and do not let an autonomous agent act on that exit
code by retrying or hand-reconciling crates.io. That reconciliation is irreversible.

## Fix

```yaml
targets:
  - {ecosystem: rust, package: project-canon-core, registry: crates.io,   adapter: <see below>}
  - {ecosystem: rust, package: project-canon-cli,  registry: crates.io,   adapter: <see below>}
  - {ecosystem: rust, package: project-canon-cli,  registry: gh-releases, adapter: cargo-dist}
  - {ecosystem: rust, package: project-canon-cli,  registry: homebrew,    adapter: cargo-dist}
distribution:
  adapter: cargo-dist
  gh_releases: true
  installers: [shell, homebrew]
  homebrew_tap: jarimustonen/homebrew-project-canon
  platforms:
    - aarch64-apple-darwin
    - aarch64-unknown-linux-musl
    - x86_64-unknown-linux-musl
```

**One thing to determine first, which I could not settle from the repo:** whether this
repo's crates.io publish is local or CI-performed. It has a `publish-crates.yml`, but
unlike orchestratectl its contract does not state which is authoritative. If CI publishes,
use `adapter: cargo-publish-ci` on both crates; if the releaser publishes locally, keep
`cargo-publish`. Do not guess — a wrong answer either double-publishes or pushes a tag and
waits 20 minutes for a publish nobody performs.

## Acceptance

- [ ] Homebrew target declared, plus a `distribution:` block
- [ ] The local-vs-CI crates.io question is answered and recorded in the contract's rationale
- [ ] `ossctl contract validate` passes
- [ ] Once `verify-gh-release-missing` is fixed, a cut verifies all four targets and exits zero

## Reference: a correctly-declared contract

`ossctl`'s own `OSS-RELEASE.md` is the worked example of a **fully-declared release
surface** — every channel the repo actually ships on appears as a target, plus a
`distribution:` block that matches its `dist-workspace.toml`:

```yaml
targets:
  - {ecosystem: rust, package: ossctl-core, registry: crates.io,   adapter: cargo-publish}
  - {ecosystem: rust, package: ossctl,      registry: crates.io,   adapter: cargo-publish}
  - {ecosystem: rust, package: ossctl,      registry: gh-releases, adapter: cargo-dist}
  - {ecosystem: rust, package: ossctl,      registry: homebrew,    adapter: homebrew-tap}
distribution:
  adapter: cargo-dist
  gh_releases: true
  installers: [shell]
  homebrew_tap: jarimustonen/homebrew-ossctl
  platforms:
    - aarch64-apple-darwin
    - aarch64-unknown-linux-musl
    - x86_64-unknown-linux-musl
```

**Copy the SHAPE, not every adapter.** ossctl deliberately differs on one line, and
copying it would recreate the bug this issue is about:

| | ossctl | this repo |
|---|---|---|
| `dist-workspace.toml` | `installers = ["shell"]`, **no** `publish-jobs` | `installers = [..., "homebrew"]`, `publish-jobs = ["homebrew"]` |
| who writes the tap | the ossctl engine, in its `dist` phase | **cargo-dist's CI job**, on every tag |
| correct homebrew adapter | `homebrew-tap` (engine-owned) | **`cargo-dist`** (CI-delegated) |

The engine-owned formula carries a first-line marker
(`# Generated by ossctl; do not edit by hand`); a CI-written one does not, and must not
be required to. That is why the adapter has to say who the writer is.

Fleet context and the full per-repo audit:
`homebase/issues/cross-repo-release-standardisation/audit-2026-08-17.md`.

## Target state: the fleet ships uniformly

All four public fleet repos (`issuectl`, `glasspad`, `orchestratectl`, `project-canon`)
should end up with the same declared shape, since all four already carry
`publish-jobs = ["homebrew"]` and their own `publish-crates.yml`:

```yaml
targets:
  - {ecosystem: rust, package: <core>, registry: crates.io,   adapter: <cargo-publish | cargo-publish-ci>}
  - {ecosystem: rust, package: <bin>,  registry: crates.io,   adapter: <cargo-publish | cargo-publish-ci>}
  - {ecosystem: rust, package: <bin>,  registry: gh-releases, adapter: cargo-dist}
  - {ecosystem: rust, package: <bin>,  registry: homebrew,    adapter: cargo-dist}
distribution:
  adapter: cargo-dist
  gh_releases: true
  installers: [shell, homebrew]
  homebrew_tap: <tap>
  platforms: [aarch64-apple-darwin, aarch64-unknown-linux-musl, x86_64-unknown-linux-musl]
```

`intakectl` is the deliberate exception: it publishes nothing and uses `targets: []`.
`ossctl` is the other exception: it keeps the engine-owned tap, since it is the only
live exercise of that path.

## Required tool version

`ossctl 0.9.0` or newer. The CI-delegated homebrew adapter shipped in 0.8.0 and the
`cargo-publish-ci` adapter in 0.9.0. Check with `ossctl --version` (which also only
started working in 0.8.0).
