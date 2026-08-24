//! The winit application handler, and the window it opens.
//!
//! [`App`] is what the event loop drives; it holds everything loaded before
//! the window existed and hands it to a [`Session`] on the first resume.

use anyhow::{Context, Result};
use log::{error, info};
use oag_core::{TickClock, TickRate};

use oag_game::render::Renderer;
use oag_game::{audio, boot, display, launcher, loading, perf, prefetch, race, settings, upscale};
use oag_gameplay::ControlScheme;
use oag_input::Controls;
use oag_render::mesh_render::Anisotropy;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowId;

use crate::gpu::Gpu;
use crate::hints::RACE_TITLE;
use crate::prepare;
use crate::session::{Session, Shell};
use crate::stage::Stage;

/// One application handler for both ways in, because there is one window.
pub(crate) struct App {
    /// The cheap half of the boot, loaded before the window opened.
    ///
    /// Half a boot rather than a whole one because the other half - the movies -
    /// is still decoding on [`Self::media`] while this window opens. See
    /// `boot::Shell`.
    pub(crate) boot_shell: Option<boot::Shell>,
    /// The movies, arriving on a thread of their own.
    ///
    /// Joined by [`Session::finish_loading`], which is also where the two
    /// halves become a `boot::Boot`.
    pub(crate) media: Option<boot::MediaWorker>,
    /// `--overlay`, applied to the sequence once it exists.
    pub(crate) boot_overlay: bool,
    /// `--pick-language`: show the picker even when the settings name one.
    pub(crate) pick_language: bool,
    /// `--give`, resolved to a weapon at startup. See the CLI field.
    pub(crate) give: Option<oag_formats::weapons::Weapon>,
    /// `--autopilot`: whether the player's craft is flown for them. A
    /// verification aid - see `race::Race::set_autopilot`.
    pub(crate) autopilot: bool,
    /// `--anim-seconds`: pins the trackside animation clock. See
    /// `Session::anim_seconds`.
    pub(crate) anim_seconds: Option<f32>,
    /// A race loaded before the window opened, which is what `--race` does.
    pub(crate) race: Option<race::Loaded>,
    /// What a race started from `Launch Game` is flown on.
    ///
    /// `None` while the chooser is up: its `source` field is the one thing a
    /// run that has not picked yet cannot state, and a placeholder there would
    /// be a path nothing checked. Filled by `Session::finish_launcher`.
    pub(crate) race_options: Option<race::Options>,
    /// The disc chooser, when this run opened with no source named and found
    /// more than one image. See [`oag_game::launcher`].
    pub(crate) launcher: Option<launcher::Launcher>,
    /// What the command line decided, kept because a pick is what turns it into
    /// a boot. `None` on the `--race` route, which never chooses anything.
    pub(crate) pending: Option<prepare::Pending>,
    pub(crate) trace: bool,
    pub(crate) log_every: u32,
    pub(crate) anisotropy: Anisotropy,
    /// The control scheme resolved once at startup: `--scheme` over
    /// `[controls] scheme`. See [`Session::scheme`].
    pub(crate) scheme: ControlScheme,
    /// The persisted settings, which the menus edit and write straight back.
    pub(crate) settings: settings::Settings,
    /// What the menus need, absent on the `--race` path.
    pub(crate) shell: Option<Shell>,
    /// The mixer and its device, waiting for the window that will step it.
    ///
    /// Taken by [`Session`] on the first resume, which is why it is an
    /// `Option`: there is one of these per run, not one per window, and winit
    /// may resume more than once.
    pub(crate) audio: Option<audio::Audio>,
    /// Which Pulse releases this machine has, surveyed once before the window
    /// opened.
    ///
    /// Beside `audio` because it is the same kind of thing - a property of the
    /// run rather than of what is on screen - and it is here rather than in
    /// `Shell` because `--race` has no shell and still has music.
    pub(crate) music_discs: audio::MusicDiscs,
    /// What `--prefetch` asked for, **not started yet**.
    ///
    /// It may not start until the boot's own movies are done: both convert
    /// through `ffmpeg` into the same cache directory, and two processes
    /// writing one file is a corrupt file rather than a race that resolves.
    /// [`Session::start_prefetch`] is where it actually spawns, the moment
    /// [`Self::media`] reports finished. `None` on every ordinary boot.
    pub(crate) prefetch: Option<prefetch::Options>,
    /// The wave's glow strip and the disc's tips.
    ///
    /// Read on every windowed boot now, not only under `--prefetch`: the
    /// loading screen covers the movie decode on all of them.
    pub(crate) loading_assets: loading::Assets,
    pub(crate) state: Option<Session>,
}

impl App {
    /// Opens the window and builds whichever stage the command line asked for.
    ///
    /// `Ok(None)` means there was nothing to show, which only happens if the event
    /// loop resumes twice after the loaded state has been taken.
    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<Option<Session>> {
        let gpu = Gpu::new(
            event_loop,
            &self.settings.display,
            &self.settings.graphics.renderer,
        )?;

        // Built before the stage, because a race's depth attachment has to match
        // this rather than the window.
        let target = upscale::target_size(
            display::viewport(gpu.size(), self.settings.display.aspect),
            self.settings.graphics.render_scale,
            gpu.device.limits().max_texture_dimension_2d,
        );
        let framebuffer = upscale::Framebuffer::new(&gpu.device, gpu.config.format, target)
            .context("building the upscale pipeline")?;

        // Taken out of the boot before the front end takes the rest: it belongs
        // to the menus, which outlive the sequence that loaded it, and `--race`
        // has neither.
        //
        // Split in two here, and this is the last place both halves are in one
        // hand: the frames move onto a decode thread and the presentation - the
        // rate, the rectangle - stays behind, because a `Feed` deals in pixels
        // and knows nothing about where they go. `repeat: true`, which is the
        // whole difference between this movie and the intro.
        // **The menu's grid, which is ours and is the PSP's - not the source's.**
        // This rect is drawn by `MenuStage`'s own renderer, and `open_menus`
        // builds that one fresh and never calls `set_space`, so its `screen`
        // uniform is `Space::PSP` whatever disc is mounted. The menu layout it
        // sits behind is this project's own, authored at 480x272, which is why
        // `capture.rs` pins `Space::PSP` on the same picture.
        //
        // Handing this the *source's* space instead was a regression: on a PS2
        // disc it built the rect in a 640x448 grid for a shader normalising
        // against 480x272, and `pillarbox_in` always fills one axis of the grid
        // it is given, so the backdrop overflowed the screen on every aspect.
        // Silent on the PSP, where the two grids are the same numbers, and
        // invisible to `--menu-page`, which goes through `capture.rs`.
        // Taken before the stage rather than after it, because building the
        // front end is what starts the intro's sound and that needs the mixer
        // in hand. Moved rather than cloned: there is one mixer per run, and a
        // second one would either fight the first for the device or split a
        // `--dump-audio` capture across two buffers.
        let Some(mut audio) = self.audio.take() else {
            return Ok(None);
        };
        let stage = if let Some(launcher) = self.launcher.take() {
            // Before both of the others, because it is what comes before both:
            // nothing has been opened yet and the two branches below are about
            // what was. It draws with the engine's own glyphs - see
            // `Stage::launcher` - since there is no disc to take a font from.
            Stage::launcher(&gpu, launcher)?
        } else if let Some(loaded) = self.race.take() {
            gpu.window.set_title(RACE_TITLE);
            // Straight away on this leg only: `--race` has no boot sequence, so
            // there is no intro for the music to wait behind. The front end's
            // legs below start it from the tick loop instead, the moment the
            // sequence leaves its movie - see `Audio::start_music`. Here rather
            // than back in `main` before the load, which used to run seconds of
            // parsing and transcoding with the music already looping and no
            // window yet on screen to account for it.
            //
            // The race playlist, not `start_music`: `--race` has no menu to
            // play under a race, and no shell to return to (`escape` quits
            // outright on this leg), so there is nothing to pause and resume -
            // just the same list a menu-launched race plays, started fresh.
            audio.start_race_music(
                &self.music_discs,
                self.settings.audio.music_source,
                &boot::default_audio_cache_dir(),
            );
            Stage::race(
                &gpu,
                loaded,
                framebuffer.size(),
                self.anisotropy,
                &self.settings,
                self.scheme,
                self.autopilot,
            )?
        } else if let Some(shell) = self.boot_shell.take() {
            // **Always the loading screen**, where this used to be `--prefetch`
            // only. Every windowed boot now has something to wait for - the
            // movies, which are still decoding on `self.media` as this window
            // opens - and this is the frame that says so instead of a desktop
            // with nothing on it. `finish_loading` swaps in the front end the
            // moment they land.
            Stage::loading(
                &gpu,
                shell,
                self.media.take(),
                &self.loading_assets,
                self.trace,
            )?
        } else {
            return Ok(None);
        };

        let mut controls = Controls::new();
        // The pad's own preferences, which belong to the device rather than to
        // a race - so they are applied once here and then only by the menu row
        // that changes them. See `Session::apply_setting`.
        controls.set_trigger_mode(crate::args::resolve_triggers(&self.settings));
        controls.set_trigger_curve(self.settings.controls.trigger_sensitivity.exponent());
        for name in controls.pad().names() {
            info!("gamepad: {name}");
        }

        // Built now rather than when the setting is first turned on, and from
        // **our own** 5x7 glyphs rather than the disc's font, for the same
        // reason: the overlay has to work on every route, and `--race` never
        // loads a font at all. One pipeline and a 616-byte atlas, against a
        // fallible build and a borrow dance in the middle of `frame`.
        let overlay = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            None,
            oag_game::font::Atlas::build(),
            &oag_game::sprite::Sheet::default(),
        )
        .context("building the performance overlay")?;

        Ok(Some(Session {
            gpu,
            framebuffer,
            stage,
            controls,
            audio,
            music_discs: self.music_discs.clone(),
            clock: TickClock::new(TickRate::DEFAULT),
            last: std::time::Instant::now(),
            meter: perf::Meter::new(),
            overlay,
            stalled: true,
            next_frame: std::time::Instant::now(),
            race_options: self.race_options.clone(),
            pending: self.pending.take(),
            trace: self.trace,
            log_every: self.log_every,
            give: self.give,
            autopilot: self.autopilot,
            anim_seconds: self.anim_seconds,
            scheme: self.scheme,
            anisotropy: self.anisotropy,
            launched: false,
            settings: self.settings.clone(),
            shell: self.shell.clone(),
            quit: false,
            // Both filled by `finish_loading`, off the movies the boot's own
            // worker is still decoding as this window opens. `--race` never
            // fills them at all.
            backdrop: None,
            backdrop_shape: None,
            held_menu_backdrop: None,
            // Nothing running yet on either line: the conversion may not start
            // until the boot's own movies are done with the cache, which is
            // what `start_prefetch` waits for.
            prefetch: None,
            prefetch_pending: self.prefetch.take(),
            boot_overlay: self.boot_overlay,
            pick_language: self.pick_language,
        }))
    }

    /// Waits for a conversion the window has outlived.
    ///
    /// Here rather than in `main` because the handle moved onto the [`Session`]
    /// at the first resume - see [`Session::prefetch`]. The window is gone by
    /// now, so a conversion still running has nothing to stay responsive for,
    /// but it does have work worth not throwing away, and it prints as it goes.
    pub(crate) fn finish_prefetch(&mut self) {
        let Some(prefetch) = self
            .state
            .as_mut()
            .and_then(|session| session.prefetch.as_mut())
        else {
            return;
        };
        let outstanding = prefetch.progress();
        if !outstanding.finished {
            info!(
                "waiting for --prefetch: {} of {} converted",
                outstanding.done, outstanding.total
            );
        }
        prefetch.join();
    }

    /// Writes the `--dump-audio` WAV once the event loop has returned.
    ///
    /// Here and not in a winit callback: `exiting` is not guaranteed to run on
    /// every platform, and a capture that silently produced no file would be
    /// indistinguishable from one that produced silence.
    ///
    /// Writes nothing today, because `--dump-audio` is refused without
    /// `--screenshot` and a capture never opens a window. It is wired anyway so
    /// that lifting that restriction is one edit rather than two, and so a
    /// windowed run cannot become the one path that quietly drops its dump.
    ///
    /// # Errors
    ///
    /// Propagates a file that cannot be written.
    pub(crate) fn finish_audio(&self) -> Result<()> {
        match &self.state {
            // The window opened, so `open` took the mixer and `Session` has it.
            Some(session) => session.audio.finish(),
            // It never did - a device that would not start, or a resume that
            // never came. There is nothing to write and no reason to complain.
            None => Ok(()),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        match self.open(event_loop) {
            Ok(Some(session)) => self.state = Some(session),
            Ok(None) => event_loop.exit(),
            Err(e) => {
                error!("{e:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(session) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => session.resize(size.width, size.height),

            // A key held while the window loses focus is never seen to come up, and
            // the ship would keep turning while the player is elsewhere.
            WindowEvent::Focused(false) => session.controls.release_all(),

            WindowEvent::KeyboardInput { event, .. } => {
                // **`repeat` matters now that escape navigates.** winit resends
                // `Pressed` while a key is held, and back-one-level repeated
                // thirty times a second walks out of the menus and quits. It
                // did not matter while escape exited on the first one.
                if event.logical_key == Key::Named(NamedKey::Escape)
                    && event.state == ElementState::Pressed
                    && !event.repeat
                {
                    session.escape();
                    return;
                }
                session
                    .controls
                    .set_key(&event.logical_key, event.state == ElementState::Pressed);
            }

            WindowEvent::RedrawRequested => {
                if let Err(e) = session.frame() {
                    error!("frame error: {e:#}");
                    event_loop.exit();
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // A menu asking to quit is handled here rather than where it is raised:
        // the tick loop has no event loop to call, and quitting from inside a
        // frame would leave that frame half-drawn.
        if self.state.as_ref().is_some_and(|session| session.quit) {
            event_loop.exit();
            return;
        }
        let Some(session) = &self.state else {
            return;
        };

        // **The frame limiter is here and not in `frame`**, because the way to
        // produce fewer frames is to ask for fewer, not to draw one and then
        // sleep holding a submitted command buffer. `WaitUntil` hands the
        // waiting to the platform's own timer; `Poll` is what it was before and
        // is still what an unlimited run does.
        match session.next_frame_at() {
            Some(deadline) if std::time::Instant::now() < deadline => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
            _ => {
                event_loop.set_control_flow(ControlFlow::Poll);
                session.gpu.window.request_redraw();
            }
        }
    }
}
