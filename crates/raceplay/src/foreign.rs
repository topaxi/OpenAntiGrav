//! A foreign track: a Unity circuit read by `oag-ballisticng` and mapped into
//! this project's own track model, `oag_trackfile::Track` (ADR-0062, decision 4;
//! ADR-0063), the plain data the race consumes.
//!
//! **Nothing here is measured off the original game.** [`map`] turns what the
//! content layer *read* into this engine's shapes, and every constant that turns
//! a reading into a number is in [`choice`], each labelled *chosen, not
//! measured*. What the content does not author, or this mapping has no
//! counterpart for, is named in [`Track::notes`] - drawn as nothing, not
//! stood in for.
//!
//! A race on a foreign track runs on a host title's physics, rules and ship. The
//! output is `oag_vex::track` spline data, collision nodes, pad volumes and a
//! start slot; the simulation reads them like a disc circuit's and learns nothing
//! of where they came from.

pub mod choice;
mod frame;
mod map;
pub mod survey;

use std::path::Path;

use anyhow::Context;
use oag_trackfile::Track;

/// Reads the `.wtrk` at `path` and maps it.
///
/// # Errors
///
/// A file the wrapper or the Unity reader refuses, a track class the content
/// layer refuses, or a track with no lap loop. The message names which.
pub fn read(path: &Path) -> anyhow::Result<Track> {
    let file = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let wtrk = oag_ballisticng::wtrk::Wtrk::parse(&file)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    let bundle = oag_unity::bundle::Bundle::parse(wtrk.bundle(&file))
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    let assets = oag_unity::assets::Assets::open(&bundle)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    let track = oag_ballisticng::track::Track::read(&assets)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    map::map(&wtrk, &track)
}

/// Whether `path` is one of this project's own track manifests (`*.track.toml`)
/// rather than a Workshop `.wtrk`.
#[must_use]
pub fn is_track_file(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "toml")
}

/// Reads the track at `path` whichever format it is in, with the lines the
/// loader report starts from: what the file said about itself, what it
/// carries nothing for, and what the reader left unread.
///
/// # Errors
///
/// As [`read`] for a `.wtrk`, and as [`oag_trackfile::read`] for a manifest.
pub fn read_any(path: &Path) -> anyhow::Result<(Track, Vec<String>)> {
    if !is_track_file(path) {
        let track = read(path)?;
        let report = track.notes.clone();
        return Ok((track, report));
    }
    let loaded =
        oag_trackfile::read(path).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    let mut report = vec![format!(
        "track file {:?} ({}), format {}.{}",
        loaded.track.title, loaded.track.id, loaded.version.0, loaded.version.1
    )];
    report.extend(loaded.track.notes.iter().cloned());
    report.push(format!(
        "not authored by format {}.x, so drawn as nothing and not borrowed from the host: {}",
        loaded.version.0,
        oag_trackfile::ABSENT.join(", ")
    ));
    report.extend(loaded.unread.iter().map(|u| format!("not read: {u}")));
    Ok((loaded.track, report))
}

/// The catalogue entry for the track at `path` (a `.wtrk` or a track-file
/// manifest): a synthetic [`catalogue::Track`](crate::catalogue::Track) carrying
/// the file as data, so nothing downstream branches on a title or a name to tell
/// it from a disc circuit. Its `id` is `foreign:` and the track's own id (the
/// Workshop id for a `.wtrk`), which is also what keys its records and ghosts.
///
/// # Errors
///
/// A file the `.wtrk` wrapper or the manifest reader refuses.
pub fn entry(path: &Path) -> anyhow::Result<crate::catalogue::Track> {
    let id = if is_track_file(path) {
        oag_trackfile::read(path)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?
            .track
            .id
    } else {
        let file = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        oag_ballisticng::wtrk::Wtrk::parse(&file)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?
            .workshop_id
            .to_string()
    };
    Ok(crate::catalogue::Track {
        id: format!("foreign:{id}"),
        location: path.display().to_string(),
        reversed: false,
        available_in_zone: false,
        unlock_grid: None,
        sound_register: None,
        foreign: Some(path.to_path_buf()),
    })
}
