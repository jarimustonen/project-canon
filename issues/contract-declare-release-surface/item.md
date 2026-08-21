---
created: 2026-08-19
updated: 2026-08-21
type: bug
reporter: agent-ossctl-stint-23
status: in-progress
priority: high
lane: release-surface
commits:
- hash: 967d06f3cd61b24e1e979ae6b139ac4db6d2e25f
  summary: declare complete release surface and retire duplicate crates.io workflow
---

# declare the complete release surface and resolve the doubled publish path

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

- [x] Homebrew target declared, plus a `distribution:` block
- [x] The local-vs-CI crates.io question is answered and recorded in the contract's rationale
- [x] `ossctl contract validate` passes
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

## Comments

### 2026-08-21T07:31:45Z · @agent

**The "local or CI publish?" question is now ANSWERED — and the answer is "both", which is itself a defect worth folding into this issue.**

Evidence gathered 2026-08-21 (owner decided to do this work once the ossctl verify fix lands):

- `.github/workflows/publish-crates.yml` triggers on the **version-tag push** (`v[0-9]+.[0-9]+.[0-9]+*`) and publishes both crates to crates.io in dependency order, from CI, using a `CARGO_REGISTRY_TOKEN` repo secret. Its header comment states the intent explicitly: *"crates.io publishing happens in CI with no dependency on a local token."* It deliberately does NOT key off `release: published`, because cargo-dist creates the Release with `GITHUB_TOKEN` and GitHub emits no workflow event for that.
- **But `ossctl release cut` also publishes crates.io locally**, in its `publish` phase, which runs *before* the `tag` phase. The 2026-08-17 run journals for v0.4.0 and v0.5.0 both record `target_published` receipts for `project-canon-core` and `project-canon-cli` with `registry_url` values — i.e. the local publish really happened.
- So every release currently runs **two** crates.io publish paths: ossctl locally first, then the tag push firing `publish-crates.yml`. `gh run list --workflow=publish-crates.yml` shows that workflow **succeeded** on v0.3.2, v0.3.3, v0.4.0, v0.5.0 and v0.6.0 (23–36s each), so the second path is not erroring loudly — worth checking whether it detects the already-published version and no-ops, or whether the duplicate is being swallowed.

**Implication for the fix in this issue:** the contract currently declares `adapter: cargo-publish` (local), which matches ossctl's behaviour but contradicts the CI workflow's stated intent. Picking `cargo-publish-ci` for both crates would match the workflow — but do **not** simply switch the adapter and leave both paths live. Decide which path is authoritative and **retire the other**, otherwise the double-publish stays, just re-labelled. This is the same class of defect as the missing Homebrew declaration: reality and the declared surface disagree.

Also note this makes the repo's release even harder to reason about while `verify-gh-release-missing` is unfixed — a false-red on gh-releases plus two live publish paths is a bad combination for an autonomous agent to act on.

### 2026-08-21T09:37:06Z · @agent

#### Release-surface decision

Verified the former blocker before changing the contract: `ossctl version` reports 0.10.0, and `ossctl release verify 01M08P4D4HK25MRQXDE0XDW9NJ --json` reconciles the historical v0.5.0 run with 3 matches, 0 conflicts, and 0 missing, including `rust:project-canon-cli:gh-releases`. The blocker is cleared.

I chose the ossctl engine's local `cargo-publish` path as the sole crates.io writer and deleted `.github/workflows/publish-crates.yml`. This keeps publication inside the sealed release transaction: `dry-run-all` runs before publication, core→CLI ordering remains engine-controlled, registry receipts are journaled, verification observes the exact publish operation, and tagging happens only after both crates publish. It also matches the approved operating policy's engine-driven release and recovery model.

The rejected alternative was changing both targets to `cargo-publish-ci` and retaining the tag-triggered workflow. That would avoid requiring a local token, which was the workflow's original stated intent, but would move the irreversible writes outside the engine that coordinates and journals the cut. It also makes a tag the trigger for publication, weakening the engine's direct ordering and recovery guarantees. Keeping both paths was not considered valid because it preserves the defect.

The duplicate workflow succeeded because it explicitly swallowed only cargo's duplicate-version error. The v0.5.0 Actions log says `crate project-canon-core@0.5.0 already exists on crates.io index` and then `treating as success`, followed by the same sequence for `project-canon-cli@0.5.0`. Its `publish()` shell function regex-matched that diagnostic and returned zero; it was a deliberate no-op after ossctl had already published, not a second successful registry write.

Homebrew remains cargo-dist-owned: the new `registry: homebrew, adapter: cargo-dist` target declares and verifies the existing `publish-jobs = ["homebrew"]` behavior without introducing an engine `homebrew-tap` double writer. `ossctl contract validate --require-approved` passes, and sealed plan `793dd6de08f9ae59856af475504d248b94cbb4578ec77c2a07efcb2ae173af6d` contains all four targets, including Homebrew.

### 2026-08-21T09:51:15Z · @agent

#### Review correction: Homebrew formula identity

The required `/llm-review` and `/assess-findings` pass found one substantive flaw in the initial contract edit: the Homebrew target used the Rust package identity `project-canon-cli`, but cargo-dist publishes `Formula/project-canon.rb` (`formula = "project-canon"`). Read-only inspection of ossctl 0.10.0 confirmed that delegated Homebrew verification passes `target.package` directly to `verify_tap_formula`, which fetches `Formula/<package>.rb`. Leaving the proposed Rust target unchanged would therefore have sealed a target that later observed the wrong formula.

I applied the supported representation: `ecosystem: binary, package: project-canon, registry: homebrew, adapter: cargo-dist`, while the two crates.io targets remain Rust `cargo-publish` targets and the GitHub Release remains attached to `project-canon-cli`. The `binary` target is a distribution identity and does not need a Rust manifest version. `ossctl contract validate --require-approved` passes, and final sealed plan `94187d85f6b7df1f48a88c218dc552914096f5d1ba8c2b0c40d4918b4ffb8c52` contains exactly four targets with Homebrew as `binary:project-canon`/`cargo-dist`.

The review also led to explicit operating-policy text: ossctl is the sole crates.io writer, the release host must carry valid registry credentials, the tag-triggered crates.io workflow is deliberately absent, and release tags come only from `ossctl release cut` or `ossctl release resume`. The rejected review proposals were an immediate four-target release (forbidden by this task), deleting an external GitHub secret (destructive and not authorized), and removing the hand-written changelog fragment (the repository's fragment README explicitly permits it).
