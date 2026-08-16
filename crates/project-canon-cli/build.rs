use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=GIT_COMMIT");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/packed-refs");

    let commit = std::env::var("GIT_COMMIT").ok().or_else(git_commit);
    match commit.filter(|sha| is_full_sha(sha)) {
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

fn git_commit() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())?
        .map(|sha| sha.trim().to_owned())
}

fn is_full_sha(sha: &str) -> bool {
    sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit())
}
