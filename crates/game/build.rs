//! Embeds the build's short commit hash as `env!("OAG_GIT_HASH")`.
//!
//! The launcher screen prints it at the bottom (`crate::launcher::draw_list`)
//! so a player reporting "the Deck build doesn't find my disc" and a
//! maintainer looking at `git log` can agree on which commit that actually
//! was, without either of them having to ask.
//!
//! Best-effort, never fails the build: no `.git` at all (a source tarball), a
//! `HEAD` this cannot parse, or `git` itself missing all fall back to
//! `"unknown"` rather than an error. The last one is not hypothetical - the
//! `--container` build's Debian bookworm image
//! ([`packaging/appimage/Containerfile`](../../packaging/appimage/Containerfile))
//! installs no `git`, on purpose, since nothing else in the build needs it.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let hash = git_hash().unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=OAG_GIT_HASH={hash}");
}

/// The short commit hash of whatever `HEAD` is in the workspace this crate is
/// built from, or `None` if it cannot be worked out.
fn git_hash() -> Option<String> {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR")?);
    let dot_git = find_dot_git(&manifest_dir)?;
    let git_dir = resolve_git_dir(&dot_git)?;

    track_head(&git_dir);

    // `git` itself first: it already knows how to resolve a worktree's `.git`
    // file and pick an abbreviation long enough to stay unique. Everything
    // below is only reached in the one environment that has no `git` binary
    // at all to ask.
    via_git_command(&manifest_dir).or_else(|| via_head_file(&git_dir))
}

/// Walks upward from `start` looking for a `.git` entry - a directory in an
/// ordinary checkout, a file (`gitdir: <path>`) inside a `git worktree add`
/// checkout. `CARGO_MANIFEST_DIR` is a crate directory two levels under the
/// workspace root, so this has to look above itself rather than in place.
fn find_dot_git(start: &Path) -> Option<PathBuf> {
    let mut dir = start;
    loop {
        let candidate = dir.join(".git");
        if candidate.exists() {
            return Some(candidate);
        }
        dir = dir.parent()?;
    }
}

/// The real git directory a `.git` entry names - itself, if it is already a
/// directory, or wherever its `gitdir:` line points, for a worktree.
fn resolve_git_dir(dot_git: &Path) -> Option<PathBuf> {
    if dot_git.is_dir() {
        return Some(dot_git.to_path_buf());
    }

    let contents = std::fs::read_to_string(dot_git).ok()?;
    let line = contents
        .lines()
        .find_map(|line| line.strip_prefix("gitdir:"))?;
    let path = dot_git.parent()?.join(line.trim());
    Some(path.canonicalize().unwrap_or(path))
}

/// Tells cargo to rerun this script whenever a new commit could change the
/// answer: `HEAD` itself, and the ref file or `packed-refs` entry it points
/// at - a checked-out branch's `HEAD` is a pointer (`ref: refs/heads/main`)
/// that reads the same after every commit made on that branch, so watching
/// `HEAD` alone would miss every one of them.
fn track_head(git_dir: &Path) {
    let head_path = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head_path.display());

    if let Ok(head) = std::fs::read_to_string(&head_path)
        && let Some(reference) = head.trim().strip_prefix("ref: ")
    {
        println!(
            "cargo:rerun-if-changed={}",
            git_dir.join(reference).display()
        );
    }

    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join("packed-refs").display()
    );
}

/// Asks the `git` binary, if there is one.
fn via_git_command(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let hash = String::from_utf8(output.stdout).ok()?;
    let hash = hash.trim();
    (!hash.is_empty()).then(|| hash.to_string())
}

/// Reads `HEAD` by hand: a detached hash directly, or a `ref:` pointer
/// resolved against the ref file or, failing that, `packed-refs` - a branch
/// that has never been rewritten since a `git gc` has no loose ref file left,
/// only a packed one.
fn via_head_file(git_dir: &Path) -> Option<String> {
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();

    let full_hash = match head.strip_prefix("ref: ") {
        Some(reference) => std::fs::read_to_string(git_dir.join(reference))
            .ok()
            .map(|hash| hash.trim().to_string())
            .or_else(|| hash_from_packed_refs(git_dir, reference))?,
        None => head.to_string(),
    };

    full_hash.get(..7).map(str::to_string)
}

/// `<hash> <ref>` per line, git's own packed-refs format.
fn hash_from_packed_refs(git_dir: &Path, reference: &str) -> Option<String> {
    let packed = std::fs::read_to_string(git_dir.join("packed-refs")).ok()?;
    packed.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?;
        (name == reference).then(|| hash.to_string())
    })
}
