---
name: cli-canon
description: Apply the AI-first CLI canon (AGENTS-AI-FIRST-CLI.md, §1–§24) to a family CLI — REVIEW an existing CLI against the canon and emit a conformance matrix + prioritized, per-tool recommendation findings, or GENERATE canon-conformant surface scaffolding/guidance inside an existing repo. Reads the canon fresh (it grows §19+). Use for "review/audit tool X against AGENTS-AI-FIRST-CLI", "check shipshape/issuectl/orchestratectl against the CLI canon", "which canon sections is this CLI missing", "scaffold the CLI surface for a new family tool (inside an existing repo)". Review recommends, never fixes; it stages issue commands, it does not auto-file. NOT a generic code review (/llm-review), NOT a SKILL.md review (/llm-skill-review), NOT a domain-lens audit (/review-lens-audit), NOT new-repo bootstrap (/create-project — run it first, then generate the surface).
allowed-tools: Bash, Glob, Grep, Read, Write
---

Operationalize the **AI-first CLI canon** — `AGENTS-AI-FIRST-CLI.md`, sections §1–§24 — as
an active **reviewer/generator** for family CLI tools. The canon is a *document*; this skill is
the *tool* that applies it: given a family CLI (a repo or a
binary), it either **reviews** the tool against the canon and produces actionable findings,
or **generates** conformant surface scaffolding/guidance.

The canon is the **single source of truth and it grows** (`§19+` append over time; the
section numbers are a stable citation surface, never renumbered). So this skill **never
hardcodes the section list from memory** — it reads the canon fresh every run and drives off
`templates/conformance-probes.md`, a probe index into it. The probe table is manually
maintained, so a *newly added* canon section is not automatically probed — the startup
reconciliation step (below) detects that gap and reports it rather than silently skipping it.

## Two modes

- **`review`** — given an existing CLI (binary + repo), probe it against every in-scope
  section, build a conformance matrix, triage the gaps by the canon's own severity model, and
  **stage** per-tool recommendation findings for the user to file. Driven by
  `templates/conformance-probes.md` + `templates/review-report.md`.
- **`generate`** — given a new or evolving tool's shape *inside an existing repo*, decide
  which canon sections apply, then emit targeted scaffolding + a conformance TODO for exactly
  those sections. Driven by `templates/generate-plan.md`. (For a brand-new repo, run
  `/create-project` first — it bootstraps the repo and copies the canon in verbatim — then
  run generate for the CLI surface.)

One skill, two modes; the mode is chosen from the argument (below). Both modes share the same
probe table and the same characterization questionnaire, so "what generate scaffolds" and
"what review checks" can never drift apart.

## When to use / when NOT to use

**Use** when the unit of work is a **family CLI measured against the canon**:
- "make this CLI conformant" / "which canon sections is it missing" / "audit a CLI against
  `AGENTS-AI-FIRST-CLI.md`" → **review** (the tool's own author then applies the fixes);
- "scaffold the CLI surface for a new AI-first tool (inside an existing repo)" → **generate**;
- as Phase 3 of the `stack-cli-alignment` epic: the reusable form of the conformance audit.

**Do NOT use** for:
- a generic code/architecture review of a diff → `/llm-review` or `/llm-review-panel`;
- reviewing a **SKILL.md / agent-instruction file** → `/llm-skill-review`;
- a project's **domain-lens** review (its concepts/terminology) → `/review-lens-audit`;
- bootstrapping a whole new *repo* (git init, GitHub, AGENTS.md) → `/create-project` first,
  then this skill's generate mode scaffolds the CLI *surface* inside it;
- **applying** an existing conformance backlog (editing a tool's code to fix findings) — that
  is ordinary implementation work in the tool's own repo, not this skill.

This skill judges **conformance to the canon**, not general code quality; the two are
complementary (run `/llm-review` for the latter).

## Argument handling

Invoked as `/cli-canon <mode> <target> [notes]`.

- **mode** — the literal token `review` or `generate` **when it is the first argument**. If
  the first token is `review`/`generate` with no target, ask for the target (one plain
  sentence). If mode is omitted entirely, infer: an existing tool/repo/binary to be assessed
  → `review`; "scaffold/start/new surface" phrasing → `generate`. If a name resolves to
  neither an existing repo nor a `$PATH` binary and there is no scaffold phrasing, ask the
  user rather than guessing whether it is a typo, a binary, or an unbuilt tool.
- **target** — a repo path, a tool name (resolve to its repo via the family map below), or,
  for `review`, an installed binary with no reachable repo (a **binary-only** review — see
  the fallback in the review workflow).
- **notes** — free-form scope hints ("only the config surface", "we're adding a daemon
  mode"). A user-scoped review covers only the named sections and reports "Overall: not
  assessed — user-scoped to §N" rather than marking the rest `unknown`.

**Family repo map:** resolve a tool name from the operator's `project-canon config show --json`
output, specifically `values.family_repos`. Do not assume a repository root or tool list.
Confirm the path exists **and** `git -C <repo> remote -v` identifies the expected repo before
staging anything against it.

## Resolve the canon & templates (both modes, first thing)

Read the canon fresh from the installed `ai-first-cli-canon` skill, or use
`project-canon skill print` when that command is available. If neither source resolves, fall
back to `<target-repo>/AGENTS-AI-FIRST-CLI.md` and mark the result `canon_out_of_date` because a
repo-local copy can lag. If no canon copy resolves — or any of the mode's templates
(`templates/conformance-probes.md`, plus `generate-plan.md` / `review-report.md`) cannot be
read — **STOP and say so**; do not hallucinate the canon or the probe table. Record the
canon's `Canon version:` line in the matrix/plan header.

**Reconcile canon vs. probe table.** Parse the section IDs (`§N`) present in the canon and
those present in `templates/conformance-probes.md`. If the canon has sections the probe table
lacks, report them as **uncovered** (grade what is covered; list the uncovered as "needs a
probe" — a candidate for the Phase-4 checker), and never claim complete conformance. If the
probe table has sections the canon dropped, flag them as stale.

## Characterize the tool (both modes — the shared questionnaire)

Applicability of the conditional sections is decided by these **eight** yes/no questions.
Answer each from the request / the repo / `--help`; if a binary-only target cannot settle a
source-dependent question, mark it *unknown-source* (the dependent sections become `unknown`,
not `fail`). If several are unsettleable, ask the user **all of them in one message**, not
one at a time.

| # | Question | If **yes**, these sections apply |
|---|---|---|
| Q1 | More than one resource noun? | §6 noun-verb surface (else a flat verb surface is fine) |
| Q2 | Resolves persistent config and/or a data root? | §8 config precedence + `config path`/`show`; `--home` if it has a data root |
| Q3 | Any command creates/updates/deletes a resource? | §11 dry-run + idempotency (per mutating cmd) |
| Q4 | Any command runs >a few seconds / as a daemon? | §12 streaming + progress query + signals |
| Q5 | Stamps `created`/`updated` or time-derived ids? | §19 injected clock + hidden `--frozen-time` |
| Q6 | Owns human-editable on-disk records? | §20 `fmt` canonicalizer |
| Q7 | Scaffolds an on-disk home other commands need? | §21 `init` idempotent no-clobber |
| Q8 | Results that can be large (list/export)? | §13 `--output FILE.jsonl\|.db` |

**Always-on (every family CLI):** §1, §2, §3, §4, §5, §7, §9, §10, §14, §15, §16, §17, §18.
**Internal (SHOULD):** §22 core/cli split. A tool with **zero** persistent config makes §8
`n/a` (see the §8 severity note below).

## Probe safety & isolation (review mode — read before running any probe)

A reviewed `$TOOL` is an **untrusted binary that may mutate state**, not a passive subject.
The default posture is **static + read-only**; anything that writes is opt-in and sandboxed.

- **Never run a mutating or stateful probe against the tool's default/production
  environment.** §11 (real `create`/idempotency retry), §12 (starting a long job + signals),
  §15 (`skill install`), §20 (`fmt` twice), §21 (`init` twice) all mutate. Run each **only**
  against an isolated scratch home you create for the review:

  ```bash
  sandbox="$(mktemp -d)"; chmod 700 "$sandbox"
  # verify the tool will actually use it before any write:
  $TOOL --home "$sandbox" config show --json | jq -e '.data_root.path | startswith("/")' >/dev/null
  # seed a git-init'd fixture so a buggy fmt/init is recoverable by `git -C "$sandbox" checkout .`
  ```

  Assert the resolved home is under the sandbox (`config show --json`) **before** the write.
  If you cannot construct a safe fixture (no isolated `--home`, a remote-backed `create`, an
  unknown backend), **mark the row `unknown`** and fall back to static evidence (read the
  source, the tests, a checked-in `--help` snapshot). "Unknown from static evidence" beats an
  unsafe empirical proof.
- **Resolve placeholders, never run them raw.** Probes use `<cmd>`/`<mut>`/`<long>`/`<fetch>`
  as placeholders. The discovery step (review workflow phase 2) binds them from `--help`;
  restrict generic-input probes (§1/§5/§9) to **read-only verbs** (`list`, `show`, `version`,
  `--help`, `doctor`, `config show`, `skill list`, `skill print`). Test mutating verbs' error
  paths via `--dry-run --json` only.
- **Invoke by argv, not a shell string** (no injection from the target name); use per-command
  timeouts; write large output to a namespaced scratch dir (`"$sandbox"`), never a fixed
  `/tmp/x`; redact secrets before any output leaves the run.
- **Signals (§12):** only ever signal a child process you spawned and whose handle you hold —
  never `pgrep`/name-match. If no safe workload exists, mark §12 `unknown`.
- **`Write` in review mode is for the scratchpad/fixture only — never the reviewed repo.**

## Review workflow

Follow `templates/conformance-probes.md` then `templates/review-report.md`:

1. **Characterize** (the shared questionnaire) → which conditional sections are in scope. A
   conditional section whose Applies-when doesn't hold is `n/a`, never a failure.
2. **Discover the surface.** Run `$TOOL --help` (capture to the scratch dir) and, for each
   subcommand, `$TOOL <sub> --help`; enumerate the subcommand tree and classify each command
   **read-only / mutating / long-running**. This binds the probe placeholders (`<cmd>`,
   `<mut>`, `<long>`, `<fetch>`) to real commands. Never run a probe against an unresolved
   placeholder.
3. **Run the mechanical probes** — for each in-scope section, run its probe(s) under the
   safety rules above and record real evidence (`$TOOL version --json | jq …`, a `grep isatty`
   hit, `Cargo.toml` members, a shipped `SKILL.md`'s frontmatter). Handle the "tool doesn't
   speak the contract yet" branch: if `$TOOL version --json` errors, that *is* the §10 `fail`
   evidence — capture the error, don't let a broken `jq` corrupt the matrix. No evidence →
   the row is `unknown`, not `pass`.
   - **Binary-only target:** source-dependent probes (§10 `build.rs`, §22 `Cargo.toml`,
     source-level §9/§19) become `unknown` with reason "unavailable without source", and
     Q5/Q6 may be unsettleable. Findings go to the **user**, not a staged issue.
   - **Non-Rust ecosystem:** the probes name the Rust/clap shape (the family's concrete form,
     per the canon's own preamble). For a Node/Go/Python tool, map to the equivalent (a
     `build.rs` → the toolchain's build-time SHA stamp; `Cargo.toml` members → the workspace
     manifest) — do **not** hard-`fail` a tool merely for not being Rust.
4. **Settle the judgment sections inline.** For sections a grep can't decide (§6 surface
   shape, §7 domain-verb legitimacy, §10 schema-as-API discipline, §11 idempotency
   semantics), evaluate the surface against the canon excerpt **yourself** and argue the call
   in the matrix — do **not** invoke `/llm-review` (its prompt is a general code-review, it
   has no rubric parameter, and it writes to the wrong repo's history). If an external opinion
   is genuinely warranted, write **one** bounded judgment brief (all judgment sections
   together, canon excerpt + captured evidence, asking for `status/evidence/rationale` per
   section) and call `consult-llm` **once** with `--task review` — never one call per section.
5. **Dimension-discovery (active, not passive).** The epic wants the canon to grow from
   practices recurring across **≥2** tools. A single-tool review cannot prove recurrence
   alone, so when you spot a canon-absent practice worth cataloguing: (a) read the existing
   candidates in the selected canon-maintenance issue tracker (and open issues labelled
   `cli-canon`); if this tool exhibits an *existing* candidate, **add this tool's evidence to
   it**: that is how the ≥2 threshold is reached; (b) else `grep` 2–3 other family repos for
   the same practice and only write a **new** candidate (`applies-when / look-for / rationale /
   evidence: [tool→source]`) once ≥2 tools show it. This feeds the canon-maintenance backlog
   and the Phase-4 checker.
6. **Build the matrix, triage by canon severity, stage findings.** Assemble the conformance
   matrix (tool × section, evidence + severity). **The canon's severity model IS the triage**
   — do **not** run `/assess-findings` (it triages by production-likelihood and would DROP
   mandatory conformance failures that are structurally-real-but-rarely-hit; it also emits
   Finnish issue text and writes to the wrong repo). You may borrow its four axes as an inline
   sanity check, but MUST/MUST-when-applies `fail`s survive regardless of runtime likelihood.
   Only `fail`/confirmed-`partial` rows become findings; `unknown` becomes a
   *coverage/manual-verify note*, never a filed gap. Then **stage, don't auto-file**, per
   `templates/review-report.md`: produce the would-file list (repo verified via `git remote`,
   deduped against existing `cli-canon`-labelled issues, known-aspirational gaps defaulted to
   report-only), and present it for the user to approve. When approved, file with the target
   repo explicitly scoped: `(cd "$TARGET_REPO" && issuectl new …)` — a bare `/issue`/`issuectl`
   is cwd-bound and would file into the wrong repo.
7. **Report** — the matrix plus a short plain-language brief (MUST-gap count, top 3 by
   impact, broadly-conformant vs. structural-gaps, any canon-candidate, any coverage gaps
   from reconciliation). Numbers + structure, not per-section prose.

## Generate workflow

Follow `templates/generate-plan.md` (which uses the same eight questions above):

1. **Characterize** → the applicable section set (always-on + the conditional sections the
   questions switch on).
2. **Confirm write intent.** Generate **defaults to a preview** (the plan + scaffold shown,
   nothing written). Writing files into the target repo happens only when the user asks
   ("write it", `--write`), the target repo is confirmed and its tree is clean-or-consented,
   and writes are confined to the repo root. Never run a generated `build.rs` or other code
   during generation.
3. **Emit scaffold + guidance** — for each applicable section, the *conformant shape* with the
   canonical reference samples the template carries (the §2 error→exit table, the §10 `version
   --json` shape and error envelope, the §8 `config` surface). Prefer directing the tool at a
   thin `<family>-cli-common` crate (§22) over re-hand-rolling the plumbing — noting that
   crate is an assumed/optional dependency, with local fallback templates if it doesn't exist.
4. **Emit the conformance TODO as the matrix skeleton** — the probe table filtered to the
   applicable sections, rendered as the **same matrix shape** review later populates (every
   row an unchecked box the new tool must flip to `pass` with evidence). Generate's output is
   thus a direct review input.
5. **Optionally review the plan** — for a substantial tool, write the plan to a file and run
   **one** `consult-llm --task review` pass with the canon as rubric (not `/llm-review`).

## Severity discipline (both modes)

- **MUST** — unconditional agent-facing surface (§1, §2, §3, §4, §5, §6, §7, §9, §10, §14
  drill-down, §15, §16, §17, §18). Absence is a conformance failure.
- **MUST-when-applies** — a hard gate, but only when the section's Applies-when holds (§8 —
  for any tool that resolves *any* persistent config or data root; §11, §12, §13-when-large,
  §19, §20, §21). §8's `config path`/`config show` is the family's single most consistent
  historical miss — for a tool that resolves config, treat its absence as a failure, not a
  gap. A tool with **zero** persistent config or data root has §8 `n/a`.
- **SHOULD** — a strong convention, never a hard readiness gate (§13 unless results are large,
  §14's `--help --json` which is a known family-wide gap, §22 internal layout). Report as a
  recommendation, never a MUST failure.

Canon v4 remains **deliberately aspirational** — several mandates (mandatory `config`, real
provenance, the exit-code remap) make existing tools non-conformant *by design*. That gap is
the backlog the findings populate; a `fail` against an aspirational mandate is expected and
correct. Because such a gap recurs on **every** run, default known-aspirational fails to
**report-only** (not auto-staged) so re-runs don't spam the tracker.

## Critical rules

- **Read the canon fresh every run; homebase copy first.** Never grade against a remembered
  section list. Cite everything by `§N`. Reconcile canon vs. probe table at startup and
  report uncovered sections rather than silently skipping them.
- **The probe table + questionnaire are the shared contract.** Generate scaffolds what review
  probes, off the same eight questions; refine `templates/conformance-probes.md` and they stay
  in lockstep.
- **Recommend and stage, never auto-fix or auto-file a foreign tool.** Review edits no other
  product's code and files no issue without the user's go; it stages the `issuectl` commands
  scoped to the target repo. Generate defaults to preview.
- **Isolate every mutating probe** (scratch `--home`, verified before the write) and restrict
  generic-input probes to read-only verbs. `unknown` beats an unsafe real mutation.
- **Respect Applies-when** — a conditional section out of scope is `n/a`, not a failure — and
  **`unknown` is never a filed gap** (it is a coverage note).
- **File tool bugs where they live.** A bug/idea in `issuectl`/`orchestratectl`/etc. found
  while running this skill is filed in *that tool's* repo (`type: bug`/`feature`), on `main`,
  scoped with `(cd <repo> && …)`.
- **Context discipline** — never read a tool's large outputs (or a whole transcript/corpus)
  into context; probe the field with `jq`, spill blobs to the namespaced scratch dir.

## Templates (the refinement surface)

- `templates/conformance-probes.md` — the §1–§24 probe index (Applies / Signal / Probe / Fail
  / Severity + an effect-class per probe), shared by both modes, plus the dimension-discovery
  hook. Refine probes here. (This prose index is the **seed for the Phase-4 machine-readable
  registry** — see the epic note; it is deliberately still prose so the skill stays readable.)
- `templates/generate-plan.md` — the generate-mode applicability steps + canonical samples.
- `templates/review-report.md` — the review-mode matrix format, findings shape, and the
  stage-don't-file emission rules.

## Relationship to the epic

This is Phase 3 of `stack-cli-alignment`. Phase 4 (`cli-canon-own-project`) will extract this
into its own project — likely a mini-tool shipping this skill **plus** a `doctor`-style
machine-checker that runs these probes as a CI gate. Keep the logic self-contained and the
probe blocks machine-shaped (`Applies / Look-for / Probe / Fail / effect-class` is exactly
what a checker consumes). The known extraction seams to carry over — **captured here so
Phase 4 is a lift, not a rediscovery**: (1) convert the prose probe table to a machine-
readable registry (YAML/JSON) with stable probe ids, effect-classes, and assertions;
(2) externalize the family repo map to config (no repository-root assumptions); (3) give every
judgment section a mechanical fallback so the checker isn't LatentlyLLM-dependent; (4) add a
non-interactive `--assume-defaults` mode (a CI gate has no user to ask); (5) a cross-run
aggregate/family-matrix so dimension-discovery evidence accumulates across tools.
