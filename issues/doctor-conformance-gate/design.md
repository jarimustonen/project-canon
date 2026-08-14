# `doctor` — mechanical conformance gate (design note)

The FIRST verb of `project-canon`. Read-only, non-interactive, CI-shaped. It reads the
two-layer model from `project-canon-core`, resolves the target repo's profile, runs the
**mechanically-decidable** probes, emits a pass/fail matrix, and exits non-zero on a
mechanically-decided MUST gap. Mirrors canon **§18** (doctor discipline) and **§2**
(exit-code discipline). No LLM/human judgment — that is `review`'s job (ADR 0009 §2).

## Command surface

```
project-canon doctor [--profile <archetype>] [--assume-defaults] [--json] [--verbose] [--help] [<repo>]
```

| flag/arg | default | meaning |
|---|---|---|
| `<repo>` (positional) | `.` (cwd) | The target repo to characterize. Read-only. Must be an existing directory, else exit `2`. |
| `--profile <archetype>` | `cli` | Which profile layers onto base. One of `cli`/`service`/`library`/`release`. Unknown value → exit `2`. |
| `--assume-defaults` | (implied) | Characterize non-interactively with conservative questionnaire defaults (all conditional triggers **off**). This is the *only* mode at v0 (§3 — doctor never prompts); the flag makes it explicit and reserves the seam for a future characterization path. |
| `--json` | off | Emit the §10 structured report on stdout instead of the human matrix. |
| `--verbose` | off | Also list `skipped` checks (n/a + deferred) in the human matrix; §2 diagnostic toggle. |
| `--help` | — | Usage, exit `0` (§2 help/version are exit-0 events). |

Strict input validation (§1): unknown flags, a `--profile` value outside the archetype set,
a missing value for `--profile`, a repeated flag, or an extra positional argument are all
errors that **echo the offending token** and exit `2` — never a silent fixup.

### Default profile / characterization

`--profile` defaults to **`cli`**: it is the only profile with content at v0 and the family's
reason to exist; the base scaffold dimensions are archetype-invariant regardless. True
*mechanical archetype auto-detection* (and mechanical answering of the eight Q1–Q8
characterization questions) needs judgment and is therefore **out of scope for doctor** —
it belongs to `review`, or to a later heuristic pass. At v0 doctor always resolves with the
conservative default questionnaire (all conditionals `false`), so every `MustWhenApplies`
section is `n/a` unless a future flag turns it on. This is deliberately conservative: doctor
never asserts a conditional requirement it cannot mechanically prove applies.

## How a dimension becomes a mechanical probe

The core model gives each `Dimension` a `Probe` with an `EffectClass` (`Static`, `ExecRo`,
`SandboxWrite`). Doctor grades a dimension **only when a decidable file/structure check
exists** for it — grep / file-existence / repo-shape. It never builds or runs the target
tool, so an `ExecRo`/`SandboxWrite` probe (which requires *running the target's binary*) is
**not** a doctor probe; nor is any dimension needing prose/LLM judgment. Those are reported
as `skipped` with reason `deferred-to-review`, **never as a fail**.

The mechanical-probe registry (`doctor::probes`) maps a dimension id → a decidable check:

| dimension | severity | check | on miss |
|---|---|---|---|
| `base.doc-pattern` | MUST | `AGENTS.md` **and** `CLAUDE.md` both resolve (following symlinks) to a regular file at root | FAIL |
| `base.issue-tracking` | MUST | `issues/` directory exists | FAIL |
| `base.git-hygiene` | MUST | `.git` exists (dir or gitfile) | FAIL |
| `base.readme` | SHOULD | `README.md` exists | WARN |
| `base.gitignore` | SHOULD | `.gitignore` exists | WARN |
| `canon.s22` | SHOULD | a `crates/*-core` **and** a `crates/*-cli` dir exist (core/cli split) | WARN |

Every other resolved dimension that *applies* has no mechanical probe at v0 and is reported
`skipped` / `deferred-to-review`. This honestly under-claims rather than over-claiming
mechanical conformance — the explicit anti-goal is a doctor that pretends to mechanically
grade a behavioral section (§1–§7, §9, §10, §14, …) it can only judge by running the tool.

Severity → gating: a **MUST** (or a `MustWhenApplies` whose trigger is on) that mechanically
FAILs is a *gap* that flips the exit code. A **SHOULD** miss is a WARN and **never** gates
(cli-canon's "SHOULD is never a hard gate"). An `n/a` conditional is never a fail.

## Pass/fail matrix

Per resolved dimension, one row with a status in `{ok, warn, fail, skipped}` (mirrors §18's
`OK/WARN/FAIL`, plus `skipped` for n/a + deferred). A row `gates` iff a `fail` on it flips
the exit code (mechanical probe present **and** severity is MUST / applicable-MustWhenApplies).

Human (default) prints only graded rows (`ok`/`warn`/`fail`) + a summary line; `--verbose`
also lists `skipped` rows. `--json` always includes every row.

```
OK    base.git-hygiene       git repository present
FAIL  base.doc-pattern       AGENTS.md missing at repo root
WARN  base.readme            README.md missing
summary: 3 ok, 1 warn, 1 fail, 16 skipped  →  1 mechanical MUST gap  (non-conformant)
```

## `--json` output schema (§10-shaped)

```json
{
  "schema_version": 1,
  "tool": "project-canon",
  "verb": "doctor",
  "target": "/abs/path/to/repo",
  "profile": "cli",
  "surface_shape": "flat-verb",
  "checks": [
    {
      "id": "base.doc-pattern",
      "title": "Consolidated AGENTS.md per directory, with CLAUDE.md a symlink to it",
      "severity": "must",
      "layer": "base",
      "canon_section": null,
      "status": "fail",
      "gates": true,
      "message": "AGENTS.md missing at repo root",
      "reason": null,
      "gated_by": null
    },
    {
      "id": "canon.s11",
      "title": "Dry-run, idempotency, retry safety",
      "severity": "must-when-applies",
      "layer": "profile:cli",
      "canon_section": 11,
      "status": "skipped",
      "gates": false,
      "message": "n/a — conditional trigger off",
      "reason": "not-applicable",
      "gated_by": "Q3"
    }
  ],
  "summary": {"ok": 3, "warn": 1, "fail": 1, "skipped": 16, "gaps": 1},
  "conformant": false,
  "exit_code": 1
}
```

- `severity` ∈ `must | must-when-applies | should`; `layer` ∈ `base | profile:<archetype>`.
- `status` ∈ `ok | warn | fail | skipped`; `reason` (skipped only) ∈ `not-applicable |
  deferred-to-review`; `gated_by` (n/a only) is the `Qn` label.
- `surface_shape` ∈ `noun-verb | flat-verb | null` (null for non-CLI profiles).
- Data → stdout, diagnostics → stderr (§2). JSON is emitted by a small in-crate escape-correct
  writer, keeping the workspace dependency-free (core's "serde is a deferred seam" ethos).

## Exit-code contract

Doctor is a **gate**, so it follows the `diff`/`grep`/`test` convention — a designed non-zero
for "gate tripped", distinct from a non-zero for "couldn't evaluate":

| exit | meaning |
|---|---|
| `0` | conformant — every mechanically-decided MUST passed |
| `1` | non-conformant — ≥1 mechanically-decided MUST gap (the gate's designed non-zero; §18 "fail → 1") |
| `2` | usage / operational error — unknown flag, bad `--profile`, missing/unreadable target repo, an **I/O fault while probing** (permission denied, transient error, target vanished mid-run), or malformed `PROJECT_CANON_*` env |

This deliberately reserves `1` for the gate result and folds *all* invocation/operational
errors (both §2's caller-actionable and system buckets) into `2`, so CI can branch cleanly on
"repo is non-conformant" (`1`, actionable: fix the repo) vs. "doctor could not run" (`2`,
actionable: fix the invocation). This is a conscious reconciliation of the §2 (1 = caller
error) / §18 (1 = fail) tension in favour of the gate use-case; recorded as a discussion item.
`--help` is exit `0` (§2) — it short-circuits before env validation, so a malformed environment
never blocks help. Probes distinguish `NotFound` (a genuine conformance miss) from any other I/O
error (an operational fault that routes to `2`), so a `chmod 000` target is never mis-reported as
a MUST gap. Because doctor is a mechanical gate, the pass verdict reads **"mechanically
conformant"** (human) / `conformant: true` scoped to mechanical MUSTs (JSON) — behavioral sections
stay deferred to `review`.

## Placement

Verb logic lives in **`crates/project-canon-cli/src/doctor.rs`** (+ a tiny `json.rs`), not in
core: mechanical probing reads the filesystem, and core is I/O-free by contract. `doctor.rs`
consumes the model unchanged (`Model::standard()` → `resolve`), keeping it disjoint from the
future `new`/`review` sibling modules so they can split into parallel lanes later. `main.rs`
gains a subcommand dispatch (`doctor`), keeping its no-verb stub for the bare invocation.

## Deferred / out of scope (→ follow-ups)

- Mechanical archetype auto-detection + mechanical Q1–Q8 characterization (judgment → review).
- Adopting `clap` for the full multi-verb surface + §14 `--help --json` (a cross-verb concern,
  introduced when `new`/`review` land, not unilaterally by doctor).
- `--fix` corrective twin (§18): doctor is read-only at v0; `--fix` would mutate the target.
- Growing the mechanical-probe registry as more §N sections gain decidable file checks.
