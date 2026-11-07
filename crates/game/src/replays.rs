//! Where a recorded race is kept: `<data dir>/oag/replays/{auto,saved}/`, and the ring that
//! bounds `auto/`.
//!
//! A race that can be recorded is written to `auto/` when it ends (ADR-0087, decision 1).
//! `auto/` is a ring: the newest [`MAX_FILES`] files and at most [`MAX_BYTES`] bytes, oldest
//! deleted first, so a player who never looks at the list never fills a disc. `saved/` is the
//! player's own and is never pruned: a file enters it by KEEP, a copy from `auto/`.
//!
//! The data directory, not the config directory: a replay is the player's content, as a photo
//! is, and `ghosts/` stays beside `records.toml`. A file is `oag-<title>-<unix seconds>-<n>.oagr`
//! and its **age is its name**, never its modification time, which a backup or a copy resets.
//! Every function takes the root it works under, so a test never writes into the player's own
//! directory; only [`dir`] knows where that is.
//!
//! **Chosen, not measured:** the 20 files, the 32 MiB (about three times twenty of the largest
//! race measured, ADR-0087 decision 1, so the count is what normally bites) and the names.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::debug;
use oag_replay::{Header, ReadError, Replay};

pub use crate::ghosts::EXTENSION;

pub mod page;

/// The directory under the user's data directory replays are kept in.
pub const DIRECTORY: &str = "replays";
/// The ring: written when a race ends, pruned.
pub const AUTO: &str = "auto";
/// The player's own: kept, never pruned.
pub const SAVED: &str = "saved";
/// The most files `auto/` holds.
pub const MAX_FILES: usize = 20;
/// The most bytes `auto/` holds.
pub const MAX_BYTES: u64 = 32 * 1024 * 1024;

/// Whether this build has a directory the player can reach to keep a replay in: a desktop does;
/// a page and a phone do not, and the row is left out there rather than offered and failing.
pub const SAVES: bool = !cfg!(target_arch = "wasm32") && !oag_mobile::MOBILE;

/// The directory every replay lives under: `<data dir>/oag/replays`.
#[must_use]
pub fn dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("oag").join(DIRECTORY))
}

/// Which of the two folders a file is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Folder {
    /// The ring.
    Auto,
    /// The player's own.
    Saved,
}

impl Folder {
    fn name(self) -> &'static str {
        match self {
            Self::Auto => AUTO,
            Self::Saved => SAVED,
        }
    }
}

/// What the header of a listed file said.
#[derive(Debug, Clone, PartialEq)]
pub enum Head {
    /// A header this build reads.
    Read(Box<Header>),
    /// A header naming another kind of file or a later major than this build reads: a file
    /// from a newer version, not a damaged one.
    Newer,
    /// Not a replay, or a header that does not parse.
    Damaged,
}

/// One file in `auto/` or `saved/`.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Where it is.
    pub path: PathBuf,
    /// Which folder.
    pub folder: Folder,
    /// The unix seconds in its name, `0` for a name this build did not give.
    pub unix: u64,
    /// The counter in its name, which orders two files of one second.
    pub n: u32,
    /// Its size in bytes.
    pub bytes: u64,
    /// What its header said.
    pub head: Head,
}

impl Entry {
    /// What names this file among the listed ones: its folder and file name. A kept copy has
    /// the name of the automatic file it was copied from, so the name alone would not do.
    #[must_use]
    pub fn key(&self) -> String {
        let name = self
            .path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        format!("{}/{name}", self.folder.name())
    }
}

/// The title as a file-name segment: lower-case letters and digits, anything else a `-`.
fn slug(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// The name of the file `title` recorded at `unix` seconds is kept under, counter `n`.
fn file_name(title: &str, unix: u64, n: u32) -> String {
    format!("oag-{}-{unix}-{n}.{EXTENSION}", slug(title))
}

/// The `(unix seconds, n)` a name carries: the last two `-` separated numbers of its stem.
fn stamp(name: &str) -> (u64, u32) {
    let stem = name.strip_suffix(&format!(".{EXTENSION}")).unwrap_or(name);
    let mut parts = stem.rsplit('-');
    let n = parts.next().and_then(|n| n.parse().ok());
    let unix = parts.next().and_then(|unix| unix.parse().ok());
    match (unix, n) {
        (Some(unix), Some(n)) => (unix, n),
        _ => (0, 0),
    }
}

/// Where the next replay of `title`, ended at `unix` seconds, goes in `root`'s `auto/`: a name no
/// earlier file has, so two races ending in one second do not replace each other.
#[must_use]
pub fn auto_path(root: &Path, title: &str, unix: u64) -> PathBuf {
    let auto = root.join(AUTO);
    (1_u32..)
        .map(|n| auto.join(file_name(title, unix, n)))
        .find(|path| !path.exists())
        .unwrap_or_else(|| auto.join(file_name(title, unix, 0)))
}

/// Writes `replay` into `root`'s `auto/` and prunes the ring. Returns where it went.
///
/// The write goes to a temporary name and is renamed into place, so a crash leaves no half of a
/// file for the list to find.
///
/// # Errors
///
/// A directory that cannot be made or a file that cannot be written; the caller logs it and
/// carries on, as it does for `records.toml`.
pub fn write_auto_in(root: &Path, title: &str, unix: u64, replay: &Replay) -> Result<PathBuf> {
    let path = auto_path(root, title, unix);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let partial = path.with_extension(format!("{EXTENSION}.partial"));
    std::fs::write(&partial, replay.to_bytes())
        .with_context(|| format!("writing {}", partial.display()))?;
    std::fs::rename(&partial, &path)
        .with_context(|| format!("renaming {} into place", partial.display()))?;
    prune_in(root);
    Ok(path)
}

/// The files of one folder, with sizes and the ages their names carry, oldest first. The head
/// is not read.
fn files(root: &Path, folder: Folder) -> Vec<(PathBuf, (u64, u32), u64)> {
    let Ok(read) = std::fs::read_dir(root.join(folder.name())) else {
        return Vec::new();
    };
    let mut found: Vec<_> = read
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if path.extension()? != EXTENSION {
                return None;
            }
            let bytes = entry.metadata().ok()?.len();
            Some((stamp(name), path.clone(), bytes))
        })
        .map(|(stamp, path, bytes)| (path, stamp, bytes))
        .collect();
    found.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    found
}

/// Deletes the oldest files of `root`'s `auto/` until it holds at most [`MAX_FILES`] files and
/// [`MAX_BYTES`] bytes, keeping the newest whatever its size. Returns what it deleted.
pub fn prune_in(root: &Path) -> Vec<PathBuf> {
    prune_within(root, MAX_FILES, MAX_BYTES)
}

/// [`prune_in`] against other limits, so a test need not write 32 MiB.
pub fn prune_within(root: &Path, max_files: usize, max_bytes: u64) -> Vec<PathBuf> {
    let mut held = files(root, Folder::Auto);
    let mut bytes: u64 = held.iter().map(|(_, _, size)| size).sum();
    let mut deleted = Vec::new();
    while held.len() > 1 && (held.len() > max_files || bytes > max_bytes) {
        let (path, _, size) = held.remove(0);
        match std::fs::remove_file(&path) {
            Ok(()) => {
                debug!(
                    "replays: the ring deleted {} ({size} bytes)",
                    path.display()
                );
                bytes -= size;
                deleted.push(path);
            }
            Err(e) => {
                log::warn!("replays: could not delete {}: {e}", path.display());
                // Counted gone so the loop ends rather than retrying a file it cannot remove.
                bytes -= size;
            }
        }
    }
    deleted
}

/// The head of the file at `path`, read and no more.
fn read_head(path: &Path) -> Head {
    use std::io::Read;
    let Ok(file) = std::fs::File::open(path) else {
        return Head::Damaged;
    };
    // The header is the first chunk and a few hundred bytes; 64 KiB is room for any it holds.
    let mut bytes = Vec::new();
    if file.take(64 * 1024).read_to_end(&mut bytes).is_err() {
        return Head::Damaged;
    }
    match Replay::read_head(&bytes) {
        Ok(header) => Head::Read(Box::new(header)),
        Err(ReadError::Schema(_) | ReadError::Version(_)) => Head::Newer,
        Err(_) => Head::Damaged,
    }
}

/// Every replay under `root`: `saved/` then `auto/`, newest first within each.
#[must_use]
pub fn list_in(root: &Path) -> Vec<Entry> {
    let mut out = Vec::new();
    for folder in [Folder::Saved, Folder::Auto] {
        let mut held = files(root, folder);
        held.reverse();
        out.extend(held.into_iter().map(|(path, (unix, n), bytes)| Entry {
            head: read_head(&path),
            path,
            folder,
            unix,
            n,
            bytes,
        }));
    }
    out
}

/// [`list_in`] under [`dir`]; empty on a platform with no data directory.
#[must_use]
pub fn list() -> Vec<Entry> {
    dir().map(|root| list_in(&root)).unwrap_or_default()
}

/// Copies the `auto/` file `entry` into `root`'s `saved/`, where the ring never reaches it.
/// Returns the copy's path; a file already in `saved/` is returned as it is.
///
/// # Errors
///
/// A directory that cannot be made or a copy that fails.
pub fn keep_in(root: &Path, entry: &Entry) -> Result<PathBuf> {
    if entry.folder == Folder::Saved {
        return Ok(entry.path.clone());
    }
    let saved = root.join(SAVED);
    std::fs::create_dir_all(&saved).with_context(|| format!("creating {}", saved.display()))?;
    let name = entry
        .path
        .file_name()
        .with_context(|| format!("{} has no file name", entry.path.display()))?;
    let to = saved.join(name);
    std::fs::copy(&entry.path, &to)
        .with_context(|| format!("copying {} to {}", entry.path.display(), to.display()))?;
    Ok(to)
}

/// Deletes the file `entry` names.
///
/// # Errors
///
/// A file that cannot be removed.
pub fn delete(entry: &Entry) -> Result<()> {
    std::fs::remove_file(&entry.path).with_context(|| format!("deleting {}", entry.path.display()))
}

#[cfg(test)]
mod tests;
