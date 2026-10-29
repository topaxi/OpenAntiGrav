//! The winit application handler, and the window it opens.
//!
//! [`App`] is what the event loop drives; it holds everything loaded before
//! the window existed and hands it to a [`Session`] on the first resume.

use anyhow::{Context, Result};
use log::{debug, error, info};
use oag_core::{TickClock, TickRate};

use oag_game::render::Renderer;
use oag_game::{boot, launcher, loading, prefetch, settings};
use oag_gameplay::ControlScheme;
use oag_input::Controls;
use oag_mesh::mesh_render::Anisotropy;
use oag_present::perf;
use oag_present::upscale;
use oag_raceplay as race;
use oag_raceplay::pilots;
use oag_source::source;
use oag_ui::strings;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey, NativeKeyCode, PhysicalKey};
use winit::window::WindowId;

use crate::gpu::Gpu;
use crate::hints;
use crate::prepare;
use crate::session::lifecycle::Away;
use crate::session::{Session, Shell};
use crate::stage::Stage;
use crate::windows;

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
    /// `--measure-race-load`, the number of races to time. See
    /// `session::load_probe`.
    pub(crate) measure_race_load: Option<u32>,
    /// `--give`, resolved to a weapon at startup. See the CLI field.
    pub(crate) give: Option<oag_tables::weapons::Weapon>,
    /// `--no-intro`: skip the pre-race flyby. See the CLI field.
    pub(crate) no_intro: bool,
    /// `--preseat`, `--script`, `--frame-log`. See `oag_game::devrun`.
    pub(crate) dev: oag_game::devrun::Dev,
    /// `--connect`: the race is a server's. Joined once the race is built, then
    /// handed to the session. See `oag_game::net_play`.
    pub(crate) net: Option<oag_game::net_play::NetPlay>,
    /// `--net-bot`: fly this machine's craft with the player's own driver in a network race,
    /// `--connect` or a LAN one from the menus.
    pub(crate) net_bot: bool,
    /// `--prompt-style`, parsed. See `session::prompts`.
    pub(crate) prompt_style: Option<oag_input::prompt::PromptStyle>,
    /// `--autopilot`: whether the player's craft is flown for them. A
    /// verification aid - see `race::Race::set_autopilot`.
    pub(crate) autopilot: bool,
    /// `--autopilot-pilot`, resolved to a pilot at startup. See the CLI field.
    pub(crate) autopilot_pilot: Option<oag_ai::Pilot>,
    /// `--autopilot-skill`. See the CLI field.
    pub(crate) autopilot_skill: Option<oag_ai::Difficulty>,
    /// `--ai`: whose driver flies the opponents. See `oag_game::ai_choice`.
    pub(crate) ai_choice: oag_game::ai_choice::AiChoice,
    /// `--players`: split-screen views of a direct `--race`. See `oag_game::split_screen`.
    pub(crate) players: u8,
    /// `--windows` / `--window-seats`: the people dealt across the OS windows, window 1 first;
    /// empty for the one window. See `crate::windows`.
    pub(crate) window_layout: Vec<oag_game::split_screen::WindowSeats>,
    /// `--windows-fullscreen`.
    pub(crate) windows_fullscreen: bool,
    /// `--split-cut`: which way two views are cut.
    pub(crate) split_cut: oag_render::viewport::Cut,
    /// `--anim-seconds`: pins the trackside animation clock. See
    /// `Session::anim_seconds`.
    pub(crate) anim_seconds: Option<f32>,
    /// `--pvs`, carried to the session. See `Session::pvs_culling`.
    pub(crate) pvs_culling: Option<bool>,
    /// `--camera-jitter`, carried to the session. See `Session::camera_jitter`.
    pub(crate) camera_jitter: bool,
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
    /// The title `--race` opened, for the render profile it has no shell to
    /// resolve. `None` on every route that has a shell.
    pub(crate) race_title: Option<&'static oag_title::Title>,
    /// The platform that same source is for - see `settings::profile_key`,
    /// which is why this travels beside `race_title` rather than alone: a
    /// render profile is keyed on both.
    pub(crate) race_platform: Option<oag_disc::Platform>,
    /// The mixer and its device, waiting for the window that will step it.
    ///
    /// Taken by [`Session`] on the first resume, which is why it is an
    /// `Option`: there is one of these per run, not one per window, and winit
    /// may resume more than once.
    pub(crate) audio: Option<oag_sound::Audio>,
    /// Which Pulse releases this machine has, surveyed once before the window
    /// opened.
    ///
    /// Beside `audio` because it is the same kind of thing - a property of the
    /// run rather than of what is on screen - and it is here rather than in
    /// `Shell` because `--race` has no shell and still has music.
    pub(crate) music_discs: oag_sound::MusicDiscs,
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
    /// The platform took the window away (`Suspended`) and has not given one
    /// back. Nothing is drawn meanwhile; the next `Resumed` rebuilds the
    /// surface. Only ever set on Android.
    pub(crate) suspended: bool,
    /// The window is minimised or hidden (`Occluded(true)`, or a zero-sized
    /// resize), as opposed to [`Self::suspended`], which Android sends.
    pub(crate) occluded: bool,
}

impl App {
    /// Opens the window and builds whichever stage the command line asked for.
    ///
    /// `Ok(None)` means there was nothing to show, which only happens if the event
    /// loop resumes twice after the loaded state has been taken.
    /// The render profile for the title `--race` opened, or the default.
    ///
    /// `None` on every other leg: the launcher has chosen nothing yet and a
    /// boot has not resolved its title, so both correctly get the default and
    /// pick the real one up through `Session::render_profile` once a `Shell`
    /// exists. See [`Self::race_title`].
    fn race_profile(&self) -> settings::RenderProfile {
        self.race_title
            .zip(self.race_platform)
            .and_then(|(title, platform)| {
                self.settings
                    .render_profiles
                    .get(&settings::profile_key(title, platform))
            })
            .cloned()
            .unwrap_or_default()
    }

    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<Option<Session>> {
        let gpu = Gpu::new(
            event_loop,
            &self.settings.display,
            &self.settings.graphics.renderer,
            self.settings.language.as_deref(),
        )?;
        self.open_with(gpu)
    }

    /// The session on a device that is already up: the second half of [`Self::open`], and what
    /// `--drive` calls with a device that has no window (`crate::drive`).
    pub(crate) fn open_with(&mut self, gpu: Gpu) -> Result<Option<Session>> {

        // Built before the stage, because a race's depth attachment has to match
        // this rather than the window. On the launcher and boot legs it is too
        // early to know a title to resolve `settings.render_profiles` against,
        // so it falls back to the default and `Session::frame` resizes it
        // against the real per-title value on its first frame once a `Shell`
        // exists. On the `--race` leg the title *is* known - see
        // [`Self::race_profile`] - and using it here saves that first-frame
        // reallocation.
        let render_profile = self.race_profile();
        let rect = crate::window::scene_rect_of(
            &self.settings.display,
            crate::window::is_fullscreen(&gpu.output.window),
            gpu.output.size(),
        );
        let gamescope =
            crate::window::under_gamescope_cached(gpu.output.window.current_monitor().as_ref());
        let target = oag_game::settings::web::render_target(
            rect,
            render_profile.scale_for((rect.2, rect.3), gamescope),
            gpu.device.limits().max_texture_dimension_2d,
        );
        let framebuffer = upscale::Framebuffer::new(&gpu.device, gpu.output.config.format, target)
            .context("building the upscale pipeline")?;
        // Read once here and polled from the frame loop after: the built-ins
        // are in the binary, and the player's directory is one `read_dir`.
        let screen_filters =
            oag_game::screen::Catalogue::load(oag_game::screen::Catalogue::directory());

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
        let mut music_place = oag_game::music_state::Run::default();
        // Windows beyond the first: a front-end run asks for them here and opens them once
        // its title is known; a `--race` run sets this where it seats its views, below.
        let mut windows_wanted = if self.race.is_none() {
            u8::try_from(self.window_layout.len()).unwrap_or(u8::MAX)
        } else {
            0
        };
        let stage = if let Some(launcher) = self.launcher.take() {
            // Before both of the others, because it is what comes before both:
            // nothing has been opened yet and the two branches below are about
            // what was. It draws with the engine's own glyphs - see
            // `Stage::launcher` - since there is no disc to take a font from.
            Stage::launcher(&gpu, launcher)?
        } else if let Some(loaded) = self.race.take() {
            gpu.output.window
                .set_title(&hints::race_title(&strings::project_table(
                    self.settings.language.as_deref(),
                )));
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
            //
            // The saved place is put back first and written back like a menu
            // race's (`Session::save_race_music`, keyed on `race_title`), the
            // same `music_state::Run` on both routes. The headless captures
            // below this (`--screenshot`, `--trace-out`) stay out of it so a
            // capture never depends on a profile.
            if let Some(title) = self.race_title {
                music_place.resume(&mut audio, &self.music_discs, title.name);
            }
            oag_game::sound::hold_race_song(&mut audio, loaded.title);
            audio.start_race_music(
                &self.music_discs,
                self.settings.audio.music_source,
                &oag_source::cache::default_audio_cache_dir(),
            );
            let ai_difficulty = loaded.setup.difficulty;
            let mut staged = Stage::race(
                &gpu,
                loaded,
                framebuffer.allocation(),
                self.anisotropy,
                &self.settings,
                // **The title's own profile**, and it has to be *this* one
                // rather than the per-frame value `Session::render_profile`
                // hands out: `msaa` is baked into every scene pipeline here,
                // at `race::Scene::new`, and never read again for the life of
                // the race. Passing a default here left `--race` rasterizing
                // at one sample whatever the row or `--msaa` said, with the
                // per-frame rows all correct around it - a half-applied
                // profile, which reads as "MSAA costs nothing" rather than as
                // a bug.
                &render_profile,
                self.scheme,
                self.autopilot,
                self.autopilot_pilot,
                self.autopilot_skill,
                // `self.race_options` is this same load's own request - see
                // `run_race` in `headless.rs`, the only place that builds
                // both `self.race` and `self.race_options` together.
                self.race_options
                    .as_ref()
                    .and_then(|options| options.track.as_deref()),
            )?;
            // A direct `--race` is never a campaign launch, so this is the
            // free case: the original driver unless `--ai ours`.
            if let Stage::Race(race_stage) = &mut staged
                && let Some(net) = &self.net
            {
                // The server's race: its AI, its seats, its start. No split.
                let title = self.race_title.map_or("", |title| title.name);
                net.join(&mut race_stage.race, title)?;
            } else if let Stage::Race(race_stage) = &mut staged {
                let players = self.players;
                // `--windows` / `--window-seats`: the people are dealt across windows, each cut
                // up inside its own. The windows themselves open on the first pass of the event
                // loop (`Session::screens_to_open`), when the title is known to offer them.
                let windowed = players > 1 && !self.window_layout.is_empty();
                let views = if windowed {
                    oag_game::split_screen::apply_windows(
                        &mut race_stage.race,
                        &self.window_layout,
                    )
                } else {
                    oag_game::split_screen::apply(
                        &mut race_stage.race,
                        usize::from(players),
                        self.split_cut,
                    )
                };
                if windowed && views > 1 {
                    windows_wanted = u8::try_from(self.window_layout.len()).unwrap_or(u8::MAX);
                }
                // After the people are dealt out, as the menu route does: the driver is
                // chosen by how many humans race, and with a second one seated this
                // project's driver flies (the original's is single-human). Applied before
                // the split, a `--race --players 2` run flew the original on a two-human grid.
                oag_game::ai_choice::apply(
                    &mut race_stage.race,
                    self.ai_choice,
                    oag_game::ai_choice::Source::Free(ai_difficulty),
                );
            }
            staged
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
                self.settings.language.as_deref(),
            )?
        } else {
            return Ok(None);
        };

        let mut controls = Controls::new();
        // `--players N`: the keyboard and the first pad on slot 0, each further
        // pad on the next slot in connection order. Chosen, not measured; the
        // join screen (`Session::open_join`) seats by who pressed instead.
        if let Stage::Race(stage) = &stage
            && stage.race.viewport_count() > 1
        {
            controls.seat_default(stage.race.viewport_count());
        }
        // The pad's own preferences, which belong to the device rather than to
        // a race - so they are applied once here and then only by the menu row
        // that changes them. See `Session::apply_setting`.
        controls.set_trigger_mode(crate::args::resolve_triggers(&self.settings));
        controls.set_trigger_curve(self.settings.controls.trigger_sensitivity.exponent());
        // The keyboard's own live table - a rebind changes it again, and
        // persists it, through `Session::rebind`.
        controls.set_bindings(crate::args::resolve_bindings(&self.settings));
        for name in controls.pad().names() {
            info!("gamepad: {name}");
        }
        for name in controls.pad().ignored() {
            info!("not a gamepad, ignored: {name}");
        }

        // Built now rather than when the setting is first turned on, and from
        // **our own** 5x7 glyphs rather than the disc's font, for the same
        // reason: the overlay has to work on every route, and `--race` never
        // loads a font at all. One pipeline and a 616-byte atlas, against a
        // fallible build and a borrow dance in the middle of `frame`.
        // Its sheet is the pointer's - the one sprite the overlay pass
        // draws. The chooser's own cursor to begin with; `Session::draw`
        // swaps in the title's the frame a shell exists. See
        // `oag_game::cursor`.
        let cursor_sheet = oag_game::cursor::sheet(oag_game::cursor::LAUNCHER);
        let overlay = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.output.config.format,
            None,
            oag_ui::font::Atlas::build(),
            &cursor_sheet,
        )
        .context("building the performance overlay")?;

        // How long the scene pass takes on the GPU, which is the only signal
        // dynamic resolution could be controlled on: the frame-loop interval
        // is pinned to the refresh under vsync and to the limit under any
        // frame limit. `None` on a device that came back without
        // `TIMESTAMP_QUERY` - the feature is asked for in
        // `mesh_render::optional_features` and intersected with the adapter's,
        // so a device that lacks it still boots and simply cannot be asked.
        // Said out loud either way: which of the two happened decides whether
        // the eventual DRS row can be offered at all.
        let pass_timer = oag_gpu::timing::PassTimer::new(&gpu.device, &gpu.queue);
        // A ring of its own, for the reason `Session::upscale_timer` gives:
        // sharing the scene pass's would cost the resolution controller a
        // reading every frame the upscaler took a slot.
        let upscale_timer = oag_gpu::timing::PassTimer::new(&gpu.device, &gpu.queue);
        // And one more beside it, for the *other* half of that chain: the two
        // presentation-resolution dispatches are a cost the resolution
        // controller cannot lower, and telling them apart from the six that
        // shrink is the whole of ADR-0045.
        let upscale_presented_timer = oag_gpu::timing::PassTimer::new(&gpu.device, &gpu.queue);
        // And a third, for the same reason again - see `Session::blur_cost`.
        let blur_timer = oag_gpu::timing::PassTimer::new(&gpu.device, &gpu.queue);
        // And a fourth - see `Session::hd_bloom_cost`.
        let hd_bloom_timer = oag_gpu::timing::PassTimer::new(&gpu.device, &gpu.queue);
        debug!(
            "GPU timing: {}",
            if pass_timer.is_some() {
                "the scene pass, the motion-blur chain, the HD bloom chain and the FSR 3.1 chain are timed"
            } else {
                "no timestamps on this device"
            }
        );

        Ok(Some(Session {
            gpu,
            framebuffer,
            stage,
            controls,
            pad_dir: None,
            pointer: crate::pointer::Window::default(),
            touch_overlay: crate::touch::Overlay::default(),
            audio,
            music_discs: self.music_discs.clone(),
            music_place,
            screen_filters,
            missing_screen_filter: None,
            clock: TickClock::new(TickRate::DEFAULT),
            last: web_time::Instant::now(),
            meter: perf::Meter::new(),
            scene_cost: perf::Meter::new(),
            drs: oag_present::drs::Controller::new(),
            pass_timer,
            upscale_cost: perf::ChainCost::new(),
            upscale_timer,
            upscale_presented_cost: perf::ChainCost::new(),
            upscale_presented_timer,
            blur_cost: perf::ChainCost::new(),
            blur_timer,
            blur_drawn: Default::default(),
            hd_bloom_cost: perf::ChainCost::new(),
            hd_bloom_timer,
            cpu_cost: perf::Meter::new(),
            present_cost: perf::Meter::new(),
            governor: oag_present::governor::Governor::new(),
            governor_gpu: None,
            last_present_seconds: 0.0,
            frame_timing: Default::default(),
            phases: crate::session::phases::PhaseClock::new(self.dev.frame_log),
            drs_unreachable_said: false,
            window_ahead: Default::default(),
            frame_index: 0,
            stall_frame: 0,
            memory: perf::memory::Probe::new(),
            overlay,
            cursor_sheet,
            cursor_title: None,
            stalled: true,
            pause: oag_game::pause::State::Running,
            wall: 0.0,
            go_at: None,
            wall_origin: web_time::Instant::now(),
            next_frame: web_time::Instant::now(),
            logged_limit: None,
            cli_foreign_track: self
                .race_options
                .as_ref()
                .and_then(|options| options.foreign_track.clone()),
            race_options: self.race_options.clone(),
            pending: self.pending.take(),
            trace: self.trace,
            log_every: self.log_every,
            give: self.give,
            no_intro: self.no_intro,
            dev: std::mem::take(&mut self.dev),
            dev_clock: crate::session::devrun::Clock::default(),
            net: self.net.take(),
            net_bot: self.net_bot,
            net_over_ticks: 0,
            prompt: oag_game::prompts::PromptState {
                style_override: self.prompt_style,
                family_logged: None,
            },
            autopilot: self.autopilot,
            autopilot_pilot: self.autopilot_pilot,
            autopilot_skill: self.autopilot_skill,
            ai_choice: self.ai_choice,
            anim_seconds: self.anim_seconds,
            pvs_culling: self.pvs_culling,
            camera_jitter: self.camera_jitter,
            zone_hold: oag_mesh::mesh_render::zone::Hold::default(),
            scheme: self.scheme,
            anisotropy: self.anisotropy,
            launched: false,
            pending_event: None,
            races_launched: 0,
            settings: self.settings.clone(),
            // Loaded fresh here rather than carried on `App`: there is one
            // window per run and `App::open` only ever runs once for it, so
            // this is not a repeated disk read on the hot path any settings
            // or pilot load is not already on. See `oag_game::records::load`,
            // which cannot fail - a records file this build cannot make sense
            // of degrades to an empty store rather than failing the boot.
            records: oag_game::records::load(),
            shell: self.shell.clone(),
            race_title: self.race_title,
            race_platform: self.race_platform,
            // Race Remix's title pickers - see the field's own doc comment
            // for why this is a fresh survey rather than the chooser's.
            titles: launcher::distinct_titles(&launcher::survey(&source::candidates())),
            // The search path's tracks, then the mods' that are on (`docs/tools/mods.md`).
            custom_tracks: {
                let mut scan = oag_game::custom_tracks::Scan::discover();
                if let Some(shell) = &self.shell {
                    scan.set_mod_files(&shell.custom_campaigns.tracks);
                }
                scan
            },
            // Moved rather than cloned: `Assets` owns a decoded glow strip and
            // a full-screen backdrop, and `App` has no use for either once the
            // window it opened has them. What is left behind is the drawable
            // default, which is what a `--race` run has always carried.
            loading_assets: std::mem::take(&mut self.loading_assets),
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
            race_ready_at: None,
            load_probe: self
                .measure_race_load
                .and_then(crate::session::LoadProbe::new),
            suspended_race: None,
            campaign_cell: None,
            campaign_custom: None,
            custom_campaign: None,
            campaign_home: None,
            campaign_cursor: crate::campaign_stage::CellCursor::default(),
            pure_chain_class: 0,
            split_lobby: None,
            split_cut: self.split_cut,
            screens: Vec::new(),
            screens_opened: false,
            windowed_sizes: Vec::new(),
            windows_wanted,
            window_layout: self.window_layout.clone(),
            windows_fullscreen: self.windows_fullscreen,
            window_monitors: Vec::new(),
            window_layout_chosen: false,
            focus: crate::windows::Focus::default(),
            seat_clicks: 0,
            multiplayer: false,
            lan: None,
            online: None,
            lan_team: String::new(),
            lan_facts: None,
            pane_people: Vec::new(),
            mp_devices: Vec::new(),
            split_picks: Vec::new(),
            pure_classic_entries: std::collections::BTreeMap::new(),
            campaign_difficulty: None,
            campaign_ai_skill_scale: None,
            campaign_add_on: None,
            tournament: None,
            awaiting_binding: None,
            // A load failure here is not fatal - it is the same "corrupt
            // file" case `pilots::load_from` always could hit, just reached
            // from the composition root instead of `race::start` - so this
            // falls back to the built-ins alone and logs, rather than
            // failing the whole boot over a pilot file the AI PILOTS page
            // will let the player see and fix.
            pilot_roster: pilots::load().unwrap_or_else(|e| {
                error!("could not load pilots: {e:#}");
                pilots::Roster::built_in()
            }),
        }))
    }

    /// Waits for a conversion the window has outlived.
    ///
    /// Here rather than in `main` because the handle moved onto the [`Session`]
    /// at the first resume - see [`Session::prefetch`]. The window is gone by
    /// now, so a conversion still running has nothing to stay responsive for,
    /// but it does have work worth not throwing away, and it prints as it goes.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
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
    /// Writes the `--dump-audio` file when the window closes (a scripted
    /// front-end walk, the only way to hear the session's own cues), and
    /// nothing without that flag or under a device.
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
        if let Some(session) = self.state.as_mut() {
            if self.suspended {
                match session.gpu.recreate_surface(event_loop) {
                    Ok(()) => {
                        self.suspended = false;
                        let (width, height) = session.gpu.output.size();
                        session.resize(width, height);
                        // A minimised desktop window never gets here, but a
                        // resume on top of one would be wrong to restart.
                        if !self.occluded {
                            session.window_back();
                        }
                    }
                    Err(e) => {
                        error!("{e:#}");
                        event_loop.exit();
                    }
                }
            }
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

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.suspended = true;
        if let Some(session) = self.state.as_mut() {
            session.controls.release_all();
            session.pointer.release();
            // Home, app switch or screen off: the race parks in its pause
            // menu and does not come back by itself, and the device stops.
            session.window_away(Away::Suspended);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.suspended {
            return;
        }
        let Some(session) = self.state.as_mut() else {
            return;
        };

        // A `--windows` run: which window is this for? The keyboard is one device whichever
        // window has the focus, so its events go the way window 1's do; a seat window's
        // own life (resize, close, focus) is handled here; the rest of what a seat
        // receives is not for it (the front end, the pointer and drops are window 1's).
        if session.screens_opened {
            let ids = session.screen_ids();
            let primary = session.gpu.output.window.id();
            let Some(which) = windows::which(id, primary, &ids) else {
                // A window this run let go of: whatever it still has to say (a late
                // focus, a resize, a redraw) is not window 1's.
                return;
            };
            let is_seat = which != windows::Which::Primary;
            if let (WindowEvent::CloseRequested | WindowEvent::Destroyed, windows::Closed::Screen(index)) =
                (&event, windows::closed(id, primary, &ids))
            {
                session.close_screen(index);
                return;
            }
            match &event {
                WindowEvent::Focused(gained) => {
                    session.focus.changed(id, *gained, web_time::Instant::now());
                    // A key held in a window that lost focus is never seen to come up: the
                    // keyboard seat of that window lets go (its other windows' seats keep theirs,
                    // and so does a seat bound to its physical keyboard).
                    if !*gained {
                        if session.keys_by_window() {
                            session.window_lost_focus(id);
                        } else if session.windowed_race() {
                            session.controls.let_go();
                        } else {
                            session.controls.release_all();
                        }
                        if !is_seat {
                            session.pointer.release();
                        }
                    }
                    return;
                }
                // A click on a further window is that window's first seat's Start while the
                // race is paused (`Session::seat_clicked`); the rest of its pointer is
                // ignored, as before.
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: winit::event::MouseButton::Left,
                    ..
                } if is_seat => {
                    if let windows::Which::Screen(index) = which {
                        session.seat_clicked(index + 1);
                        session.seat_pointer(index, &event);
                    }
                    return;
                }
                // The rest of a seat window's mouse and touch is its own pointer's, which the
                // end screens read (`Session::seat_pointers`).
                WindowEvent::CursorMoved { .. }
                | WindowEvent::CursorLeft { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::Touch(_)
                    if is_seat =>
                {
                    if let windows::Which::Screen(index) = which {
                        session.seat_pointer(index, &event);
                    }
                    return;
                }
                WindowEvent::Resized(size) if is_seat => {
                    if let Some(index) = ids.iter().position(|seat| *seat == id) {
                        session.resize_screen(index, *size);
                    }
                    return;
                }
                WindowEvent::KeyboardInput { .. } | WindowEvent::ModifiersChanged(_) => {}
                _ if is_seat => return,
                _ => {}
            }
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                log::debug!("window resized to {}x{}", size.width, size.height);
                let hidden = size.width == 0 || size.height == 0;
                if hidden != self.occluded {
                    self.occluded = hidden;
                    if hidden {
                        session.window_away(Away::Minimized);
                    } else {
                        session.window_back();
                    }
                }
                let monitor = session.gpu.output.window.current_monitor();
                let held = crate::window::held_to_monitor(
                    size,
                    crate::window::under_gamescope_cached(monitor.as_ref()),
                    monitor.map(|m| (m.size().width, m.size().height)),
                );
                session.resize(held.width, held.height);
            }

            // A key held while the window loses focus is never seen to come up, and
            // the ship would keep turning while the player is elsewhere. A click
            // that landed just before was for whatever took the focus.
            WindowEvent::Focused(false) => {
                session.controls.release_all();
                session.pointer.release();
                session.window_away(Away::FocusLost);
            }

            // Minimised or hidden, where winit reports it: pause the race and
            // stop the device. Not reported by every backend (X11 has no such
            // event), so a zero-sized resize, which Windows sends on minimise,
            // counts too.
            WindowEvent::Occluded(hidden) => {
                self.occluded = hidden;
                if hidden {
                    session.window_away(Away::Minimized);
                } else {
                    session.window_back();
                }
            }

            // The mouse and the touchscreen, latched for the tick loop - see
            // `crate::pointer`. Every stage reads them through
            // `Session::frame`; nothing is mapped or decided here.
            WindowEvent::CursorMoved { position, .. } => {
                session.pointer.cursor_moved(position.x, position.y);
            }
            WindowEvent::CursorLeft { .. } => session.pointer.cursor_left(),
            WindowEvent::MouseInput { state, button, .. } => {
                session.pointer.button(button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => session.pointer.wheel(delta),
            WindowEvent::Touch(touch) => session.touch(touch),

            // A file dragged onto the chooser: a disc key or a disc image
            // (see `Launcher::dropped`). winit's Wayland backend reports none
            // of these, so a drop works under X11 and on Windows and macOS.
            WindowEvent::HoveredFile(_) => session.file_hover(true),
            WindowEvent::HoveredFileCancelled => session.file_hover(false),
            WindowEvent::DroppedFile(path) => session.file_dropped(&path),

            WindowEvent::ModifiersChanged(modifiers) => {
                crate::clipboard::set_ctrl_held(modifiers.state().control_key());
            }

            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                // The keyboard is the device in use now, whatever the key:
                // the drawn cursor goes until the mouse moves again. See
                // `crate::pointer::Window::other_device`.
                if event.state == ElementState::Pressed {
                    session.pointer.other_device();
                }
                // A CONTROLS row is waiting for its new key, and this raw event
                // is the only place that key still exists as itself - past
                // this point `Controls::set_key` would already have mapped it
                // through the very table a rebind is here to change, dropping
                // it silently if it is not already bound to something. Neither
                // the abstract `Input` nor `Menu::update` sees this event at
                // all while a capture is open.
                if let Some(awaiting) = session.awaiting_binding {
                    match crate::rebind::decide(
                        awaiting,
                        &event.logical_key,
                        event.state == ElementState::Pressed,
                        event.repeat,
                    ) {
                        crate::rebind::Capture::Bind(button, name) => session.rebind(button, name),
                        crate::rebind::Capture::Cancel => session.cancel_binding(),
                        crate::rebind::Capture::Ignore => {}
                    }
                    return;
                }
                // F11: this window goes borderless on its own monitor, or back. Never a game
                // input, so it returns before any menu, race or seat routing sees it.
                #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
                if crate::window::is_fullscreen_key(
                    &event.logical_key,
                    event.state == ElementState::Pressed,
                    event.repeat,
                ) {
                    session.toggle_window_fullscreen(id);
                    return;
                }
                // **`repeat` matters now that escape navigates.** winit resends
                // `Pressed` while a key is held, and back-one-level repeated
                // thirty times a second walks out of the menus and quits. It
                // did not matter while escape exited on the first one.
                if event.state == ElementState::Pressed && !event.repeat {
                    // Escape quits where nothing is behind it; the Android
                    // system Back (winit names it `BrowserBack`) never does.
                    if event.logical_key == Key::Named(NamedKey::Escape) {
                        session.escape();
                        return;
                    }
                    if event.logical_key == Key::Named(NamedKey::BrowserBack) {
                        session.back();
                        return;
                    }
                }
                // An on-screen keyboard is open, and this key is one it
                // understands. **Diverted rather than shared**: a letter that
                // is also bound to a game button would otherwise type itself
                // *and* press the grid's selected key. Anything the prompt
                // does not understand - the arrows above all - falls through
                // below, which is what leaves the grid navigable from the
                // same keyboard that is typing into it.
                // An Android gamepad's button: unidentified to winit, carrying
                // the keycode. A pad key is never also a keyboard key.
                if let PhysicalKey::Unidentified(NativeKeyCode::Android(code)) =
                    event.physical_key
                    && session
                        .controls
                        .android_key(code, event.state == ElementState::Pressed)
                {
                    return;
                }
                if session.typing_is_open() {
                    if event.state == ElementState::Pressed
                        && !event.repeat
                        && crate::clipboard::ctrl_held()
                        && matches!(&event.logical_key, Key::Character(c) if c.eq_ignore_ascii_case("v"))
                    {
                        session.request_paste();
                        return;
                    }
                    let typed = crate::typing::decide_with(
                        &event.logical_key,
                        event.state == ElementState::Pressed,
                        event.repeat,
                        |c| session.typing_accepts(c),
                    );
                    if session.typed(typed) {
                        return;
                    }
                }
                // In a race dealt across windows a key that was let go as the focus moved
                // stays released until it is pressed again (`Controls::let_go`). So winit's
                // own reports of the focus changing are not keys: X11 sends a synthetic
                // release for every key down to the window that loses focus (which would
                // end the wait for the key to come up before the focus event that starts
                // it) and a synthetic press for each to the window that gains it, and the
                // auto-repeat of a key still held then reaches the new window as plain
                // presses, which `let_go` holds off until the key is really released.
                if (event.repeat || is_synthetic) && session.windowed_race() {
                    return;
                }
                // A race dealt across windows reads the key by the window it arrived on; the
                // front end, a pause menu and a single window's race by the first keyboard.
                if session.keys_by_window() {
                    session.window_key(id, &event.logical_key, event.state == ElementState::Pressed);
                    return;
                }
                session
                    .controls
                    .set_key(&event.logical_key, event.state == ElementState::Pressed);
            }

            WindowEvent::RedrawRequested => {
                #[cfg(target_arch = "wasm32")]
                if session.too_early_for_frame(web_time::Instant::now()) {
                    return;
                }
                if let Err(e) = session.frame() {
                    error!("frame error: {e:#}");
                    event_loop.exit();
                }
            }

            _ => {}
        }
    }

    /// A physical keyboard's raw key event, which X11 and Windows send with the keyboard's id
    /// and Wayland and macOS do not send at all: the race keys of a second keyboard
    /// (`Session::device_key`). Delivered while one of our windows has the focus.
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if self.suspended {
            return;
        }
        if let (Some(session), winit::event::DeviceEvent::Key(raw)) = (self.state.as_mut(), event) {
            session.device_key(device_id, raw.physical_key, raw.state == ElementState::Pressed);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Every way out of the run ends here (window close, a menu's quit, a
        // quit from the race), so the playlist's place is saved once, whichever.
        if let Some(session) = self.state.as_mut() {
            session.save_race_music();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // The further windows of a `--windows` run, opened on the first pass the title is
        // known on, and the focus decision across all of them.
        if let Some(session) = self.state.as_mut() {
            #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
            if let Some(count) = session.screens_to_open() {
                session.open_screens(event_loop, count);
            }
            if session.screens_opened && session.focus.away(web_time::Instant::now()) {
                // Nothing has the focus now, so which keys are down is unknown: the next
                // press of any key counts again.
                session.controls.release_all();
                session.controls.forget_let_go();
                session.window_away(Away::FocusLost);
            }
        }
        // A menu asking to quit is handled here rather than where it is raised:
        // the tick loop has no event loop to call, and quitting from inside a
        // frame would leave that frame half-drawn.
        if self.state.as_ref().is_some_and(|session| session.quit) {
            // The browser cannot start a second loop in this page, so the page
            // is told and reloads itself (docs/tools/web.md, "Quitting").
            #[cfg(target_arch = "wasm32")]
            super::gpu::web::quit();
            event_loop.exit();
            return;
        }
        let Some(session) = &self.state else {
            return;
        };
        if self.suspended {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }

        // **The frame limiter is here and not in `frame`**, because the way to
        // produce fewer frames is to ask for fewer, not to draw one and then
        // sleep holding a submitted command buffer. `WaitUntil` hands the
        // waiting to the platform's own timer; `Poll` is what it was before and
        // is still what an unlimited run does.
        match session.next_frame_at() {
            // On the web winit's `Poll` and `WaitUntil` both wake the loop
            // through a `postTask` plus an `AbortController` per wake, a
            // quarter of the page's thread in a race (measured, 2026-10-10).
            // `Wait` plus a redraw request is driven by
            // `requestAnimationFrame` alone, and the limit holds by skipping
            // redraws in `RedrawRequested`.
            #[cfg(target_arch = "wasm32")]
            _ => {
                event_loop.set_control_flow(ControlFlow::Wait);
                session.gpu.output.window.request_redraw();
            }
            #[cfg(not(target_arch = "wasm32"))]
            Some(deadline) if web_time::Instant::now() < deadline => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
            #[cfg(not(target_arch = "wasm32"))]
            _ => {
                event_loop.set_control_flow(ControlFlow::Poll);
                session.gpu.output.window.request_redraw();
            }
        }
    }
}
