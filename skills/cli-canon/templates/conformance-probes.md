# Conformance probes — `AGENTS-AI-FIRST-CLI.md` §1–§23

This is the operational checklist. **Read the canon fresh first** — it is the source
of truth and grows (`§19+` append over time); this table is a probe index into it, not
a replacement. Cite findings by section number (`§N` is a stable citation surface, never
renumbered). Substitute the real binary for `$TOOL`.

**Severity classes**
- **MUST** — unconditional agent-facing surface. Absence is a conformance failure.
- **MUST‑when‑applies** — a hard requirement, but only when the section's *Applies‑when*
  holds. Do **not** flag it against a tool the trigger doesn't reach.
- **SHOULD** — a strong convention, never a hard readiness gate (e.g. §22 internal layout).

For each section: *Applies* (when it is in scope) · *Signal* (the conformant shape) ·
*Probe* (how to observe it) · *Fail* (the anti‑pattern) · *Severity*.

**Effect class (safety).** Each probe is tagged `[static]` (read source/manifests only),
`[exec-ro]` (runs a read-only verb — `list`/`show`/`version`/`--help`/`doctor`/`config
show`/`skill list`/`skill print`), or `[sandbox-write]` (mutates state — **must** run only
against an isolated scratch `--home` per SKILL.md § *Probe safety & isolation*, or be marked
`unknown` from static evidence). Never run a `[sandbox-write]` probe against the tool's
default/production environment, and never run a generic-`<cmd>` probe until the discovery
step has bound it to a real, read-only command.

---

### §1 Strict input validation — no silent fixups · **MUST**
- **Applies:** always.
- **Signal:** empty/whitespace/unknown‑flag/out‑of‑range inputs → error that echoes the
  actual bad value; never coerced, trimmed silently, or defaulted.
- **Probe:** `$TOOL <cmd> ""` · `$TOOL <cmd> --no-such-flag` · pass an out‑of‑range value.
- **Fail:** silent trim/default/coerce; an ignored unknown flag; a generic message that
  omits the offending value.

### §2 Structured, parseable output + exit codes + JSONL logs · **MUST**
- **Applies:** always.
- **Signal:** global `--json`; data→stdout, errors→stderr; exit codes **classified via one
  central map** (`0` ok · `1` caller/domain‑actionable · `2` system/internal · `130/143`
  cancel); clap usage errors remapped `2→1`; `--help`/`--version` display exit `0`; logs are
  **JSONL** (one event/line) with trace fields (`user_id`, `trace_id`/`run_id`, entity ids,
  `target`).
- **Probe:** `$TOOL <cmd> --json | jq .` · `$TOOL bogus-subcmd; echo $?` (expect `1`) ·
  `$TOOL <cmd> 1>out 2>err` and confirm the split.
- **Fail:** every error collapsed to `1` (or to `2`); a `2` for a caller‑actionable
  `not_found`; human prose mixed into `--json`; multi‑line / prefixed log records.

### §3 No interactive prompts · **MUST**
- **Applies:** always.
- **Signal:** no `y/N` confirmations, no TTY-dependent behavior, no pager/`less`/`$EDITOR`;
  valid input succeeds, invalid fails with a diagnostic + non-zero exit; destructive actions
  gated behind explicit `--force`/`--yes` flags.
- **Probe:** `[exec-ro]` run a read-only command with stdin closed (`$TOOL <ro-cmd> </dev/null`)
  and confirm it neither blocks on input nor pages; `[static]` grep source for
  `read_line`/`prompt`/`confirm`/`Term::` / a spawned `$EDITOR`/pager.
- **Fail:** a Y/N prompt; a spawned pager or editor; a command that hangs waiting on stdin.

### §4 Informative error messages · **MUST**
- **Applies:** always.
- **Signal:** the error carries the actual invalid value **and** the expected set/format;
  multi‑step failures name the failing step.
- **Probe:** trigger each error path; read the message.
- **Fail:** `"invalid input"` with no value and no expected set.

### §5 Composable commands · **MUST**
- **Applies:** always.
- **Signal:** fetch/read commands write to stdout by default; `--output FILE` alternative;
  `-` accepted as stdin; flag names consistent across subcommands (`--target`, `--output`,
  `--json`).
- **Probe:** `[exec-ro]` (bind `<fetch>`/`<cmd>` to read-only verbs from discovery)
  `$TOOL <fetch> | head` · `$TOOL <ro-cmd> --output "$sandbox/x"` · `echo … | $TOOL <ro-cmd> -`.
- **Fail:** a fetch that only writes a file; per‑subcommand flag drift.

### §6 CLI surface: noun‑verb imperative, `apply` opt‑in · **MUST**
- **Applies:** always (shape of the whole surface).
- **Signal:** resource‑first, action‑second for multi‑resource tools (`$TOOL job create`);
  a flat verb surface only for a genuinely single‑resource tool; declarative `apply -f` is
  an *additional* entry point, present **only** when field‑count + real reconciliation
  justify it.
- **Probe:** `$TOOL --help`; inspect the subcommand tree.
- **Fail:** a `apply`‑only surface for an imperative op; a noun layer invented for a
  one‑resource CLI.

### §7 Subcommand verbs: one set, no synonyms · **MUST**
- **Applies:** always.
- **Signal:** exactly `list`/`show`/`create`/`update`/`delete` for CRUD; the two closed
  exception sets only (numbered meta‑verbs `apply`/`exec`/`skill`/`version`/`doctor`/`fmt`/
  `init`; genuine domain state‑transitions with no `update` equivalent). `update` is
  selective‑patch by default; full replace is opt‑in `--replace`/`--replace-file`.
- **Probe:** `$TOOL --help`; grep the surface for `ls|get|new|add|edit|set|patch|rm|remove|
  destroy|describe|view|index`.
- **Fail:** a synonym (`get`, `new`, `rm`); a domain word masking a plain `update`
  (`done`/`won`/`set-status` that only sets a field); one verb meaning both list‑many and
  show‑one.

### §8 Config precedence + inspectable config **and** data root · **MUST**
- **Applies:** any tool that resolves persistent config and/or operates on a data root.
- **Signal:** per‑key precedence `flag > env > file > default`; env mirrors flag
  (`--api-url` ↔ `$<TOOL>_API_URL`); **`config path` and `config show` are mandatory**;
  `config show --json` reports each value's `source` and **redacts secrets** by default
  (`--show-secrets` warns); data root selected by the single global **`--home`**
  (`$<TOOL>_HOME`) with five‑layer precedence incl. upward discovery, and `config show`
  reports the resolved root + source + matched marker; missing root → `data_root_unresolved`
  error naming what was searched.
- **Probe:** `$TOOL config path` · `$TOOL config show --json | jq` · `$TOOL --home /x config show`.
- **Fail:** **no `config path`/`config show`** (the family's single most consistent miss —
  treat as a failure, not a gap); secrets printed unredacted; a `--repo`/`--home` synonym
  pair; a silent operate‑on‑cwd when no root resolves.

### §9 Output format is fixed, not TTY‑detected · **MUST**
- **Applies:** always.
- **Signal:** format set only by `--json`/`--output`; identical bytes piped vs. tty; color
  off by default with only `--color=always|never` (**no `--color=auto`**).
- **Probe:** `$TOOL <cmd> | cat` vs. run in a terminal — bytes must match; grep source for
  `isatty|atty|is_terminal|IsTerminal`; confirm `--color=auto` does not exist.
- **Fail:** table‑vs‑line or color switching on `isatty()`; auto‑pagination.

### §10 Schema versioning, errors, warnings, deprecation, provenance · **MUST**
- **Applies:** always (any `--json` surface).
- **Signal:** every `--json` payload carries `schema_version`; `$TOOL version --json` →
  `{version, commit, schema_version, supported_schemas, skills[]}`; global `--json` works on
  **every** subcommand incl. `version` (no `--output json`‑only special case); `commit` is a
  **real 40‑hex SHA** or exactly `null` + a `build_provenance {kind,note}` object — never
  `"unknown"`/placeholder; error envelope `{schema_version, error:{code,message,
  invalid_value,expected}}` on **stderr**; non‑fatal `warnings:[]` live in the **stdout**
  payload; deprecations emit a structured warning naming the removal window, suppressible via
  `<TOOL>_NO_DEPRECATION_WARNINGS=1`.
- **Probe:** `$TOOL version --json | jq '{commit,schema_version,supported_schemas,skills}'` ·
  trigger an error under `--json`, inspect the envelope + channel.
- **Fail:** `commit:"unknown"`; missing `supported_schemas`; a subcommand with a private JSON
  toggle; warnings on stderr under `--json`; payload without `schema_version`.

### §11 Dry‑run, idempotency, retry safety · **MUST‑when‑applies**
- **Applies:** every command that creates/updates/deletes a resource.
- **Signal:** `--dry-run` runs all validation + read‑only checks and emits the **planning
  envelope** (`{dry_run:true, would:[…]}`), never partially applies; a truthful dry‑run is
  impossible → explicit `dry_run_unsupported` (exit 1), never faked; at least one convergence
  affordance — `--idempotency-key` (echoed), or `--if-not-exists`/`--if-exists`.
- **Probe:** `[exec-ro]` `$TOOL <mut> --dry-run --json | jq` (dry-run is safe *to read*, but
  since the probe is testing whether dry-run is truthful, treat it as untrusted). `[sandbox-write]`
  the real idempotency retry (`create` twice with one key) mutates — run **only** against an
  isolated scratch `--home` (verify via `config show --json` first), or, for a remote/unknown
  backend, skip it and read the tool's idempotency tests instead → status from static evidence.
- **Fail:** no `--dry-run` on a mutating command; a fake dry‑run that actually writes; a
  `--dry-run` result envelope indistinguishable from the real one.

### §12 Long‑running: streaming events, progress query, signals · **MUST‑when‑applies**
- **Applies:** any command running more than a few seconds, or as a daemon/background job.
- **Signal:** `--output=jsonl` emits one event/line (`schema_version`, `event`, monotonic
  `seq`) with exactly one terminal `result`/`cancelled`/`error`; `--json` single‑doc is
  **forbidden** for primarily long‑running commands; background jobs expose a paired
  `<noun> show/status <id>`; `SIGINT`/`SIGTERM` → final `cancelled` event, exit `130`/`143`;
  text progress is one line/step on stderr, **no spinners/ANSI/CR‑overwrite**.
- **Probe:** `[sandbox-write]` `$TOOL <long> --output=jsonl | jq -c .` against a **fixture
  workload in a scratch `--home`** only (a real long job may touch production); spawn it as a
  child you hold the handle to, send SIGTERM to *that* PID (never `pgrep`/name-match), check
  the `cancelled` event + exit `143`. No safe workload constructible → mark `unknown`.
- **Fail:** a spinner/progress bar; format switching by elapsed time; no progress query for a
  detached job.

### §13 Large outputs go to a queryable file · **SHOULD** (MUST when results can be large)
- **Applies:** any `list`/export whose result may be large.
- **Signal:** `--output FILE.jsonl` or `--output FILE.db` (SQLite w/ documented schema);
  stdout prints only file metadata (path, count, `schema_version`, a query hint); `--limit`
  is a guardrail, not the primary paging mechanism.
- **Probe:** `[exec-ro]` `$TOOL list --output "$sandbox/x.jsonl"` then `wc -l "$sandbox/x.jsonl"` / `jq`.
- **Fail:** 10k rows dumped inline; cursor‑paging as the only escape hatch.

### §14 `--help` is agent‑first, structured, drill‑down · **MUST** (`--help --json`: **SHOULD‑strong**)
- **Applies:** always.
- **Signal:** top‑level `--help` lists subcommands + the small global‑flag set (not every
  flag); `<sub> --help` is the full drill‑down (flags, defaults, per‑flag env var,
  exit‑code semantics); every `--help` accepts `--json` emitting a structured, `examples[]`‑
  bearing, `schema_version`‑stamped payload; examples use the §7 verbs and are copy‑pasteable.
- **Probe:** `$TOOL --help` · `$TOOL <sub> --help` · `$TOOL <sub> --help --json | jq`.
- **Fail:** a top‑level flag firehose; no machine‑readable help (a known family‑wide gap —
  flag as SHOULD‑strong, not a MUST failure, unless the tool claims it).

### §15 `skill` subcommand: install companion AI‑skills · **MUST**
- **Applies:** always.
- **Signal:** `$TOOL skill list` (available skills, one‑line descriptions); `$TOOL skill
  install [<name>]` (into `~/.claude/skills/` by default, `--target <dir>`); skills live
  in‑repo and version with the binary.
- **Probe:** `[exec-ro]` `$TOOL skill list` · `[sandbox-write]` `$TOOL skill install --target
  "$sandbox/skills"` (a `mktemp -d` target, never a shared/fixed path or the real
  `~/.claude/skills`).
- **Fail:** no `skill` door at all (three family tools miss this); a `skill list` referencing
  a removed flag.

### §16 `skill print`: stream skill content, no side effects · **MUST**
- **Applies:** always (pairs with §15).
- **Signal:** `$TOOL skill print <name>` writes the SKILL.md to stdout byte‑identical to what
  install would persist; `--json` → `{schema_version,name,cli_version,schema_version_skill,
  content,path_in_repo}`; unknown name → §10 envelope, exit 1; no writes, no network.
- **Probe:** `$TOOL skill print <name> | head` · `$TOOL skill print <name> --json | jq keys`.
- **Fail:** a rendered‑vs‑raw distinction; a side effect on print.

### §17 Skill–CLI version synchronization · **MUST**
- **Applies:** any tool shipping companion skills.
- **Signal:** every SKILL.md frontmatter declares `cli_version` + `schema_version`; `skill
  print` is pinned to the running binary (never a stale disk copy; mismatch →
  `skill_version_mismatch`); `skill install` warns on older on‑disk, errors on newer unless
  `--force`; `version --json` exposes `skills:[{name,cli_version,schema_version}]`; a CI gate
  bumps the skill when the CLI surface changes.
- **Probe:** `$TOOL version --json | jq .skills` · inspect a shipped SKILL.md's frontmatter.
- **Fail:** skill frontmatter without `cli_version`; `skill print` served from a stale copy.

### §18 `doctor`: read‑only self‑diagnostic · **MUST**
- **Applies:** always.
- **Signal:** `$TOOL doctor` (one line/check: `OK`/`WARN`/`FAIL` + summary); `--json` →
  `{schema_version, checks:[{id,status,message,fix_suggestion?}], summary:{ok,warn,fail}}`;
  exit `0` unless any `FAIL` (then `1`); **read‑only by default**, corrective twin is opt‑in
  `--fix`; check categories cover schema/deps/skill‑sync/config/data integrity, each with a
  stable `id`.
- **Probe:** `$TOOL doctor --json | jq '.checks[].id, .summary'`.
- **Fail:** no `doctor` (two family tools miss it); a `doctor` that mutates state by default.

### §19 Deterministic clock: inject time, never read it ad hoc · **MUST‑when‑applies**
- **Applies:** the tool stamps `created`/`updated`, or derives filenames/ids from the clock.
- **Signal:** a single **hidden** global `--frozen-time <RFC3339>`; an injected `Clock`/
  `FakeClock` seam in the core (not scattered `now()`); golden/byte‑stable fixtures; a
  determinism claim only where other nondeterminism (ids, map order, readdir, locale, tz) is
  also pinned.
- **Probe:** `$TOOL <cmd> --help --json | jq '..|.name? // empty' | grep frozen-time`; grep
  core for `Utc::now()`/`SystemTime::now()` vs. a `Clock` trait.
- **Fail:** ad‑hoc `now()` in domain logic; a `--now`/`--frozen-time` synonym pair; using
  `--frozen-time` to forge a real record's provenance.

### §20 `fmt`: idempotent canonicalizer for on‑disk records · **MUST‑when‑applies**
- **Applies:** the tool owns human‑editable on‑disk records (markdown + YAML frontmatter,
  git‑native data files).
- **Signal:** `$TOOL fmt` rewrites to canonical form and is **idempotent** (second run =
  no change); sorts **only** schema‑declared order‑insensitive arrays; **never** bumps the
  `updated:`/modification field; atomic all‑or‑nothing writes; `--dry-run` plan; `--strict`
  no‑write CI mode scoped to formatting only; optional `fmt install-hooks` / `install-merge-driver`.
- **Probe:** `[exec-ro]` `$TOOL fmt --dry-run --json`. `[sandbox-write]` copy a fixture record
  set into a **git-init'd scratch `--home`**, run `fmt` twice, hash the tree before/after:
  the second run is a no-op and `updated:` is unchanged. Never run `fmt` against the tool's
  real records (a buggy `fmt` is exactly what you're testing for).
- **Fail:** a `fmt` that bumps `updated:`; non‑idempotent output; reordering a semantic array.

### §21 `init`: idempotent, no‑clobber bootstrap · **MUST‑when‑applies**
- **Applies:** the tool scaffolds an on‑disk home other commands need.
- **Signal:** `$TOOL init` writes the `.<tool>/` marker + schema/policy scaffold; target home
  resolved explicitly (§8), never silently from cwd (ambiguous → error for `--home`); skill
  install is **not** implicit (prints follow‑up or gated behind `--install-skill`);
  idempotent fill‑in on re‑run; a defined already‑initialized‑vs‑conflicting boundary (foreign
  marker/incompatible version → §4 error, no writes); re‑scaffold behind `--force` that spares
  records; atomic; `--json` reports `{created,existed,skipped}`; `--dry-run` plan.
- **Probe:** `[exec-ro]` `$TOOL init --dry-run --json`. `[sandbox-write]` run `init` twice into
  a **freshly-created empty scratch dir** (verify it did not pre-exist), confirm the second is
  a no-op fill-in. Never `init` into a real/default home (the canon itself warns `init` can
  clobber — that's the failure mode under test).
- **Fail:** an `init` that clobbers/resets an existing home; a silent global skill write; a
  scaffold into a surprising cwd.

### §22 Internal layout: library‑first `core` / `cli` split · **SHOULD** (never a hard gate)
- **Applies:** any tool expected to be deeply unit‑tested, reused as a library, or grown past
  one file.
- **Signal:** `crates/<tool>-core` (pure domain, no clap/I/O, dependency‑light) +
  `crates/<tool>-cli`; the injected `Clock` (§19) lives in core; shared plumbing (§2 error→exit
  map, §10 envelope + `version`, §8 `config`, §15 `skill`, §18 `doctor`) factored into a thin
  `<family>-cli-common` crate rather than re‑hand‑rolled per tool. Single‑crate is acceptable
  for a genuinely small tool as a **documented** choice.
- **Probe:** read `Cargo.toml` `members`; grep core for clap/I/O imports.
- **Fail:** — never a readiness failure; report as a `SHOULD` recommendation only.

### §23 Public artifacts contain no user-specific facts · **MUST**
- **Applies:** every publicly distributed repository, package, scaffold, skill, fixture, and
  documentation set. Internal-only artifacts are outside this rule's binding scope.
- **Signal:** shipped defaults are neutral or absent; exact private markers come only from the
  §8 `user_specific_deny_list` / `PROJECT_CANON_USER_SPECIFIC_DENY_LIST`; the target's git
  remote and package metadata derive known-good own coordinates; generated output and fixtures
  remain fictional. The project's own URL, badge, tap, install coordinate, and genuine public
  dependencies are allowed.
- **Probe:** `$TOOL doctor --json | jq '.checks[] | select(.id == "canon.s23")'`; inspect
  `config show --json` for deny-list provenance; then use `review` to judge hostnames, internal
  URLs, borderline names, public-dependency legitimacy, and generated deployment assumptions.
- **Fail:** a configured private marker in shipped text; a personal path/account/private repo in
  a built-in default, fixture, scaffold, skill, or generated output; a heuristic scanner that
  flags the project's own README install line or public coordinates.

---

## Dimension‑discovery hook (review mode) — active, not passive

The canon grows from practices recurring across **≥2** family tools (shared error‑envelope
shapes, test layout, fixture conventions, telemetry, exit‑code nuances, changelog format,
MSRV policy). A single‑tool review cannot prove recurrence on its own, so make discovery a
**concrete step**, not a hope that the model "notices":

1. When the target exhibits a useful practice the canon doesn't name, first read the existing
   candidates in the target project's issue tracker (and open issues labelled `cli-canon`).
2. If this practice matches an **existing** candidate, **append this tool's evidence to it**
   — reaching the ≥2‑tool threshold is precisely this accumulation.
3. Otherwise `grep` 2–3 other family repos for the same practice. Only when **≥2** tools show
   it do you write a **new** candidate: `applies‑when / look‑for / rationale / evidence:
   [tool→source]`.
4. Stage it (do not auto-file) as a candidate in the target project's issue tracker; it also
   seeds the Phase-4 checker's expanding probe list. A single-tool
   observation is a *watch‑list note*, not yet a candidate.

This is the mechanism that keeps the canon and this skill from freezing at §22.
