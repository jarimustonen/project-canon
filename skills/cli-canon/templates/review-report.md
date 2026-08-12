# Review output — conformance matrix + staged findings

Review mode produces two artifacts: a **conformance matrix** (tool × canon section) and a
**prioritized gap list** of recommendation findings. It **recommends and stages, never
auto-fixes and never auto-files**: every surviving gap becomes a *staged* `issuectl` command
for the tool's **own repo**, presented for the user to approve — not a code edit, and not a
blind `issue create`.

Header the report with: `Tool: <name>` · `Canon version: <N>` (from the canon's `Canon
version:` line) · `Target: <repo path | binary-only>` · any `canon_out_of_date` /
reconciliation notes.

## The conformance matrix

One row per canon section that is *in scope* for this tool (drop the conditional sections
whose Applies-when doesn't hold — mark them `n/a`, don't fail them). Columns:

| § | Dimension | Status | Evidence | Severity |
|---|---|---|---|---|
| §2 | structured output + exit codes | `pass`/`partial`/`fail`/`unknown`/`blocked`/`n/a` | the exact command output / grep hit that settles it | MUST |

- **Status vocabulary** (grounded in the probe, not opinion — quote the command run and what
  it returned):
  - `pass` — conformant, with evidence.
  - `partial` — the shape exists but misses a sub-requirement (e.g. `version --json` present
    but no `supported_schemas`).
  - `fail` — non-conformant, with evidence.
  - `unknown` — no evidence obtainable (binary-only target, no safe fixture for a
    `[sandbox-write]` probe, source unavailable). **Never** a filed gap — it is a
    coverage/manual-verify note.
  - `blocked` — a `[sandbox-write]` probe that could not be run safely (no isolated `--home`,
    remote/unknown backend); note the safe manual probe to run instead.
  - `n/a` — the section's Applies-when doesn't hold for this tool.
- **Evidence** is a real observation. No evidence → the row is `unknown`, not `pass`.

## Findings (the prioritized gap list)

Only `fail` and confirmed `partial` in-scope rows become findings. `unknown`/`blocked` become
coverage notes, never gap findings. Rank most-severe first. Per finding:

- **Section:** `§N — name`
- **Severity:** MUST-fix (a MUST or MUST-when-applies gate that failed) · SHOULD-fix.
- **Observed:** what the probe actually showed (the anti-pattern).
- **Expected:** the conformant shape, cited to the canon (`§N`).
- **Fix sketch:** the smallest change that would flip the row to `pass` — a concrete command,
  flag, or code seam. Not a full patch.

**Triage is by canon severity — do NOT run `/assess-findings`.** That skill triages by
production-likelihood and readability and would DROP a mandatory conformance failure that is
structurally real but rarely hit at runtime (Rule 1a/1b); it also reads a `history/review-*.md`
artifact this skill doesn't produce, emits Finnish issue text, and writes to the current
repo. The canon's MUST / MUST-when-applies / SHOULD model **is** the triage: a MUST/`fail`
survives regardless of runtime likelihood. You may borrow `/assess-findings`' four axes as an
inline sanity check, but they can only *downgrade evidence confidence to `unknown`*, never
erase a MUST row.

## Emission — stage, don't auto-file

1. **Verify the target repo.** `git -C "$TARGET_REPO" remote -v` must identify the expected
   repo; path-existence alone is not enough. A binary-only target has no repo → findings go to
   the **user** in the report, nothing is staged.
2. **Dedup.** Before staging, list existing `cli-canon`-labelled issues in the target
   (`issuectl ls`/`gh issue list --label cli-canon`); if a `§N` issue already exists, propose
   *updating* it, not a duplicate.
3. **Default known-aspirational fails to report-only.** A gap against a deliberately-
   aspirational canon-v2 mandate recurs on every run; keep it in the report but do not stage
   it unless the user opts in — otherwise re-runs spam the tracker.
4. **Stage, one issue per section** (preserve §N ↔ issue traceability; group only sub-parts of
   the *same* section, e.g. §8-config + §8-data-root). Present the full would-file list —
   target repo + titles + labels — and get the user's go. Then file with the repo explicitly
   scoped (a bare `/issue`/`issuectl` is cwd-bound and would file into the wrong repo):

   ```bash
   ( cd "$TARGET_REPO" && issuectl new --type improvement \
       --title "cli-canon: §N <gap>" --body-file "$sandbox/finding-N.md" \
       --label tooling --label cli-canon )
   ```

   Body = Observed / Expected / Fix sketch.
5. **Canon-addition candidates** (from the active dimension-discovery hook, only once ≥2 tools
   show the practice) are staged against **homebase**'s `cli-canon-consolidate` — a separate
   scoped block, separately confirmed:

   ```bash
   ( cd ~/Sources/homebase && issuectl new --type task \
       --title "cli-canon dimension: <practice>" --body-file "$sandbox/candidate.md" \
       --label cli-canon )
   ```

## Plain-language summary (for the user)

Close with a short brief a non-implementer can read: how many MUST gaps, the top 3 by impact,
whether the tool is broadly conformant or has structural gaps, any `unknown`/`blocked`
coverage gaps, and any canon-addition candidate discovered. Numbers + structure, not a wall of
per-section prose.

## Context discipline

Do **not** read large tool outputs into context. Probe with `--json | jq` for the field you
need; write any large evidence (a full `--help --json`, a big `list`) to the namespaced scratch
dir (`"$sandbox"`) and query it. The matrix cites the field, not the blob.
