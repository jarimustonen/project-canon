---
created: 2026-08-23
updated: 2026-08-23
type: improvement
reporter: jari
status: open
priority: normal
lane: build
---

# Enforce the 1024-char skill description limit mechanically

## Description

Canon §15 now states the Agent Skills format limit: frontmatter description ≤ 1024 characters (over-limit descriptions are rejected/truncated by consuming runtimes, silently breaking discovery). Add a mechanical check so the limit cannot regress: (a) a unit test over the bundled skills' rendered frontmatter (ai-first-cli-canon is synthetic — measure the rendered form), and (b) a review/doctor probe for repos whose skills project-canon can locate. Current lengths: ai-first-cli-canon 390, cli-canon 867.
