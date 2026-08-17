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

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// User-configured inputs for probes whose answer depends on operator knowledge.
pub struct ProbeContext<'a> {
    /// Exact, case-insensitive markers known to identify the operator's private environment.
    pub user_specific_deny_list: &'a BTreeSet<String>,
}

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
pub fn mechanical_probe(
    id: &str,
) -> Option<fn(&Path, &ProbeContext<'_>) -> std::io::Result<ProbeOutcome>> {
    match id {
        "base.doc-pattern" => Some(|repo, _| probe_doc_pattern(repo)),
        "base.issue-tracking" => Some(|repo, _| probe_issue_tracking(repo)),
        "base.git-hygiene" => Some(|repo, _| probe_git_hygiene(repo)),
        "base.readme" => Some(|repo, _| probe_readme(repo)),
        "base.gitignore" => Some(|repo, _| probe_gitignore(repo)),
        "canon.s22" => Some(|repo, _| probe_core_cli_split(repo)),
        "canon.s23" => Some(probe_public_artifact_specifics),
        "canon.s24" => Some(probe_verified_deferrals),
        _ => None,
    }
}

/// The ids of every dimension that carries a mechanical probe — the registry's key set, kept in
/// lockstep with [`mechanical_probe`] and cross-checked against the model by
/// `every_mechanical_probe_id_exists_in_the_model`.
#[cfg(test)]
const MECHANICAL_PROBE_IDS: [&str; 8] = [
    "base.doc-pattern",
    "base.issue-tracking",
    "base.git-hygiene",
    "base.readme",
    "base.gitignore",
    "canon.s22",
    "canon.s23",
    "canon.s24",
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

/// §23's mechanically decidable subset. The operator names private markers; doctor scans the
/// distributed tree without guessing what a username looks like. The target's own public
/// coordinates are derived from its git remote and exempted.
fn probe_public_artifact_specifics(
    repo: &Path,
    context: &ProbeContext<'_>,
) -> std::io::Result<ProbeOutcome> {
    if context.user_specific_deny_list.is_empty() {
        return Ok(ProbeOutcome::pass(
            "no user-specific markers configured; set user_specific_deny_list or PROJECT_CANON_USER_SPECIFIC_DENY_LIST to enable the §23 scan",
        ));
    }

    let own = own_coordinates(repo)?;
    let markers: Vec<String> = context
        .user_specific_deny_list
        .iter()
        .map(|marker| marker.to_lowercase())
        .collect();
    let files = tracked_text_candidates(repo)?;
    for file in files {
        let rel = file.strip_prefix(repo).unwrap_or(&file);
        if std::fs::metadata(&file)?.len() > 1_048_576 {
            use std::io::Read;
            let mut prefix = [0u8; 8192];
            let mut handle = std::fs::File::open(&file)?;
            let read = handle.read(&mut prefix)?;
            if prefix[..read].contains(&0) {
                continue;
            }
            return Ok(ProbeOutcome::fail(format!(
                "text-like distributed file {} exceeds the 1 MiB §23 scan limit",
                rel.display()
            )));
        }
        let bytes = std::fs::read(&file)?;
        let Ok(text) = std::str::from_utf8(&bytes) else {
            if bytes.contains(&0) {
                continue;
            }
            return Ok(ProbeOutcome::fail(format!(
                "text-like distributed file {} is not UTF-8 and could not be scanned",
                rel.display()
            )));
        };
        for (line_index, line) in text.lines().enumerate() {
            let searchable = line.to_lowercase();
            for (marker_index, marker) in markers.iter().enumerate() {
                let leaked = searchable.match_indices(marker).any(|(start, _)| {
                    !own.is_allowed_occurrence(&searchable, marker, start, start + marker.len())
                });
                if leaked {
                    return Ok(ProbeOutcome::fail(format!(
                        "configured user-specific marker #{} found in {}:{}",
                        marker_index + 1,
                        rel.display(),
                        line_index + 1
                    )));
                }
            }
        }
    }
    Ok(ProbeOutcome::pass(format!(
        "no configured user-specific markers found ({} marker(s)); own public coordinates exempt",
        context.user_specific_deny_list.len()
    )))
}

/// §24's mechanically decidable subset. Deferral ownership is local: every recognized issue slug
/// must resolve in this target's issue tracker. A cross-repository issue may be supporting evidence,
/// but it cannot replace an open local mirror that doctor can verify offline.
fn probe_verified_deferrals(
    repo: &Path,
    _context: &ProbeContext<'_>,
) -> std::io::Result<ProbeOutcome> {
    let mut findings = BTreeSet::new();
    let mut issue_states = std::collections::BTreeMap::<String, IssueState>::new();
    let mut references_seen = 0usize;
    let mut skipped_files = 0usize;

    for file in tracked_text_candidates(repo)? {
        let rel = file.strip_prefix(repo).unwrap_or(&file);
        let metadata = std::fs::metadata(&file)?;
        if metadata.len() > 1_048_576 {
            use std::io::Read;
            let mut prefix = [0u8; 8192];
            let mut handle = std::fs::File::open(&file)?;
            let read = handle.read(&mut prefix)?;
            if prefix[..read].contains(&0) {
                skipped_files += 1;
                continue;
            }
            skipped_files += 1;
            continue;
        }
        let bytes = std::fs::read(&file)?;
        if bytes.contains(&0) {
            skipped_files += 1;
            continue;
        }
        let Ok(text) = std::str::from_utf8(&bytes) else {
            skipped_files += 1;
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        let mut seen_in_file = BTreeSet::new();
        for line_index in 0..lines.len() {
            let mut context = Vec::new();
            for (offset, line) in lines[line_index..lines.len().min(line_index + 3)]
                .iter()
                .enumerate()
            {
                if offset > 0 && line.trim().is_empty() {
                    break;
                }
                if let Some(fragment) = scannable_fragment(rel, line) {
                    context.push((line_index + offset + 1, fragment));
                } else if offset > 0 {
                    // Source code between two comments is a logical boundary, not continuation.
                    break;
                }
            }
            let suppression_start = line_index.saturating_sub(2);
            let suppressed = lines[suppression_start..=line_index]
                .iter()
                .rev()
                .take_while(|line| !line.trim().is_empty())
                .any(|line| line.contains("canon:s24-allow"));
            if suppressed
                || context
                    .iter()
                    .any(|(_, fragment)| fragment.contains("canon:s24-allow"))
            {
                continue;
            }
            let tokens = context_tokens(&context);
            if !looks_like_deferral(&tokens) {
                continue;
            }
            for reference in issue_references(&tokens) {
                if !seen_in_file.insert((reference.line, reference.slug.clone())) {
                    continue;
                }
                references_seen += 1;
                let state = match issue_states.get(&reference.slug) {
                    Some(state) => state.clone(),
                    None => {
                        let state = resolve_issue_state(repo, &reference.slug)?;
                        issue_states.insert(reference.slug.clone(), state.clone());
                        state
                    }
                };
                let location = format!("{}:{}", rel.display(), reference.line);
                match state {
                    IssueState::Open => {}
                    IssueState::Missing => {
                        findings.insert(format!(
                            "deferral at {location} names unresolved local issue {:?}",
                            reference.slug
                        ));
                    }
                    IssueState::NonOpen(status) => {
                        findings.insert(format!(
                            "deferral at {location} names local issue {:?} with non-open status {status:?}",
                            reference.slug
                        ));
                    }
                    IssueState::Malformed => {
                        findings.insert(format!(
                            "deferral at {location} names local issue {:?} with malformed or missing status frontmatter",
                            reference.slug
                        ));
                    }
                }
            }
        }
    }

    if findings.is_empty() {
        Ok(ProbeOutcome::pass(format!(
            "all {references_seen} detected deferral issue reference(s) resolve to open local issues; {skipped_files} binary/oversized/non-UTF-8 tracked file(s) skipped"
        )))
    } else {
        let total = findings.len();
        let sample = findings.into_iter().take(5).collect::<Vec<_>>().join("; ");
        Ok(ProbeOutcome::fail(format!(
            "{total} unverified deferral reference(s): {sample}"
        )))
    }
}

#[derive(Clone)]
enum IssueState {
    Open,
    NonOpen(String),
    Missing,
    Malformed,
}

struct IssueReference {
    slug: String,
    line: usize,
}

struct ContextToken {
    text: String,
    line: usize,
}

fn scannable_fragment<'a>(path: &Path, line: &'a str) -> Option<&'a str> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let full_text = matches!(
        extension,
        "md" | "mdx"
            | "rst"
            | "txt"
            | "toml"
            | "yaml"
            | "yml"
            | "json"
            | "jsonc"
            | "ini"
            | "cfg"
            | "conf"
    );
    if full_text {
        return Some(line);
    }

    let markers: &[&str] = match extension {
        "rs" | "js" | "jsx" | "ts" | "tsx" | "c" | "cc" | "cpp" | "h" | "hpp" | "java" | "go"
        | "swift" => &["//", "/*"],
        "py" | "rb" | "sh" | "bash" | "zsh" => &["#"],
        "html" | "htm" | "xml" => &["<!--"],
        "sql" | "lua" => &["-- "],
        _ => &["//", "#", "/*", "<!--", "-- "],
    };
    comment_start_outside_quotes(line, markers)
        .map(|(index, length)| &line[index + length..])
        .or_else(|| {
            line.trim_start()
                .starts_with('*')
                .then(|| line.trim_start_matches([' ', '*']))
        })
}

fn comment_start_outside_quotes(line: &str, markers: &[&str]) -> Option<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if quote.is_some() && byte == b'\\' {
            escaped = true;
            index += 1;
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            if quote == Some(byte) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(byte);
            }
            index += 1;
            continue;
        }
        if quote.is_none() {
            if let Some(marker) = markers
                .iter()
                .find(|marker| bytes[index..].starts_with(marker.as_bytes()))
            {
                return Some((index, marker.len()));
            }
        }
        index += 1;
    }
    None
}

fn context_tokens(lines: &[(usize, &str)]) -> Vec<ContextToken> {
    lines
        .iter()
        .flat_map(|(line, text)| {
            text.split(|character: char| !(character.is_ascii_alphanumeric() || character == '-'))
                .filter(|token| !token.is_empty())
                .map(|token| ContextToken {
                    text: token.to_ascii_lowercase(),
                    line: *line,
                })
        })
        .collect()
}

fn looks_like_deferral(tokens: &[ContextToken]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        matches!(
            token.text.as_str(),
            "defer" | "deferred" | "deferral" | "disabled" | "skipped" | "blocked" | "blocker"
        ) || (token.text == "owned" && tokens.get(index + 1).is_some_and(|next| next.text == "by"))
            || (token.text == "not" && tokens.get(index + 1).is_some_and(|next| next.text == "yet"))
            || token.text == "until"
            || (token.text == "tracks"
                && tokens[index + 1..tokens.len().min(index + 5)]
                    .iter()
                    .any(|next| matches!(next.text.as_str(), "closing" | "gap")))
    })
}

fn issue_references(tokens: &[ContextToken]) -> Vec<IssueReference> {
    let mut references = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.text == "issue" {
            if let Some(slug) = tokens
                .get(index + 1)
                .filter(|token| is_issue_slug(&token.text))
            {
                references.insert((slug.line, slug.text.clone()));
            }
            continue;
        }
        if !is_issue_slug(&token.text) {
            continue;
        }
        let following = &tokens[index + 1..tokens.len().min(index + 5)];
        let preceding = &tokens[index.saturating_sub(5)..index];
        let ownership = preceding.windows(2).any(|pair| {
            matches!(pair[0].text.as_str(), "owned" | "blocked") && pair[1].text == "by"
        });
        if ownership
            && following
                .first()
                .is_some_and(|candidate| candidate.text == "issue")
        {
            references.insert((token.line, token.text.clone()));
        }
    }
    references
        .into_iter()
        .map(|(line, slug)| IssueReference { slug, line })
        .collect()
}

fn is_issue_slug(token: &str) -> bool {
    token.len() <= 128
        && token.contains('-')
        && token.split('-').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn resolve_issue_state(repo: &Path, slug: &str) -> std::io::Result<IssueState> {
    // `is_issue_slug` admits no separators or dots, so this join cannot escape `issues/`.
    let issue = repo.join("issues").join(slug).join("item.md");
    let contents = match std::fs::read_to_string(issue) {
        Ok(contents) => contents,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(IssueState::Missing)
        }
        Err(error) => return Err(error),
    };
    Ok(match issue_status(&contents) {
        Some(status) if is_open_issue_status(status) => IssueState::Open,
        Some(status) => IssueState::NonOpen(status.to_string()),
        None => IssueState::Malformed,
    })
}

fn issue_status(contents: &str) -> Option<&str> {
    let mut lines = contents.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    let mut status = None;
    let mut closed = false;
    for line in lines {
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        if let Some(value) = line.strip_prefix("status:") {
            let value = value.trim().trim_matches(['"', '\'']);
            if status.is_some() || value.is_empty() {
                return None;
            }
            status = Some(value);
        }
    }
    closed.then_some(status).flatten()
}

fn is_open_issue_status(status: &str) -> bool {
    // Mirrors the active statuses in this target's issuectl-managed `issues/.schema.yaml`.
    matches!(
        status,
        "open" | "in-progress" | "testing" | "untriaged" | "deferred" | "needs-info"
    )
}

#[derive(Default)]
struct OwnCoordinates {
    owner: Option<String>,
    repo: Option<String>,
}

impl OwnCoordinates {
    fn is_allowed_occurrence(&self, line: &str, marker: &str, start: usize, end: usize) -> bool {
        let (Some(owner), Some(repo)) = (&self.owner, &self.repo) else {
            return false;
        };
        let owner = owner.to_lowercase();
        let repo = repo.to_lowercase();

        // The repository/package name is intrinsically this project's public identity, including
        // package suffixes such as `<repo>-cli`.
        if marker == repo {
            return true;
        }
        // An owner is allowed only as the owner segment of a coordinate. A separately configured
        // private repository marker on the same line remains visible and still fails.
        if marker == owner && line.as_bytes().get(end) == Some(&b'/') {
            return true;
        }

        // For markers that overlap an own coordinate, exempt only this specific occurrence. Never
        // delete text before scanning: deletion can concatenate or erase unrelated private names.
        for coordinate in [
            format!("{owner}/{repo}"),
            format!("{owner}/homebrew-{repo}"),
        ] {
            for (coordinate_start, _) in line.match_indices(&coordinate) {
                let coordinate_end = coordinate_start + coordinate.len();
                if start >= coordinate_start && end <= coordinate_end {
                    return true;
                }
            }
        }
        false
    }
}

fn own_coordinates(repo: &Path) -> std::io::Result<OwnCoordinates> {
    let dot_git = repo.join(".git");
    let config = if dot_git.is_dir() {
        Some(dot_git.join("config"))
    } else if dot_git.is_file() {
        let pointer = std::fs::read_to_string(&dot_git)?;
        pointer
            .trim()
            .strip_prefix("gitdir:")
            .map(str::trim)
            .map(|path| {
                let gitdir = PathBuf::from(path);
                let gitdir = if gitdir.is_absolute() {
                    gitdir
                } else {
                    repo.join(gitdir)
                };
                let local = gitdir.join("config");
                if local.is_file() {
                    local
                } else {
                    let common = std::fs::read_to_string(gitdir.join("commondir"))
                        .unwrap_or_else(|_| ".".to_string());
                    gitdir.join(common.trim()).join("config")
                }
            })
    } else {
        None
    };

    if let Some(config) = config {
        if let Ok(contents) = std::fs::read_to_string(config) {
            let mut in_origin = false;
            for line in contents.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') {
                    in_origin = trimmed == "[remote \"origin\"]";
                    continue;
                }
                if !in_origin {
                    continue;
                }
                let Some((key, value)) = trimmed.split_once('=') else {
                    continue;
                };
                if key.trim() == "url" {
                    if let Some((owner, name)) = parse_github_coordinate(value.trim()) {
                        return Ok(OwnCoordinates {
                            owner: Some(owner),
                            repo: Some(name),
                        });
                    }
                }
            }
        }
    }

    Ok(coordinates_from_manifest(repo).unwrap_or_default())
}

fn coordinates_from_manifest(repo: &Path) -> Option<OwnCoordinates> {
    let contents = std::fs::read_to_string(repo.join("Cargo.toml")).ok()?;
    let manifest: toml::Value = contents.parse().ok()?;
    let repository = manifest
        .get("package")
        .and_then(|package| package.get("repository"))
        .or_else(|| {
            manifest
                .get("workspace")
                .and_then(|workspace| workspace.get("package"))
                .and_then(|package| package.get("repository"))
        })?
        .as_str()?;
    let (owner, repo) = parse_github_coordinate(repository)?;
    Some(OwnCoordinates {
        owner: Some(owner),
        repo: Some(repo),
    })
}

fn parse_github_coordinate(url: &str) -> Option<(String, String)> {
    let path = url
        .strip_prefix("git@github.com:")
        .or_else(|| url.strip_prefix("https://github.com/"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
    let mut parts = path
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .split('/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    (!owner.is_empty() && !repo.is_empty()).then_some((owner, repo))
}

fn tracked_text_candidates(repo: &Path) -> std::io::Result<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .args(["-C", repo.to_string_lossy().as_ref(), "ls-files", "-z"])
        .output();
    if let Ok(output) = output {
        if output.status.success() {
            let mut files = output
                .stdout
                .split(|byte| *byte == 0)
                .filter(|path| !path.is_empty())
                .filter_map(|path| std::str::from_utf8(path).ok())
                .map(PathBuf::from)
                .filter(|path| {
                    !path.is_absolute()
                        && path
                            .components()
                            .all(|component| matches!(component, std::path::Component::Normal(_)))
                })
                .map(|path| repo.join(path))
                .filter(|path| {
                    std::fs::symlink_metadata(path)
                        .is_ok_and(|metadata| metadata.file_type().is_file())
                })
                .collect::<Vec<_>>();
            files.sort();
            return Ok(files);
        }
    }

    // Synthetic fixtures and source archives may not have a functioning git command. Fall back
    // to a bounded tree walk with component-level exclusions.
    let mut files = Vec::new();
    collect_text_candidates(repo, repo, &mut files)?;
    Ok(files)
}

fn collect_text_candidates(
    root: &Path,
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    let mut entries = std::fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(&path);
        // The source-archive fallback excludes metadata/build/scratch components at any depth.
        let excluded_component = rel.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some(".git" | "target" | "node_modules" | "history")
            )
        });
        if excluded_component {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect_text_candidates(root, &path, files)?;
        } else if kind.is_file() {
            files.push(path);
        }
    }
    Ok(())
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
    fn public_artifact_probe_flags_a_configured_private_marker() {
        let repo = TmpRepo::new("private-marker");
        repo.touch("src/defaults.rs");
        std::fs::write(
            repo.path.join("src/defaults.rs"),
            "const DEFAULT_REPO: &str = \"private-widget\";",
        )
        .unwrap();
        let deny = BTreeSet::from(["private-widget".to_string()]);
        let context = ProbeContext {
            user_specific_deny_list: &deny,
        };
        let outcome = probe_public_artifact_specifics(&repo.path, &context).unwrap();
        assert!(!outcome.passed);
        assert!(outcome.message.contains("src/defaults.rs:1"));
    }

    #[test]
    fn public_artifact_probe_exempts_the_projects_own_public_coordinates() {
        let repo = TmpRepo::new("own-coordinates");
        repo.mkdir(".git");
        std::fs::write(
            repo.path.join(".git/config"),
            "[remote \"origin\"]\n    url = git@github.com:example-owner/example-tool.git\n",
        )
        .unwrap();
        std::fs::write(
            repo.path.join("README.md"),
            "[![CI](https://github.com/example-owner/example-tool/actions/badge.svg)]\n\
             brew install example-owner/example-tool/example-tool\n\
             https://github.com/example-owner/homebrew-example-tool\n\
             https://github.com/example-owner/public-dependency\n",
        )
        .unwrap();
        let deny = BTreeSet::from(["example-owner".to_string(), "example-tool".to_string()]);
        let context = ProbeContext {
            user_specific_deny_list: &deny,
        };
        let outcome = probe_public_artifact_specifics(&repo.path, &context).unwrap();
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn public_artifact_probe_derives_own_coordinates_from_a_package_manifest_without_git() {
        let repo = TmpRepo::new("manifest-coordinates");
        std::fs::write(
            repo.path.join("Cargo.toml"),
            "[package]\nname = \"example-tool\"\nversion = \"0.1.0\"\nrepository = \"https://github.com/example-owner/example-tool\"\n",
        )
        .unwrap();
        std::fs::write(
            repo.path.join("README.md"),
            "https://github.com/example-owner/example-tool\n",
        )
        .unwrap();
        let deny = BTreeSet::from(["example-owner".to_string(), "example-tool".to_string()]);
        let context = ProbeContext {
            user_specific_deny_list: &deny,
        };
        let outcome = probe_public_artifact_specifics(&repo.path, &context).unwrap();
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn public_artifact_probe_still_flags_an_other_private_repo_under_the_owner() {
        let repo = TmpRepo::new("other-private-coordinate");
        repo.mkdir(".git");
        std::fs::write(
            repo.path.join(".git/config"),
            "[remote \"origin\"]\n    url = https://github.com/example-owner/example-tool.git\n",
        )
        .unwrap();
        std::fs::write(
            repo.path.join("README.md"),
            "https://github.com/example-owner/private-widget\n",
        )
        .unwrap();
        let deny = BTreeSet::from(["example-owner".to_string(), "private-widget".to_string()]);
        let context = ProbeContext {
            user_specific_deny_list: &deny,
        };
        let outcome = probe_public_artifact_specifics(&repo.path, &context).unwrap();
        assert!(!outcome.passed);
        assert!(outcome.message.contains("README.md:1"));
        assert!(!outcome.message.contains("private-widget"));
    }

    fn verify_deferrals(repo: &Path) -> ProbeOutcome {
        let deny = BTreeSet::new();
        let context = ProbeContext {
            user_specific_deny_list: &deny,
        };
        probe_verified_deferrals(repo, &context).unwrap()
    }

    fn write_issue(repo: &TmpRepo, slug: &str, status: &str) {
        let path = repo.path.join("issues").join(slug).join("item.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("---\nstatus: {status}\n---\n\n# Fixture\n")).unwrap();
    }

    #[test]
    fn deferral_probe_flags_an_unresolvable_reference() {
        let repo = TmpRepo::new("unresolved-deferral");
        let slug = ["missing", "widget"].join("-");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature disabled until issue {slug} is resolved.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);
        assert!(outcome.message.contains("unresolved local issue"));
    }

    #[test]
    fn deferral_probe_accepts_a_valid_open_reference() {
        let repo = TmpRepo::new("open-deferral");
        let slug = ["enable", "widget"].join("-");
        write_issue(&repo, &slug, "open");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature disabled until issue {slug} is resolved.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn deferral_probe_rejects_a_closed_reference() {
        let repo = TmpRepo::new("closed-deferral");
        let slug = ["enable", "widget"].join("-");
        write_issue(&repo, &slug, "done");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature disabled until issue {slug} is resolved.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);
        assert!(outcome.message.contains("non-open status \"done\""));
    }

    #[test]
    fn deferral_probe_fails_closed_for_cross_repository_references() {
        let repo = TmpRepo::new("cross-repo-deferral");
        let tracker = ["example", "tracker"].join("-");
        let slug = ["enable", "widget"].join("-");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature disabled until {tracker} issue {slug} is resolved.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);
        assert!(outcome.message.contains("unresolved local issue"));
    }

    #[test]
    fn cross_repository_support_passes_when_the_slug_has_an_open_local_mirror() {
        let repo = TmpRepo::new("cross-repo-mirror");
        let tracker = ["example", "tracker"].join("-");
        let slug = ["enable", "widget"].join("-");
        write_issue(&repo, &slug, "open");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature disabled until {tracker} issue {slug} is resolved.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn deferral_probe_catches_an_owner_named_before_the_issue_noun() {
        let repo = TmpRepo::new("reverse-owner");
        let slug = ["missing", "owner"].join("-");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature is owned by the separate {slug} issue until it lands.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);
        assert!(outcome.message.contains(&slug));
    }

    #[test]
    fn deferral_probe_does_not_treat_a_tool_name_as_an_issue_without_the_noun() {
        let repo = TmpRepo::new("tool-owner");
        std::fs::write(
            repo.path.join("design.md"),
            "The generated file is owned by cargo-dist as a settled design boundary.\n",
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn source_scan_checks_comments_but_not_equivalent_code_strings() {
        let repo = TmpRepo::new("source-comments");
        let slug = ["missing", "owner"].join("-");
        std::fs::write(
            repo.path.join("example.rs"),
            format!("const TEXT: &str = \"Feature disabled until issue {slug} lands.\";\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);

        std::fs::write(
            repo.path.join("example.rs"),
            format!("// Feature disabled until issue {slug} lands.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);

        std::fs::write(
            repo.path.join("example.rs"),
            format!(
                "const URL: &str = \"https://example.invalid/#issue\"; // Feature disabled until issue {slug} lands.\n"
            ),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(
            !outcome.passed,
            "a real comment after a URL string must be scanned"
        );
    }

    #[test]
    fn blocked_by_reverse_owner_is_detected() {
        let repo = TmpRepo::new("blocked-owner");
        let slug = ["missing", "owner"].join("-");
        std::fs::write(
            repo.path.join("design.md"),
            format!("Feature is blocked by the separate {slug} issue.\n"),
        )
        .unwrap();
        assert!(!verify_deferrals(&repo.path).passed);
    }

    #[test]
    fn an_explicit_historical_suppression_skips_the_logical_block() {
        let repo = TmpRepo::new("historical-allow");
        let slug = ["old", "owner"].join("-");
        std::fs::write(
            repo.path.join("CHANGELOG.md"),
            format!(
                "<!-- canon:s24-allow: historical quotation -->\nFeature was disabled until issue {slug} landed.\n"
            ),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);
    }

    #[test]
    fn issue_status_requires_closed_frontmatter_and_accepts_a_quoted_value() {
        assert_eq!(
            issue_status("---\nstatus: \"open\"\n---\n# body"),
            Some("open")
        );
        assert_eq!(issue_status("---\nstatus: open\n# body status: done"), None);
    }

    #[test]
    fn tracked_file_enumeration_ignores_an_untracked_deferral() {
        let repo = TmpRepo::new("tracked-only");
        assert!(std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo.path)
            .status()
            .unwrap()
            .success());
        let slug = ["missing", "owner"].join("-");
        std::fs::write(
            repo.path.join("untracked.md"),
            format!("Feature disabled until issue {slug} lands.\n"),
        )
        .unwrap();
        let outcome = verify_deferrals(&repo.path);
        assert!(outcome.passed, "{}", outcome.message);

        assert!(std::process::Command::new("git")
            .args(["add", "untracked.md"])
            .current_dir(&repo.path)
            .status()
            .unwrap()
            .success());
        let outcome = verify_deferrals(&repo.path);
        assert!(!outcome.passed);
    }

    #[test]
    fn the_deferral_probe_passes_on_this_repository() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let outcome = verify_deferrals(&workspace);
        assert!(outcome.passed, "{}", outcome.message);
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
