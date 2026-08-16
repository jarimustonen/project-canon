---
created: 2026-08-16
updated: 2026-08-16
type: bug
status: open
priority: high
---

# Release workflow fails: profile `dist` is not defined

## Description

The `Release` workflow has failed on **both** releases cut so far — v0.2.0
([run 31956531877](https://github.com/jarimustonen/project-canon/actions/runs/31956531877))
and v0.3.0
([run 31962262265](https://github.com/jarimustonen/project-canon/actions/runs/31962262265)).
Every `build-local-artifacts` matrix leg dies identically:

```
build-local-artifacts (aarch64-apple-darwin)      error: profile `dist` is not defined
build-local-artifacts (x86_64-unknown-linux-musl) error: profile `dist` is not defined
build-local-artifacts (aarch64-unknown-linux-musl) error: profile `dist` is not defined
##[error]Process completed with exit code 255
```

## Root cause

The cargo-dist release job builds with `--profile dist`, but the root
`Cargo.toml` declares no `[profile.dist]` section. `cargo-dist` normally injects
it when it generates the workflow; here the workflow was committed without the
matching manifest change, so the build fails before it produces a single binary.

## Fix

Add to the workspace root `Cargo.toml`:

```toml
[profile.dist]
inherits = "release"
lto = "thin"
```

Then re-run the release job for the current tag (or cut a patch release) so the
artifacts land on the existing v0.3.0 release.

## Impact — this is the blocking part

`Publish to crates.io` succeeds, so the crate is on crates.io and the GitHub
release object exists. What does **not** exist is any **binary artifact** on
that release: every platform build failed. The fleet's Linux hosts install
shared CLIs from the GitHub release installer, and the standing policy is to
install only from Homebrew or release artifacts, never `cargo build` as a
workaround — so until the `dist` profile is defined, `project-canon` cannot be
installed on a Linux fleet host by the sanctioned path at all.
