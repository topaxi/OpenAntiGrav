//! Getting a run under way: the prefetch worker, the loading screen's own
//! progress, and the handoff into a race.

use anyhow::Result;
use log::{debug, error, info, warn};

use oag_game::render::VideoFormat;
use oag_game::{boot, loading, movie, prefetch, records};
use oag_raceplay as race;
use oag_ui::frontend::EarnedTier;
use oag_ui::strings;

use crate::hints;
use crate::loading_stage::RaceBuildError;
use crate::stage::Stage;

use super::menus::combine_variant;
use super::{BackdropShape, Session};

impl Session {
    /// Turns a picked disc image into the boot the window hands on to.
    ///
    /// The other half of `main`'s windowed route, run a few seconds later: the
    /// same [`crate::prepare::Pending::windowed`] call, and then the same
    /// [`Stage::loading`] the ordinary boot starts on. A launched boot and a
    /// named one are the same boot from here, which is the point of the split.
    ///
    /// **The `Pending` is dropped on success and kept on failure**, which is
    /// what lets a player whose first choice will not boot pick another: the
    /// chooser is still on screen and everything it needs is still here.
    /// [`Self::finish_loading`] takes its `Shell` up front for the opposite
    /// reason - there is nothing to go back to there.
    ///
    /// # Errors
    ///
    /// Propagates the load, which is a disc that will not open. Reporting it to
    /// the player rather than dying is the caller's business - see
    /// [`Self::frame`].
    pub(crate) fn finish_launcher(&mut self, source: &str) -> Result<()> {
        if self.pending.is_none() {
            return Ok(());
        }

        // **Dropped from the frame timer, not measured.** What follows opens
        // the archives, parses the front end and surveys every image on the
        // search path for the music - a second or so inside one frame. See the
        // `stalled` field, and the note in `Session::frame` about why a load is
        // not a frame time.
        self.stalled = true;
        info!("disc image: {source}");

        let prepared = {
            let Some(pending) = self.pending.as_ref() else {
                return Ok(());
            };
            match pending.windowed(source) {
                Ok(prepared) => prepared,
                // **Not necessarily this title's refusal.** A source that will
                // not open at all fails the same way, so
                // `finish_launcher_placeholder` re-opens it and checks the
                // title's own front end before doing anything with the pick -
                // and hands this error straight back when it is not the one
                // case this route exists for. See
                // `crate::session::placeholder`.
                Err(e) => return self.finish_launcher_placeholder(source, e),
            }
        };
        self.pending = None;
        self.race_options = Some(prepared.race_options);
        self.music_discs = prepared.music_discs;
        self.shell = Some(prepared.shell);
        self.prefetch_pending = prepared.prefetch;
        let assets = prepared.loading_assets;
        self.stage = Stage::loading(
            &self.gpu,
            prepared.boot_shell,
            Some(prepared.media),
            &assets,
            self.trace,
            self.settings.language.as_deref(),
        )?;
        // Kept rather than dropped with the stage: the same screen goes up
        // again for every race this run launches. See `Session::loading_assets`.
        self.loading_assets = assets;
        Ok(())
    }

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
        // **Answered before the prefetch snapshot is consulted at all.** A race
        // load happens long after the boot, so `--prefetch` has usually
        // finished and its `finished: true` would fade this screen out over a
        // circuit that is still being read.
        if let Some(worker) = stage.race.as_ref() {
            return (loading::Phase::Race, worker.progress().into());
        }
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
                load_stage: 0,
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
        // The race path first, and it returns rather than falling through: a
        // race-loading stage carries no `Shell`, so the `None` below would send
        // it back with the screen still up and the circuit already read.
        if matches!(&self.stage, Stage::Loading(stage) if stage.race.is_some()) {
            return self.finish_race_loading();
        }
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
        oag_raceplay::loader_log::lines(&media.report);
        let mut loaded = boot::assemble(shell, media);
        if self.boot_overlay {
            loaded.frontend.set_overlay(true);
        }
        // Wipeout 2048's own career, folded in once here rather than
        // threaded through `boot::load_shell`/`assemble`: this is the one
        // touch-front-end screen this build ever shows (see
        // `docs/formats/2048-frontend.md`'s "Wired" section - a race always
        // returns into this build's own menus, never back to `newFEshell`),
        // so "once at boot, from the save already loaded into
        // `self.records`" is the whole of what a player's progression needs.
        // A no-op on every title but 2048: `campaign_events()` is empty
        // everywhere else. See `oag_ui::frontend::campaign_map`'s own
        // "Progression" section.
        let title_name = loaded.title.name;
        loaded.frontend.refresh_campaign_progress(|name| {
            self.records
                .campaign_medal(title_name, name)
                .and_then(|row| row.best_medal)
                .map(|medal| match medal {
                    records::Medal::Gold => EarnedTier::Elite,
                    records::Medal::Silver | records::Medal::Bronze => EarnedTier::Pass,
                })
        });
        // What a launch off the map would fly before the player ever
        // touches `Team` this session - see `oag_ui::frontend::campaign_map`'s
        // own `CampaignMap::craft_seed` doc for why a restricted event's own
        // launch gate needs this at all. `combine_variant`'s own warning is
        // dropped rather than logged: an unrecognised `settings.race.team`
        // is the same "racing the first one it does offer" case the RACE
        // page already accepts silently at boot, and this is only ever a
        // fallback the player's own `Team` choice overrides the moment they
        // touch it.
        loaded.frontend.seed_craft(
            combine_variant(
                loaded.title,
                &self.settings.race.team,
                &self.settings.race.variant,
            )
            .0,
        );
        // A language chosen on an earlier run skips the picker. Reported either
        // way: silently not asking is indistinguishable from a broken picker,
        // and silently asking again is indistinguishable from a setting that
        // did not save. Here rather than before the window, which is where it
        // used to be: the sequence this asks does not exist until the two
        // halves have met.
        match (self.pick_language, self.settings.language.as_deref()) {
            (false, Some(name)) if loaded.frontend.preselect_language(name) => {
                debug!("language {name} from settings, skipping the picker");
            }
            (false, Some(name)) => {
                warn!("this source does not offer {name:?}, so the picker is shown");
            }
            // A fresh run with no settings language yet: on HD alone, this
            // screen never presents to a player, so it must not be the one
            // this build waits on - see
            // `Frontend::skip_never_shown_picker`.
            (false, None) if loaded.frontend.skip_never_shown_picker(loaded.title) => {
                debug!(
                    "HD's own language picker never presents to a player; defaulting to English"
                );
            }
            _ => {}
        }
        oag_raceplay::loader_log::lines(&loaded.report);
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
        // **The menu's grid, which is the source's.** This rect is drawn by
        // `MenuStage`'s own renderer, and what matters is only that the rect and
        // that renderer's `screen` uniform are in the same grid: `pillarbox_in`
        // always fills one axis of whatever it is given, so a rect built in one
        // grid and normalised against another overflows the screen on every
        // aspect.
        //
        // This was `Space::PSP` on both sides until the menus started drawing a
        // 1920x1080 source's skin at its own numbers. `open_menus` now calls
        // `set_space` with the same value read here, and `capture.rs` does the
        // same on the `--menu-page` path. Being wrong in the same direction in
        // all three places is what kept the PS2's 640x448 mismatch invisible for
        // as long as it was.
        let space = loaded.frontend.space();
        (self.backdrop, self.backdrop_shape) = match loaded.backdrop.take() {
            Some(movie) => {
                let shape = BackdropShape {
                    frame_rate: movie.frame_rate,
                    // Pillarboxed rather than stretched, because the PS2's cut is
                    // not the PSP's shape: an `.IPF` declares its own display
                    // aspect. The PSP's `.PMF` is already 480x272, so this is the
                    // full screen there and changes nothing.
                    rect: oag_display::space::pillarbox_in(space, movie.display_aspect),
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

    /// Reads this source's loading-screen assets again, for the styling now
    /// chosen.
    ///
    /// **The illustration is a decoded image**, so a styling that changed has to
    /// be fetched rather than switched to - see
    /// `oag_title::loading::FeatureStyle`, and `Session::apply_setting`, which
    /// is the only caller. A run with no source yet keeps what it has, which is
    /// the drawable default.
    ///
    /// Failures are already notes on the assets themselves and are printed the
    /// same way the boot prints them, so this cannot fail: the worst outcome is
    /// a screen with no illustration, which is what a title with no features
    /// draws anyway.
    pub(crate) fn reload_loading_assets(&mut self) {
        let Some(source) = self
            .race_options
            .as_ref()
            .map(|options| options.source.clone())
        else {
            return;
        };
        let Some(strings) = self.shell.as_ref().map(|shell| shell.strings.clone()) else {
            return;
        };
        let entries = self.shell.as_ref().and_then(|shell| shell.entries.clone());
        let assets = loading::Assets::load(
            &source,
            &strings,
            entries.as_deref(),
            crate::args::style_of(&self.settings),
        );
        oag_raceplay::loader_log::lines(&assets.notes);
        self.loading_assets = assets;
    }

    /// Puts the loading screen up and starts reading the circuit behind it.
    ///
    /// **This used to be the load itself**, on the frame thread: `self.stalled
    /// = true; let loaded = race::load(options)?;`. That call is 1.5 to 5
    /// seconds depending on the circuit and the disc, and for all of it the
    /// event loop was not pumping - so the window stopped answering the
    /// compositor and the last menu frame sat frozen on screen. The `stalled`
    /// flag was the whole of the mitigation, and all it does is stop the frame
    /// pacer counting the stall as a dropped frame.
    ///
    /// So the load goes to [`race::LoadWorker`] and the screen the original
    /// itself puts up goes over it. The hand-off to the grid is
    /// [`Self::finish_loading`], the same call that hands the boot's own screen
    /// to the front end.
    ///
    /// The window, the device and the surface are the ones already open, so
    /// this costs a thread and not a second window.
    ///
    /// # Errors
    ///
    /// Building the loading screen's own renderer, and - unreachably in
    /// practice - a run with no source yet: only the menus fire this, and there
    /// are no menus before a boot. **The circuit load's own failure is not
    /// here** any more; it arrives at [`Self::finish_loading`], where the
    /// player is put back in the menus rather than dropped out of the game.
    pub(crate) fn launch_race(&mut self) -> Result<()> {
        self.stalled = true;
        // The LAUNCH RACE row means a fresh race even when one is parked - see
        // `Session::suspended_race`, which `Session::resume_race` is the way
        // back to instead. Dropped here rather than left to sit until some
        // later resize or race end reaches for it.
        //
        // **Dropped on a thread of its own**, not here: releasing a whole
        // race scene's buffers, textures and pipelines measured 86 ms on the
        // frame thread in a release build on a desktop - the menu frame the
        // player just pressed LAUNCH RACE on, frozen. See
        // `docs/architecture/race-load-transition.md`.
        if let Some(parked) = self.suspended_race.take() {
            crate::race_build::drop_off_thread(parked);
        }
        let mut options = self
            .race_options
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no disc image has been chosen yet"))?;
        let options_mode = options.mode;
        // **Refreshed here, not trusted from whenever `race_options` was last
        // built or mutated** (boot, RACE REMIX, a campaign cell launch): the
        // OPTIONS page's LANGUAGE row writes `self.settings.language` without
        // touching `race_options` at all, and every launch path funnels
        // through this one function, so this is the one place a launch
        // cannot race a value picked-then-abandoned earlier in the session.
        // See `race::Options::language`'s own doc.
        options.language = self.settings.language.clone();
        // The RACE page's own row for this circuit, which is the disc's
        // localised name rather than the `PI_Track` id - see
        // `oag_raceplay::catalogue::label`. `None` on a run whose source offered
        // no circuit at all, which draws no line rather than an id.
        let label = self.shell.as_ref().and_then(|shell| {
            let entry = options.track.as_deref()?;
            shell
                .tracks
                .iter()
                .find(|(track, _)| track.entry_name() == entry)
                .map(|(_, name)| name.clone())
        });
        // Taken out of the outgoing `MenuStage` before it is dropped below -
        // not the playhead, which `menu_playhead` restarts deliberately on the
        // way back, but the picture it was last drawing, so `escape` does not
        // flash black waiting for the restarted feed's first frame. See
        // `Session::held_menu_backdrop`.
        self.held_menu_backdrop = match &mut self.stage {
            Stage::Menu(stage) => stage
                .backdrop
                .as_mut()
                .and_then(|backdrop| backdrop.held.take()),
            _ => None,
        };
        // Bumped before the screen is built, so the first race of a run draws
        // with 1 rather than with the same seed a capture uses.
        self.races_launched = self.races_launched.saturating_add(1);
        // The frame loop's own device: the scene is built from it, so each
        // texture goes up as it is decoded rather than waiting on the CPU for
        // every other one. See `race::TextureSink`.
        let sink = Some(race::TextureSink {
            device: self.gpu.device.clone(),
            queue: self.gpu.queue.clone(),
        });
        // A campaign event the front end asked for, or the menus' own race.
        let worker = match self.pending_event.take() {
            Some(event) => race::LoadWorker::spawn_event(options, event, label, sink),
            None => race::LoadWorker::spawn(options, label, sink),
        };
        // Spawned alongside the circuit read, for the same reason: fetching
        // this synchronously at the hand-off used to decode a full track on
        // the frame thread, measured at 2.83s for a cold PS2 one - the whole
        // of the gap that used to open up behind the loading screen's fade.
        // `reserve_race_music_index` settles which track before the worker
        // starts, so it and `Audio::finish_race_music` (called once this
        // stage's fade runs out) agree on one without either asking `discs`
        // twice. See `LoadingStage::music` and `oag_sound::MusicFetchWorker`.
        let music_index = self.audio.reserve_race_music_index(&self.music_discs);
        let music = oag_sound::MusicFetchWorker::spawn(
            self.music_discs.clone(),
            self.settings.audio.music_source,
            oag_source::cache::default_audio_cache_dir(),
            music_index,
            "race-music",
        );
        let shell = self.shell.clone().ok_or_else(|| {
            anyhow::anyhow!("this run has no menus, so it has no font to draw with")
        })?;
        self.stage = Stage::race_loading(
            &self.gpu,
            worker,
            music,
            &shell.font,
            &shell.sprites,
            &self.loading_assets,
            self.races_launched,
            crate::loading::executable_mode(options_mode),
            self.trace,
            self.settings.language.as_deref(),
        )?;
        Ok(())
    }

    /// Starts building the race scene as soon as the circuit's own load
    /// lands, and collects it once built - both without ever waiting.
    ///
    /// **Two fixes live here.** The first is the gap that used to sit behind
    /// the black screen: `LoadingStage::race_ready` used to mean only "the
    /// disc read is done", so the fade ran to zero opacity while the scene was
    /// still unbuilt, and only then did [`Self::finish_race_loading`] build it.
    /// `race_ready` now waits on [`LoadingStage::built_race`], so the fade
    /// cannot start until there is nothing left to build.
    ///
    /// The second is the freeze that fix left: the build itself - measured at
    /// 8.0 s for Pulse PSP in a release build, most of it wgpu turning WGSL
    /// into pipelines - ran here, inside one frame of the loading screen,
    /// which stopped the wave dead for the whole of it. It runs on a
    /// [`crate::race_build::BuildWorker`] now, and this only polls it, so the
    /// loading screen draws every frame until the race is ready. See
    /// `docs/architecture/race-load-transition.md`.
    ///
    /// A no-op off the loading stage and on the boot path, where there is no
    /// worker to poll, and once [`LoadingStage::built_race`] is `Some`.
    pub(crate) fn advance_race_build(&mut self) {
        // Read before the `Stage::Loading` borrow below, the same reason
        // `Session::frame` reads it before its own `match &mut self.stage`:
        // both take `&self` as a whole.
        let pvs_culling = self.pvs_culling();
        let render_profile = self.render_profile();
        let Stage::Loading(stage) = &mut self.stage else {
            return;
        };
        if stage.built_race.is_some() {
            return;
        }
        if let Some(build) = stage.build.as_mut() {
            let Some(built) = build.take() else {
                return;
            };
            stage.build = None;
            let mut result = built.stage;
            if let Ok(race_stage) = &mut result {
                // A resize that landed while the build ran: the scene was
                // sized for a framebuffer that no longer exists, and
                // `Session::draw`'s own resize only reaches a scene that was
                // already in `built_race` when it happened.
                let allocation = self.framebuffer.allocation();
                if built.allocation != allocation {
                    race_stage
                        .scene
                        .resize(&self.gpu.device, self.gpu.config.format, allocation);
                }
                // Drained here, not read: this is the one place a launch that
                // set `self.campaign_cell` (`Session::launch_campaign_cell`)
                // and a stage that is actually about to become live meet - see
                // `RaceStage::campaign_cell`'s own doc. `.take()` leaves
                // `self.campaign_cell` empty for whatever races next.
                race_stage.campaign_cell = self.campaign_cell.take();
                race_stage.campaign_difficulty = self.campaign_difficulty.take();
                race_stage.campaign_ai_skill_scale = self.campaign_ai_skill_scale.take();
                // Applied here, once, before the stage ever ticks - see
                // `Session::campaign_ai_skill_scale`'s own doc.
                // `oag_ai::Difficulty::tune_at_scale` is the continuous
                // generalisation of `Setup::difficulty`'s own discrete
                // `tune()`, which `race::Race::start` already ran once with
                // whatever ambient difficulty the RACE page had selected -
                // this overrides it with the campaign's own resolved
                // position rather than adding to it, the same "replaces,
                // not adds" shape `AI_ResolveSkillScale`'s own campaign
                // branch has.
                if let Some(scale) = race_stage.campaign_ai_skill_scale {
                    race_stage
                        .race
                        .set_ai_tuning(oag_ai::Difficulty::tune_at_scale(
                            scale,
                            &oag_ai::Tuning::default(),
                        ));
                    // The finished player's thrust reads the same scale:
                    // `AI_ResolveSkillScale` is its one source too.
                    race_stage.race.set_finished_thrust_skill(scale);
                }
            }
            if let Some(probe) = self.load_probe.as_mut() {
                probe.span("race-build thread: build_race_stage", built.build);
                probe.span("race-build thread: warm_up", built.warm_up);
            }
            stage.built_race = Some(result);
            // The reference point the other two diagnostic timestamps are read
            // against - see `Session::race_ready_at`.
            self.race_ready_at = Some(web_time::Instant::now());
            return;
        }
        let Some(worker) = stage.race.as_mut() else {
            return;
        };
        if !worker.is_finished() {
            return;
        }
        // Finished, so this join returns at once - see `LoadWorker::join`.
        let loaded = worker
            .join()
            .unwrap_or_else(|| Err(anyhow::anyhow!("the circuit's load thread would not start")));
        match loaded {
            Ok(loaded) => {
                oag_raceplay::loader_log::lines(&loaded.report);
                let request = crate::race_build::Request {
                    gpu: self.gpu.handles(),
                    allocation: self.framebuffer.allocation(),
                    // The extent: this is a viewport, not an attachment size.
                    extent: self.framebuffer.extent(),
                    warm_target: self.framebuffer.view().clone(),
                    anisotropy: self.anisotropy,
                    settings: self.settings.clone(),
                    render_profile,
                    scheme: self.scheme,
                    autopilot: self.autopilot,
                    autopilot_pilot: self.autopilot_pilot,
                    autopilot_skill: self.autopilot_skill,
                    // The same options this load itself was started from -
                    // see `Session::launch_race`, which clones this field into
                    // the worker rather than taking it.
                    track_entry: self
                        .race_options
                        .as_ref()
                        .and_then(|options| options.track.clone()),
                    pvs_culling,
                    anim_seconds: self.anim_seconds,
                };
                stage.build = Some(crate::race_build::BuildWorker::spawn(request, loaded));
            }
            Err(e) => {
                stage.built_race = Some(Err(RaceBuildError::Load(e)));
                self.race_ready_at = Some(web_time::Instant::now());
            }
        }
    }

    /// Swaps the loading screen for the grid, once the circuit has landed and
    /// the fade has run out.
    ///
    /// The race counterpart of the front-end hand-off in [`Self::finish_loading`],
    /// and called from it rather than beside it: both are "the loading screen is
    /// done, hand the window to whatever it was covering", and having one caller
    /// is what stops a future screen being handed on twice.
    ///
    /// **The heavy work is already done by now.** [`Self::advance_race_build`]
    /// built the scene as soon as the circuit landed, long before the fade
    /// finished counting down - see [`LoadingStage::built_race`] - so this is
    /// just the swap and the same audio hand-off it always was.
    ///
    /// # Errors
    ///
    /// **Not the circuit's own load.** That failed on the frame thread before
    /// this existed, where `launch_race`'s caller reported it and left the menus
    /// up; it fails on a worker thread now, and the recovery has to be here or a
    /// bad circuit would take the window down through
    /// [`Self::finish_loading`]'s caller. So a failed load is reported and the
    /// menus are reopened, which is the same thing escaping a race does.
    ///
    /// What does propagate is reopening those menus, and a GPU pipeline that
    /// would not build - not something to carry on from, exactly as before
    /// this moved to [`Self::advance_race_build`].
    fn finish_race_loading(&mut self) -> Result<()> {
        let Some((built, music)) = (match &mut self.stage {
            Stage::Loading(stage) if stage.screen.is_done() => stage
                .built_race
                .take()
                .map(|built| (built, stage.music.take())),
            _ => None,
        }) else {
            return Ok(());
        };
        let race_stage = match built {
            Ok(mut race_stage) => {
                // Begun here, at the hand-over, so no frame is drawn with the chase camera and
                // the HUD before the flyby's first tick. A race resumed from the menus
                // (`escape.rs`) is not a new one and does not come through here.
                if !self.no_intro {
                    race_stage.race.begin_intro();
                }
                race_stage
            }
            Err(RaceBuildError::Load(e)) => {
                error!("cannot start a race: {e:#}");
                return self.open_menus();
            }
            Err(RaceBuildError::Gpu(e)) => return Err(e),
        };
        // The second diagnostic timestamp - see `Session::race_ready_at`. Not
        // taken: `Session::frame` reads it again once the first race frame
        // presents.
        if let Some(ready_at) = self.race_ready_at {
            debug!(
                "race hand-off: {:?} since the scene was ready",
                ready_at.elapsed()
            );
        }
        self.stage = Stage::Race(race_stage);
        // Never inherited from whatever race was on screen before this one -
        // see `Session::paused`.
        self.paused = false;
        // The outgoing race's held voices, if this is a relaunch rather than a
        // first start. The back-out path does this too, but a race launched
        // straight from a race never passes through it - and a carried-over
        // `SfxVoices` holds a `VoiceId` into a pool the new race is about to
        // reuse. See `Audio::stop_race_sfx`.
        self.audio.stop_race_sfx();
        self.audio.enter_race_mix();
        // After the stage swap succeeds, not before: a failed launch must
        // leave the menu music playing rather than having already silenced
        // it. See `Audio::finish_race_music`.
        //
        // Timed as a diagnostic, kept from the round that found the decode
        // running here: `Audio::finish_race_music` only ever joins an
        // already-finished `MusicFetchWorker` (see `LoadingStage::race_ready`),
        // so this should now read near-instant rather than the 2.83s a cold
        // PS2 track measured before the worker existed.
        let music_start = web_time::Instant::now();
        match music {
            Some(worker) => self.audio.finish_race_music(
                worker,
                &self.music_discs,
                self.settings.audio.music_source,
                &oag_source::cache::default_audio_cache_dir(),
            ),
            // Defensive rather than load-bearing: `Session::launch_race`
            // always spawns one for the path this function only runs on
            // (`Stage::race_loading`). Falls back to the old synchronous
            // fetch rather than silently leaving the race without music if
            // that invariant is ever violated.
            None => self.audio.start_race_music(
                &self.music_discs,
                self.settings.audio.music_source,
                &oag_source::cache::default_audio_cache_dir(),
            ),
        }
        debug!("race music started in {:?}", music_start.elapsed());
        self.gpu
            .window
            .set_title(&hints::race_title(&strings::project_table(
                self.settings.language.as_deref(),
            )));
        Ok(())
    }
}
