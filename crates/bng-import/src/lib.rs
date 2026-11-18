//! A foreign track: a Unity circuit read by `oag-ballisticng` and mapped into
//! this project's own track model, `oag_trackfile::Track` (ADR-0062, decision 4;
//! ADR-0063), the plain data the race consumes.
//!
//! **Nothing here is measured off the original game.** [`map`] turns what the
//! content layer *read* into this engine's shapes, and every constant that turns
//! a reading into a number is in [`choice`], each labelled *chosen, not
//! measured*. What the content does not author, or this mapping has no
//! counterpart for, is named in [`oag_trackfile::Track::notes`] - drawn as
//! nothing, not stood in for.
//!
//! A race on a foreign track runs on a host title's physics, rules and ship. The
//! output is `oag_vex::track` spline data, collision nodes, pad volumes and a
//! start slot; the simulation reads them like a disc circuit's and learns nothing
//! of where they came from.

pub mod choice;
mod frame;
mod map;

use std::path::Path;

use oag_trackfile::Track;

/// Reads the `.wtrk` at `path` and maps it.
///
/// # Errors
///
/// A file the wrapper or the Unity reader refuses, a track class the content
/// layer refuses, or a track with no lap loop. The message names which.
pub fn read(path: &Path) -> anyhow::Result<Track> {
    let file =
        std::fs::read(path).map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
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

/// What a `.wtrk`'s wrapper says about itself, without reading its Unity bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The Workshop item id, which keys the track's records and ghosts.
    pub workshop_id: String,
    /// The author's title, untrimmed.
    pub title: String,
    /// The author's tags.
    pub tags: Vec<String>,
}

impl Header {
    /// Whether the author tagged the item `tag` (compared exactly).
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

/// Why [`header`] has no header to give.
#[derive(Debug)]
pub enum HeaderError {
    /// The file could not be read.
    Unreadable(std::io::ErrorKind),
    /// The wrapper refuses the bytes. `damaged` is set when they open the way a
    /// Workshop track does, so the file is a broken track rather than never one.
    Refused {
        /// The wrapper's own message.
        reason: String,
        /// Whether the file starts like a Workshop track.
        damaged: bool,
    },
}

/// Reads the wrapper of the `.wtrk` at `path`.
///
/// # Errors
///
/// A file that cannot be read, or that the wrapper refuses.
pub fn header(path: &Path) -> Result<Header, HeaderError> {
    let file = std::fs::read(path).map_err(|e| HeaderError::Unreadable(e.kind()))?;
    let wtrk = oag_ballisticng::wtrk::Wtrk::parse(&file).map_err(|e| HeaderError::Refused {
        reason: e.to_string(),
        damaged: file
            .get(1..)
            .is_some_and(|f| f.starts_with(b"Workshop Track")),
    })?;
    Ok(Header {
        workshop_id: wtrk.workshop_id.to_string(),
        title: wtrk.title,
        tags: wtrk.tags,
    })
}

/// The Workshop id a `.wtrk` carries, which keys its records and ghosts.
///
/// # Errors
///
/// A file that cannot be read, or that the wrapper refuses.
pub fn workshop_id(path: &Path) -> anyhow::Result<String> {
    let file =
        std::fs::read(path).map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
    Ok(oag_ballisticng::wtrk::Wtrk::parse(&file)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?
        .workshop_id
        .to_string())
}
