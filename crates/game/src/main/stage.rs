//! What is on screen, as one enum over the four things it can be.
//!
//! [`Stage`] itself is only the switch; each variant is a module of its own
//! beside this one - [`crate::menu_stage`], [`crate::loading_stage`],
//! [`crate::frontend_stage`] and [`crate::race_stage`] - because a stage owns
//! its own assets and its own update. They are siblings rather than children
//! named `stage::menu` and the rest: the library already owns `menu`,
//! `loading`, `frontend` and `race`, and a child module of any of those names
//! shadows the import the body right here needs.

use anyhow::{Context, Result};
use log::info;

use oag_game::render::{Renderer, VideoFormat};
use oag_game::{audio, boot, loading, movie, race, settings};
use oag_gameplay::ControlScheme;
use oag_render::mesh_render::Anisotropy;

use crate::frontend_stage::{FrontendStage, PendingMovie};
use crate::gpu::Gpu;
use crate::launcher_stage::LauncherStage;
use crate::loading_stage::LoadingStage;
use crate::menu_stage::MenuStage;
use crate::race_stage::RaceStage;

/// What the window is showing.
///
/// Both variants are boxed: a race carries the whole `World`, eight ship slots
/// wide, and an enum is as large as its largest variant wherever it is stored.
pub(crate) enum Stage {
    /// Which disc image to boot, when the run named none and found several.
    Launcher(Box<LauncherStage>),
    /// The wave and the counts, while `--prefetch` converts the disc.
    Loading(Box<LoadingStage>),
    /// The boot sequence: the intro reel, then the language picker.
    Frontend(Box<FrontendStage>),
    /// Our own menus, between the boot sequence and a race.
    Menu(Box<MenuStage>),
    /// A ship on a track.
    Race(Box<RaceStage>),
}

impl Stage {
    /// The disc chooser, which is the one stage with no source behind it.
    ///
    /// **The engine's own 5x7 glyphs, not a disc font**, for the same reason
    /// `App::open` builds the performance overlay from them: this runs before
    /// any archive is open, so there is no disc font to have. It is also the
    /// reason there is no `set_space` call - a space belongs to a source's own
    /// front-end XML, and the renderer's default is the PSP's 480x272, which is
    /// the grid this screen is authored in.
    ///
    /// # Errors
    ///
    /// Propagates the renderer's own pipeline build.
    pub(crate) fn launcher(gpu: &Gpu, launcher: oag_game::launcher::Launcher) -> Result<Self> {
        let renderer = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            None,
            oag_game::font::Atlas::build(),
            &oag_game::sprite::Sheet::default(),
        )
        .context("building the disc chooser")?;
        Ok(Self::Launcher(Box::new(LauncherStage {
            renderer,
            launcher,
            picked: None,
        })))
    }

    /// The loading screen, holding the half-boot it will hand the window to.
    ///
    /// **The boot sequence is carried as data rather than built and paused.**
    /// [`Stage::frontend`] spawns the intro's decode thread, and starting that
    /// ten minutes before anything reads a frame would leave a worker filling a
    /// ring nobody drains. So the front end is built at the hand-off, in
    /// [`Session::finish_loading`], and until then this owns the two halves.
    ///
    /// `media` is the worker still decoding the movies, `None` only if it was
    /// already taken - which cannot happen today, the window opening once.
    pub(crate) fn loading(
        gpu: &Gpu,
        shell: boot::Shell,
        media: Option<boot::MediaWorker>,
        assets: &loading::Assets,
        trace: bool,
    ) -> Result<Self> {
        // The **disc's** font, not the built-in 5x7 set: the tips are the
        // disc's own prose in the player's own language, and `push_text` skips
        // a glyph the atlas has no cell for - so accented text would silently
        // lose characters rather than fail. No video pipeline, this screen
        // having no movie; there is no loading movie in either build.
        let renderer = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            None,
            shell.font.clone(),
            // **The feature illustration's own sheet where this title ships
            // one**, not the front end's: `Draw::Sprite` addresses whichever
            // sheet the renderer was built with, and the loading screen draws
            // exactly one image. The front end's sheet is the right answer for a
            // title with no features and is what this drew before either way.
            assets.art.as_ref().map_or(&shell.sprites, |art| &art.sheet),
        )?;
        // The face's own line height, taken before the shell is moved into the
        // stage below: it is what the screen's own text scale is normalised
        // against, so HD's much larger face draws at the size this layout was
        // written for. See `loading::Screen::new`.
        let line_height = shell.font.line_height;
        // Sample count 1, matching `upscale::Framebuffer`'s target, which is
        // what this draws into.
        let wave = oag_render::loading::Pipeline::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            &assets.strip,
            1,
        );
        Ok(Self::Loading(Box::new(LoadingStage {
            atlas: shell.font.clone(),
            renderer,
            wave,
            // Seed 0 on the boot screen: it goes up once per run, and the
            // title that has features does not draw this one anyway.
            screen: loading::Screen::new(assets, line_height, 0),
            shell: Some(shell),
            media,
            race: None,
            built_race: None,
            trace,
        })))
    }

    /// The same screen again, over a circuit being read rather than the boot's
    /// movies.
    ///
    /// **The screen the original also has**, which the two waits above are not:
    /// a transcode and a prefetch are this build's own, and a race load is the
    /// one every release covers with something. So this is where a title's own
    /// loading presentation is drawn - Wipeout HD's full-screen still and its
    /// `FE_LOADINGDOT` caption, Pulse's wave and one of its 26 tips.
    ///
    /// Takes the font and sheet rather than a `boot::Shell`, because by this
    /// point there is no boot shell left: the running session has the menus'
    /// own copies and the boot's was consumed when the front end was built.
    pub(crate) fn race_loading(
        gpu: &Gpu,
        worker: race::LoadWorker,
        font: &oag_game::font::Atlas,
        sprites: &oag_game::sprite::Sheet,
        assets: &loading::Assets,
        draw: u64,
        trace: bool,
    ) -> Result<Self> {
        let renderer = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            None,
            font.clone(),
            // The illustration's own sheet where the title ships one; see
            // `Stage::loading`, which chooses the same way and for the same
            // reason.
            assets.art.as_ref().map_or(sprites, |art| &art.sheet),
        )?;
        let wave = oag_render::loading::Pipeline::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            &assets.strip,
            1,
        );
        Ok(Self::Loading(Box::new(LoadingStage {
            atlas: font.clone(),
            renderer,
            wave,
            screen: loading::Screen::new(assets, font.line_height, draw),
            shell: None,
            media: None,
            race: Some(worker),
            built_race: None,
            trace,
        })))
    }

    /// `audio` is taken because this is the moment the intro's sound starts,
    /// and there are two ways here - straight off the boot, or out of the
    /// loading screen when its fade runs out. Passing the mixer in makes
    /// starting it part of *becoming* the front end rather than something each
    /// of those two has to remember, which is the difference between a missed
    /// call being a compile error and being a silent movie on one path only.
    pub(crate) fn frontend(
        gpu: &Gpu,
        mut loaded: boot::Boot,
        video_format: Option<VideoFormat>,
        trace: bool,
        audio: &mut audio::Audio,
    ) -> Result<Self> {
        // Before the renderer is built rather than after, so the sound and the
        // first frame of picture start on the same tick: `Session::frame` draws
        // before it ticks, and a movie whose voice started a frame late would
        // be a frame late for the whole reel.
        let renderer = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            video_format,
            loaded.font.clone(),
            &loaded.sprites,
        )?;
        // The sequence's rects are in its source's grid, and the renderer maps
        // rects onto the viewport - so it has to be told which grid, or a PS2
        // screen's widgets are drawn a third again too big. See
        // `frontend::Space`.
        let mut renderer = renderer;
        renderer.set_space(loaded.frontend.space());
        // The store moves onto its own thread and the `Movie` around it is done
        // with: everything else it carried - the frame count, the rate, the
        // aspect - was read into the sequence and the renderer before this.
        // `repeat: false`, because the intro ends rather than starting again.
        let spawn = |movie: Option<movie::Movie>| {
            movie.and_then(|movie| {
                let (width, height) = (movie.width, movie.height);
                movie
                    .frames
                    .map(|frames| movie::Feed::spawn(frames, false, width, height))
            })
        };
        // **Every movie is keyed to the screen that plays it**, and installed on
        // the tick that screen is entered - including the first, whose screen is
        // the boot step on Pulse but *not* on Pure, where the reel plays two steps
        // in. Starting it at load would sound the reel under Pure's language
        // picker, which `--pick-language` makes plainly audible.
        //
        // Spawned now rather than lazily, because the `Movie` and the frames
        // inside it only exist here, on `loaded`.
        let states = loaded.frontend.movie_states();
        let mut pending: Vec<PendingMovie> = Vec::new();
        for (at, sound, film) in [
            (0usize, loaded.movie_sound.take(), loaded.movie),
            (
                1,
                loaded.after_language_movie_sound.take(),
                loaded.after_language_movie,
            ),
        ] {
            if let Some(state) = states.get(at).copied() {
                pending.push(PendingMovie {
                    state,
                    feed: spawn(film),
                    sound,
                });
            }
        }
        let mut stage = FrontendStage {
            renderer,
            frontend: loaded.frontend,
            feed: None,
            pending,
            shown: false,
            backdrop_shown: false,
            held_backdrop: None,
            backdrop_in_planes: false,
            trace,
        };
        // The boot screen gets no `Enter` event - it is where the machine starts -
        // so its own movie, if it has one, is installed here.
        if let Some(state) = stage.frontend.machine().current().map(str::to_string) {
            stage.install_movie(&state, audio);
        }
        Ok(Self::Frontend(Box::new(stage)))
    }

    /// `size` is the **framebuffer's**, not the window's: the scene's depth
    /// attachment has to match the colour one it will be drawn with, and that is
    /// the offscreen target rather than the surface. Getting it from the window
    /// is right only at a render scale of 100 % on an unshaped aspect, which is
    /// exactly the case that would let it ship looking correct.
    pub(crate) fn race(
        gpu: &Gpu,
        loaded: race::Loaded,
        size: (u32, u32),
        anisotropy: Anisotropy,
        settings: &settings::Settings,
        scheme: ControlScheme,
        autopilot: bool,
    ) -> Result<Self> {
        Ok(Self::Race(Self::build_race_stage(
            gpu, loaded, size, anisotropy, settings, scheme, autopilot,
        )?))
    }

    /// The scene-building half of [`Self::race`], split out so it can be run
    /// eagerly rather than only at the loading screen's hand-off.
    ///
    /// **This is the part that used to hide behind the fade.** Uploading
    /// meshes and building pipelines is itself a stall, and running it only
    /// once `LoadingStage::screen` had already reached zero opacity is what put
    /// a second, silent wait behind a screen that had already gone black. See
    /// [`Session::advance_race_build`], which calls this as soon as the
    /// circuit's own load lands, and [`LoadingStage::built_race`], which is
    /// where the result waits until the fade actually runs out.
    ///
    /// Returns the boxed [`RaceStage`] rather than a whole [`Stage`]: what is
    /// built ahead of time is the scene, not "being on screen", and
    /// [`LoadingStage::built_race`] holds exactly that distinction.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn build_race_stage(
        gpu: &Gpu,
        loaded: race::Loaded,
        size: (u32, u32),
        anisotropy: Anisotropy,
        // The whole settings block rather than the three values a race reads out
        // of it. Three separate parameters is what this used to be, and each new
        // display preference added a fourth: the values travel together, they all
        // come from one place, and none of them is ever overridden per race. The
        // exception is `anisotropy`, which stays its own parameter precisely
        // because `--anisotropy` *can* override it.
        settings: &settings::Settings,
        scheme: ControlScheme,
        // `--autopilot`, its own parameter for the same reason `anisotropy` is:
        // it is a run flag rather than a stored preference, and nothing in the
        // settings file has any business turning it on.
        autopilot: bool,
    ) -> Result<Box<RaceStage>> {
        let race::Loaded {
            setup,
            hud,
            track_model,
            liveries,
            collision_model,
            sky_model,
            pad_model,
            weapon_pad_model,
            rocket_model,
            shield_cockpit,
            fog_volumes,
            light,
            authored_fog,
            hd_bloom,
            visibility,
            flare,
            noise,
            trail_blend,
            trail_shape,
            ..
        } = loaded;
        let scene = race::Scene::new(
            &gpu.device,
            &gpu.queue,
            track_model,
            &liveries,
            collision_model,
            sky_model,
            pad_model,
            weapon_pad_model,
            setup.mode,
            rocket_model,
            shield_cockpit,
            flare,
            noise,
            trail_blend,
            trail_shape,
            gpu.config.format,
            size,
            anisotropy,
            settings.graphics.bloom,
            visibility,
            settings.graphics.anti_aliasing,
            fog_volumes,
            light,
            authored_fog,
            hd_bloom,
        )?;
        // Against the **surface** format, like every other renderer here, because
        // the HUD is composited into the offscreen target which shares it.
        let overlay = oag_game::hud::Overlay::new(&gpu.device, &gpu.queue, gpu.config.format, &hud)
            .context("building the HUD overlay")?;
        // The results table, built from the same assets and drawn into the same
        // target - see `oag_game::scoreboard`, which is where the "this is ours,
        // the disc's own Race End chain is not built" argument lives. Built with
        // the race rather than when it ends: a load is the one moment a stage may
        // stall, and doing it at the finish would drop frames on the lap the
        // player is most likely to be watching.
        let scoreboard =
            oag_game::scoreboard::Overlay::new(&gpu.device, &gpu.queue, gpu.config.format, &hud)
                .context("building the scoreboard overlay")?;
        let mut race = race::Race::start(setup);
        race.set_boost_fov_kick(settings.graphics.boost_fov_kick);
        // Applied before the first tick, but unlike the kick this one is also
        // set again whenever the cycle button or the menu row moves it - see
        // `Session::cycle_camera_view`.
        race.set_camera_view(settings.graphics.camera_view);
        race.set_control_scheme(scheme);
        race.set_autopilot(autopilot);
        if autopilot {
            info!("--autopilot: the player's craft is being flown for them");
        }
        Ok(Box::new(RaceStage {
            scene,
            race,
            hud: overlay,
            scoreboard,
        }))
    }
}
