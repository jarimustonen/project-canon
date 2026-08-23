---
created: 2026-08-23
updated: 2026-08-23
type: chore
status: untriaged
priority: normal
provenance: other
provenance_detail: Fleet-wide product rename authorized by maintainer
source_ref: orchestratectl:01m0qg3nr6vtghqydwdp1tdz01/task:shipshape-migration
originating_run: 01m0qg3nr6vtghqydwdp1tdz01
originating_run_kind: spinoff
commits:
- hash: e993c5d
  summary: 'chore: migrate release tooling to Shipshape'
---

# Migrate active release tooling references to Shipshape

## Description

Migrate this repository's active release-tool product references from ossctl to Shipshape, following Shipshape ADR 0005. Update executable calls, `/shipshape-*` skill names, generated ownership markers, release guidance, contract rationale, distribution comments, bundled skill examples, and tests/fixtures where applicable. Preserve historical issue/changelog/TODO evidence and permanent compatibility identifiers such as `OSS-RELEASE.md` and `oss-changelog:*` markers.

Acceptance: semantic grep audit documents retained compatibility/history matches; the repository full green gate passes; changes are committed and merged by orchestratectl run 01m0qg3nr6vtghqydwdp1tdz01.

## Decisions

### 2026-08-23T14:38:08Z · @agent

#### Migration design decisions

Semantically classified every `git grep -Iil ossctl` match rather than globally replacing it. Active commands and release guidance now use `shipshape`; active generated ownership markers use `shipshape-{ci,dist,readme}` because direct inspection of the canonical Shipshape templates confirms those exact spellings. The bundled `cli-canon` example now names the maintained product and has a focused packaged-resource assertion.

Retained legacy strings deliberately: dated `/oss-init` and `ossctl 0.8.0` provenance in `OSS-RELEASE.md`/`dist-workspace.toml`; historical TODO, issue, and released CHANGELOG evidence; and the permanent `oss-changelog:*` marker. `OSS-RELEASE.md` remains unchanged as the compatibility filename. The old bootstrap `/oss-*` references in that contract are a dated approval snapshot, not current operational guidance.

The review caught one non-mechanical mapping: Shipshape's actual ten-skill catalog has `shipshape-release`, not `shipshape-release-cut`, so the fragment guide names `/shipshape-release` as orchestrator and `/shipshape-changelog --finalize` as its operation. Rejected retaining old marker names (the ADR freezes only changelog markers), advertising an ossctl alias/trigger, and mutating machine installation state from this worker; ADR 0005 assigns convergence to the conductor.

### 2026-08-23T14:40:53Z · @agent

Post-rebase semantic check found concurrent main had added transitional guidance saying to use whichever ossctl/shipshape binary was installed and that /oss-* skills were unchanged. ADR 0005 says the opposite: Shipshape is canonical, old skill names are actionable refusals, and ossctl is only a frozen rollback binary. Updated the active policy to require shipshape and /shipshape-*; a missing installation is a conductor convergence gap, not authorization to cut with ossctl. Also migrated the concurrent /oss-contributing reference.


## Agent Runs

### 2026-08-23T14:39:25Z · @agent

Delivered by orchestratectl run 01m0qg3nr6vtghqydwdp1tdz01 in commit e993c5d. Full repository green gate passed after review fixes: fmt, clippy -D warnings, workspace tests, workspace build, and rustdoc -D warnings. The item remains untriaged and unlaned for the required human lane-or-close disposition.
