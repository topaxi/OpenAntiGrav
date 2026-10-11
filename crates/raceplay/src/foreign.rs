//! A foreign track: a Unity circuit read by `oag-ballisticng` and mapped into the
//! plain data the race already consumes (ADR-0062, decision 4).
//!
//! **Nothing here is measured off the original game.** [`map`] turns what the
//! content layer *read* into this engine's shapes, and every constant that turns
//! a reading into a number is in [`choice`], each labelled *chosen, not
//! measured*. What the content does not author, or this mapping has no
//! counterpart for, is named in [`ForeignTrack::report`] - drawn as nothing, not
//! stood in for.
//!
//! A race on a foreign track runs on a host title's physics, rules and ship. The
//! output is `oag_vex::track` spline data, collision nodes, pad volumes and a
//! start slot; the simulation reads them like a disc circuit's and learns nothing
//! of where they came from.

pub mod choice;
mod frame;
mod map;

use std::path::Path;

use anyhow::Context;
use oag_vex::collision::CollisionNode;
use oag_vex::pads::PadVolume;
use oag_vex::track::{AiTrack, StartPosition};

/// A foreign track, mapped.
#[derive(Debug, Clone)]
pub struct ForeignTrack {
    /// The Workshop item id (`1392209921`).
    pub id: String,
    /// The author's title.
    pub title: String,
    /// The spline graph: one closed ring over the lap loop.
    pub ai: AiTrack,
    /// The start slot (the engine lays its own grid from it).
    pub start: StartPosition,
    /// Floor and wall collision.
    pub nodes: Vec<CollisionNode>,
    /// Speed pads.
    pub speedup_pads: Vec<PadVolume>,
    /// Weapon pads.
    pub weapon_pads: Vec<PadVolume>,
    /// Everything read, refused, ignored or chosen, for the loader report.
    pub report: Vec<String>,
}

/// Reads the `.wtrk` at `path` and maps it.
///
/// # Errors
///
/// A file the wrapper or the Unity reader refuses, a track class the content
/// layer refuses, or a track with no lap loop. The message names which.
pub fn read(path: &Path) -> anyhow::Result<ForeignTrack> {
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

/// The catalogue entry for the foreign track at `path`: a synthetic
/// [`catalogue::Track`](crate::catalogue::Track) carrying the file as data, so
/// nothing downstream branches on a title or a name to tell it from a disc
/// circuit. Its `id` is `foreign:` and the Workshop id, which is also what
/// keys its records and ghosts.
///
/// # Errors
///
/// A file the `.wtrk` wrapper refuses.
pub fn entry(path: &Path) -> anyhow::Result<crate::catalogue::Track> {
    let file = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let wtrk = oag_ballisticng::wtrk::Wtrk::parse(&file)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    Ok(crate::catalogue::Track {
        id: format!("foreign:{}", wtrk.workshop_id),
        location: path.display().to_string(),
        reversed: false,
        available_in_zone: false,
        unlock_grid: None,
        sound_register: None,
        foreign: Some(path.to_path_buf()),
    })
}
