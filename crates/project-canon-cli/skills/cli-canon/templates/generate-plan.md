# Generation plan — canon‑conformant CLI scaffolding

Generate mode does **not** dump the whole canon. It (1) decides which sections *apply* to
the tool's shape, then (2) emits targeted scaffolding + a conformance TODO for exactly those
sections. Read the canon (`AGENTS-AI-FIRST-CLI.md`) fresh before generating — cite by `§N`.

## Step 1 — Characterize the tool (the shared eight questions)

Use the **eight yes/no questions in SKILL.md § *Characterize the tool*** — the single source
of truth both modes read (Q1 multi-resource → §6 · Q2 config/data-root → §8 · Q3 mutating →
§11 · Q4 long-running → §12 · Q5 timestamps → §19 · Q6 records → §20 · Q7 scaffolds-home →
§21 · Q8 large results → §13). Each answer switches conditional sections on/off; if several
are unsettleable, ask the user all of them in one message.

**Always-on (every family CLI, regardless of shape):** §1 strict validation · §2 structured
output + classified exit codes + JSONL logs + `--verbose` · §3 no interactive prompts · §4
informative errors · §5 composable · §7 verb vocabulary · §9 fixed non-TTY format · §10
schema/version/provenance + error envelope · §14 agent-first `--help` · §15 `skill install` ·
§16 `skill print` · §17 skill↔CLI version sync · §18 `doctor`.

**Internal (SHOULD):** §22 `core`/`cli` split — recommend it up front; a new tool should
start in that shape, not refactor into it later.

## Step 2 — Confirm write intent (preview by default)

Generate **defaults to a preview**: show the plan + scaffold, write nothing. Persist files
into the target repo only when the user asks (`--write` / "write it"), the target repo is
confirmed and its tree clean-or-consented, and writes stay inside the repo root. Never run a
generated `build.rs` (or other generated code) during generation. For a brand-new repo, run
`/create-project` first (it copies the canon in verbatim), then generate the surface into it.

## Step 3 — Emit the scaffold + guidance

For each applicable section, emit the *conformant shape*, not the canon prose. Use these
**canonical reference samples** so generate and review agree on the exact shapes:

*§2 — one central error→exit map* (route every error through it, don't reinvent per handler):

| condition | exit |
|---|---|
| success · clap `--help`/`--version` display | `0` |
| `invalid_*`, `not_found`, `already_exists`, `validation_*`, usage/parse error, `dry_run_unsupported` | `1` |
| `io_*`, `network_*`, `internal_*`, dependency-missing, invariant-violation | `2` |
| SIGINT / SIGTERM | `130` / `143` |

*§10 — `version --json` payload* and *error envelope* (on stderr):

```json
{"version":"0.6.3","commit":"<40-hex-sha | null>","schema_version":1,
 "supported_schemas":[1],"skills":[{"name":"foo","cli_version":"0.6.3","schema_version":1}]}
```
```json
{"schema_version":1,"error":{"code":"invalid_target","message":"Invalid target 'foobar'. Available: local, staging, demo, prod","invalid_value":"foobar","expected":["local","staging","demo","prod"]}}
```
(`commit` is a real 40-hex SHA or exactly `null` + a `build_provenance {kind,note}` sibling —
never `"unknown"`.)

Then, per section:

- **Surface & verbs (§6/§7):** propose the subcommand tree — resource nouns × the five CRUD
  verbs, plus only the sanctioned meta‑verbs the tool needs (`version`, `doctor`, `config`,
  `skill`, and `init`/`fmt` if Q6/Q7). Flag any verb that would violate §7.
- **Shared plumbing (§22 + §2/§8/§10/§15/§18):** direct the tool at (or propose) a thin
  `<family>-cli-common` crate carrying the **one central error→exit map**, the §10 envelope +
  `version` payload, and the `config`/`skill`/`doctor` scaffolds — rather than hand‑rolling
  the envelope again. Give the error→exit table (§2) and the `version --json` shape (§10)
  explicitly.
- **Config & data root (§8, if Q2):** the `config path` + `config show --json` (source‑
  tagged, secret‑redacted) surface, the `--home`/`$<TOOL>_HOME` selector, the five‑layer
  precedence, and the `data_root_unresolved` error.
- **Provenance (§10):** the `build.rs` (or toolchain equivalent) that stamps the 40‑hex SHA
  into `commit`; the `null` + `build_provenance` fallback for no‑git builds. Never `"unknown"`.
- **Companion skill (§15/§16/§17):** a `skill` subcommand (`list`/`install`/`print`), an
  in‑repo `SKILL.md` with `cli_version` + `schema_version` frontmatter, `print` pinned to the
  binary, and the CI gate that bumps the skill on surface changes.
- **Conditional lifecycle (§11/§12/§19/§20/§21):** emit only for the questions that answered
  yes — the dry‑run planning envelope, the JSONL streaming contract, the `Clock` seam +
  `--frozen-time`, the idempotent `fmt`, the no‑clobber `init`.
- **`doctor` (§18):** the read‑only check set (schema/deps/skill‑sync/config/data), each with
  a stable `id`, and the `--fix` opt‑in twin.

## Step 4 — Emit the conformance TODO as the matrix skeleton

Produce the acceptance gate = `templates/conformance-probes.md` **filtered to the applicable
sections**, rendered as the **same matrix shape review later populates** (`§ | Dimension |
Status | Evidence | Severity`) with every `Status` starting unchecked/`todo`. The new tool
flips each to `pass` with evidence; generate's output is thus a direct review input, and the
two modes are verifiably the same contract.

## Step 5 (optional) — Review the plan

For a substantial new tool, write the plan to a file and run **one** `consult-llm --task
review` pass with the canon excerpt as the rubric before the author commits — not
`/llm-review` (its prompt is a general code-review with no rubric parameter and it writes to
the repo's history), and not `/assess-findings` (its production-likelihood triage doesn't fit
normative canon conformance — the severity model above is the triage).
