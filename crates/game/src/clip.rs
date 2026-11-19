//! A short looping Bink clip a screen plays in a panel of its own: the race
//! loading screen's mode icon and HD's Track Select fly-by.
//!
//! One opener for both: [`open`] runs the clip through the movie cache, the one
//! path every other Bink file takes, and [`Player`] does it on a thread of its
//! own and pumps the decoded frames to whoever draws them.
//!
//! **A clip that will not open and a source with no `ffmpeg` both draw
//! nothing**, never a stand-in, and the log says why.

use std::path::Path;

use anyhow::{Result, anyhow};

use crate::movie::{self, Decode, Extent, Feed, Movie, VideoFrame};
use crate::render::VideoFormat;
use oag_thread::Task;

/// The plane geometry `blob`'s frames need, from its own Bink header.
///
/// Read before the clip is decoded because a renderer's video pipeline is sized
/// when it is built, and the decode runs on a thread after that.
#[must_use]
pub fn format(blob: &[u8]) -> Option<VideoFormat> {
    let header = oag_video::bik::parse(blob).ok()?;
    Some(VideoFormat {
        width: header.width,
        height: header.height,
        chroma_width: header.width.div_ceil(2),
        chroma_height: header.height.div_ceil(2),
    })
}

/// The movie-cache key of the clip `entry` (its archive path) whose bytes are
/// `len` long: the one spelling [`open`] and [`crate::prefetch`] both use, so a
/// clip the prefetch converted is the file the screen finds.
#[must_use]
pub fn cache_key(entry: &str, len: usize) -> String {
    format!("{}-{len}", entry.replace(['\\', '/'], "_").to_lowercase())
}

/// Opens the clip `entry` (its archive path, whose `blob` it is) through the
/// movie cache, transcoding on a miss.
///
/// Blocking: a caller on the frame thread must not call it.
///
/// # Errors
///
/// A clip the cache cannot make frames of, `ffmpeg` missing included.
pub fn open(entry: &str, blob: &[u8], cache: &Path) -> Result<Movie> {
    open_refreshing(entry, blob, cache, false)
}

/// [`open`], converting again even when the cache holds the clip when `refresh`
/// is set (`--refresh-video`).
///
/// # Errors
///
/// As [`open`].
pub fn open_refreshing(entry: &str, blob: &[u8], cache: &Path, refresh: bool) -> Result<Movie> {
    let key = cache_key(entry, blob.len());
    let movie = movie::open(
        blob,
        &key,
        cache,
        Extent::Whole,
        Decode {
            // The AV1 cache, as the prefetch builds it: a platform decoder
            // would leave no file for the next run to find.
            prefer_cache: true,
            refresh,
            ..Decode::default()
        },
        None,
    )?;
    if movie.frames.is_none() {
        return Err(anyhow!(
            "{entry}: no picture: {}",
            movie.no_picture_reason.as_deref().unwrap_or("unknown")
        ));
    }
    Ok(movie)
}

/// A clip being opened on a thread, then playing.
pub(crate) struct Player {
    /// What the log calls this clip's screen.
    label: &'static str,
    opening: Option<Task<Result<Playing, String>>>,
    playing: Option<Playing>,
    /// Whether a frame of the clip has reached the caller's planes.
    shown: bool,
}

struct Playing {
    feed: Feed,
    format: VideoFormat,
    frame_rate: (u64, u64),
}

impl Player {
    /// Starts opening a clip: `open` runs on a thread of its own, and its
    /// frames loop for as long as they are asked for.
    pub(crate) fn spawn(
        label: &'static str,
        open: impl FnOnce() -> Result<Movie> + Send + 'static,
    ) -> Self {
        let opening = Task::spawn(label, move || {
            let movie = open().map_err(|e| format!("{e:#}"))?;
            let format = VideoFormat::of(&movie).ok_or("no frames")?;
            let (width, height, frame_rate) = (movie.width, movie.height, movie.frame_rate);
            let frames = movie.frames.ok_or("no frames")?;
            Ok(Playing {
                feed: Feed::spawn(frames, true, width, height),
                format,
                frame_rate,
            })
        });
        Self {
            label,
            opening: Some(opening),
            playing: None,
            shown: false,
        }
    }

    /// Takes the opened clip, if it has finished opening, and says so in the
    /// log when it could not be.
    fn poll(&mut self) {
        if !self.opening.as_ref().is_some_and(Task::is_finished) {
            return;
        }
        match self.opening.take().and_then(|mut task| task.join()) {
            Some(Ok(Ok(playing))) => self.playing = Some(playing),
            Some(Ok(Err(why))) => log::warn!("{}: {why}", self.label),
            _ => log::warn!("{}: its thread died", self.label),
        }
    }

    /// The clip's plane geometry and frame rate once it has opened.
    pub(crate) fn shape(&mut self) -> Option<(VideoFormat, (u64, u64))> {
        self.poll();
        self.playing
            .as_ref()
            .map(|playing| (playing.format, playing.frame_rate))
    }

    /// Hands `upload` the frame due at `position` (counting every loop), and
    /// returns whether the caller's planes hold a picture to draw.
    pub(crate) fn pump(
        &mut self,
        position: u64,
        upload: impl FnOnce(&VideoFrame) -> Result<()>,
    ) -> bool {
        self.poll();
        let Some(playing) = self.playing.as_mut() else {
            return false;
        };
        if let Some(why) = playing.feed.take_error() {
            log::warn!("{}: decoding: {why}", self.label);
            self.playing = None;
            self.shown = false;
            return false;
        }
        if let Some(frame) = playing.feed.take_upto(position) {
            match upload(&frame.picture) {
                Ok(()) => self.shown = true,
                Err(why) => log::warn!("{}: {why:#}", self.label),
            }
        }
        self.shown
    }
}
