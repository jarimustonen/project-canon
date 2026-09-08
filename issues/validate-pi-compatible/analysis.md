# Pi-compatible Agent Skills requirements

Source baseline reviewed on 2026-09-08:

- [Agent Skills specification](https://agentskills.io/specification.md)
- [Agent Skills client implementation guide](https://agentskills.io/client-implementation/adding-skills-support.md)
- [pi 0.84.4 skill documentation](https://github.com/earendil-works/pi/blob/v0.84.4/packages/coding-agent/docs/skills.md)
- [pi 0.84.4 skill loader](https://github.com/earendil-works/pi/blob/v0.84.4/packages/coding-agent/src/core/skills.ts) and [frontmatter parser](https://github.com/earendil-works/pi/blob/v0.84.4/packages/coding-agent/src/utils/frontmatter.ts)

Project Canon should validate the strict portable format. Pi's deliberate leniency is useful at load time but is not a suitable release standard for artifacts intended for Claude, pi, and Codex.

## Requirements matrix

| Requirement | Agent Skills specification | pi 0.84.4 behavior | Project Canon enforcement |
|---|---|---|---|
| Skill form | Directory containing exact `SKILL.md` | Recursively discovers `SKILL.md`; also accepts direct root `.md` skills in pi-native locations | Validate declared `SKILL.md` trees; do not treat unrelated `.md` files as portable trees |
| Frontmatter | YAML frontmatter followed by Markdown | Uses the `yaml` parser; malformed YAML is warned and skipped | MUST parse as YAML with opening/closing fences and a top-level mapping |
| `name` | Required string, 1–64 chars, ASCII lowercase/digits/hyphens, no edge/consecutive hyphens, matches parent directory | Missing name falls back to parent; syntax/mismatch warnings generally still load; mismatch is deliberately allowed | Enforce the strict portable constraints, including parent match |
| `description` | Required non-empty string, 1–1024 chars; should explain what and when | Missing/non-string/empty is skipped; over-limit warns but loads | Enforce type, non-empty value, and decoded Unicode-character limit |
| `license` | Optional license name or bundled-file reference | Ignored by loader | If present, require a string |
| `compatibility` | Optional non-empty string, max 500 chars | Ignored by loader | If present, enforce type, non-empty value, and decoded character limit |
| `metadata` | Optional map from string keys to string values | Ignored by loader | If present, enforce mapping shape and string keys/values |
| `allowed-tools` | Optional space-separated string; experimental | Ignored by loader | If present, require a string; semantics remain runtime-specific |
| `disable-model-invocation` | Not in the base standard | Pi boolean extension; only literal `true` hides the skill from model discovery | Permit it as an extension, but require a boolean when present |
| Unknown top-level fields | Clients may carry extensions; Project Canon §17 requires `cli_version` and `schema_version` | Ignored | Permit unknown fields so version metadata and runtime extensions remain interoperable |
| Body | Markdown instructions; no detailed format restrictions | Loaded on explicit/model activation | Review usefulness under §15; do not invent a mechanical shape/length rule beyond the specification |
| Discovery locations | Location is client-defined; `.agents/skills` is the shared convention | User: `~/.pi/agent/skills`, `~/.agents/skills`; project: `.pi/skills`, `.agents/skills` from cwd through ancestors; packages/settings/CLI paths also supported | Scan repository source/install trees under `skills`, `.agents/skills`, `.claude/skills`, `.pi/skills`, `.pi/agent/skills`, and `.codex/skills` |
| Recursive discovery | Client guide recommends subdirectories containing `SKILL.md` | Recursive; a found `SKILL.md` defines a skill root and stops recursion below it; hidden dirs and `node_modules` skipped; ignore files honored | Recurse to nested skill roots, stop below a found skill, skip hidden dirs/`node_modules`, and confine canonical paths to the audited repo. Inspect materialized release trees even when locally ignored; ignore rules are not a portability waiver |
| Name collisions | Deterministic precedence plus warning recommended | First loaded skill wins; collision warns; duplicate real file via symlink is skipped | A single repository artifact may legitimately render into several runtime roots, so cross-root duplicate names are not a static failure; each tree must still validate independently |
| Invalid skill handling | Reference validator is strict | Pi is lenient for most name/length violations, but malformed YAML and missing descriptions are skipped | A release gate must fail every strict portable violation even where pi merely warns |
| Authoring recommendations | `SKILL.md` under 500 lines / 5000 tokens; shallow relative references; focused resources | Documented as guidance | Keep as review guidance, not a mechanical MUST; these are recommendations rather than compatibility requirements |

## Scanner safety policy

The conventional paths are collection roots, matching the Agent Skills guide's `<skills-dir>/<skill-name>/SKILL.md` layout and Project Canon's prior child-directory scan. A root-level `SKILL.md` is therefore a layout violation, not a skill that may mask the rest of the collection; the scanner reports it and continues looking for named child skills.

The repository scan is iterative and independently bounded to 64 directory levels, 10,000 directories, 50,000 entries, 2,000 located skills, 1 MiB of frontmatter per skill, eight displayed violations, and 1,000 characters per displayed violation. Exceeding a scan budget is an actionable §15 failure rather than an Agent Skills format rule or an unbounded traversal.

Internal directory symlinks are followed only while their canonical targets remain in the audited repository. Every logical alias is validated against its own parent name because that is the path a runtime discovers. Directly symlinked `SKILL.md` files are rejected. Confinement assumes a stable repository snapshot; the tool revalidates targets immediately before reading but does not claim race-free `openat` semantics.

## Observed failure

An unquoted plain YAML scalar containing `: ` can become malformed YAML:

```yaml
---
name: worktree-bug-analysis
description: Spawn an autonomous worker: analyze one bug
---
```

Pi surfaces the parser diagnostic and skips the skill. The strengthened `canon.s15` static probe must fail the same artifact before release and preserve enough of the parser message to identify the quoting/block-scalar correction.
