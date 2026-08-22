# Contributing to project-canon

Thanks for your interest in project-canon — conformance tooling for AI-first CLI
projects. Bug reports, feature requests, documentation fixes, and code are all
welcome.

## Reporting issues

- **Bugs and feature requests** — file a
  [GitHub issue](https://github.com/jarimustonen/project-canon/issues). This is the
  channel for external contributors.
- The canonical issue tracker is **in-repo**, under [`issues/`](issues/), managed with
  [`issuectl`](https://github.com/jarimustonen/issuectl). Only contributors with commit
  access write to it; accepted GitHub reports are triaged into it by the maintainer.
- **Security vulnerabilities** — do not open a public issue; see
  [`SECURITY.md`](SECURITY.md).

## Development setup

A stable Rust toolchain is all you need:

```sh
git clone https://github.com/jarimustonen/project-canon.git
cd project-canon
cargo build --workspace
cargo test --workspace
```

## The green gate

Every change must pass these checks before a pull request can merge (CI runs the
same set):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

The last command is easy to miss locally: a broken intra-doc link fails CI's docs
job even when every test passes.

## Branches and pull requests

- Fork the repository and branch from `main`.
- Keep a pull request focused on one logical change.
- Open the pull request against `main`; the green gate must pass before review.

## Commit messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/):
`type(scope): summary`. Types in use include `feat`, `fix`, `docs`, and `chore`.
When a change resolves or relates to an issue in the in-repo tracker, a
`Fixes-Issue: <slug>` or `Refs-Issue: <slug>` trailer links them — if you don't
know the slug, the maintainer will add it during review.

## Changelog

Release notes are compiled at release time from the issue trailers on the commits
in the release range, so **most changes need no changelog action**. Add a fragment
file under [`changelog/fragments/`](changelog/fragments/) only when you want a
change worded explicitly in the release notes — see
[`changelog/fragments/README.md`](changelog/fragments/README.md).

## Design conventions

The CLI surface follows the AI-first CLI canon,
[`AGENTS-AI-FIRST-CLI.md`](crates/project-canon-core/AGENTS-AI-FIRST-CLI.md) (§1–§24) — read it before
designing or changing any CLI surface; the tool enforces on itself what it checks
in others. [`AGENTS.md`](AGENTS.md) is the agent-facing repository documentation.
One rule deserves emphasis: **this repository is public, and no public artifact
may contain user-specific facts** (private repo names, personal paths or
hostnames, account names as built-in defaults). Fixtures and examples use
obviously fictional values.

## Licensing

By contributing, you agree that your contributions are licensed under the
project's [MIT License](LICENSE) (inbound = outbound).
