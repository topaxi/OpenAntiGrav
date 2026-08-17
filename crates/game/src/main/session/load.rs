//! Getting a run under way: the prefetch worker, the loading screen's own
//! progress, and the handoff into a race.

use anyhow::Result;

use oag_game::frontend::{self};
use oag_game::render::VideoFormat;
use oag_game::{boot, loading, movie, prefetch, race};

use crate::hints::RACE_TITLE;
use crate::stage::Stage;

use super::{BackdropShape, Session};

impl Session {
    /// Starts `--prefetch`, now that the boot's own movies are out of the cache.
    ///
    /// **Not before.** Both convert through `ffmpeg` into the same directory,
    /// and the boot's two reels are on the worker's list too - two processes
    /// writing one file is a corrupt cache, not a race that resolves. The
    /// ordering used to come free from `boot::load` being blocking; now that
    /// the movies run on a thread, it is this call that keeps it. Taking the
    /// options is what makes it once-only.
    pub(crate) fn start_prefetch(&mut self) {
        let Some(options) = self.prefetch_pending.take() else {
            return;
        };
        self.prefetch = Some(prefetch::Prefetch::spawn(options));
    }

    /// How far the conversion has got, right now.
    ///
    /// A run with no worker reads as finished rather than as
    /// [`prefetch::Progress::default`], which would be "nothing done of
    /// nothing" - true, but it is `finished: false`, and a loading screen shown
    /// that would never freeze or hand the window on.
    ///
    /// **A worker that has been asked for and not yet started reads as
    /// unfinished**, which is the one case the sentence above does not cover:
    /// between the window opening and [`Self::start_prefetch`] there is no
    /// handle, and reading that as "nothing to wait for" would fade the loading
    /// screen out a moment before the conversion it exists for even began.
    fn prefetch_progress(&self) -> prefetch::Progress {
        if self.prefetch_pending.is_some() {
            return prefetch::Progress::default();
        }
        self.prefetch.as_ref().map_or(
            prefetch::Progress {
                finished: true,
                ..prefetch::Progress::default()
            },
            prefetch::Prefetch::progress,
        )
    }

    /// The one snapshot the loading screen draws, over both of its waits.
    ///
    /// The screen counts whatever is converting *now*, and the two things that
    /// convert never overlap: `--prefetch` is not started until the boot's own
    /// movies are done with the cache (see the call to
    /// [`Self::start_prefetch`]), so this is a hand-off rather than a sum. While
    /// the media phase runs its counts are the ones shown; after it, the
    /// prefetch worker's own are, unchanged.
    ///
    /// **`finished` is never the media phase's to answer.** It means "the
    /// prefetch worker has nothing left to do", and the fade is gated on it
    /// *and* [`LoadingStage::media_ready`] separately - folding the two here
    /// would let a boot with no prefetch fade out while a movie was still
    /// decoding.
    pub(crate) fn loading_progress(&self) -> (loading::Phase, prefetch::Progress) {
        let progress = self.prefetch_progress();
        let Stage::Loading(stage) = &self.stage else {
            return (loading::Phase::Prefetch, progress);
        };
        let Some(media) = stage.media.as_ref().filter(|m| !m.is_finished()) else {
            return (loading::Phase::Prefetch, progress);
        };
        let media = media.progress();
        // Every field is stated rather than taken from the prefetch snapshot:
        // while the media phase runs, none of that worker's figures describe
        // what is on screen, and `..progress` would let a later field quietly
        // arrive from the wrong phase.
        (
            loading::Phase::Media(media.step),
            prefetch::Progress {
                total: media.total,
                done: media.done,
                current: media.current,
                // Not the media phase's to report: nothing here is skipped for
                // being cached and nothing here fails - a movie that will not
                // load is a `None` and a report line, and the phase carries on.
                cached: 0,
                failed: 0,
                planning: false,
                // Never this phase's to answer either. `finished` means the
                // prefetch worker has nothing left to do, and the fade is gated
                // on it *and* `LoadingStage::media_ready` separately; a run with
                // no `--prefetch` reports `true` here from the moment it starts,
                // and letting that through would head the screen "READY" over a
                // movie that is still decoding.
                finished: false,
            },
        )
    }

    /// Hands the window to the front end once the loading screen's fade is out.
    ///
    /// **Also where the boot's two halves become one.** The movies have landed
    /// by now - the fade does not start until they have, see
    /// [`LoadingStage::media_ready`] - so joining here never actually waits,
    /// and `boot::assemble` is the tail of the load that could not run until
    /// the reels had been measured.
    ///
    /// Taking the `Shell` is what makes this idempotent: a second call finds
    /// `None` and does nothing, so a failed `Stage::frontend` cannot be retried
    /// once a frame for the rest of the run.
    pub(crate) fn finish_loading(&mut self) -> Result<()> {
        let (shell, mut media, trace) = match &mut self.stage {
            Stage::Loading(stage) if stage.screen.is_done() => {
                let Some(shell) = stage.shell.take() else {
                    return Ok(());
                };
                (shell, stage.media.take(), stage.trace)
            }
            _ => return Ok(()),
        };
        let media = media
            .as_mut()
            .map(boot::MediaWorker::join)
            .unwrap_or_default();
        for line in &media.report {
            println!("{line}");
        }
        let mut loaded = boot::assemble(shell, media);
        if self.boot_overlay {
            loaded.frontend.set_overlay(true);
        }
        // A language chosen on an earlier run skips the picker. Reported either
        // way: silently not asking is indistinguishable from a broken picker,
        // and silently asking again is indistinguishable from a setting that
        // did not save. Here rather than before the window, which is where it
        // used to be: the sequence this asks does not exist until the two
        // halves have met.
        match (self.pick_language, self.settings.language.as_deref()) {
            (false, Some(name)) if loaded.frontend.preselect_language(name) => {
                println!("language {name} from settings, skipping the picker");
            }
            (false, Some(name)) => {
                eprintln!("this source does not offer {name:?}, so the picker is shown");
            }
            _ => {}
        }
        for line in &loaded.report {
            println!("{line}");
        }
        // A source with no intro reel at all - which is every PS2 source, whose
        // intro is an MPEG-2 program stream outside the archives - has no video
        // format either, and the front end draws without one.
        let video_format = loaded.movie.as_ref().and_then(VideoFormat::of);

        // Taken out of the boot before the front end takes the rest: it belongs
        // to the menus, which outlive the sequence that loaded it.
        //
        // Split in two here, and this is the last place both halves are in one
        // hand: the frames move onto a decode thread and the presentation - the
        // rate, the rectangle - stays behind, because a `Feed` deals in pixels
        // and knows nothing about where they go. `repeat: true`, which is the
        // whole difference between this movie and the intro.
        //
        // **The menu's grid, which is ours and is the PSP's - not the
        // source's.** This rect is drawn by `MenuStage`'s own renderer, and
        // `open_menus` builds that one fresh and never calls `set_space`, so
        // its `screen` uniform is `Space::PSP` whatever disc is mounted. The
        // menu layout it sits behind is this project's own, authored at
        // 480x272, which is why `capture.rs` pins `Space::PSP` on the same
        // picture.
        //
        // Handing this the *source's* space instead was a regression: on a PS2
        // disc it built the rect in a 640x448 grid for a shader normalising
        // against 480x272, and `pillarbox_in` always fills one axis of the grid
        // it is given, so the backdrop overflowed the screen on every aspect.
        // Silent on the PSP, where the two grids are the same numbers, and
        // invisible to `--menu-page`, which goes through `capture.rs`.
        let space = frontend::Space::PSP;
        (self.backdrop, self.backdrop_shape) = match loaded.backdrop.take() {
            Some(movie) => {
                let shape = BackdropShape {
                    frame_rate: movie.frame_rate,
                    // Pillarboxed rather than stretched, because the PS2's cut is
                    // not the PSP's shape: an `.IPF` declares its own display
                    // aspect. The PSP's `.PMF` is already 480x272, so this is the
                    // full screen there and changes nothing.
                    rect: frontend::pillarbox_in(space, movie.display_aspect),
                };
                let (width, height) = (movie.width, movie.height);
                // No frames is no backdrop, and then there is no shape to keep
                // either: the two are `Some` and `None` together everywhere below.
                match movie.frames {
                    Some(frames) => (
                        Some(movie::Feed::spawn(frames, true, width, height)),
                        Some(shape),
                    ),
                    None => (None, None),
                }
            }
            None => (None, None),
        };
        // Building the front end spawns the intro's decode thread and uploads a
        // sprite sheet; that is a load, and a load is not a frame time.
        self.stalled = true;
        self.stage = Stage::frontend(&self.gpu, loaded, video_format, trace, &mut self.audio)?;
        Ok(())
    }

    /// Replaces whatever is on screen with the race the menus ask for.
    ///
    /// The window, the device and the surface are the ones already open, so the
    /// handoff costs a load and not a second window.
    pub(crate) fn launch_race(&mut self) -> Result<()> {
        self.stalled = true;
        let loaded = race::load(&self.race_options)?;
        for line in &loaded.report {
            println!("{line}");
        }
        self.stage = Stage::race(
            &self.gpu,
            loaded,
            self.framebuffer.size(),
            self.anisotropy,
            &self.settings,
            self.scheme,
            self.autopilot,
        )?;
        // After the stage swap succeeds, not before: both loads above can fail
        // with `?`, and a failed launch must leave the menu music playing
        // rather than having already silenced it. See `Audio::start_race_music`.
        self.audio.start_race_music(
            &self.music_discs,
            self.settings.audio.music_source,
            &boot::default_audio_cache_dir(),
        );
        self.gpu.window.set_title(RACE_TITLE);
        Ok(())
    }
}
