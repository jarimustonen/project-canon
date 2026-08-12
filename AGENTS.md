# project-canon

Project-scoped conformance tool for the AI-first CLI / project family. Ships a **base
project canon** plus **per-archetype profiles** (`cli`, `service`, …), with the AI-first CLI
canon (`AGENTS-AI-FIRST-CLI.md` §1–§22) as the `cli` profile. Intended verb surface:
`new` (scaffold a conformant repo), `doctor` (machine conformance gate for CI), `review`
(recommending audit). Consumes an expanding dimension-discovery registry. See homebase
**ADR 0009** (`docs/decisions/0009-project-canon-scope.md`) for the scope/subsumption/name
decision that created this repo.

> **Status: bootstrap only.** This repo currently holds the scaffold + the shared canon copy.
> The canon/skill extraction from homebase and the `new` / `doctor` / `review` implementation
> are **not built yet** — they are tracked as issues here. **Open an issue before building a
> feature**; do not pre-design the tool in this file.

## CLI Design Principles

This project follows the AI-first CLI conventions in [`AGENTS-AI-FIRST-CLI.md`](AGENTS-AI-FIRST-CLI.md) — strict input validation, `--json` output, JSONL logs, no interactive prompts, informative errors, composable commands. Read that file before designing or changing CLI surface. The file is a verbatim copy from `homebase`; treat it as shared canon, not a project-local doc to edit. (Note: making this repo the *maintained home* of that canon — so homebase copies from here rather than the reverse — is part of the planned extraction, per ADR 0009; until that lands, homebase remains the source.)

## Gitignored directories

- `history/` — agent scratchpad and ephemeral planning docs (not tracked)
- `/target` — Rust build artifacts

## Documentation Pattern

Every directory follows this structure:

- `CLAUDE.md` — symlink to `AGENTS.md`
- `AGENTS.md` — all AI-relevant info (consolidated)
- `AGENTS-<TOPIC>.md` — complex topics split out (optional)

## Issues & Planning

Issue tracking is managed by [`issuectl`](https://github.com/jarimustonen/issuectl). Use the `/issue` skill (installed by `issuectl init`) to create, search, update, and close issues.

- `issues/<slug>/item.md` — every issue and epic (flat layout — no numeric prefix, no `open/closed/` split)
- Status lives in the `status:` frontmatter field, not in the path
- `issues/AGENTS.md` — issue schema, types, workflow (owned by issuectl)
- `.issuectl/AGENTS.md` — repo-local policy for AI agents (owned by issuectl)

All planning documents (plans, analyses, validations, designs, breakdowns, todos) belong under their parent issue directory — not as standalone files. If work needs a planning document, it also needs an issue.

- `issues/<slug>/plan.md` — architecture, implementation plans
- `issues/<slug>/analysis.md` — research and analysis
- `issues/<slug>/design.md` — design documents
- `issues/<slug>/breakdown.md` — epic → child-issue breakdown with dependencies and critical path
- `issues/<slug>/todo.md` — task checklists
