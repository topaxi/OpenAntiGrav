//! The mode icon: the Bink clip the original plays in the panel right of the
//! progression bar.
//!
//! `LoadingScreen_Draw` binds a video widget's texture into a quad there
//! (`docs/ghidra/functions/ps3-hdfury-eu/loading-screen.md`). Which clip a mode
//! gets is [`oag_title::loading::FeatureStyle::mode_icons`]; this file reads
//! that row against the race's mode and opens the clip through the movie
//! cache, the one path every other Bink file takes.
//!
//! **A mode with no row, a clip that will not open and a source with no
//! `ffmpeg` all draw nothing**, never a stand-in: the panel stays its label and
//! brackets, and the loader report says why.

use std::path::Path;

use anyhow::Result;

use super::assets::{Assets, ModeIcon};
use crate::movie::Movie;
use crate::render::VideoFormat;

/// The clip for a race in `mode`, when the title's chosen style maps one.
#[must_use]
pub fn clip(assets: &Assets, mode: Option<u32>) -> Option<&ModeIcon> {
    let mode = mode?;
    assets.mode_icons.iter().find(|icon| icon.mode == mode)
}

/// The plane geometry the clip's frames need, from its own header.
#[must_use]
pub fn format(clip: &ModeIcon) -> Option<VideoFormat> {
    crate::clip::format(&clip.blob)
}

/// Opens the clip through the movie cache, transcoding on a miss.
///
/// Blocking: a caller on the frame thread must not call it.
///
/// # Errors
///
/// A clip the cache cannot make frames of, `ffmpeg` missing included.
pub fn open(clip: &ModeIcon, cache: &Path) -> Result<Movie> {
    crate::clip::open(&clip.entry, &clip.blob, cache)
}
