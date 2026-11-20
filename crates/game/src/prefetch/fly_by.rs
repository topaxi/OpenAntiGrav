//! HD's Track Select fly-by clips: one `preview.bik` per circuit.
//!
//! The picker plays `<circuit location>\FE\preview.bik`
//! ([`oag_title::FrontEnd::circuit_fly_by`], `crate::app::picker_stage`'s
//! `pump_fly_by`) through [`crate::clip`], and on a cold cache the first open of
//! each circuit waits for a lossless transcode. This lists the same entries, from
//! the same circuit list Track Select is built from, and keys them with
//! [`crate::clip::cache_key`], so the file converted here is the one the screen
//! finds.
//!
//! A title with no `circuit_fly_by` (every one but HD) plans nothing.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::clip;
use crate::movie;

/// One circuit's clip, read.
#[derive(Debug)]
pub(super) struct Clip {
    /// The archive entry name the fly-by asks for.
    pub entry: String,
    /// Its movie-cache key.
    pub key: String,
    pub blob: Vec<u8>,
}

/// What [`find`] found on a source.
#[derive(Debug, Default)]
pub(super) struct Found {
    pub clips: Vec<Clip>,
    /// Circuits that ship no clip, and archives that would not read.
    pub notes: Vec<String>,
}

/// Every circuit's clip on `source` (`dlc` are the downloadable-content
/// folders to mount, as the game does), or `None` when its title plays none.
pub(super) fn find(source: &str, dlc: &[PathBuf]) -> Result<Option<Found>> {
    let (packs, pure_packs, _) =
        oag_source::dlc::packs_from_defaults(dlc, &oag_source::cache::default_dlc_cache_dir());
    let oag_source::title::Opened {
        mut archives,
        title,
    } = oag_source::title::open_source(source, packs, pure_packs)
        .with_context(|| format!("opening the archives in {source}"))?;
    let Some(clip_path) = title
        .front_end
        .and_then(|front_end| front_end.circuit_fly_by)
    else {
        return Ok(None);
    };
    let mut found = Found::default();
    for location in crate::boot::roster::circuit_locations(&mut archives, title) {
        let entry = oag_ui_screens::picker::hd::track::fly_by_src(&location, clip_path);
        match archives.read_name(&entry) {
            Ok(blob) => found.clips.push(Clip {
                key: clip::cache_key(&entry, blob.len()),
                entry,
                blob,
            }),
            Err(e) => found
                .notes
                .push(format!("{location} has no fly-by clip ({e})")),
        }
    }
    Ok(Some(found))
}

/// Whether the cache already holds `clip`'s frames, answered from the Bink
/// header alone: the cache file's name carries the key and the picture size,
/// and `all` for the frame count, because a Bink clip is converted whole
/// (`crate::movie`'s `bink::open` plans [`movie::Extent::Whole`]).
pub(super) fn is_cached(clip: &Clip, existing: &[String]) -> bool {
    let Ok(header) = oag_video::bik::parse(&clip.blob) else {
        return false;
    };
    existing.contains(&movie::cache_name(
        &clip.key,
        header.width,
        header.height,
        None,
    ))
}

/// Whether the movie cache in `movies` already holds the clip `entry` whose
/// bytes are `blob`: what the prefetch asks before it plans a conversion.
#[must_use]
pub fn is_converted(entry: &str, blob: &[u8], movies: &Path) -> bool {
    let clip = Clip {
        key: clip::cache_key(entry, blob.len()),
        entry: entry.to_string(),
        blob: blob.to_vec(),
    };
    let existing: Vec<String> = std::fs::read_dir(movies)
        .into_iter()
        .flatten()
        .flatten()
        .map(|file| file.file_name().to_string_lossy().into_owned())
        .collect();
    is_cached(&clip, &existing)
}

/// Converts `clip` through the call the screen makes.
pub(super) fn convert(clip: &Clip, cache: &Path, refresh: bool) -> Result<()> {
    let movie = clip::open_refreshing(&clip.entry, &clip.blob, cache, refresh)?;
    super::report_picture(&movie)
}

/// The entry and cache key of every circuit's clip on `source`: what the
/// prefetch plans, for a test that checks it against the archives.
///
/// # Errors
///
/// A source that will not open.
pub fn clip_keys(source: &str, dlc: &[PathBuf]) -> Result<Vec<(String, String)>> {
    Ok(find(source, dlc)?
        .map(|found| {
            found
                .clips
                .into_iter()
                .map(|clip| (clip.entry, clip.key))
                .collect()
        })
        .unwrap_or_default())
}

/// One clip's bytes out of `source`, as the picker reads them.
///
/// # Errors
///
/// A source that will not open or an entry it does not hold.
pub fn read_entry(source: &str, entry: &str) -> Result<Vec<u8>> {
    let oag_source::title::Opened { mut archives, .. } =
        oag_source::title::open_source(source, Vec::new(), Vec::new())?;
    Ok(archives.read_name(entry)?)
}
