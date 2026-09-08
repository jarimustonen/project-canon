---
created: 2026-09-06
updated: 2026-09-08
type: task
status: done
priority: high
lane: build
lane_seq: 20
closed: 2026-09-08
commits:
- hash: 08ac429
  summary: converge Taskfleet reference artifacts
---

# Converge project-canon references on Taskfleet

## Goal

Converge Project Canon's active Taskfleet identity references from the retired product name to canonical `taskfleet`, while leaving stable protocol identifiers and immutable issue records untouched.

## Authorizing evidence

Taskfleet E1 owner map commit `8b8652a964a1353dc869e89fd541e8cf5b30f1e6`, P1-P3: https://github.com/jarimustonen/taskfleet/blob/8b8652a964a1353dc869e89fd541e8cf5b30f1e6/issues/taskfleet-dependent-owner-discovery/owner-map.md

## Required work

- Refresh the repo-local, Issuectl-owned issue-intake Claude/Codex dogfood copies through the released Issuectl supported path.
- Update Project Canon's canonical CLI-canon source and tests where examples refer to the current Taskfleet product by its retired name.
- Preserve stable `OCTL_*` protocol identifiers, the telemetry contract id, compatibility fixtures, and immutable issue/evidence records.
- Do not add historical migration narratives to `AGENTS.md`; keep operating guidance current and actionable.
- Run template/snapshot/integrity tests and the full repository gate. Follow Project Canon's normal release policy if its distributed catalog changes.

## Acceptance Criteria

- [x] Repo-local Issuectl dogfood copies match the released owner template and identify Taskfleet correctly.
- [x] Project Canon's active CLI-canon examples name canonical Taskfleet consistently.
- [x] Stable protocol and compatibility identifiers remain unchanged.
- [x] Full gate passes and any required release is verified.

## Decisions

### 2026-09-08T09:03:05Z · @agent

Kept this as a narrow owner-generated convergence. I installed the exact pinned released generator with `cargo install issuectl --version 0.18.2 --locked` into a disposable prefix, then ran scoped `issuectl skill install issue-intake` operations for Claude and Codex only. Both checked-in artifacts compare byte-for-byte with independent fresh 0.18.2 output. The prior stamps were 0.18.3; the explicit task pin controls despite that later release also being published. The install also proposed `issues/AGENTS.md`, which I restored byte-for-byte because it and the tracked pi copy are outside this task's artifact boundary.

The canonical CLI-canon Taskfleet examples and focused Rust guard were already present on main and released in Project Canon v0.8.2, so I did not duplicate or churn them. Multi-model review found that the existing guard incorrectly classified stable `OCTL_*` protocol identifiers as retired branding. I removed only that prohibition, renamed the test around the retired product identity, and retained the non-contiguous retired executable spelling so the guard does not itself pollute repository-wide active-content scans.

Rejected alternatives: blind repository-wide replacement; rewriting immutable issues/evidence or compatibility fixtures; refreshing pi, other Issuectl skills, or `issues/AGENTS.md` for symmetry; hand-editing generated content; changing Codex's Issuectl-owned layout; substituting Issuectl 0.18.3 for the explicitly pinned generator; and adding an improvised network-backed regeneration CI framework. The last is separate policy design, not required to prove this exact byte-verified refresh.

Validation: focused skill/template/integrity tests passed, then the exact full gate passed in order: fmt, clippy with warnings denied, workspace tests, workspace build, and rustdoc with warnings denied. `/llm-review` completed with gemini-3.1-pro-preview, gpt-5.6-sol, claude-fable-5, and deepseek-v4-pro over two cross-review rounds; `/assess-findings` classified the localized OCTL guard fix as confirmed and found no spin-off requiring a new issue.

## Resolution

### 2026-09-08T09:03:44Z · @issuectl

Completed the scoped Issuectl 0.18.2 dogfood refresh and corrected the packaged guidance guard to preserve stable protocol identifiers. Exact focused checks, multi-model review plus assessment, and the complete five-command repository gate passed. Project Canon v0.8.2 already released the active CLI-canon Taskfleet wording; this operational dogfood/test-only follow-up requires no additional product release.
