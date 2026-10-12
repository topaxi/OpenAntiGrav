//! This project's own custom-track format (ADR-0063): a glTF 2.0 binary (`.glb`)
//! holding every number the race reads, and a TOML manifest beside it holding the
//! version, the names and the integer structure.
//!
//! **Nothing here is measured off an original game.** The model is the plain
//! data a race already consumes from a disc circuit (`oag_vex` types), so a
//! disc exporter and a foreign importer both target it and the simulation learns
//! nothing of where a track came from. What the format has no field for is
//! [`ABSENT`]; a loader names it rather than borrowing it from a host.
//!
//! Every `f32` the simulation reads travels as little-endian binary in a glTF
//! accessor, never as decimal text, so a read is bit-identical on every
//! platform and a write-read round trip is exact.

mod error;
pub mod fixture;
mod glb;
mod manifest;
mod model;

use std::path::{Path, PathBuf};

pub use error::Error;
pub use manifest::{FORMAT_MAJOR, SCHEMA_KIND};
pub use model::{ABSENT, Loaded, Track};

/// A manifest and its glb as bytes: what [`write`] puts on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parts {
    /// The TOML manifest.
    pub manifest: String,
    /// The glTF binary.
    pub glb: Vec<u8>,
}

/// Serialises `track` to its manifest and glb.
///
/// `glb_name` is the file name the manifest records for the glb, relative to
/// the manifest's own directory.
///
/// # Errors
///
/// A track the format cannot hold: an empty id, an empty collision mesh, a
/// non-finite number, a count past what the format indexes.
pub fn to_parts(track: &Track, glb_name: &str) -> Result<Parts, Error> {
    let glb = glb::write(track)?;
    let manifest = manifest::write(track, glb_name)?;
    Ok(Parts { manifest, glb })
}

/// Reads a track from a manifest and the bytes of the glb it names.
///
/// # Errors
///
/// Malformed or unsupported input, each named: an unknown major version, a
/// count the manifest and the glb disagree on, an unknown collision tag, a
/// transformed record node, a strided or mistyped accessor.
pub fn from_parts(manifest: &str, glb: &[u8]) -> Result<Loaded, Error> {
    let parsed = manifest::read(manifest)?;
    glb::read(parsed, glb)
}

/// Writes `track` as `<manifest>` and the glb beside it (same stem, `.glb`).
///
/// # Errors
///
/// As [`to_parts`], and any I/O failure.
pub fn write(track: &Track, manifest: &Path) -> Result<(), Error> {
    let glb_path = glb_path_beside(manifest);
    let name = glb_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::Path(manifest.display().to_string()))?
        .to_string();
    let parts = to_parts(track, &name)?;
    std::fs::write(&glb_path, parts.glb).map_err(|e| Error::io(&glb_path, &e))?;
    std::fs::write(manifest, parts.manifest).map_err(|e| Error::io(manifest, &e))
}

/// Reads the track whose manifest is at `manifest`.
///
/// # Errors
///
/// As [`from_parts`], and any I/O failure.
pub fn read(manifest: &Path) -> Result<Loaded, Error> {
    let text = std::fs::read_to_string(manifest).map_err(|e| Error::io(manifest, &e))?;
    let parsed = manifest::read(&text)?;
    let glb_path = manifest
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(&parsed.glb);
    let bytes = std::fs::read(&glb_path).map_err(|e| Error::io(&glb_path, &e))?;
    glb::read(parsed, &bytes)
}

fn glb_path_beside(manifest: &Path) -> PathBuf {
    let stem = manifest
        .file_name()
        .and_then(|n| n.to_str())
        .map_or("track", |n| n.strip_suffix(".track.toml").unwrap_or(n));
    manifest.with_file_name(format!("{stem}.glb"))
}
