# Design — canon as an installable skill

Resolves the four forks the issue posed, then specifies the `skill` verb. Owner call
(2026-08-14). This is the mechanism only; the homebase-side consumption
([[homebase-canon-cutover]]) lands later, in that repo.

## The decisive constraint: dogfood §15/§16/§17

The canon this repo maintains **already specifies the shape** of a companion-skill installer:
`AGENTS-AI-FIRST-CLI.md` §15 (`skill list` / `skill install [<name>]` / `skill show`),
§16 (`skill print` — the read-only twin of install), §17 (skill↔CLI version sync, the
`cli_version`/`schema_version` frontmatter, drift warning on install). §14 pins `skill` as a
**closed meta-verb** in the noun-verb vocabulary. project-canon is the maintained home of that
canon (ADR 0009 §6); it must *dogfood* it. So the installer is not a bespoke `install-skills`
subcommand — it is the canonical `project-canon skill …` surface. Where the issue's wording and
the canon disagree, the canon wins (and it's recorded as a discussion item for the owner).

## Fork 1 — one skill or two? → **Two.**

- **`cli-canon`** (already here, `skills/cli-canon/`) is a *behavior* skill: the
  reviewer/generator that **applies** the canon to audit or scaffold *other* family CLIs. It
  carries probe tables, a questionnaire, and templates. Its audience is a maintainer working on
  a family tool, invoked as `/cli-canon`.
- **`ai-first-cli-canon`** (new, this issue) is a *reference/content* skill: it carries the
  **canon text itself** so an agent working inside an adopting repo has the family's binding CLI
  conventions available as an installed, versioned skill — the thing that today is a hand-copied
  `AGENTS-AI-FIRST-CLI.md`.

They are kept **separate, not merged**: merging would force every adopter that just wants the
canon-as-reference to also carry the heavyweight review/generate apparatus (probes, templates),
and would blur "the rules" (content) with "the auditor" (behavior). The content skill is the
one this verb installs; `cli-canon` continues to ship as-is.

## Fork 2 — which command installs it? → **`project-canon skill install`** (the §15 meta-verb).

Surface (mirrors §15/§16 and the family `issuectl`/`orchestratectl` ergonomics):

- `project-canon skill list [--json]` — the shipped skills + one-line descriptions.
- `project-canon skill install [<name>] [--target <dir>] [--agent claude|codex|all] [--force]
  [--dry-run] [--json]` — installs the named skill (default: **all** shipped skills; v0 ships
  one, `ai-first-cli-canon`) into the target.
- `project-canon skill print <name> [--agent claude|codex] [--json]` — §16 read-only twin;
  streams the exact bytes install would write, no side effects.

**Not part of `new`.** `new` scaffolds a *fresh* repo and already bundles the canon file pinned
at creation (that copy does not "drift" — a brand-new repo has nothing to drift from yet). The
drift problem this issue solves is *ongoing sync in already-adopted repos*, which is exactly
what a standalone, idempotent, re-runnable `skill install`/upgrade is for. Keeping the verb
independent (not auto-called by `new`) keeps it composable (canon: composable commands) and
independently testable. Whether `new` should *also* install the skill is a deferred follow-up
(noted for the owner), not wired here.

## Fork 3 — packaging + provenance → **single source via `include_str!`; the skill is generated, never a second on-disk copy.**

The `ai-first-cli-canon` skill body is **assembled by the binary** from:
1. a small frontmatter + provenance header authored as a `const` in `skills.rs`
   (name/description + `cli_version`/`schema_version` per §17), and
2. the canon body via `include_str!("../../../AGENTS-AI-FIRST-CLI.md")` — the **same master**
   `new` already bundles.

There is therefore **no second full copy of the canon checked into `skills/`** — that would be
the very drift we are removing. Unlike `cli-canon` (a hand-authored dir), `ai-first-cli-canon`
is *synthetic*: it materializes only in the install target. The single-source invariant is
mechanically asserted in tests: the installed file must contain the master canon bytes verbatim.

## Fork 4 — install target/format + idempotency.

**Target.** `--target <dir>` is the install **base**; per §15 the default base is `$HOME`
(giving §15's `~/.claude/skills/`). Passing `--target <repo-root>` installs into a *repo's*
agent dirs — the adopting-repo distribution case. Tests always pass a temp `--target`, so no
write ever lands in `$HOME` or this repo's real `.claude/`.

**Format** (per `--agent`, default `all`):
- claude → `<base>/.claude/skills/ai-first-cli-canon/SKILL.md` (YAML frontmatter + provenance
  comment + canon body).
- codex → `<base>/.codex/prompts/ai-first-cli-canon.md` (no frontmatter — matches the shipped
  `/issue` Codex form — provenance comment + canon body).

**Idempotency / upgrade / clobber** (per file):
- absent → **install** (write).
- present, byte-identical → **unchanged** (no write; re-install is a clean no-op).
- present, differs, **ours** (carries the `project-canon skill install` provenance marker):
  read the on-disk `cli_version` from the marker and apply §17 drift rules —
  older/equal → **upgrade** (write; older also emits a §10 drift warning);
  newer than the running binary → **error unless `--force`**.
- present, differs, **not ours** (no marker — a user file at that path) → **refuse unless
  `--force`** (never clobber a foreign file silently).

`--dry-run` computes every per-file action and prints the plan, writing nothing (not even the
target dirs). `--json` emits the §10 envelope (`schema_version`, `tool`, `verb`, `target`,
per-file rows with `agent`/`path`/`action`, `summary`, `exit_code`). Writes use
`create_dir_all` for parents; the skill file itself is written with an explicit
overwrite/no-overwrite decision from the action table (never blind `create_new`, because
*upgrade* must replace).

## Exit codes

- `0` — success: installed / upgraded / unchanged / dry-run plan / list / print.
- `2` — usage/operational: bad flag, bad `--agent`, unknown skill name, foreign-file clobber
  without `--force`, newer-on-disk without `--force`, I/O fault, malformed `PROJECT_CANON_*`.

§16 literally says unknown-name in `print` exits 1; the binary-wide contract (new/review) uses
`2` for *all* usage errors and reserves `1` for a gate outcome (doctor). Consistency across the
binary matters more to an agent than §16's literal digit, so unknown-name is `2` here. Recorded
as a discussion item.

## Non-goals (v0)

- Wiring `new` to auto-install the skill (deferred follow-up).
- A `version --json` `skills:` array (§17 audit hook) — the `version`/`--json` top-level surface
  is a separate deferred verb; `skill list --json` already exposes the versions.
- `skill install` with no `--target` writing to a real `~/.claude` in tests — tests always scope
  `--target` to a temp dir.
