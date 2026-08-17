# `review` — recommending audit (design note)

The THIRD and last verb of `project-canon`, and the human-facing one. Where `doctor` is a
mechanical pass/fail **gate**, `review` is the **advisory audit**: it reads the same two-layer
model + mechanical-probe substrate, triages every in-scope dimension by the canon's **severity
model**, and emits severity-ranked **findings** plus **staged** `issuectl` commands and
dimension-discovery pointers. It **recommends and stages; it NEVER auto-fixes and NEVER
auto-files** (ADR 0009 §2). It folds in `cli-canon`'s *review* mode (`review-report.md`).

## `doctor` vs. `review` (kept distinct)

| | `doctor` | `review` |
|---|---|---|
| role | mechanical conformance **gate** (CI) | advisory **audit** (human) |
| decides | only mechanically-probeable rows | every in-scope row, mechanical + judgment-deferred |
| a deferred/behavioral row | `skipped` (silently under-claims) | a **manual-verify coverage note** with the probe's how-to |
| a mechanical MUST gap | `fail`, **flips exit to 1** | a **must-fix finding** + a staged `issuectl` command, **exit stays 0** |
| output | pass/fail matrix | severity-ranked findings + staged commands |
| side effects | none (read-only) | none — stages/prints, never files or fixes |

`review` is the **advisory superset** of `doctor` on the shared dimensions: it reuses the exact
same mechanical probes (extracted to `crates/project-canon-cli/src/probes.rs`, consumed by
both), so a mechanically-confirmed gap carries the same evidence in both verbs — review just
adds severity triage, a staged issue command, and (for the behavioral sections doctor drops)
an actionable manual-verify note built from the model's `Probe` descriptor.

## Command surface

```
project-canon review [--profile <archetype>] [--assume-defaults] [--json] [--verbose] [--help] [<repo>]
```

| flag/arg | default | meaning |
|---|---|---|
| `<repo>` (positional) | `.` (cwd) | Target repo to audit. Read-only. Must be an existing directory, else exit `2`. |
| `--profile <archetype>` | `cli` | Which profile layers onto base (`cli`/`service`/`library`/`release`). Unknown → exit `2`. |
| `--assume-defaults` | (implied) | Characterize non-interactively with conservative defaults (all conditional triggers **off**) — the only mode at v0 (§3, review never prompts). Conditional sections thus resolve `n/a`; review never asserts a conditional gap it can't prove applies. |
| `--json` | off | Emit the §10 structured report on stdout. |
| `--verbose` | off | Also list manual-verify coverage notes + `n/a` rows (terse default shows confirmed gaps + staged commands only). |
| `--help` | — | Usage, exit `0` (§2). |

Strict input validation (§1), uniform with `doctor`/`new`: unknown flags, a bad `--profile`,
a missing/inline value on a valueless flag, a repeated flag, an extra positional — all echo the
offending token and exit `2`. The env layer is validated strictly (a malformed `PROJECT_CANON_*`
→ exit 2) even though review, like doctor, consumes no env field for its own logic (it audits an
explicit target path; the staged command is scoped to that path via `cd`, so no env-derived
repo location is hardcoded).

## Finding / severity schema

Each in-scope resolved dimension becomes one **row**, triaged into a `kind`:

- **`confirmed-gap`** — a *mechanical* probe exists and **failed**: real evidence (the probe
  message). Ranked by severity into a `fix_class`:
  - MUST / MUST-when-applies fail → **must-fix**
  - SHOULD fail → **should-fix**
  Every confirmed gap gets a **staged `issuectl` command** (printed, never run).
- **`manual-verify`** — the dimension *applies* but has **no mechanical probe** (the behavioral
  §1–§7/§9/§10/§14 sections doctor defers). This is the review-vocabulary `unknown`: a
  coverage/verify-by-hand note carrying the model `Probe`'s `signal` (expected shape),
  `command_hint` (how to observe), `fail` (the anti-pattern) and `effect` class
  (static/exec-ro/sandbox-write — tells the auditor how to run it safely). **Never staged** —
  per `review-report.md`, `unknown` is never a filed gap.
- **`n/a`** — a conditional whose trigger is off. Dropped from findings (shown only under
  `--verbose`); never a gap.

A `pass` (mechanical probe succeeded) is not a finding — counted in the summary, listed only
under `--verbose`.

Ranking (most-severe first): confirmed must-fix → confirmed should-fix → manual-verify (by
severity) → (verbose) pass/n-a; ties broken by canon §N / dimension id. Evidence-backed gaps
always outrank verify-by-hand notes.

## Staged `issuectl` commands (printed, NOT executed)

One command per **confirmed gap** (preserving §N ↔ issue traceability), rendered from the model
+ the target path, shell-safe, and **printed** for a human/agent to run — review never shells
out. Reuses `new`'s POSIX `shell_quote` for every interpolated value:

```bash
( cd '<repo>' && issuectl new --type improvement --title 'cli-canon: §N <title>' \
    --slug cli-canon-sNN --label tooling --label cli-canon )
```

- Canon rows title `cli-canon: §N <title>` / slug `cli-canon-sNN`; base scaffold rows title
  `project-canon: <title>` / slug `canon-<dim>`. The stable slug makes re-runs idempotent
  (issuectl dedups on slug) so review doesn't spam the tracker.
- Scoped to the target with `( cd '<repo>' && … )` — the `review-report.md` idiom — so a bare
  cwd-bound `issuectl` never files into the wrong repo. Manual-verify rows are **not** staged.

## Dimension-discovery candidates

Discovery admits a practice to the canon only once it recurs across **≥2** family tools — a
judgment a single-repo mechanical pass cannot make. So at v0 review emits a **named-but-empty**
candidate list (`discovery_candidates: []`) plus a printed pointer explaining that real
candidates are staged against homebase's `cli-canon-consolidate`:

```bash
( cd /tmp/example-repo && issuectl new --type task \
    --title 'cli-canon dimension: <practice>' --label cli-canon )
```

This keeps the contract visible without fabricating candidates the binary can't justify. The
≥2-tool recurrence detector is a deferred judgment seam (follow-up), mirroring the empty-profile
extension points.

## `--json` output schema (§10-shaped)

```json
{
  "schema_version": 1,
  "tool": "project-canon",
  "verb": "review",
  "advisory": true,
  "target": "/abs/path/to/repo",
  "profile": "cli",
  "surface_shape": "flat-verb",
  "findings": [
    {
      "id": "base.doc-pattern", "title": "…", "severity": "must",
      "layer": "base", "canon_section": null,
      "kind": "confirmed-gap", "fix_class": "must-fix", "effect": "static",
      "observed": "AGENTS.md missing at repo root",
      "expected": "AGENTS.md and CLAUDE.md both present …",
      "fail_mode": "…", "command_hint": "…",
      "staged_command": "( cd '/abs/repo' && issuectl new … )"
    },
    {
      "id": "canon.s01", "title": "Strict input validation …", "severity": "must",
      "layer": "profile:cli", "canon_section": 1,
      "kind": "manual-verify", "fix_class": "must-fix", "effect": "exec-ro",
      "observed": "no mechanical probe — verify by hand",
      "expected": "…", "fail_mode": "…", "command_hint": "…",
      "staged_command": null
    }
  ],
  "staged_commands": ["( cd '/abs/repo' && issuectl new … )"],
  "discovery_candidates": [],
  "summary": {
    "confirmed_gaps": 1, "must_fix": 1, "should_fix": 0,
    "manual_verify": 15, "pass": 4, "not_applicable": 7, "staged": 1
  },
  "exit_code": 0
}
```

- `findings[]` carries only the **actionable** rows — `kind ∈ {confirmed-gap, manual-verify}`.
  `pass` and `n/a` are **not findings**: they appear only in the `summary` counts
  (`pass` / `not_applicable`) and, for a human, under `--verbose`. So a JSON consumer never sees
  a `pass`/`not-applicable` `kind`. (`--verbose` is a **human-only** flag; the JSON payload does
  not change with it — a consumer wanting the full pass/n-a matrix uses `doctor`.)
- `fix_class` ∈ `must-fix | should-fix`; `effect` ∈ `static | exec-ro | sandbox-write`;
  `staged_command` is `null` for manual-verify (only confirmed gaps stage a command).
- `staged_commands` is the flat, ordered list of the confirmed-gap commands (a convenience
  mirror of the per-finding `staged_command`s) — the "would-run list" the human approves.
- `advisory: true` and `exit_code: 0` are invariant (see below). Data → stdout, diagnostics →
  stderr (§2); serialized by the in-crate escape-correct `Json` writer (dependency-free).

## Exit-code contract — advisory, NOT a gate

**A conformance outcome NEVER flips review's exit code.** Findings — even must-fix ones — leave
the exit at `0`. Only a usage/operational failure exits non-zero:

| exit | meaning |
|---|---|
| `0` | review ran and produced its report — **regardless of how many gaps it found** |
| `2` | usage/operational error — bad flag, bad `--profile`, missing/unreadable target, an I/O fault while probing, or malformed `PROJECT_CANON_*` env |

Justification (the issue's explicit prompt): review is **advisory** by contract (ADR 0009 §2);
`doctor` is the gate. Giving review a "found-gaps" non-zero would invite CI to wire it as a
gate, blurring the doctor/review split and turning an advisory human-audit into a build-breaker
— exactly the conflation ADR 0009 separates the two verbs to avoid. If a caller wants review's
richer triage to gate CI, that is a future **`--strict`** opt-in (deferred), never the default.
This is a genuine human-decision fork → recorded as a discussion item.

Note `1` is deliberately unused: reserving it keeps the door open for a future `--strict` that
mirrors `doctor`'s `1 = gap` semantics without a breaking change.

## Placement

Verb logic in **`crates/project-canon-cli/src/review.rs`**; the mechanical probes shared with
`doctor` are extracted to **`crates/project-canon-cli/src/probes.rs`** (both verbs depend on
`probes`, neither on the other — keeping them splittable into parallel lanes). Core is consumed
unchanged (`Model::standard()` → `resolve`); core stays I/O-free. `main.rs` gains the `review`
dispatch.

## Deferred / out of scope (→ follow-ups)

- `--strict` gate mode (exit `1` on a must-fix finding) for CI callers that want it.
- The ≥2-tool dimension-discovery recurrence detector (judgment; needs cross-repo scan).
- Running behavioral (`exec-ro`/`sandbox-write`) probes against the built target binary — the
  manual-verify notes carry the how-to; actually executing them (sandboxed) is a bigger seam.
- `--body-file` on staged commands (would need a scratch write); the gap detail lives in
  review's own output, so the staged title/slug suffice at v0.
- A minor-robustness note (fold into wrap-up, not a filed issue): the staged command assumes the
  target has an `issues/` tree; review could pre-note when `issues/` is absent.
- Family-wide I/O-edge hardening (shared with the landed `doctor`/`new`, so a *cross-verb* pass,
  not a review-only fix): `std::env::vars()` panics on a non-UTF-8 env var (→ exit 101, not the
  documented 2); `p.display().to_string()` is lossy for a non-UTF-8 target path; `print!`/
  `println!` panic on a broken pipe. All three are exotic and identical across the three verbs;
  the right locus is the env layer + the I/O edge, uniformly. Noted for a follow-up, not filed.
