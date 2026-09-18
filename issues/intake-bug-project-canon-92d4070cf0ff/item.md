---
created: 2026-09-18
updated: 2026-09-18
type: bug
reporter: jari
status: untriaged
priority: normal
provenance: agent:homebase-wrapup
source_ref: agent:homebase-wrapup/reporter:jari/id:wrapup-native-agent-host-placeholder-issuectl-link
---

# Generated AGENTS.md links issuectl to example-org placeholder

## Description

Generated AGENTS.md links issuectl to example-org placeholder

## Observed

Creating a repository with:

```sh
project-canon new "$HOME/Sources/native-agent-host" \
  --profile service \
  --name native-agent-host \
  --description "Multi-user web platform for hosting native AI agent sessions and application-specific agent interfaces." \
  --emoji "🤖" \
  --assume-defaults \
  --json
```

generated this text in the root `AGENTS.md`:

```markdown
Issue tracking is managed by [`issuectl`](https://github.com/example-org/issuectl).
```

The `example-org` URL is a scaffold placeholder rather than the real project location.

## Expected

Generated public artifacts should use issuectl's canonical repository URL or omit the hyperlink when no canonical URL is configured. A newly generated repository should not contain `example-org` placeholders.

## Environment

- project-canon 0.9.1 (`c50976c7a4c27a71bb5d8180d4badcbbce0c358f`)
- generated profile: `service`
