---
created: 2026-10-22
updated: 2026-10-22
type: bug
reporter: jari
status: open
priority: normal
lane: unlaned
preparation: ready
---

# skill install --target <repo> writes Pi skills where Pi does not load project skills

## Description

Observed 2026-10-22 in open-habitat: `project-canon skill install --target <repo-root>` writes the Pi layout to `<repo>/.pi/agent/skills/<name>/`. That mirrors the user-level layout (`~/.pi/agent/skills`), but Pi loads trusted project skills from `<cwd>/.pi/skills` and from `.agents/skills` in the cwd and its ancestors up to the repository root (pinned Pi 1.3.0: dist/core/package-manager.js addAutoDiscoveredResources; verified in open-habitat issues/proto-skill-catalogue/result.md). So repo-installed canon skills are invisible to Pi agents working in that repository. Fix the repo-target Pi layout (likely `.pi/skills`, or `.agents/skills` which Pi and other harnesses share), keep the user-level layout unchanged, migrate managed files from the wrong path as the installer already does for legacy paths, and cover both targets in tests. Jari asked that the fix be released and installed through the fleet.

## Decisions

### 2026-10-22T05:40:32Z · @jari

Jari 2026-10-22: fix it, release it and install it through the fleet.
