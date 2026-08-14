---
created: 2026-08-14
updated: 2026-08-14
type: feature
status: done
priority: normal
epic: project-canon-v0
labels: [tooling]
commits:
- hash: a8d69e3
  summary: skill install/list/print verb, design.md, tests
- hash: '4440648'
  summary: apply multi-model review FIXes
closed: 2026-08-14
---

# Canon as an installable skill that project-canon installs (not a copied markdown file)

## Description

Turn the AI-first CLI canon (`AGENTS-AI-FIRST-CLI.md`, §1–§22) into a **skill that
`project-canon` installs** into consuming repos, mirroring how `issuectl init` installs the
`/issue` skill and `orchestratectl` installs its worktree skills. Today the canon is a
markdown doc that other repos are expected to **copy** ("homebase copies FROM here"); the
adoption/distribution mechanism should instead be a `project-canon`-managed skill install, so
consuming repos get the canon as an installed, versioned skill rather than a hand-copied file
that drifts.

## Why

Owner call (2026-08-14). A copied `.md` drifts and has no install/upgrade story; the family's
other tools (`issuectl`, `orchestratectl`) already distribute their canon/help **as installed
skills**. Making the canon an installable skill gives it the same versioned install/upgrade
path and removes the "copy the file" step from every adopting repo.

## Scope / open questions (do NOT pre-design — settle in the issue's design.md)

- Relationship to the existing `skills/cli-canon/` companion skill (apply-the-canon) vs. the
  canon **content** itself becoming a skill — one skill or two?
- Which verb/subcommand installs it (part of `new`? a dedicated `install`/`skills` command?).
- Skill-install target/format (Claude + Codex, like issuectl's `--agent`).
- **Reshapes [[homebase-canon-cutover]]**: the cutover becomes "install the canon skill from
  project-canon", not "copy the markdown". Reconcile that issue's plan once this lands.

## Relationship to rollout

Part of the adoption story, so it sits **before/with** the release + go-wide gate: the
install mechanism is how the canon actually reaches homebase and the other repos. Sequence
in the `build` lane (touches the CLI surface + skill packaging); place after the v0 verbs
unless the owner reprioritizes.

