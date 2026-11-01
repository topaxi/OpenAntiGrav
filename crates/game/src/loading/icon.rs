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

use anyhow::{Result, anyhow};

use super::assets::{Assets, ModeIcon};
use crate::movie::{self, Decode, Extent, Movie};
use crate::render::VideoFormat;

/// The clip for a race in `mode`, when the title's chosen style maps one.
#[must_use]
pub fn clip(assets: &Assets, mode: Option<u32>) -> Option<&ModeIcon> {
    let mode = mode?;
    assets.mode_icons.iter().find(|icon| icon.mode == mode)
}

/// The plane geometry the clip's frames need, from its own header.
///
/// Read before the clip is decoded because the renderer's video pipeline is
/// sized when it is built, and the decode runs on a thread after that.
#[must_use]
pub fn format(clip: &ModeIcon) -> Option<VideoFormat> {
    let header = oag_video::bik::parse(&clip.blob).ok()?;
    Some(VideoFormat {
        width: header.width,
        height: header.height,
        chroma_width: header.width.div_ceil(2),
        chroma_height: header.height.div_ceil(2),
    })
}

/// Opens the clip through the movie cache, transcoding on a miss.
///
/// Blocking: a caller on the frame thread must not call it.
///
/// # Errors
///
/// A clip the cache cannot make frames of, `ffmpeg` missing included.
pub fn open(clip: &ModeIcon, cache: &Path) -> Result<Movie> {
    let key = format!(
        "{}-{}",
        clip.entry.replace(['\\', '/'], "_").to_lowercase(),
        clip.blob.len()
    );
    let movie = movie::open(
        &clip.blob,
        &key,
        cache,
        Extent::Whole,
        Decode {
            // The AV1 cache, as the prefetch builds it: a platform decoder
            // would leave no file for the next run to find.
            prefer_cache: true,
            ..Decode::default()
        },
        None,
    )?;
    if movie.frames.is_none() {
        return Err(anyhow!(
            "{}: no picture: {}",
            clip.entry,
            movie.no_picture_reason.as_deref().unwrap_or("unknown")
        ));
    }
    Ok(movie)
}
