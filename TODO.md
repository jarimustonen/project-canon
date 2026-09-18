# TODO

Pointers to open issues. Descriptions and plans live in the linked
`issues/<slug>/item.md` — do not duplicate them here.

## 🔄 Continue here (handoff)

_**2026-09-18 — Canon skill-only scaffolding shipped in Project Canon 0.9.1.** `project-canon new`
no longer generates a repo-local `AGENTS-AI-FIRST-CLI.md`; generated `AGENTS.md` and
`CONFORMANCE.md` direct agents to the versioned `/ai-first-cli-canon` skill instead. The full Rust
green gate passed, and Shipshape verified the crates.io packages, GitHub Release, cargo-dist
artifacts, Homebrew formula, tag, and advanced `main`._

_**Consumer and machine convergence.** The downstream CLI repository removed its stale canon copy
and updated its two references; `project-canon doctor` reports it conformant. Project Canon 0.9.1
and its complete skill catalog are installed on the Linux release host and through Homebrew on the
attached macOS seat. The cargo-dist family is aligned on the newest stable 0.33.0 release, and the
disposable release binary was removed after use._

_**Follow-up context.** Release execution exposed two Homebase integration gaps: autonomous
Shipshape cuts need a safe, non-persistent path for disposable cargo-dist plus SOPS-backed registry
credentials, and focused fleet convergence must detect an outdated but correctly owned Homebrew
formula instead of skipping it. These were filed to Homebase intake. Three Project Canon intake
candidates remain untriaged context awaiting human lane-or-close decisions: separate repository
and CLI names, profile selection for generated non-CLI repositories, and the generated Issuectl
link placeholder. They are not accepted agenda items._

_**Direction from here.** There is no accepted follow-up agenda prepared. Use the live issue DAG
for scheduling state and perform a fresh planning pass before starting new product work._

## Scheduling

Canonical scheduling lives in `issuectl` frontmatter (`lane:`, `lane_seq:`, `blocked_by:`, `collision:`). Do not maintain a markdown DAG or adjacent backlog in this file.

Use these views instead:

```bash
issuectl dag
issuectl dag --json
issuectl ls --status open
issuectl ls --status in-progress
```

`TODO.md` is only the session handoff and project notes; issue bodies and `issuectl dag` are the source of truth.
