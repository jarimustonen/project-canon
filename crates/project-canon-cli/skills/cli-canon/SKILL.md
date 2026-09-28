---
name: cli-canon
description: "Apply the AI-first CLI canon (AGENTS-AI-FIRST-CLI.md, §1–§24) to a CLI tool by judgment, picking up where project-canon's mechanical checks stop. Review mode audits an existing CLI (a repo or a bare binary) against every canon section, settles the rows that project-canon review leaves as manual-verify, and produces a conformance matrix plus staged, per-tool recommendation findings that it never files itself. Generate mode emits canon-conformant surface scaffolding and a conformance TODO for a CLI inside an existing repo. Use for: review or audit this CLI against the canon, which canon sections is this tool missing, scaffold the CLI surface for a new tool in this repo. Not a general code review, not a review of a skill file, and not a new-repo bootstrap (that is project-canon new)."
allowed-tools: Bash, Glob, Grep, Read, Write
---

# cli-canon

## What this is for

The AI-first CLI canon (`AGENTS-AI-FIRST-CLI.md`, §1–§24) describes what a CLI in this
family should look like when its main caller is an agent. `project-canon` already checks
mechanically whatever can be checked mechanically: `doctor` is the CI gate, `review` is the
advisory audit that prints severity-ranked findings and staged `issuectl` commands, and
`review --run <binary>` adds a fixed set of read-only runtime probes (the project-canon repo's
`docs/review-runtime-probes.md` lists exactly which). What the binary leaves open is judgment
and behaviour: whether the verb vocabulary is honest, whether errors carry the bad value,
whether `fmt` is idempotent, whether `skill install` really lands all three runtimes. This
skill is the reviewer for that remainder, and the generator that gives a new or growing tool
the conformant shape before review has anything to check.

A good review leaves the tool's author with a matrix they can trust, one row per in-scope
section with a status and the evidence that settles it, and a short list of findings they
could act on tomorrow. A good generate pass leaves a repo with the conformant shape for
exactly the sections that apply to it, plus a TODO in the same matrix shape so review can
later close the loop.

## Sources, and why they matter

Read the canon itself, fresh, every run. `project-canon skill print ai-first-cli-canon`
streams the version the running binary ships, and the installed `ai-first-cli-canon` skill is
the same content. A repo-local `AGENTS-AI-FIRST-CLI.md` may lag behind, so if that is all
you have, say so in the report. The canon grows by appending sections and `§N` is a stable
citation surface, so cite by number and never grade from a remembered section list. Record
the canon's `Canon version:` line in the report or plan header.

The templates next to this file are the working material:

- `templates/conformance-probes.md`: per section, when it applies, what conformance looks
  like, how to observe it, what failure looks like, its severity class, and an effect class
  (`static`, `exec-ro`, `sandbox-write`). It also carries the dimension-discovery hook.
- `templates/review-report.md`: the matrix columns, the status vocabulary, the finding
  shape, and the emission rules for staged issues.
- `templates/generate-plan.md`: the generate steps and the canonical reference samples (the
  §2 exit map, the §10 payloads, the §8 config surface).

The probe table is hand-maintained and the canon is not, so a newly appended section may
have no probe yet. Compare the section ids in both at the start and report anything the
table lacks as uncovered, rather than letting the matrix imply complete coverage. A probe for
a section the canon no longer has is stale and worth flagging. If the canon or a template
you need cannot be read at all, say so and stop; a matrix graded against an imagined canon
is worse than none.

Family repositories come from the operator's configuration: `project-canon config show
--json`, key `values.family_repos`. It is empty by default and there is no built-in map. A
tool name that resolves to neither a configured repo nor a binary on `$PATH` is something to
ask about rather than guess, because the answers (a typo, an unbuilt tool, a binary-only
review) lead to different work.

## Characterize the tool

Applicability of the conditional canon sections turns on eight yes/no questions. Answer them
from the request, the repo, and `--help`. `project-canon-core` mirrors this table in its
questionnaire module, so if you change one, change the other.

| # | Question | If **yes**, these sections apply |
|---|---|---|
| Q1 | More than one resource noun? | §6 noun-verb surface (else a flat verb surface is fine) |
| Q2 | Resolves persistent config and/or a data root? | §8 config precedence + `config path`/`show`; `--home` if it has a data root |
| Q3 | Any command creates/updates/deletes a resource? | §11 dry-run + idempotency (per mutating cmd) |
| Q4 | Any command runs >a few seconds / as a daemon? | §12 streaming + progress query + signals |
| Q5 | Stamps `created`/`updated` or time-derived ids? | §19 injected clock + hidden `--frozen-time` |
| Q6 | Owns human-editable on-disk records? | §20 `fmt` canonicalizer |
| Q7 | Scaffolds an on-disk home other commands need? | §21 `init` idempotent no-clobber |
| Q8 | Results that can be large (list/export)? | §13 `--output FILE.jsonl\|.db` |

Always on for every family CLI: §1, §2, §3, §4, §5, §7, §9, §10, §14, §15, §16, §17, §18,
§23, §24. §6 always applies too; Q1 only selects its shape. §22 (core/cli split) is a SHOULD.
A tool with zero persistent config and no data root has §8 as `n/a`.

When a binary-only target cannot settle a source-dependent question, the dependent sections
are `unknown`, not failed. If several questions are unsettleable, ask them all in one message;
each interruption costs the user attention, and the answers are cheap to give together.

## Reviewing

Start from the binary. `project-canon review --verbose [--run <binary>] <repo>` gives you the
mechanically settled rows and, for every section it cannot decide, a manual-verify entry with
the probe, the expected shape, and the failure shape. That list plus the judgment sections is
your work. Discover the surface next: the `--help` tree, each subcommand's help, and a
classification of every command as read-only, mutating, or long-running. The probe table uses
`<cmd>`, `<mut>`, `<long>`, and `<fetch>` as placeholders, and this classification is what
binds them to real commands; a placeholder run unresolved is a random command run against a
tool you do not know.

The target is a program you did not write. It may write to disk, to a remote, or to the
operator's real data root, and it is being reviewed precisely because its behaviour is
unverified. The binary never runs anything except fixed read-only argument vectors with null
stdin and a timeout, and that is the right default posture for you too. The probes for §11,
§12, §15 install, §20, and §21 mutate. Run them only against a scratch home you created for
the review, after `config show --json` confirms the tool resolves its data root there, and
seed it as a git repo so a buggy `fmt` or `init` is a `git checkout` away from recovery. A
tool with no home selector, a remote-backed `create`, or an unknown backend has no safe
fixture; for those, `unknown` from static evidence (source, tests, a checked-in help
snapshot) is the correct answer, and a real mutation of someone's environment is not.
Generic-input probes (§1, §5, §9) belong on read-only verbs, and a mutating verb's error path
is reachable through `--dry-run --json`. Signal only a child process you spawned and whose
handle you hold. Spill large output to the scratch directory instead of reading it into
context. Do not write into the reviewed repo at all: review recommends, and the tool's own
author applies.

A row says what the evidence shows. An erroring `version --json` is the §10 failure
evidence, not a broken probe, so capture it rather than letting a failed `jq` pipeline
corrupt the matrix. No evidence means `unknown`, never `pass`. A non-Rust tool maps to its
ecosystem's equivalents (a build-time SHA stamp for `build.rs`, the workspace manifest for
`Cargo.toml` members), because the canon names Rust only as the concrete shape of
language-agnostic rules. A binary-only target has source-dependent rows `unknown` with the
reason "unavailable without source", and its findings go to the user, since there is no repo
to stage into.

Some sections cannot be settled by a probe: §6 surface shape, §7 verb legitimacy, §10
schema-as-API discipline, §11 idempotency semantics, and the judgment remainders the binary
names for §15, §23, and §24. Argue those calls yourself in the matrix against the canon text.
General-purpose review and triage skills do not fit here: a code-review prompt has no slot
for the canon as rubric, and a triage that weighs production likelihood would drop a MUST gap
that is structurally real but rarely hit, which is exactly the kind of gap a conformance
audit exists to keep. If a second opinion is worth its cost, write one bounded brief covering
all judgment sections together, with the canon excerpt and the captured evidence, rather than
a call per section.

Severity is the canon's own model, stated per section in the probe table: MUST,
MUST-when-applies, SHOULD. A SHOULD is never a hard gate, an out-of-scope conditional is
`n/a` rather than a failure, and `unknown` is a coverage note rather than a finding. Two
things worth repeating from the templates: §8 `config path`/`config show` is the family's most
consistent historical miss, so for any tool that resolves config or a data root treat its
absence as a failure and not a gap to soften. And the canon calls its v2 mandates deliberately
aspirational; some mandates make existing tools non-conformant by design, so a `fail` against
one is correct. Such a failure recurs on every run, though, so keep it in the report and
default it to report-only rather than staging it again.

Findings are staged, never filed. Confirm the target repo with `git -C <repo> remote -v`
first; a path that exists is not proof it is the repo you mean, and the family map can be
stale. Reuse the binary's staged command form, `( cd -- <repo> && issuectl new … --slug
cli-canon-sNN --label cli-canon )`: `issuectl` refuses a slug that already exists, so a re-run
cannot duplicate, and the explicit `cd` matters because a bare `issuectl` files into whatever
repo you happen to be standing in. Check existing `cli-canon`-labelled issues in the target
before staging, present the would-file list, and let the user run it.

The canon grows from practices that recur across at least two family tools, and a
single-tool review cannot show recurrence by itself. The dimension-discovery hook in the
probe table explains the mechanics: add this tool's evidence to an existing candidate before
writing a new one, look at other family repos for the same practice, and treat a lone
observation as a watch-list note.

The report is the matrix, the findings most-severe first, and a short plain-language summary:
how many MUST gaps, the top few by impact, whether the tool is broadly conformant or has
structural gaps, any coverage gaps from the reconciliation, any canon candidate. When the
user scoped the review ("only the config surface"), assess the named sections and say the
rest was not assessed, rather than marking it `unknown`.

## Generating

Generate is for a CLI surface inside an existing repo. A brand-new repo comes from
`project-canon new`, whose scaffold points at the installed `ai-first-cli-canon` skill rather
than a pasted canon copy; run generate afterwards for the surface. Characterize the tool to
get the applicable section set, then follow `templates/generate-plan.md`.

The repo is someone's working tree, so generate defaults to a preview: the plan and the
scaffold shown, nothing written. Write when the user asks for it, into the confirmed repo,
within its root, and with a clean or consented tree. Never execute generated code during
generation; a generated `build.rs` running in the user's environment is arbitrary code
execution, and generation does not need it.

Emit the conformant shape per applicable section using the canonical samples in the plan
template, so that what generate scaffolds and what review probes are the same shapes. Prefer
pointing the tool at a thin shared `<family>-cli-common` crate (§22) over re-rolling the
error envelope and config plumbing, while noting that the crate is optional and giving local
fallbacks if it does not exist. Finish with the conformance TODO rendered as the review
matrix filtered to the applicable sections, every status starting at `todo`; generate's
output is then a direct review input, and the two modes cannot drift apart.

## Arguments

`/cli-canon <mode> <target> [notes]`. Mode is `review` or `generate`; when omitted, an
existing tool, repo, or binary to assess means review, and scaffold or new-surface phrasing
means generate. Target is a repo path, a family tool name, or, for review, an installed
binary with no reachable repo. Notes are free-form scope hints and are honoured as scope.
