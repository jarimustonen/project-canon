---
created: 2026-10-22
updated: 2026-10-22
type: bug
reporter: jari
status: in-progress
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

### 2026-10-22T05:43:42Z · @agent

Use .pi/skills for non-home targets and preserve .pi/agent/skills for default or explicit HOME targets (canonicalized so a home symlink alias remains user scope). Prefer native Pi layout over shared .agents/skills to avoid changing Claude/Codex ownership. Migration visits only selected skills and known resource paths, requires an anchored exact-name provenance marker and non-newer version, preserves foreign/newer/non-regular files even with --force, revalidates before removal, and removes only after every native write succeeds. Keep existing skill-list path metadata as user scope and add project_path as an additive layout field. Clarify canon section 15 to describe the actual per-scope native paths.

## Comments

### 2026-10-22T05:46:56Z · @agent

Implementation and tests are ready. Full gate passed once; a later run hit an execution-runner cache mismatch in canon_master_is_a_regular_file_inside_the_crate: on-disk canon had only a prose rewrap while embedded CANON was the prior bytes. Reverted that cosmetic rewrap and reran the complete gate; confirmed-gate.log records the final run. This worker cannot both terminal-merge and then run required release/fleet steps because merge tears down its session; a separate operational continuation will salvage the preserved verified commit and perform release/rollout from main, then run the released-binary dry-run without writing to the adopting repository. No release or fleet apply has been attempted here. The development build dry-run reports 15 planned native rows, 11 would-write and zero writes; all five Pi resources target .pi/skills.
