---
created: 2026-08-17
updated: 2026-08-17
type: improvement
status: in-progress
priority: normal
labels: [canon]
lane: canon-rollout
lane_seq: 40
commits:
- hash: 907ea59
  summary: make --version a full version verb alias
---

# --version must be a full alias of the version verb

## Description

## Description

`<tool> --version` should be **equivalent to** `<tool> version`, not a degraded text-only alias.

Maintainer decision, 2026-08-16: the family uses a noun-verb surface, and `version` is one of
the verbs — but people are so used to typing `<command> --version` that the flag form must
behave the same as the verb.

**This requires a canon amendment as well as a code change** — see *Canon conflict* below.

### Current behaviour (project-canon 0.3.0)

```
$ project-canon --version
project-canon 0.3.0                          # exit 0

$ project-canon version
project-canon 0.3.0                          # exit 0  — same, good

$ project-canon version --json
{"schema_version":1,"tool":"project-canon","version":"0.3.0","commit":"…", …}   # exit 0

$ project-canon --version --json
{"schema_version":1,"error":{"code":"usage_error",
  "message":"--version is text-only; use `project-canon version --json` for structured version data"}}
```

So the two forms agree in text mode and diverge under `--json`. The divergence is deliberate:
`--help --json` documents the flag as *"Print the human-readable version (text-only
compatibility alias)."*

### Canon conflict — must be resolved first

`AGENTS-AI-FIRST-CLI.md` §10 currently **mandates** the present behaviour:

> The structured contract lives on the **`version` subcommand**; clap's parser-level
> `--version` flag (which prints plain text and cannot honor `--json`) is at most a text
> convenience alias — an agent that needs the payload always calls `<tool> version --json`,
> never `<tool> --version --json`

The current implementation is therefore canon-conformant *by design*, and changing it without
amending the canon would make `project-canon review`/`doctor` flag the repo against its own
published rule.

The canon's stated justification — that the flag "cannot honor `--json`" — does not hold.
That is true of clap's built-in `version` action, but a tool can declare `--version` as an
ordinary flag and dispatch it itself. project-canon already intercepts it, which is how it
produces the custom error above instead of clap's default output. The real constraint is
convention, not the parser.

## Acceptance

- `<tool> --version` and `<tool> version` produce **identical** output and exit code in every
  mode, including `--json`: `--version --json` emits the same §10 payload as `version --json`.
- Argument order does not matter: `--json --version` and `--version --json` behave alike.
- The usage error that currently steers callers from the flag to the verb is removed.
- `--help --json` no longer describes `--version` as "text-only"; it documents the flag as an
  alias of the verb.
- Canon §10 is amended: the flag form is a **full alias** of the `version` subcommand rather
  than "at most a text convenience alias", and the incorrect "cannot honor `--json`" rationale
  is dropped. Keep the guidance that the verb is the canonical form agents should prefer —
  the change is that the flag is no longer *lesser*, only *less preferred*.
- Golden tests cover both spellings in both modes, asserting they agree.
- `project-canon review --assume-defaults .` still reports zero confirmed gaps afterwards.

## Comments

- Consider whether the same equivalence should apply to `--help` vs a `help` verb, for
  consistency. Not in scope here — flag it if the canon amendment makes the asymmetry look odd.
- The canon amendment lands the same rule on every family CLI, so expect follow-up work in the
  sibling tools. Their §10 conformance will start reporting a gap once the canon changes; that
  is intended, and is how the family stays aligned.
- Sequence against `canon-no-user-specifics` — both edit the canon master
  (`crates/project-canon-core/AGENTS-AI-FIRST-CLI.md`) and the shared CLI surface, so they
  collide and must not run in parallel.
