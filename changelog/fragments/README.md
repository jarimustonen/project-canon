# Changelog fragments

Changelog entries for this project are collected here as **fragments** (one change per file)
and compiled into [`../../CHANGELOG.md`](../../CHANGELOG.md) at release time by
`/shipshape-release` (`/shipshape-changelog --finalize`).

This project's changelog source is **issuectl trailers**: at a release cut, the compiled
notes are generated from the `Refs-Issue:` / `Fixes-Issue:` trailers on the commits in the
release range (via `issuectl changelog`), so most entries do not need a hand-written fragment
here. Add a fragment file only for a change you want worded explicitly in the release notes.
