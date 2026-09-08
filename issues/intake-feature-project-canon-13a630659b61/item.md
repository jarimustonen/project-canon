---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jari
status: untriaged
priority: normal
provenance: agent:homebase-wrapup
source_ref: agent:homebase-wrapup/reporter:jari/id:homebase-wrapup-project-canon-separate-repo-cli-name-2026-09-08
---

# Allow repository and CLI names to differ in project-canon new

## Description

`project-canon new` currently uses one `--name` value for both generated project/package naming and the GitHub repository hook. This prevents a common CLI naming pattern where the repository is suffixed with `-cli` but the executable and project name are not.

Exact command:

```sh
PROJECT_CANON_GH_ACCOUNT=jarimustonen \
PROJECT_CANON_REPO_ROOT="$HOME/Sources" \
PROJECT_CANON_TW_ENABLED=true \
PROJECT_CANON_TW_PROJECTS_CONF="$HOME/.config/tmux/projects.conf" \
project-canon new "$HOME/Sources/reitti-cli" \
  --profile cli \
  --name reitti \
  --description 'Plan and compare HSL-area journeys' \
  --emoji '🚌' \
  --assume-defaults \
  --dry-run \
  --json
```

Observed:
- `--name reitti` produces the desired Rust package/binary naming, but the `github-create` hook proposes `gh repo create reitti` rather than `reitti-cli`.
- Using `--name reitti-cli` instead makes the CLI-profile package name `reitti-cli-cli` and does not produce the desired `reitti` command.

Expected:
Allow callers to independently specify the repository/GitHub slug and the canonical project or executable name, while preserving current defaults when only one name is provided. The dry-run JSON hooks should use the repository slug; generated package and binary files should use the project/executable name.

## Triage analysis

**Classification:** Feature request — the report describes a legitimate limitation in `project-canon new` where a single `--name` value is used for both the generated package/binary names and the GitHub repository slug, preventing a common `repo-name-cli` → `repo-name` naming pattern.

**Likely implementation surfaces:**

1. **`crates/project-canon-cli/src/new.rs` — `bootstrap_hooks` (line 647)**
   The central touch point. Currently uses `name` for both the Rust package/binary naming (via `cli_surface_files`, `base_files`, etc.) and the GitHub repository slug (`github-create`, `git-remote-ssh`, `tw-register` hooks). A `--repo-name` / `--gh-repo` flag would be introduced; `bootstrap_hooks` would receive a separate `repo_name: &str` parameter and use it for the `ssh_url`, `repo_slug`, and tw registry line, while `name` continues to drive the package/binary templates.

2. **`crates/project-canon-cli/src/new.rs` — `NewArgs` struct (line ~184)**
   Needs a new `repo_name: Option<String>` field. Default behaviour: when absent, `repo_name` falls back to `name` (preserving the current single-name contract).

3. **`crates/project-canon-cli/src/new.rs` — `parse_args` (line ~218)**
   Needs a `--repo-name` flag handler that stores the value and validates it with `validate_name` (same slug constraints as `--name`).

4. **`crates/project-canon-cli/src/new.rs` — `build_plan` (line ~580)**
   Needs to pass `repo_name` (resolved from `repo_name` option → `name` fallback) into `bootstrap_hooks`.

5. **`crates/project-canon-cli/src/new.rs` — `run` function**
   Resolves the effective repo name: `repo_name = parsed.repo_name.clone().unwrap_or(name.clone())`. Passes `repo_name` to `build_plan` alongside `name`.

6. **`crates/project-canon-cli/src/new.rs` — `HELP` text (line ~485)**
   Document `--repo-name` flag with its defaulting behaviour.

7. **`crates/project-canon-cli/src/new.rs` — `Report` struct and `to_json` method**
   The JSON output should include a `repo_name` key alongside the existing `name` so machine consumers (e.g. the intake launcher) can inspect the resolved values independently.

8. **`crates/project-canon-core/src/env.rs` — `EnvConfig` / `EnvConfigLayer`**
   Optionally, a `repo_naming_suffix` convention (e.g. `-cli` suffix appended to `name` for the repo slug) could be added as an env-config extension point, though the explicit `--repo-name` flag is the primary solution.

9. **Test fixtures: `crates/project-canon-cli/tests/new_cli.rs` and `new.rs` tests**
   Need new test cases exercising `--repo-name` with a different value from `--name`, verifying the hooks use `repo_name` while generated files use `name`. Also a fallback test (no `--repo-name` → hooks use `name`).

10. **`crates/project-canon-core/src/scaffold.rs`**
    No changes needed — scaffold dimensions are name-agnostic at this layer.

**Design notes:**
- The `repo_name` must pass through the same `validate_name` slug constraints as `name` (leading ASCII letter, alphanumerics/`-`/`_`, ≤64 chars) to prevent path traversal and shell injection in the hook commands.
- The JSON report should surface both names so the caller can verify which value was used for which purpose.
- The `--repo-name` flag is optional; when omitted the behaviour is identical to today (single-name model), so this is a strictly additive change with no breaking impact.
- The tw registry line uses `name` for the display name (the project name as registered) and `repo_name` for the slug portion of the SSH URL — this is the correct split because the registry key should match the project name, not the GitHub slug.
