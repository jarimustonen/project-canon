use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=GIT_COMMIT");
    track_git_state();

    match git_override_or_commit() {
        Some(commit) => {
            println!("cargo:rustc-env=PROJECT_CANON_BUILD_COMMIT={commit}");
            println!("cargo:rustc-env=PROJECT_CANON_BUILD_PROVENANCE_KIND=git");
            println!(
                "cargo:rustc-env=PROJECT_CANON_BUILD_PROVENANCE_NOTE=git commit stamped at build time"
            );
        }
        None => {
            println!("cargo:rustc-env=PROJECT_CANON_BUILD_COMMIT=");
            println!("cargo:rustc-env=PROJECT_CANON_BUILD_PROVENANCE_KIND=tarball");
            println!(
                "cargo:rustc-env=PROJECT_CANON_BUILD_PROVENANCE_NOTE=no .git commit available in source archive"
            );
        }
    }
}

/// Track the real Git paths rather than assuming `.git` lives beside this crate. `--git-path`
/// resolves both normal repositories and linked worktrees, and follows a symbolic HEAD to the ref
/// whose content changes on a commit.
fn track_git_state() {
    let Some(head) = git_path("HEAD") else {
        return;
    };
    println!("cargo:rerun-if-changed={}", head.display());

    if let Some(reference) = git_output(&["symbolic-ref", "-q", "HEAD"]) {
        if let Some(reference_path) = git_path(&reference) {
            println!("cargo:rerun-if-changed={}", reference_path.display());
        }
    }
    if let Some(packed_refs) = git_path("packed-refs") {
        println!("cargo:rerun-if-changed={}", packed_refs.display());
    }
}

fn git_override_or_commit() -> Option<String> {
    match std::env::var("GIT_COMMIT") {
        Ok(value) if value.trim().is_empty() => git_commit(),
        Ok(value) => {
            let value = value.trim().to_owned();
            assert!(
                is_full_sha(&value),
                "invalid GIT_COMMIT override {value:?}: canon §10 requires a 40-character hex SHA"
            );
            Some(value)
        }
        Err(_) => git_commit(),
    }
}

fn git_commit() -> Option<String> {
    let sha = git_output(&["rev-parse", "HEAD"])?;
    is_full_sha(&sha).then_some(sha)
}

fn git_path(path: &str) -> Option<PathBuf> {
    git_output(&["rev-parse", "--git-path", path]).map(PathBuf::from)
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output.status.success().then(|| {
        String::from_utf8(output.stdout)
            .ok()
            .map(|output| output.trim().to_owned())
    })?
}

fn is_full_sha(sha: &str) -> bool {
    sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit())
}
