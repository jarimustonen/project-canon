# project-canon 📏

A project-scoped conformance tool for the AI-first CLI / project family. It carries a **base
project canon** plus **per-archetype profiles** (`cli`, `service`, …) and, once built, will
offer three verbs:

- `new` — scaffold a new repo that conforms to the canon (subsumes homebase's `create-project` generator role)
- `doctor` — machine-verify a repo against the applicable profile (a CI conformance gate)
- `review` — a recommending audit against the canon

The AI-first CLI canon (`AGENTS-AI-FIRST-CLI.md`) is carried as the `cli` profile.

**Status: Private, early.** Bootstrap scaffold only — the tool is not implemented yet; work is
tracked as issues in this repo. Design rationale: homebase ADR 0009.

## License

MIT.
