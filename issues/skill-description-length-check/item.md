---
created: 2026-08-23
updated: 2026-08-23
type: improvement
reporter: jari
status: in-progress
priority: normal
lane: build
commits:
- hash: fde1413844a8298d06dfcedea7f2d07318c39755
  summary: start implementation
- hash: f60b0041add425b2b10c2733f04d97c2e731d946
  summary: enforce skill description length
- hash: 73ded9231e65dd2da92064fd448ffeea287ac85d
  summary: harden probe after review
---

# Enforce the 1024-char skill description limit mechanically

## Description

Canon §15 now states the Agent Skills format limit: frontmatter description ≤ 1024 characters (over-limit descriptions are rejected/truncated by consuming runtimes, silently breaking discovery). Add a mechanical check so the limit cannot regress: (a) a unit test over the bundled skills' rendered frontmatter (ai-first-cli-canon is synthetic — measure the rendered form), and (b) a review/doctor probe for repos whose skills project-canon can locate. Current lengths: ai-first-cli-canon 390, cli-canon 867.

## Decisions

### 2026-08-23T19:15:56Z · @agent

Design decisions after implementation and multi-model review:

- Reused the resolved `canon.s15` dimension and shared mechanical-probe registry so doctor and static review see the same result. The probe is deliberately scoped: no located skills is a pass for the length subcheck, while review retains a manual judgment remainder for the rest of §15.
- Measure the YAML-decoded `description` value in Unicode scalar characters, not source bytes. This correctly handles quoted escapes and folded/literal scalars and accepts exactly 1024 characters.
- Locate direct-child `SKILL.md` resources under repository `skills/`, `.agents/skills/`, `.claude/skills/`, and `.pi/agent/skills/`. Canonical paths must remain inside the target repository. Reads are bounded to a 1 MiB frontmatter prefix, so a large body is allowed without unbounded allocation.
- Compose static and runtime evidence in `review`: `--run` only adds evidence. A static or runtime gap dominates; runtime unavailability cannot hide a static §15 gap or its judgment remainder.
- Test both native bundled renderings (Claude and Pi); Codex is excluded because its prompt layout has no Agent Skills frontmatter.

Rejected alternatives:

- Splitting §15 into new model dimensions/subcheck IDs: rejected as architectural overreach that would disturb the stable resolved model for one additive probe.
- Scanning tracked files only: rejected because newly authored or locally installed repository skills may be untracked and still need validation.
- Rejecting a skill based on total file size: rejected because only frontmatter needs a bound; a valid skill body may be large.
- Aggregating all violations in one row: deferred as optional remediation UX; deterministic first-failure reporting satisfies the mechanical gate.
- Replacing `serde_yaml` in this issue: its archived status is maintenance debt, but bounded input makes it non-blocking and replacement is not needed for this accepted scope.
