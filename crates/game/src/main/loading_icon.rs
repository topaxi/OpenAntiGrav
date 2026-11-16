//! The race loading screen's mode icon clip, opened and played off the frame
//! thread.
//!
//! The clip is opened through the movie cache, which on a cold cache runs an
//! `ffmpeg` transcode, so [`Icon::spawn`] does that on a thread of its own and
//! the stage polls it each frame. Until the first decoded frame is in the
//! renderer's planes the screen draws no video (`loading::Screen::set_mode_icon`),
//! and a clip that fails says so in the log and draws nothing, never a stand-in.

use oag_game::movie::Feed;
use oag_game::render::Renderer;
use oag_game::{loading, movie};
use oag_thread::Task;

/// The clip being opened, then playing.
pub(crate) struct Icon {
    opening: Option<Task<Result<Feed, String>>>,
    feed: Option<Feed>,
    /// Whether a frame of the clip is in the renderer's planes.
    shown: bool,
}

impl Icon {
    /// Starts opening `clip`. The caller has already built its renderer with
    /// [`loading::icon::format`]'s planes, which is why it hands the clip in
    /// whole.
    pub(crate) fn spawn(clip: loading::ModeIcon, cache: std::path::PathBuf) -> Self {
        let opening = Task::spawn("loading-mode-icon", move || {
            let movie = loading::icon::open(&clip, &cache).map_err(|e| format!("{e:#}"))?;
            let (width, height) = (movie.width, movie.height);
            let frames = movie
                .frames
                .ok_or_else(|| format!("{}: no frames", clip.entry))?;
            // Looped: the original's panel plays the clip over and over for as
            // long as the load takes (two RPCS3 grabs 2 s apart are on
            // different beats of it).
            Ok(movie::Feed::spawn(frames, true, width, height))
        });
        Self {
            opening: Some(opening),
            feed: None,
            shown: false,
        }
    }

    /// Takes the opened clip, uploads the frame due at `position`, and returns
    /// whether the planes hold a picture to draw.
    pub(crate) fn pump(&mut self, renderer: &Renderer, queue: &wgpu::Queue, position: u64) -> bool {
        if self.opening.as_ref().is_some_and(Task::is_finished) {
            match self.opening.take().and_then(|mut task| task.join()) {
                Some(Ok(Ok(feed))) => self.feed = Some(feed),
                Some(Ok(Err(why))) => log::warn!("loading screen mode icon: {why}"),
                _ => log::warn!("loading screen mode icon: its thread died"),
            }
        }
        let Some(feed) = self.feed.as_mut() else {
            return false;
        };
        if let Some(why) = feed.take_error() {
            log::warn!("loading screen mode icon: decoding: {why}");
            self.feed = None;
            self.shown = false;
            return false;
        }
        if let Some(frame) = feed.take_upto(position) {
            match renderer.upload_frame(queue, &frame.picture) {
                Ok(()) => self.shown = true,
                Err(why) => log::warn!("loading screen mode icon: {why:#}"),
            }
        }
        self.shown
    }
}
