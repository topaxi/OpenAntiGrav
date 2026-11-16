//! Embeds the build's short commit hash and version suffix as `env!("OAG_GIT_HASH")` and
//! `env!("OAG_VERSION_SUFFIX")`, which [`crate::BUILD_VERSION`] spells and every binary that
//! reports a build reads (`oag-game`, `oag-server`, the ghost header, the launcher).
//!
//! It also embeds the commit's date as `env!("OAG_BUILD_DATE")` (`YYYY-MM-DD`, empty when
//! unknown), for display only: it is not part of [`crate::BUILD_VERSION`] and nothing a
//! handshake, ghost or replay header compares. `OAG_BUILD_DATE` wins when set, even empty
//! (`just test-data` sets it empty, like the hash, so a commit does not rebuild every test
//! binary); then `SOURCE_DATE_EPOCH` (reproducible release builds); then `git log -1 --format=%cs`.
//! Never the wall clock.
//!
//! Lives in `oag-net` because the server and the client compare their builds and both link
//! this crate; `oag-game` takes the strings from here rather than computing them again.
//!
//! Best-effort, never fails the build: no `.git` at all (a source tarball), a
//! `HEAD` this cannot parse, or `git` itself missing all fall back to
//! `"unknown"` rather than an error. The last one is not hypothetical - the
//! `--container` build's Debian bookworm image
//! ([`packaging/appimage/Containerfile`](../../packaging/appimage/Containerfile))
//! installs no `git`, on purpose, since nothing else in the build needs it -
//! so `scripts/build-appimage.sh` and `scripts/build-server-portable.sh` pass the host's hash
//! in as `OAG_GIT_HASH`, which wins over everything below when set. `just test-data` sets it to
//! `test-data` so a commit does not rebuild every test binary.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=OAG_GIT_HASH");
    let hash = std::env::var("OAG_GIT_HASH")
        .ok()
        .filter(|hash| !hash.is_empty())
        .or_else(git_hash)
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=OAG_GIT_HASH={hash}");
    // The submodule commits this commit pins, for the game to compare against the files it
    // loads. Only beside `git`'s own hash: a hash passed in (an AppImage, `just test-data`)
    // builds from no checkout the game will run in, and must not rebuild on a commit.
    let pins = if std::env::var("OAG_GIT_HASH").is_ok_and(|hash| !hash.is_empty()) {
        String::new()
    } else {
        submodule_pins().unwrap_or_default()
    };
    println!("cargo:rustc-env=OAG_SUBMODULE_PINS={pins}");
    println!("cargo:rerun-if-env-changed=OAG_BUILD_DATE");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    let date = build_date().unwrap_or_default();
    println!("cargo:rustc-env=OAG_BUILD_DATE={date}");
    // The nightly workflow passes `-nightly`, so `oag-game --version` reads
    // `0.1.0-nightly (abc1234)`. Unset, it is empty and a release is unchanged.
    println!("cargo:rerun-if-env-changed=OAG_VERSION_SUFFIX");
    let suffix = std::env::var("OAG_VERSION_SUFFIX").unwrap_or_default();
    println!("cargo:rustc-env=OAG_VERSION_SUFFIX={suffix}");
}

/// The date to show beside the hash, or `None` for no date at all.
fn build_date() -> Option<String> {
    if let Ok(date) = std::env::var("OAG_BUILD_DATE") {
        return Some(date.trim().to_string());
    }
    if let Some(epoch) = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|epoch| epoch.trim().parse::<i64>().ok())
    {
        return Some(iso_date(epoch));
    }
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR")?);
    let output = Command::new("git")
        .args(["log", "-1", "--format=%cs", "HEAD"])
        .current_dir(manifest_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let date = String::from_utf8(output.stdout).ok()?;
    let date = date.trim();
    (!date.is_empty()).then(|| date.to_string())
}

/// `path=hash;path=hash` for every submodule `HEAD` pins (`git ls-tree`'s gitlinks, short),
/// or `None` without `git`. Reruns with the hash ([`track_head`]): a pin changes only with a
/// commit.
fn submodule_pins() -> Option<String> {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR")?);
    let output = Command::new("git")
        .args(["ls-tree", "-r", "HEAD"])
        .current_dir(manifest_dir.join("../.."))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let pins: Vec<String> = text
        .lines()
        .filter_map(|line| {
            let (meta, path) = line.split_once('\t')?;
            let mut parts = meta.split_whitespace();
            let (_, kind, hash) = (parts.next()?, parts.next()?, parts.next()?);
            let short = hash.get(..7)?;
            (kind == "commit").then(|| format!("{path}={short}"))
        })
        .collect();
    Some(pins.join(";"))
}

/// `YYYY-MM-DD` (UTC) of a Unix timestamp, by the civil-from-days algorithm.
fn iso_date(epoch: i64) -> String {
    let z = epoch.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
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

/// The directory the refs live in. A linked worktree's git directory
/// (`.git/worktrees/<name>`) holds only its own `HEAD` and `index`; the
/// branch ref files and `packed-refs` stay in the main repository, which the
/// worktree's `commondir` file names (relative to the worktree's git
/// directory). An ordinary checkout has no `commondir` and is its own.
fn common_git_dir(git_dir: &Path) -> PathBuf {
    std::fs::read_to_string(git_dir.join("commondir"))
        .ok()
        .map(|rel| {
            let path = git_dir.join(rel.trim());
            path.canonicalize().unwrap_or(path)
        })
        .unwrap_or_else(|| git_dir.to_path_buf())
}

/// Tells cargo to rerun this script whenever a new commit could change the
/// answer: `HEAD` itself, and the ref file or `packed-refs` entry it points
/// at - a checked-out branch's `HEAD` is a pointer (`ref: refs/heads/main`)
/// that reads the same after every commit made on that branch, so watching
/// `HEAD` alone would miss every one of them.
///
/// **Only paths that exist are emitted.** Cargo treats a missing
/// `rerun-if-changed` path as changed on every build, so naming the
/// worktree's own `refs/heads/<branch>` (which is never there, see
/// [`common_git_dir`]) or a `packed-refs` that has not been written yet made
/// this crate - and the 130-odd test binaries that link it - rebuild and
/// relink on every `cargo` invocation in a worktree.
fn track_head(git_dir: &Path) {
    let common = common_git_dir(git_dir);

    let mut tracked = vec![git_dir.join("HEAD")];
    if let Ok(head) = std::fs::read_to_string(git_dir.join("HEAD"))
        && let Some(reference) = head.trim().strip_prefix("ref: ")
    {
        tracked.push(git_dir.join(reference));
        tracked.push(common.join(reference));
    }
    tracked.push(common.join("packed-refs"));

    for path in tracked.into_iter().filter(|path| path.exists()) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
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
    let common = common_git_dir(git_dir);

    let full_hash = match head.strip_prefix("ref: ") {
        Some(reference) => std::fs::read_to_string(common.join(reference))
            .ok()
            .map(|hash| hash.trim().to_string())
            .or_else(|| hash_from_packed_refs(&common, reference))?,
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
