### Fixed

- `review` without `--profile` now audits the profile the repository declares in its `CONFORMANCE.md`, resolved exactly as `doctor` does (explicit `--profile`, then the declaration, then `cli`), so the two verbs agree on which canon sections apply. Its `--json` report gains `profile_source` (`flag`, `conformance-md`, or `default`), and `--run` in a repository declaring a non-`cli` profile asks for an explicit `--profile cli`.
