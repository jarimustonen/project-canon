# skills/

This directory is the maintained home of the agent skills that `project-canon` ships with the
AI-first CLI canon. Every copy anywhere else, under an operator's `~/.claude/skills/`,
`.pi/agent/skills/`, or `.codex/skills/`, or inside an adopting repo, was written by
`project-canon skill install` from the sources described here and opens with a provenance
comment saying so. Those installed copies are consumers: an edit made in one is overwritten by
the next install and reaches nobody else. The older arrangement, in which homebase and other
repos copied skill files out of this directory by hand, is what `skill install` replaced; the
provenance note at the top of the canon master still describes that arrangement.

## Two skills, one canon

`cli-canon/` is the behaviour skill: the reviewer and generator that applies the canon to a CLI
by judgment, picking up where `doctor` and `review` stop. It is `SKILL.md` plus `templates/`
(the probe table, the review-report shape, the generate plan). It is the companion skill the
canon's §15 asks a family tool to ship, and the installer stamps it with the binary's version
so §17 drift detection works.

`ai-first-cli-canon` is the content skill: the canon text itself, installed so an agent working
in an adopting repo has the family's binding conventions at hand. It has no directory here on
purpose. The binary assembles it at install or print time from a description constant in
`crates/project-canon-cli/src/skill.rs` and `project_canon_core::CANON`, the embedded master. A
checked-in `SKILL.md` with a pasted canon body would be a second copy of the canon, and a
second copy that goes stale is the problem this design exists to remove; a test asserts that
the rendered skill contains the master bytes verbatim. The two skills stay separate because
most adopters want the rules without the auditing apparatus. The forks and their reasons are
in `issues/canon-installable-skill/design.md`; its install formats and packaging details
predate pi support (0.6.0), native Codex skill trees (0.8.1), and scaffolds that reference the
skill instead of copying the canon (0.9.1), so trust the code over it there.

## Where the files physically live

`skills/cli-canon` is a symlink. The physical directory is
`crates/project-canon-cli/skills/cli-canon/`, and `skill.rs` pulls every resource in with
`include_str!`. It lives inside the crate because `cargo package` ships only files under the
crate directory and the `include_str!` paths have to resolve inside the published tarball, so
the crates.io source archive carries the whole resource tree without a second copy. Either
path edits the same files. What breaks the design is a copy: another `SKILL.md` for either
skill anywhere in this repo, in homebase, or in a scaffold.

## Changing the skill

The installer rewrites the frontmatter on the way out. It keeps the fields `SKILL.md` declares,
appends `cli_version` and `schema_version`, and inserts the provenance comment as the first body
line, which is also how a later install recognises the file as its own. So the source
frontmatter has to exist (rendering panics without it) and should not carry those two fields
itself. A test validates the rendered frontmatter of every shipped skill as portable Agent
Skills frontmatter for all three runtimes, and `doctor` applies the same validator to other
repos, so a frontmatter change that fails here would fail for adopters too.

Some tests in `skill.rs` pin the text. The `cli-canon` render is expected to contain the phrase
"check shipshape/issuectl/taskfleet" and not its `ossctl` predecessor, because that phrase
names a real review use case and a rewrite once dropped it and broke CI. No shipped skill
source or the canon may mention the retired product name that `taskfleet` replaced. The
resource count is pinned at four, so adding a template means adding it to
`CLI_CANON_RESOURCES` and updating that test, and renaming one means updating its entry there;
the same list is what `skill list --json` and `skill print --json` advertise as printable
resources.

`skill list` describes `cli-canon` from the catalog constant in `skill.rs`, not from the
frontmatter, and the two wordings already differ. A description change worth making belongs
in both places, or the list will describe a skill that no longer reads that way.

The skill's claims about the binary (`review --verbose`, the manual-verify rows,
`config show --json`) are held to the source in reviews; `/fact-check-instructions` exists for
that check. The canon only appends sections and is cited by number, so the `§1–§24` range in
both descriptions (the `cli-canon` frontmatter and the `ai-first-cli-canon` constant in
`skill.rs`, which also names canon `v4`) follows the canon version rather than the other way
round.

## What stays out of the text

The skill text ships to crates.io and lands in every adopter's home directory. A repository map,
an account name, or a path convention from the maintainer's machine would become a default in a
public artifact, which canon §23 forbids after versions 0.1.1 and 0.2.0 shipped exactly that.
Family repositories come from operator configuration, `project-canon config show --json` under
`values.family_repos`, and the default is empty. The skill should keep asking the configuration
rather than knowing an answer. Examples use obviously fictional values.
