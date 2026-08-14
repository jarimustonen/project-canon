//! The shared **mechanical-probe substrate** — file-existence / repo-shape checks that both the
//! `doctor` gate and the `review` audit run over a target repo.
//!
//! A mechanical probe is a *decidable* check (grep / file-existence / repo-shape); it never
//! builds or runs the target tool, so anything needing the target's binary or prose judgment is
//! intentionally absent (those dimensions are `deferred-to-review` in `doctor` and become
//! `manual-verify` coverage notes in `review`). Extracting them here keeps the two verbs reading
//! the *same* substrate — a mechanically-confirmed gap carries identical evidence in both — and
//! keeps `doctor`/`review` disjoint (both depend on `probes`, neither on the other).
//!
//! A probe returns `io::Result<ProbeOutcome>`: `Ok(ProbeOutcome)` is a decidable pass/miss, an
//! `Err` is an *operational* I/O fault (permission denied, transient error) that each verb wraps
//! into its own exit-2 fault — keeping "could not evaluate" distinct from "the check missed".

use std::path::Path;

/// The outcome of running one mechanical probe: a decidable pass/miss. An operational I/O error is
/// *not* an outcome — probes return `io::Result<ProbeOutcome>`, and an `Err` is the caller's to
/// route to its own operational-fault exit.
pub struct ProbeOutcome {
    /// Whether the conformance check passed.
    pub passed: bool,
    /// The human-facing evidence line (the observation that settled the row).
    pub message: String,
}

impl ProbeOutcome {
    fn pass(message: impl Into<String>) -> ProbeOutcome {
        ProbeOutcome {
            passed: true,
            message: message.into(),
        }
    }
    fn fail(message: impl Into<String>) -> ProbeOutcome {
        ProbeOutcome {
            passed: false,
            message: message.into(),
        }
    }
}

/// Follow-symlinks metadata, treating only `NotFound` as "absent" (`Ok(None)`); any other error
/// (permission denied, transient I/O) propagates so it can become an operational fault.
fn stat(path: &Path) -> std::io::Result<Option<std::fs::Metadata>> {
    match std::fs::metadata(path) {
        Ok(m) => Ok(Some(m)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// No-follow metadata (the link entry itself), with the same `NotFound` → `Ok(None)` treatment.
fn lstat(path: &Path) -> std::io::Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(m) => Ok(Some(m)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Map a dimension id → its mechanical probe, or `None` when the dimension has no
/// mechanically-decidable check at v0. Only file-existence / repo-shape checks live here; anything
/// needing the built binary or prose judgment is intentionally absent. Every id here is asserted
/// to resolve in `Model::standard` by `probe_ids_exist_in_model`, so a core-side rename can't
/// silently turn an enforced MUST into a deferred/verify skip.
pub fn mechanical_probe(id: &str) -> Option<fn(&Path) -> std::io::Result<ProbeOutcome>> {
    match id {
        "base.doc-pattern" => Some(probe_doc_pattern),
        "base.issue-tracking" => Some(probe_issue_tracking),
        "base.git-hygiene" => Some(probe_git_hygiene),
        "base.readme" => Some(probe_readme),
        "base.gitignore" => Some(probe_gitignore),
        "canon.s22" => Some(probe_core_cli_split),
        _ => None,
    }
}

/// The ids of every dimension that carries a mechanical probe — the registry's key set, kept in
/// lockstep with [`mechanical_probe`] and cross-checked against the model by
/// `every_mechanical_probe_id_exists_in_the_model`.
#[cfg(test)]
const MECHANICAL_PROBE_IDS: [&str; 6] = [
    "base.doc-pattern",
    "base.issue-tracking",
    "base.git-hygiene",
    "base.readme",
    "base.gitignore",
    "canon.s22",
];

/// `AGENTS.md` and `CLAUDE.md` both present as files at the repo root (§ base.doc-pattern).
/// `CLAUDE.md` is normally a symlink to `AGENTS.md`; following it must land on a regular file, so a
/// dangling symlink, a directory, or a FIFO named `CLAUDE.md` is correctly a miss.
fn probe_doc_pattern(repo: &Path) -> std::io::Result<ProbeOutcome> {
    let agents = stat(&repo.join("AGENTS.md"))?.is_some_and(|m| m.is_file());
    let claude = stat(&repo.join("CLAUDE.md"))?.is_some_and(|m| m.is_file());
    Ok(match (agents, claude) {
        (true, true) => ProbeOutcome::pass("AGENTS.md and CLAUDE.md present"),
        (false, true) => ProbeOutcome::fail("AGENTS.md missing at repo root"),
        (true, false) => ProbeOutcome::fail("CLAUDE.md missing or not a file at repo root"),
        (false, false) => ProbeOutcome::fail("AGENTS.md and CLAUDE.md both missing at repo root"),
    })
}

/// `issues/` directory present (§ base.issue-tracking).
fn probe_issue_tracking(repo: &Path) -> std::io::Result<ProbeOutcome> {
    Ok(if stat(&repo.join("issues"))?.is_some_and(|m| m.is_dir()) {
        ProbeOutcome::pass("issues/ directory present")
    } else {
        ProbeOutcome::fail("issues/ directory missing")
    })
}

/// A `.git` entry present — a directory for a normal repo, or a gitfile for a worktree/submodule.
/// No-follow (`lstat`) so a symlinked `.git` counts by the link's presence; a permission error
/// faults rather than reading as "missing" (§ base.git-hygiene).
fn probe_git_hygiene(repo: &Path) -> std::io::Result<ProbeOutcome> {
    Ok(if lstat(&repo.join(".git"))?.is_some() {
        ProbeOutcome::pass(".git present")
    } else {
        ProbeOutcome::fail(".git missing — not a git repository")
    })
}

/// `README.md` front door present (§ base.readme, SHOULD).
fn probe_readme(repo: &Path) -> std::io::Result<ProbeOutcome> {
    Ok(
        if stat(&repo.join("README.md"))?.is_some_and(|m| m.is_file()) {
            ProbeOutcome::pass("README.md present")
        } else {
            ProbeOutcome::fail("README.md missing")
        },
    )
}

/// `.gitignore` present (§ base.gitignore, SHOULD).
fn probe_gitignore(repo: &Path) -> std::io::Result<ProbeOutcome> {
    Ok(
        if stat(&repo.join(".gitignore"))?.is_some_and(|m| m.is_file()) {
            ProbeOutcome::pass(".gitignore present")
        } else {
            ProbeOutcome::fail(".gitignore missing")
        },
    )
}

/// §22 core/cli split: a `crates/*-core` and a `crates/*-cli` directory both exist (SHOULD). A
/// missing `crates/` — or a `crates` that exists but is **not** a directory (a stray file) — is a
/// *conformance miss*, not an operational fault: it is repo shape, decidable without running the
/// tool. Only a genuine permission/transient I/O error (reading the dir, or a per-entry `metadata`
/// read) faults.
fn probe_core_cli_split(repo: &Path) -> std::io::Result<ProbeOutcome> {
    let crates = repo.join("crates");
    let entries = match std::fs::read_dir(&crates) {
        Ok(e) => e,
        // NotFound (`crates/` absent) and NotADirectory (`crates` is a file/other) are both
        // decidable repo-shape misses — never an exit-2 operational fault.
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(ProbeOutcome::fail(
                "no crates/ directory (missing or not a directory) — no core/cli split",
            ));
        }
        Err(e) => return Err(e),
    };
    let (mut has_core, mut has_cli) = (false, false);
    for entry in entries {
        let entry = entry?; // a per-entry read error faults rather than being silently dropped
                            // `entry.metadata()` (follows symlinks) surfaces a metadata I/O error as a fault,
                            // unlike `path().is_dir()`, which would swallow it as "not a directory".
        if !entry.metadata()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        has_core |= name.ends_with("-core");
        has_cli |= name.ends_with("-cli");
    }
    Ok(match (has_core, has_cli) {
        (true, true) => ProbeOutcome::pass("crates/*-core + *-cli split present"),
        _ => ProbeOutcome::fail("missing a crates/*-core and/or crates/*-cli directory"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use project_canon_core::Model;

    /// A throwaway temp dir under the OS temp root; removed on drop. Avoids a tempfile dep.
    struct TmpRepo {
        path: std::path::PathBuf,
    }

    impl TmpRepo {
        fn new(tag: &str) -> TmpRepo {
            use std::sync::atomic::{AtomicU32, Ordering};
            static N: AtomicU32 = AtomicU32::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("pc-probes-{tag}-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            TmpRepo { path }
        }
        fn touch(&self, rel: &str) {
            let p = self.path.join(rel);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&p, b"x").unwrap();
        }
        fn mkdir(&self, rel: &str) {
            std::fs::create_dir_all(self.path.join(rel)).unwrap();
        }
        #[cfg(unix)]
        fn symlink(&self, target: &str, link: &str) {
            std::os::unix::fs::symlink(target, self.path.join(link)).unwrap();
        }
    }

    impl Drop for TmpRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// Convenience: run a probe and unwrap the (test-only, never-faulting) I/O result to `passed`.
    fn passed(outcome: std::io::Result<ProbeOutcome>) -> bool {
        outcome.expect("no I/O fault on a tmp repo").passed
    }

    #[test]
    fn doc_pattern_probe_distinguishes_missing_files() {
        let repo = TmpRepo::new("doc");
        assert!(!passed(probe_doc_pattern(&repo.path)));
        repo.touch("AGENTS.md");
        assert!(!passed(probe_doc_pattern(&repo.path))); // CLAUDE.md still missing
        repo.touch("CLAUDE.md");
        assert!(passed(probe_doc_pattern(&repo.path)));
    }

    #[cfg(unix)]
    #[test]
    fn doc_pattern_probe_rejects_a_dangling_claude_symlink() {
        let repo = TmpRepo::new("doc-symlink");
        repo.touch("AGENTS.md");
        // A valid CLAUDE.md -> AGENTS.md symlink passes (followed to a real file)…
        repo.symlink("AGENTS.md", "CLAUDE.md");
        assert!(passed(probe_doc_pattern(&repo.path)));
        // …but a dangling symlink is a miss, not a pass.
        std::fs::remove_file(repo.path.join("CLAUDE.md")).unwrap();
        repo.symlink("nowhere.md", "CLAUDE.md");
        assert!(!passed(probe_doc_pattern(&repo.path)));
    }

    #[cfg(unix)]
    #[test]
    fn doc_pattern_probe_rejects_a_directory_named_claude() {
        let repo = TmpRepo::new("doc-dir");
        repo.touch("AGENTS.md");
        repo.mkdir("CLAUDE.md"); // a directory must not satisfy the doc pattern
        assert!(!passed(probe_doc_pattern(&repo.path)));
    }

    #[test]
    fn structural_probes_detect_presence() {
        let repo = TmpRepo::new("struct");
        assert!(!passed(probe_issue_tracking(&repo.path)));
        assert!(!passed(probe_git_hygiene(&repo.path)));
        assert!(!passed(probe_readme(&repo.path)));
        assert!(!passed(probe_gitignore(&repo.path)));
        repo.mkdir("issues");
        repo.mkdir(".git");
        repo.touch("README.md");
        repo.touch(".gitignore");
        assert!(passed(probe_issue_tracking(&repo.path)));
        assert!(passed(probe_git_hygiene(&repo.path)));
        assert!(passed(probe_readme(&repo.path)));
        assert!(passed(probe_gitignore(&repo.path)));
    }

    #[test]
    fn core_cli_split_probe_needs_both_crates() {
        let repo = TmpRepo::new("split");
        assert!(!passed(probe_core_cli_split(&repo.path))); // no crates/
        repo.mkdir("crates/foo-core");
        assert!(!passed(probe_core_cli_split(&repo.path))); // core only
        repo.mkdir("crates/foo-cli");
        assert!(passed(probe_core_cli_split(&repo.path)));
    }

    #[test]
    fn core_cli_split_treats_a_crates_file_as_a_miss_not_a_fault() {
        // `crates` existing as a regular file is decidable repo shape → a conformance miss
        // (Ok(false)), never an operational I/O fault (Err → exit 2).
        let repo = TmpRepo::new("crates-file");
        repo.touch("crates");
        let outcome = probe_core_cli_split(&repo.path).expect("a stray crates file is not a fault");
        assert!(!outcome.passed);
        assert!(
            outcome.message.contains("not a directory"),
            "{}",
            outcome.message
        );
    }

    #[test]
    fn every_mechanical_probe_id_exists_in_the_model() {
        // Guards against a core-side id rename silently turning an enforced MUST into a
        // deferred/verify skip (fail-open). If this fires, update MECHANICAL_PROBE_IDS + the
        // `mechanical_probe` match to the new id.
        let model = Model::standard();
        for id in MECHANICAL_PROBE_IDS {
            assert!(
                model.dimension(id).is_some(),
                "probe id {id:?} no longer exists in the model"
            );
            assert!(
                mechanical_probe(id).is_some(),
                "probe id {id:?} missing from the mechanical_probe registry"
            );
        }
    }

    #[test]
    fn a_probe_io_fault_propagates_as_err() {
        // A permission-denied read is an operational fault, not a conformance miss.
        // Unix-only: a `chmod 000` directory is the portable way to force EACCES on read.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let repo = TmpRepo::new("fault");
            repo.mkdir("crates");
            std::fs::set_permissions(
                repo.path.join("crates"),
                std::fs::Permissions::from_mode(0o000),
            )
            .unwrap();
            let result = probe_core_cli_split(&repo.path);
            // Restore perms so Drop can clean up regardless of the assertion outcome.
            let _ = std::fs::set_permissions(
                repo.path.join("crates"),
                std::fs::Permissions::from_mode(0o755),
            );
            // Running as root bypasses permission bits; only assert when the fault actually occurs.
            if let Err(e) = &result {
                assert_ne!(e.kind(), std::io::ErrorKind::NotFound);
            }
        }
    }
}
