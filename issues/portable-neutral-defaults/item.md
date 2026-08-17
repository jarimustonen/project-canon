---
created: 2026-08-16
updated: 2026-08-16
type: bug
status: fixed
priority: high
labels: [canon]
lane: canon-rollout
lane_seq: 10
commits:
- hash: 4b61e84
  summary: use neutral environment defaults
- hash: cb042d0
  summary: neutralize optional tw integration
closed: 2026-08-16
---

# Neutral built-in defaults — no user specifics in a public artifact

## Description

## Description

`project-canon` is a public, crates.io-published, generic OSS tool — but it ships one user's
environment as **built-in defaults** in `project-canon-core`. A public artifact must not carry
user-specific facts at all; those belong in user configuration.

### Leaked values (all in `crates/project-canon-core/src/env.rs`)

```rust
gh_account: "<maintainer-account>".to_string(),          // EnvConfig::builtin_defaults
repo_root:  "<personal-repo-root>".to_string(),
const DEFAULT_FAMILY_TOOLS: [&str; 7] = [
    "issuectl", "orchestratectl", "crmctl", "tilictl", "ossctl", "intakectl", "glasspad",
];
```

Plus `crates/project-canon-cli/src/new.rs` generates scaffold content pointing at
`github.com/jarimustonen/issuectl`, and the same values are asserted in
`crates/project-canon-core/tests/env_config.rs` and `src/env.rs` unit tests.

**Three of the seven named tools are PRIVATE repositories** — `crmctl`, `tilictl`, `intakectl`
(verified via `gh repo view`). A public crate therefore discloses the names of private
projects.

### Why the existing design does not cover this

`env.rs`'s own doc comment states the intent: *"The defaults preserve today's homebase
behavior, but they now live in one place and every value is overridable — that, plus core no
longer hardcoding any of them, is the portability win."* Overridability solved **portability**
but not **publicness**: an unset default is still whatever ships in the package. For a public
artifact the correct default is *absent/neutral*, not "the maintainer's environment".

### Aggravating factor

`config show --json` (§8, shipped in 0.2.0) promotes these values from buried source constants
to a documented, first-class output surface — increasing exposure rather than reducing it.

### Affected releases

Present in `0.1.1` and `0.2.0`, both live on crates.io and Homebrew.

## Acceptance

- No user-specific value (gh account, personal repo-root convention, private repo/tool names)
  appears anywhere in shipped source, defaults, generated scaffold output, docs, or tests.
- Built-in defaults are neutral: no gh account (`None`/unset), no assumed repo-root convention,
  empty family-tool set. A missing required value produces an actionable canon error telling
  the operator which config key or env var to set — never a silent guess at someone's layout.
- The homebase/family environment is expressible **entirely** through the existing
  `defaults → file → env` layer, i.e. via a user config file outside the repo. Jari's setup
  must keep working through that path.
- Tests assert the neutral defaults and cover the "user config supplies the family map"
  path with fixture values (`example-user`, `~/Projects`, fake tool names) — no real account
  or private repo names in fixtures either.
- `config show --json` on a machine with no user config shows neutral values.
- A documented example config (`example`/docs) shows how to express a family setup, using
  clearly fictional values.

## Comments

Found 2026-08-16 immediately after the 0.2.0 release, when the maintainer asked whether the
family tool list belonged in a public repo at all. Disclosure is limited to repo *names* (no
code, no contents); 11 total downloads at the time of discovery. The forward fix (a clean
follow-up release) is preferred over yanking, since crates.io retains published files
permanently and a yank would not remove the names.

This issue is the concrete cleanup. The general rule it implies — *a public repo must not
reference user-specific facts; they belong in user config* — should also become a canon
section so `doctor`/`review` can enforce it family-wide (see `canon-no-user-specifics`).
