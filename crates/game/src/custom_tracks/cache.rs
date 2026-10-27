//! What reading each track file said, kept between runs.
//!
//! A `.wtrk` is an LZMA bundle and reading one is the cost of the survey (about
//! 20 s for 32 Workshop tracks in a debug build). The cache keys a [`Listing`]
//! by the file's path and modification time, so a file that has not been
//! touched is not decoded again, and one that has is read afresh. The path
//! stands in for the file name because a Workshop item's file name is not unique
//! across the item directories. Deleting the file is always safe.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use oag_raceplay::foreign::survey::{self, Kind, Listing};
use serde::{Deserialize, Serialize};

/// Bump when the importer's verdicts change, so a cached "cannot load" or a
/// cached title is not trusted across the change.
const REVISION: u32 = 1;

const FILE_NAME: &str = "survey.toml";

/// The cache file inside `dir`.
pub(super) fn file_in(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Stored {
    #[serde(default)]
    revision: u32,
    #[serde(default)]
    entry: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Entry {
    path: String,
    modified_secs: u64,
    modified_nanos: u32,
    own: bool,
    id: String,
    title: String,
    author: Option<String>,
    tags: Vec<String>,
    problem: Option<String>,
}

/// The modification time of `path` as the cache keys it, or `None` when the
/// file system does not say (such a file is always read).
fn stamp(path: &Path) -> Option<(u64, u32)> {
    let since = std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?;
    Some((since.as_secs(), since.subsec_nanos()))
}

fn load(cache_file: &Path) -> HashMap<String, Entry> {
    let Ok(text) = std::fs::read_to_string(cache_file) else {
        return HashMap::new();
    };
    match toml::from_str::<Stored>(&text) {
        Ok(stored) if stored.revision == REVISION => stored
            .entry
            .into_iter()
            .map(|e| (e.path.clone(), e))
            .collect(),
        _ => HashMap::new(),
    }
}

fn from_entry(entry: Entry) -> Listing {
    Listing {
        path: PathBuf::from(entry.path),
        id: entry.id,
        title: entry.title,
        author: entry.author,
        kind: if entry.own { Kind::Own } else { Kind::Workshop },
        tags: entry.tags,
        problem: entry.problem,
    }
}

fn to_entry(listing: &Listing, (secs, nanos): (u64, u32)) -> Entry {
    Entry {
        path: listing.path.display().to_string(),
        modified_secs: secs,
        modified_nanos: nanos,
        own: listing.kind == Kind::Own,
        id: listing.id.clone(),
        title: listing.title.clone(),
        author: listing.author.clone(),
        tags: listing.tags.clone(),
        problem: listing.problem.clone(),
    }
}

/// A [`survey::listing`] of every file in `files`, in their order, reusing the
/// cache for each file whose path and modification time are unchanged, then
/// writing the cache back. With no `cache_file` every file is read.
pub(super) fn survey(files: &[PathBuf], cache_file: Option<&Path>) -> Vec<Listing> {
    let mut known = cache_file.map(load).unwrap_or_default();
    let mut kept = Vec::with_capacity(files.len());
    let listings = files
        .iter()
        .map(|path| {
            let stamp = stamp(path);
            let key = path.display().to_string();
            let hit = known
                .remove(&key)
                .filter(|e| stamp == Some((e.modified_secs, e.modified_nanos)));
            let listing = hit.map_or_else(|| survey::listing(path), from_entry);
            if let Some(stamp) = stamp {
                kept.push(to_entry(&listing, stamp));
            }
            listing
        })
        .collect();
    if let Some(cache_file) = cache_file {
        store(cache_file, kept);
    }
    listings
}

fn store(cache_file: &Path, entry: Vec<Entry>) {
    let stored = Stored {
        revision: REVISION,
        entry,
    };
    let write = || -> Result<(), String> {
        if let Some(dir) = cache_file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = toml::to_string(&stored).map_err(|e| e.to_string())?;
        let partial = cache_file.with_extension("toml.part");
        std::fs::write(&partial, text).map_err(|e| e.to_string())?;
        std::fs::rename(&partial, cache_file).map_err(|e| e.to_string())
    };
    if let Err(e) = write() {
        log::warn!("custom track cache {}: {e}", cache_file.display());
    }
}
