---
created: 2026-08-16
updated: 2026-08-16
type: feature
status: open
priority: high
labels: [canon]
lane: canon-rollout
lane_seq: 20
blocked_by: ['@portable-neutral-defaults']
---

# Canon rule: public artifacts must not embed user-specific facts

## Description

## Description

Make "a public artifact must not embed user-specific facts" a **binding canon rule**, so it is
enforceable rather than a thing each repo remembers or forgets.

Maintainer decision, 2026-08-16: *"repo ei saa viitata käyttäjäspesifeihin asioihin lainkaan
jos se on public — käyttäjäspesifit asiat pitää puskea käyttäjäkonffeihin"* — this should be
outright a rule.

### Motivation

`project-canon` itself shipped `gh_account: "jarimustonen"`, `repo_root: "~/Sources"`, and a
7-tool family list naming three **private** repositories as built-in defaults, in a public
crate published to crates.io (see `portable-neutral-defaults`). The tool that audits the
family's conformance violated the rule the family needed. That is exactly the class of defect
a canon section plus a mechanical `doctor` check exists to prevent.

The existing §8 (configuration precedence: flag > env > file > built-in default) covers *where
values come from* but says nothing about *what a built-in default is allowed to contain* in a
publicly-distributed artifact. That is the gap.

### Proposed shape

A new canon section (next free number, §23) — draft framing, to be refined during
implementation:

- **Rule:** a publicly-distributed artifact (published package, public repo, generated
  scaffold output, installed skill content) MUST NOT contain user-specific or
  deployment-specific facts: personal account handles, private repo/project names, personal
  filesystem-layout conventions, hostnames, internal URLs, or org-internal identifiers.
- **Built-in defaults MUST be neutral.** Where the family's §8 layering supplies a built-in
  default, that default must be portable-generic. "Absent, with an actionable error naming the
  config key to set" is correct; "the maintainer's environment" is not — overridability does
  not launder a user-specific default, because unset still means shipped.
- **The environment goes in user config.** Site/user specifics are expressed through the §8
  file/env layers, which live outside the distributed artifact.
- **Examples and fixtures use obviously fictional values** — no real accounts, no real private
  repo names, in docs, tests, or golden files.
- **Applies to generated output too**: scaffolds a tool emits, and skill content it installs,
  inherit the rule.

Consider whether the rule should be scoped explicitly to *public* distribution or stated
unconditionally with public distribution as the sharp case — an internal-only tool embedding
its own org's layout is arguably fine, and the canon should say which it means rather than
leaving it to the reader.

### Enforcement

- `doctor` gains a mechanical check (MUST-level, so it fails CI) for the detectable subset:
  scan shipped source/defaults/scaffold templates for configured "user-specific" markers.
  The check needs a way to know what counts as user-specific for a given repo — likely a
  configured deny-list of the operator's own account/handle/private names supplied via the
  §8 user-config layer, which keeps the check itself free of user specifics.
- `review` reports the judgement-call remainder (hostnames, internal URLs, borderline naming).

### Acceptance

- A new canon section is added to `crates/project-canon-core/AGENTS-AI-FIRST-CLI.md` with
  normative MUST/SHOULD language matching the surrounding sections' style and numbering.
- The `cli` profile picks the section up, and `project-canon review` surfaces it.
- A `doctor` check exists for the mechanically detectable subset, with the deny-list sourced
  from user config (never from a list baked into this repo).
- `project-canon` passes its own new check once `portable-neutral-defaults` lands.
- The canon's own version/skill metadata reflects the added section.

## Comments

Sequence with `portable-neutral-defaults`: that issue is the concrete cleanup, this one is the
general rule and its enforcement. The rule should land in a state where project-canon itself
passes it — a canon section the home repo violates is worse than none.
