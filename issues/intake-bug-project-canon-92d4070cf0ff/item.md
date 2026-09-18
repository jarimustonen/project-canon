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

<!-- intakectl:analysis:start job:ed19a560-c189-495b-8854-75dac65e3d71 generation:0 -->
## Triage analysis

### Root cause

The `agents_md` function in `crates/project-canon-cli/src/new.rs` (line 730) hardcodes `https://github.com/example-org/issuectl` as the issuectl URL in the generated AGENTS.md template. The function only receives `name` and `desc` parameters; it has no access to the `EnvConfig` or any other configuration from which to derive the real issuectl repository URL.

### Code location

- **`crates/project-canon-cli/src/new.rs:748`** — the literal `example-org` URL in the AGENTS.md template string
- **`crates/project-canon-cli/src/new.rs:730`** — the `agents_md` function signature, which lacks any URL/config parameter
- **`crates/project-canon-cli/src/new.rs:602`** — call site where `agents_md` is invoked without config context
- **`crates/project-canon-core/src/env.rs:80`** — the `EnvConfig` struct, which has `gh_account` (usable to derive the issuectl URL) but no dedicated `issuectl_url` field

### Fix scope

1. **Add an `issuectl_url` parameter** to `agents_md` (or alternatively pass `gh_account` and construct `https://github.com/{gh_account}/issuectl` inside the function).
2. **Thread the URL through `build_plan`**: `build_plan` already receives `cfg: &EnvConfig` (which has `gh_account`), so it can derive or pass the URL to `agents_md` via `base_files`.
3. **When no URL is available** (no `gh_account` configured), omit the hyperlink entirely or use a bare text reference without a link — the issue description says "omit the hyperlink when no canonical URL is configured".
4. **Update tests** in `new.rs` that call `agents_md` or `build_plan` — they currently pass `EnvConfig` layers with `gh_account` set to `"example-user"`, which should still produce a valid URL like `https://github.com/example-user/issuectl`.

### Impact

Every repository scaffolded with `project-canon new` (any profile: `cli`, `service`, `library`, `release`) gets an `example-org` placeholder in its AGENTS.md. This is a **public-artifact neutrality violation** (§23): the generated file ships a placeholder that looks like a real coordinate but points nowhere. It also breaks the expected workflow: a developer following the link ends up at GitHub's `example-org/issuectl` 404 page.

### Severity

**High** — affects all scaffolded repos, violates public-artifact neutrality, and creates a broken link in every generated AGENTS.md.

### Recommendation

**Fix**: Derive the issuectl URL from `gh_account` (the already-configured GitHub account) or add a dedicated `issuectl_url` config key. If neither is available, render the issuectl reference as plain text without a hyperlink.

<!-- intakectl:analysis:end job:ed19a560-c189-495b-8854-75dac65e3d71 generation:0 -->

## Environment

- project-canon 0.9.1 (`c50976c7a4c27a71bb5d8180d4badcbbce0c358f`)
- generated profile: `service`
