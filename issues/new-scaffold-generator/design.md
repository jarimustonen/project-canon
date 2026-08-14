# `new` — scaffold a conformant repo (design note)

The **generator half** of `project-canon` (ADR 0009 §2/§6): emit a repo that *starts
conformant*. It reads the two-layer model from `project-canon-core` (BASE ∪ PROFILE), writes
the base-canon files + the selected profile's surface scaffolding into a target directory, and
emits — but never runs — the external/irreversible bootstrap actions (git init, private GitHub
repo, `issuectl init`, `tw` registration) as a **hook plan** sourced from the `env` config/hook
layer. Subsumes `create-project`'s generator role. The companion `cli-canon` *generate* mode is
folded in as the `cli`-profile surface scaffold.

## The side-effect boundary (read this first)

`new` at v0 makes **exactly one kind of side effect: it writes files into the target directory
the caller named.** Nothing else. Concretely:

| action | who does it | when |
|---|---|---|
| write template files / dirs into `<dir>` | **`new` itself** | on a real (non-`--dry-run`) run |
| `git init`, `issuectl init` (local bootstrap) | **emitted as a hook, not executed** | printed for the caller to run |
| `gh repo create` (private GitHub), `tw` registration (external/irreversible) | **emitted as a hook, not executed** | printed for the caller to run |

This is deliberately *stronger* than "external actions are default-off in `--dry-run`": at v0
`new` **never executes any subprocess at all** — not `gh`, not `tw`, not `git`, not `issuectl`.
Every action beyond writing files is rendered (with its exact command, filled from the env
layer) into a **hook plan** the caller runs deliberately. The file writes are the only effect,
they land only inside the named directory, and they are trivially reversible (`rm -rf <dir>`).
A `run-hooks` execution seam (opt-in, per-class, external default-off) is a **documented
deferred follow-up** — see "Deferred". This keeps testing safe by construction: the tests
exercise file generation + the dry-run plan only, and there is no code path that shells out.

Because `git init` and `issuectl init` are hooks, a freshly-generated tree has **no `.git` and
no `issues/`** until those two local hooks run — so a `new` → `doctor` round-trip is green only
*after* the two local hooks execute. The round-trip test models this by creating `.git` +
`issues/` (simulating the two local hooks) and then running the real `doctor` binary. `new`
does not hand-create `issues/`: that directory is `issuectl`'s to own (per create-project), so
`new` leaves it to the `issuectl init` hook.

## Command surface

```
project-canon new [--profile <archetype>] [--name <name>] [--description <text>] [--emoji <glyph>]
                  [--assume-defaults] [--dry-run] [--force] [--json] [--verbose] [--help] <dir>
```

| flag/arg | default | meaning |
|---|---|---|
| `<dir>` (positional, required) | — | Target directory to scaffold into. Created if absent. Non-empty → refused unless `--force`. Missing positional → exit `2` (no cwd default — scaffolding into cwd is a footgun). |
| `--profile <archetype>` | `cli` | Which profile's surface scaffold to fold in. `cli`/`service`/`library`/`release`. Unknown → exit `2`. Non-`cli` profiles emit base only (empty extension points at v0). |
| `--name <name>` | last path component of `<dir>` | Project name used in templates + the crate/workspace names. **Validated as a strict slug** (leading ASCII letter, then ASCII alnum/`-`/`_`, ≤64) — the security boundary that keeps the name out of path-traversal, flag-injection, and broken-manifest trouble everywhere it flows. A bad derived-from-dir name fails loudly; pass `--name` to override. |
| `--description <text>` | a placeholder line | Fills the one-line description slot in `AGENTS.md`/`README.md`. |
| `--emoji <glyph>` | env layer's `workmux_emoji_prefix` (else none) | `.workmux.yaml` / `tw` window prefix. |
| `--assume-defaults` | (implied) | Characterize non-interactively with the conservative questionnaire (all conditional triggers **off**) — the only mode at v0 (§3: `new` never prompts). Named explicitly; reserves the seam for a future characterization path. |
| `--dry-run` | off | Write nothing; print the file plan + hook plan (§11 planning-envelope discipline). |
| `--force` | off | Permit generating into a **non-empty** directory (fills gaps only; never overwrites an existing file). |
| `--json` | off | Emit the §10 structured plan/report on stdout instead of the human view. |
| `--verbose` | off | Also list `skip-exists` file rows + the full resolved conformance section list. |
| `--help` | — | Usage, exit `0` (§2). Short-circuits before env validation. |

Strict validation (§1), uniform with `doctor`: unknown flags, a bad `--profile`, a missing flag
value, a repeated flag, an inline value on a valueless flag (`--json=x`), an empty `--name`, or a
second positional all echo the offending token and exit `2`. `--` stops flag parsing.

## What gets generated

### Base scaffold (archetype-invariant — every `new`)

| path | kind | notes |
|---|---|---|
| `AGENTS.md` | file | From the base template; `{name}` + description filled. The consolidated per-dir doc (base.doc-pattern). |
| `CLAUDE.md` | symlink→`AGENTS.md` | Unix: a real symlink. Non-unix: a verbatim copy (doctor accepts a regular file). |
| `AGENTS-AI-FIRST-CLI.md` | file | The canon, **bundled verbatim** via `include_str!` — project-canon is the maintained home of the canon (ADR 0009 §6), so the binary carries it. |
| `README.md` | file | Human front door: title (+ emoji), one-line description, "Status: Private, early", MIT. |
| `.gitignore` | file | `history/` + (cli profile) `/target`. |
| `.workmux.yaml` | file | `window_prefix: "<emoji> "` when an emoji is resolved; a comment-only stub otherwise. |

`new` does **not** create `issues/` (issuectl-owned) or `.git` (git-init hook) — see the
boundary section.

### `cli`-profile surface scaffold (folds in cli-canon *generate* mode)

Emits the canon-conformant §22 core/cli split so a `cli` repo starts in the right shape:

| path | kind | notes |
|---|---|---|
| `Cargo.toml` | file | Workspace: `members = ["crates/<name>-core","crates/<name>-cli"]`. |
| `crates/<name>-core/Cargo.toml` | file | Pure-domain crate manifest. |
| `crates/<name>-core/src/lib.rs` | file | Domain stub — no clap/I/O (§22). |
| `crates/<name>-cli/Cargo.toml` | file | Thin binary manifest (depends on `-core`). |
| `crates/<name>-cli/src/main.rs` | file | Minimal binary stub. |
| `CONFORMANCE.md` | file | The **conformance-TODO matrix skeleton** (generate-plan Step 4): one row per *resolved* canon §N with `Status: todo`, so the new tool flips each to `pass` and the file is a direct `doctor`/`review` input. Conditional (n/a) sections are listed with their gating `Qn`. |

`service`/`library`/`release` resolve to base only (empty extension points), so `new --profile
service` emits just the base scaffold + a `CONFORMANCE.md` covering the base-canon sections.

### Hook plan (emitted, never executed at v0)

Rendered from the model + `EnvConfig`, in run order. Each carries an `id`, a `class`
(`local`/`external`), a human `description`, and the exact `command`:

| id | class | command (filled from `EnvConfig`) |
|---|---|---|
| `git-init` | local | `git init` + set default branch `main` |
| `issuectl-init` | local | `issuectl init` (owns `issues/`, `.issuectl/`, the `/issue` skill) |
| `github-create` | external | `gh repo create <name> --private --source=. --remote=origin --push` under `gh_account` |
| `git-remote-ssh` | external | force the origin to `git@github.com:<gh_account>/<name>.git` |
| `tw-register` | external | append `<name>  <repo_location>  <ssh_url>  emoji:<emoji>` to `tw.projects_conf` — **only when `tw.enabled`** |

Ordered `git-init → issuectl-init → git-commit → github-create → git-remote-ssh → tw-register`
(a `git-commit` step so the `--push` has something to send). Every interpolated value is
POSIX-`shell_quote`d (single-quote wrapped — Rust's `{:?}` is *not* a shell escaper), so the
printed command is safe to copy-paste even with a metacharacter in an env value. `github-create`
account-qualifies the repo (`gh repo create <account>/<name>`) so it is created under
`gh_account`, not gh's default. Each hook carries a `cwd` (the scaffolded repo) surfaced in the
human banner + `--json`, so a caller doesn't run `git init`/`--source=.` in the wrong repo. The
`tw-register` line records the **actual target** the caller generated into, not
`repo_location(name)`.

All env specifics (`gh_account`, `repo_root`/`repo_location`, `tw.projects_conf`, `tw.enabled`,
`workmux_emoji_prefix`) come from the resolved `EnvConfig` (defaults → file → env), never
hardcoded in the verb (ADR 0009 §6). `~`-relative paths are expanded at the edge via
`EnvConfig::expand_home` before being shown.

## `--json` schema (§10-shaped)

```json
{
  "schema_version": 1,
  "tool": "project-canon",
  "verb": "new",
  "target": "/abs/path/to/dir",
  "name": "foo",
  "profile": "cli",
  "surface_shape": "flat-verb",
  "dry_run": true,
  "force": false,
  "files": [
    {"path": "AGENTS.md", "kind": "file", "action": "create"},
    {"path": "CLAUDE.md", "kind": "symlink", "action": "create"},
    {"path": "crates/foo-core/src/lib.rs", "kind": "file", "action": "create"}
  ],
  "hooks": [
    {"id": "git-init", "class": "local", "description": "...", "command": "git init && git branch -M main"},
    {"id": "github-create", "class": "external", "description": "...", "command": "gh repo create foo --private ..."}
  ],
  "conformance_sections": [1,2,3,4,5,6,7,9,10,14,15,16,17,18,22],
  "summary": {"files": 11, "written": 11, "skipped": 0, "hooks": 5},
  "exit_code": 0
}
```

- `kind` ∈ `file | symlink`; `action` ∈ `create | skip-exists` (`skip-exists` only under
  `--force` into a non-empty dir, or `--dry-run` when the file already exists).
- `class` ∈ `local | external`; each hook also carries `cwd`; `surface_shape` ∈ `flat-verb |
  noun-verb | null` (null for non-cli). `summary` uses **stable keys** regardless of mode:
  `create` (plan size), `written` (0 under `--dry-run`), `skipped`. Data → stdout, diagnostics →
  stderr (§2). Same in-crate escape-correct JSON writer as `doctor` (workspace stays
  dependency-free).

## Exit-code contract

| exit | meaning |
|---|---|
| `0` | success — plan printed (`--dry-run`) or files written |
| `2` | usage / operational error — bad flag, bad `--profile`, missing/empty `<dir>` positional, non-empty target without `--force`, an I/O write fault, or malformed `PROJECT_CANON_*` env |

`new` has no gate semantics, so exit `1` is **reserved/unused** (kept distinct from `doctor`'s
gate `1`). A tripped clobber guard is a caller-actionable precondition error → `2`. `--help` is
`0` and short-circuits before env validation.

## Placement

All verb logic lives in **`crates/project-canon-cli/src/new.rs`**, disjoint from `doctor.rs`
(the two share only `json.rs`), so the `build` lane can split later. The core model is consumed
unchanged (`Model::standard()` → `resolve`); the plan-building functions are pure (they take
`(name, description, emoji, resolution, EnvConfig)` and return a `ScaffoldPlan` value) and are
unit-tested in-module, with file I/O confined to a thin `apply` step at the edge — mirroring
core's "I/O at the edge" discipline. No template/scaffold content is added to `core` (kept
intact per the issue). `main.rs` gains the `new` dispatch arm.

## Safety hardening (from the multi-model review)

- **Name validation** (`validate_name`) runs before the name reaches any path/template/hook —
  closing path traversal (`--name ../x`), flag injection into the printed plan, and invalid
  Cargo/Rust identifiers at the source.
- **Atomic no-clobber writes**: regular files are created with `OpenOptions::create_new(true)`,
  so "never overwrite / never follow a final-component symlink" is an OS-level guarantee, not a
  TOCTOU-racy check.
- **Symlink-root refusal**: a target path that is itself a symlink (or a non-directory) is
  refused (`DirState::NotADir` → exit 2) — following it could let writes escape the target.
- **Shell-safe hooks**: `shell_quote` (POSIX single-quote) on every interpolated value.
- **Residual (deferred, see below)**: a *parent-directory* symlink pre-existing inside a
  `--force`d tree can still be traversed by `create_dir_all` — the reachable vectors (crafted
  name, symlink root, final-component race) are closed; the capability-based (`openat`/`cap-std`)
  full fix is out of scope for v0.

## Testing (temp dirs only — no network, no `gh`/`tw` subprocess; one offline `cargo build`)

- **Unit (`new.rs`)**: arg parsing (strict holes), plan building (base files present; cli adds
  the crate split + `CONFORMANCE.md`; non-cli emits base only), the emoji/description/name
  substitution, hook rendering pulls from `EnvConfig` (gh account, tw conf, tw-disabled drops
  `tw-register`), clobber logic (empty vs non-empty vs `--force`), and never-overwrite.
- **Integration (`tests/new_cli.rs`)**: `new <temp>` writes the expected tree; `--dry-run`
  writes **nothing** but prints the plan; `--json` shape; non-empty target → exit 2, `--force`
  fills gaps; bad flag/profile → exit 2. **Round-trip**: `new` into temp, simulate the two local
  hooks (`mkdir .git`, `mkdir issues`), run the real `project-canon doctor <temp>` binary, assert
  exit 0 (`conformant`). This proves `new` emits a doctor-passing repo once bootstrapped.

## Deferred / out of scope (→ follow-ups, not filed as issues)

- A `run-hooks` execution seam (opt-in, per-class; external actions default-off) — v0 prints,
  never runs, which is the safe boundary.
- **Capability-based filesystem apply** (`openat`/`cap-std`) to also defeat *parent-directory*
  symlink traversal under `--force` in an untrusted tree — the reachable vectors are already
  closed (name validation + symlink-root refusal + `create_new`); this is the belt-and-braces
  remainder.
- **Packaging the bundled canon**: `include_str!("../../../AGENTS-AI-FIRST-CLI.md")` reaches
  outside the crate dir; fine in-workspace, but a future `cargo publish`/release pipeline should
  move the asset into the crate (or copy via `build.rs` → `OUT_DIR`). No release pipeline exists
  yet (per repo AGENTS.md), so deferred.
- **`LICENSE` emission** (the templates declare MIT but no `LICENSE` file is written — the
  copyright holder/year is env-specific; `create-project` likewise omits it).
- Interactive/mechanical archetype + Q1–Q8 characterization (judgment) — `new` uses the
  conservative questionnaire, same as `doctor`.
- Richer profile scaffolds (conditional-section stubs when a future flag turns a trigger on;
  `service`/`library`/`release` content when those profiles gain members).
- Non-UTF-8 argv/env handling (`std::env::vars()` can panic) — a family-wide pattern the owner
  already closed as `wontfix` (`osstring-argv-env`); not re-litigated here.
- `clap` adoption for the multi-verb surface + §14 `--help --json` — a cross-verb concern.
