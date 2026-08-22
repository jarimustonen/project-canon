## Summary

<!-- What does this change, and why? Link the related issue if one exists. -->

## Checklist

- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` passes
- [ ] Commit messages follow Conventional Commits (`type(scope): summary`)
- [ ] No user-specific facts in any public artifact (this repo is public — see
      [CONTRIBUTING.md](../CONTRIBUTING.md))
