### Added

- `new --repo-name <name>` sets the repository slug used by the `github-create`, `git-remote-ssh`, and `tw-register` hooks separately from `--name`, which keeps naming the package, crates, and binary (for example `--name widget --repo-name widget-cli`). It defaults to the project name, is validated like `--name`, and is reported as `repo_name` in the `--json` plan.
