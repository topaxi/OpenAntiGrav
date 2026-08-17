//! The front end: the disc's own boot sequence, movies and all.

use anyhow::{Result, bail};

use oag_game::frontend::{self, Frontend};
use oag_game::render::Renderer;
use oag_game::{at3, audio, movie};

use crate::gpu::Gpu;

pub(crate) struct FrontendStage {
    pub(crate) renderer: Renderer,
    pub(crate) frontend: Frontend,
    /// The intro, decoded on a worker thread. `None` on a source with no movie,
    /// under `--no-video`, and with no `ffmpeg` - and then the renderer was built
    /// with no video pipeline either, so the draw is skipped rather than green.
    pub(crate) feed: Option<movie::Feed>,
    /// Every movie still waiting for the screen that plays it.
    ///
    /// Held rather than started up front because they share one voice:
    /// [`crate::audio::Audio::start_movie`] stops whatever was playing, so
    /// starting two at load would mean the second silenced the first. Each is
    /// installed on the tick its own screen is entered - see
    /// [`FrontendStage::install_movie`].
    pub(crate) pending: Vec<PendingMovie>,
    /// Whether a picture has ever reached the planes. See
    /// [`FrontendStage::sync_video`].
    pub(crate) shown: bool,
    /// The same, for the backdrop `Show Logo` sits on. Kept apart from `shown`
    /// because the two movies fill the planes at different points in the
    /// sequence, and either can be the one that has not arrived yet.
    pub(crate) backdrop_shown: bool,
    /// The newest backdrop picture taken from the feed, **kept after it has
    /// been used** rather than dropped.
    ///
    /// [`movie::Feed::take_upto`] pops: a frame handed over is gone from the
    /// ring, and asking again for the same position returns `None`. The pump
    /// below runs on every frame whether or not the backdrop is on screen (see
    /// [`FrontendStage::sync_video`]), so without this the frame it popped on a
    /// frame that drew the intro was thrown away, and the first frame that
    /// wanted to *draw* the backdrop found the ring already past it - one to
    /// three frames of `Show Logo` over black before the playhead reached the
    /// next decoded frame. Holding it costs one 480x272 picture and makes the
    /// handoff seamless, both into `Show Logo` and on into the menus, which take
    /// it through [`FrontendStage::held_backdrop`].
    pub(crate) held_backdrop: Option<HeldFrame>,
    /// Whether [`FrontendStage::held_backdrop`] is what the planes hold now.
    ///
    /// One set of planes serves both movies, so an intro upload displaces the
    /// backdrop and the next backdrop draw has to put it back even though no
    /// new frame arrived. Without this the flag would say "uploaded" about a
    /// picture the intro had since overwritten.
    pub(crate) backdrop_in_planes: bool,
    pub(crate) trace: bool,
}

/// One boot movie, waiting for the screen that plays it.
pub(crate) struct PendingMovie {
    /// The screen this movie belongs to, from the title's own chain.
    pub(crate) state: &'static str,
    /// Its frames, already decoding on a worker thread. `None` under
    /// `--no-video`, with no `ffmpeg`, or when the movie could not be read.
    pub(crate) feed: Option<movie::Feed>,
    /// Its own track. `None` whenever the movie should be silent.
    pub(crate) sound: Option<at3::Pcm>,
}

/// A decoded backdrop picture kept past the moment it was taken from the feed.
///
/// The index rather than the position, because that is what a draw list reports
/// and what [`Backdrop::shown`] holds.
pub(crate) struct HeldFrame {
    pub(crate) index: usize,
    pub(crate) picture: movie::VideoFrame,
}

impl FrontendStage {
    /// Puts `state`'s own movie on screen and in the mixer, if it has one.
    ///
    /// One set of I420 planes serves every movie the front end draws, and one
    /// voice serves every movie's sound, so this is a **handover** rather than an
    /// addition: the feed is replaced and `start_movie` stops whatever was
    /// sounding. The arrived-a-picture-yet bookkeeping resets with it, or the new
    /// movie's first frame would either show the previous movie's last picture or
    /// be skipped as already shown.
    ///
    /// Idempotent by construction - the entry is taken out of `pending` - so a
    /// state re-entered later does not restart its movie.
    pub(crate) fn install_movie(&mut self, state: &str, audio: &mut audio::Audio) {
        let Some(at) = self.pending.iter().position(|movie| movie.state == state) else {
            return;
        };
        let PendingMovie { feed, sound, .. } = self.pending.remove(at);
        self.feed = feed;
        self.shown = false;
        self.backdrop_in_planes = false;
        audio.start_boot_movie(state, sound);
    }

    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        backdrop: Option<&mut movie::Feed>,
    ) -> Result<()> {
        let mut list = self.frontend.draw_list();
        self.sync_video(&gpu.queue, &mut list, backdrop)?;
        // The disc's own screens, not our menus: nothing here ever marquees.
        self.renderer.render(
            &gpu.device,
            &gpu.queue,
            encoder,
            view,
            &list,
            viewport,
            None,
        );
        Ok(())
    }

    /// What is decoding the intro, for the `dev` performance overlay. `None`
    /// on a source with no movie, under `--no-video`, or with no `ffmpeg` -
    /// see [`FrontendStage::feed`].
    pub(crate) fn video_label(&self) -> Option<&'static str> {
        self.feed.as_ref().map(movie::Feed::decoder_label)
    }

    /// Uploads the newest decoded frame at or before the one the draw list asks
    /// for.
    ///
    /// **This is the older of the two blocking decodes and it has always done
    /// it**: the intro called `FrameStore::read_frame` straight from the render
    /// thread from the day it was written, and the menu backdrop only made the
    /// cost measurable. Both go through a [`movie::Feed`] now - see
    /// [`MenuStage::render`] for the numbers and
    /// [ADR-0010](../../docs/architecture/adr/0010-movie-decode-thread.md) for
    /// the reasoning.
    ///
    /// The intro does not loop, so its position and its frame index are the same
    /// number - clamped to the cache, because `--movie-frames` can make the cache
    /// shorter than the sequence and the last frame then holds for the rest of it.
    ///
    /// `list` is trimmed rather than only read: until a picture has reached the
    /// planes there is nothing to draw, and drawing the quad anyway would put
    /// green over the screen rather than black, zeroed I420 being green. This
    /// costs the first frame or two of a 40-second movie that fades up from black
    /// anyway.
    /// **Two movies reach this, and which one is named by the draw rather than
    /// inferred.** `Show Logo` sits on the looping backdrop, so the sequence's
    /// last screen asks for a different file than its first one does; taking the
    /// frame from the intro's feed instead would draw a picture rather than fail,
    /// which is the trap `crate::capture` already documents. The two share one
    /// set of planes, which is safe only because `boot::load` declines to set the
    /// backdrop at all unless its geometry matches the intro's.
    ///
    /// # The backdrop's feed is pumped on every frame, drawn or not
    ///
    /// The backdrop's playhead runs from the moment the sequence starts - the
    /// movie is `autostart` on `FE Screen`, which the boot opens long before
    /// `Show Logo` draws it - but only `Show Logo` puts it on screen. A feed
    /// decodes a fixed few frames ahead and then parks, so a feed nobody takes
    /// from sits four frames from the start while the playhead is
    /// hundreds of frames on; the moment `Show Logo` appeared it would then rush
    /// through the whole movie at decode speed catching up.
    ///
    /// So the frame at the playhead is taken every frame whatever is on screen,
    /// and only *uploaded* when the backdrop is the movie being drawn. That
    /// keeps the one feed and the one playhead in step from boot to the menus,
    /// which is what makes handing the playhead over at `Launch Game`
    /// ([`frontend::Frontend::take_backdrop`]) a continuation rather than a
    /// second playback. It costs the worker thread decoding a 480x272 movie at
    /// 30 Hz during the intro - which is what the hardware is doing at that
    /// moment too.
    fn sync_video(
        &mut self,
        queue: &wgpu::Queue,
        list: &mut Vec<frontend::Draw>,
        backdrop: Option<&mut movie::Feed>,
    ) -> Result<()> {
        // `find_map` is only correct because a list carries at most one video,
        // which `Frontend::insert_backdrop` is what guarantees - and the
        // renderer would take the *last* one rather than this first one, so the
        // two would disagree if that ever stopped holding. Asserted rather than
        // handled: a second video needs a second plane set and a second bind
        // group, which is a renderer feature and not something to paper over
        // here.
        debug_assert!(
            list.iter()
                .filter(|draw| matches!(draw, frontend::Draw::Video { .. }))
                .count()
                <= 1,
            "a draw list carries at most one Draw::Video; the renderer draws one \
             video quad from one plane set"
        );
        let drawn = list.iter().find_map(|draw| match draw {
            frontend::Draw::Video {
                position, source, ..
            } => Some((*position, *source)),
            _ => None,
        });
        // Copied out before the renderer is touched, both being fields of
        // `self`, and because the pump below runs on frames whose draw list
        // names no movie at all.
        let playhead = self.frontend.backdrop().map(movie::Player::position);

        let has_backdrop = backdrop.is_some();
        if let (Some(feed), Some(position)) = (backdrop, playhead) {
            if let Some(reason) = feed.take_error() {
                bail!("decoding the menu backdrop: {reason}");
            }
            // Kept rather than used-or-dropped: the take is a pop, and the
            // frame popped on a frame that draws the intro is the one
            // `Show Logo` asks for a moment later. See
            // [`FrontendStage::held_backdrop`].
            if let Some(frame) = feed.take_upto(position) {
                self.held_backdrop = Some(HeldFrame {
                    index: frame.index,
                    picture: frame.picture,
                });
                self.backdrop_in_planes = false;
            }
            // Uploaded only when the backdrop is the movie on screen - one
            // renderer, one set of planes, and the intro is in them until
            // `Show Logo` - and only when the planes do not already hold it, so
            // the steady state is the same one upload per decoded frame it
            // always was.
            if matches!(drawn, Some((_, frontend::Video::Backdrop)))
                && !self.backdrop_in_planes
                && let Some(held) = &self.held_backdrop
            {
                self.renderer.upload_frame(queue, &held.picture)?;
                self.backdrop_in_planes = true;
                self.backdrop_shown = true;
            }
        }

        // The intro, only on the frames that draw it. Unlike the backdrop it
        // does not loop and nothing carries it past the sequence, so a feed left
        // parked while another screen is up has nothing to fall behind.
        if let Some((position, frontend::Video::Intro)) = drawn
            && let Some(feed) = self.feed.as_mut()
        {
            if let Some(reason) = feed.take_error() {
                bail!("decoding the intro movie: {reason}");
            }
            // `saturating_sub`, so an empty cache is a movie with no picture
            // rather than a panic on `0 - 1`. The cache can be shorter than the
            // sequence - `--movie-frames` - and then the last frame holds.
            let position = position.min(feed.len().saturating_sub(1) as u64);
            if let Some(frame) = feed.take_upto(position) {
                self.renderer.upload_frame(queue, &frame.picture)?;
                self.shown = true;
                // The intro has displaced whatever backdrop picture was in the
                // planes, so the next backdrop draw has to upload again.
                self.backdrop_in_planes = false;
            }
        }

        // A quad with nothing in its planes is a green rectangle, zeroed I420
        // not being black - so a movie with no feed at all (a source without a
        // backdrop, `--no-video`, no `ffmpeg`) and one whose first picture has
        // not arrived yet are both dropped from the list rather than drawn.
        let ready = match drawn {
            Some((_, frontend::Video::Intro)) => self.feed.is_some() && self.shown,
            Some((_, frontend::Video::Backdrop)) => has_backdrop && self.backdrop_shown,
            None => return Ok(()),
        };
        if !ready {
            list.retain(|draw| !matches!(draw, frontend::Draw::Video { .. }));
        }
        Ok(())
    }
}
