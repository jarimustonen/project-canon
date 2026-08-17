---
created: 2026-08-17
updated: 2026-08-17
type: improvement
status: in-progress
priority: high
related: ['@canon-no-user-specifics']
lane: canon-rollout
lane_seq: 20
---

# Canon rule: a deferral justification must be verified, not inherited

## Description

## The pattern

Work gets deferred with a justification written into a config file or code comment. The
justification names a blocker and, often, an owning issue. Nobody verifies either. Each
subsequent pass reads the comment, treats it as established fact, builds around it, and preserves
it — so a stale or simply wrong reason hardens into architecture.

This is the mirror image of the speculative-findings problem (see the companion rule about AI
review output inventing work that isn't needed): here an agent invents a **reason not to do
work**, which is harder to catch because it is invisible to issue triage — it lives in a config
file, not in the tracker.

## The instance that prompted this

A tool's `dist-workspace.toml` recorded:

> Homebrew auto-publish is deliberately NOT enabled here — that needs a tap write token and is
> owned by the separate `<name>` issue.

Both halves were false:

- The tap write token had existed on that repository since **2026-05-02** — about three and a
  half months before the justification was written. It was never a blocker.
- The named owning issue **did not exist** in that repository's tracker.

Consequence: the Homebrew tap sat three releases behind (0.11.0 while 0.14.0 shipped), and every
release reported success. The comment was last touched on 2026-08-16 in a workflow-standardization
commit — read, built around, and preserved, without either claim being checked.

## What to add

Two halves, matching how the no-user-specific-facts rule was handled — a canon section plus a
mechanical check, so this is enforced rather than remembered.

### Mechanical, for `doctor`

**A deferral justification that names an owning issue must resolve to a real issue.** Scan
tracked config files, source comments, and docs for references to an issue slug in the
project's own scheme; for each, assert the issue exists in the tracker. A reference that does
not resolve is a finding.

This is cheap — a grep plus an existence check — and it is the highest-yield half: in the
instance above it would have fired on the day the comment was written, three and a half months
before the damage surfaced.

Worth considering, if it stays cheap: also flag a reference to an issue that exists but is
**closed**, since "owned by <closed issue>" is the same defect with a slower fuse.

### Canon text, for judgement

**A stated blocker is not inherited — it is re-verified.** An agent that encounters a comment
justifying a disabled feature, a skipped step, or a deferred piece of work must check that the
stated blocker still holds before building around it or preserving it. Specifically:

- If the justification names a **credential, permission, or dependency** as missing, check
  whether it now exists.
- If it names an **owning issue**, check that the issue exists and is open.
- If neither can be verified, say so rather than silently propagating the comment.

A justification deserves the same scepticism as a review finding. Both are plausible-sounding
prose that arrives with implied authority and no evidence attached.

## Acceptance

- The canon carries the rule, phrased so an agent encountering such a comment knows it must
  verify rather than inherit.
- `doctor` reports unresolvable issue references in tracked files.
- The check runs on this tool's own repositories and its findings are triaged, not assumed empty.
