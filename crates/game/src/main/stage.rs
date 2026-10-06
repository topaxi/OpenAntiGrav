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
use oag_game::{boot, loading, movie, settings};
use oag_gameplay::ControlScheme;
use oag_mesh::mesh_render::Anisotropy;
use oag_raceplay as race;

use crate::frontend_stage::{FrontendStage, PendingMovie};
use crate::gpu::{Gpu, GpuContext};
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
            oag_ui::font::Atlas::build(),
            &oag_hud::sprite::Sheet::default(),
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
        // The player's chosen language, so this screen's own invented prose
        // can be overridden the same way the disc's own strings already are
        // elsewhere - see `loading::Screen::new`. Both callers have a
        // `Settings` open already; this screen goes up before any disc's own
        // language plugin does, so `Settings::language` is the only source
        // there is.
        language: Option<&str>,
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
            screen: loading::Screen::new(assets, line_height, 0, language),
            shell: Some(shell),
            media,
            race: None,
            build: None,
            built_race: None,
            music: None,
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
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn race_loading(
        gpu: &Gpu,
        worker: race::LoadWorker,
        music: oag_sound::MusicFetchWorker,
        font: &oag_ui::font::Atlas,
        sprites: &oag_hud::sprite::Sheet,
        assets: &loading::Assets,
        draw: u64,
        // The source executable's mode id for the race being loaded, which
        // picks the loading screen's feature deck. See `loading::Screen::for_mode`.
        mode: Option<u32>,
        trace: bool,
        // See `Stage::loading`'s own parameter of the same name.
        language: Option<&str>,
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
            screen: loading::Screen::for_mode(assets, font.line_height, draw, language, mode),
            shell: None,
            media: None,
            race: Some(worker),
            build: None,
            built_race: None,
            music: Some(music),
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
        audio: &mut oag_sound::Audio,
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
        // `oag_display::space::Space`.
        let mut renderer = renderer;
        renderer.set_space(loaded.frontend.space());
        // And the face that renderer draws `Draw::Text` with is the one whose
        // ink the language picker's pointer bands are centred on - measured
        // here, beside the renderer, so the two cannot name different faces.
        loaded
            .frontend
            .set_row_ink(oag_ui::pointer::RowInk::measure(&loaded.font));
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
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn race(
        gpu: &impl GpuContext,
        loaded: race::Loaded,
        size: (u32, u32),
        anisotropy: Anisotropy,
        settings: &settings::Settings,
        render_profile: &settings::RenderProfile,
        scheme: ControlScheme,
        autopilot: bool,
        autopilot_pilot: Option<oag_ai::Pilot>,
        autopilot_skill: Option<oag_ai::Difficulty>,
        // The entry name `loaded` was actually built from, for
        // `oag_game::records::Key` - see `build_race_stage`'s own parameter
        // of the same name. Not on `race::Loaded` itself: that is the
        // *result* of a load, and the request is what a record has to be
        // keyed against, the same distinction `race::Options::track` and
        // `race::Loaded::title` already draw for "what was asked" against
        // "what came back".
        track_entry: Option<&str>,
    ) -> Result<Self> {
        Ok(Self::Race(Self::build_race_stage(
            gpu,
            loaded,
            size,
            anisotropy,
            settings,
            render_profile,
            scheme,
            autopilot,
            autopilot_pilot,
            autopilot_skill,
            track_entry,
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
        gpu: &impl GpuContext,
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
        // The one relocated `[render_profiles.<title>]` field this scene build
        // reads (`anti_aliasing`). Its own parameter rather than folded into
        // `settings` above: which title's entry applies is not this
        // function's question to answer - a caller with a `Session` resolves
        // it from `self.shell`'s title, one with none (the `--race` direct
        // launch, which never opens a `Shell`) falls back to
        // [`settings::RenderProfile::default`].
        render_profile: &settings::RenderProfile,
        scheme: ControlScheme,
        // `--autopilot`, its own parameter for the same reason `anisotropy` is:
        // it is a run flag rather than a stored preference, and nothing in the
        // settings file has any business turning it on.
        autopilot: bool,
        // `--autopilot-pilot`, already resolved to a `Pilot` by
        // `crate::args::autopilot_pilot` - roster lookup needs a filesystem
        // read this function has no business making, and an unknown name is
        // an error `main` reports before a window ever opens.
        autopilot_pilot: Option<oag_ai::Pilot>,
        // `--autopilot-skill`. `clap` has already validated the token against
        // `oag_ai::Difficulty`'s `FromStr`, so this is never a string to
        // reject, only a choice to apply or not.
        autopilot_skill: Option<oag_ai::Difficulty>,
        // The circuit `loaded` was actually requested with - `None` on the
        // one path `race::Options::track` itself can be `None` on. Read here
        // rather than re-derived from `loaded`: `race::Loaded` carries no
        // `.vex` entry name of its own, only the geometry it resolved to, and
        // `oag_game::records::Key` wants the disc's own path. See
        // `oag_game::records::Key::track`'s own doc.
        track_entry: Option<&str>,
    ) -> Result<Box<RaceStage>> {
        let race::Loaded {
            setup,
            title,
            hud,
            track_panel,
            track_model,
            liveries,
            collision_model,
            sky_model,
            pad_model,
            weapon_pad_model,
            gantry,
            rocket_model,
            mine_model,
            bomb_model,
            cannon_model,
            plasma_blast_models,
            bomb_blast_models,
            leach_ball_model,
            shield_cockpit,
            countdown_model,
            fog_volumes,
            light,
            authored_fog,
            hd_bloom,
            omega_tonemap,
            zone_grade,
            visibility,
            flare,
            leach_beam_texture,
            magstrip_wake_textures,
            noise,
            trail_blend,
            trail_shape,
            cannon_quad_textures,
            clouds,
            shadows,
            shadow_hulls,
            campaign_2048_event,
            track_stats,
            ghost_static,
            ripples,
            ..
        } = loaded;
        // Read before `setup` moves into `race::Race::start` below - `mode`
        // is `Copy` and would survive that move, but `class` would not.
        //
        // **`track_entry.or(Some(title.race.track))`, not `track_entry`
        // alone.** `race::Options::track` is `None` on the ordinary `--race`
        // invocation with no `--track` flag - confirmed live: a real
        // `--race --mode time_trial --autopilot` run with no `--track`
        // wrote `track = "(no circuit)"` before this fallback existed,
        // because `race::load` resolves its own default *internally* and
        // never hands the resolved name back to the caller. `title.race`
        // (`&'static oag_title::race::RaceDefaults`) is the same fallback
        // `race::load` itself falls back to - `RaceDefaults::track`'s own
        // doc calls it "archive entry of the `.vex` a race loads when the
        // caller names none" - so this reads the disc's own stated default
        // rather than re-deriving or inventing one. Not exact for a Zone
        // race, whose own further remap (`ZoneCircuit::variant_of`) lives in
        // `race::load` and is not replicated here - the record still lands
        // on a real, disc-authored circuit id rather than [`NO_CIRCUIT`],
        // just not always the exact zone variant `race::load` actually drew.
        let result_key = oag_game::records::Key::new(
            title.name,
            track_entry.or(Some(title.race.track)),
            setup.mode.name(),
            &setup.class,
        );
        // What the HUD's `RECORD` readout chases, read once here so the
        // standing best cannot move mid-race: the row this race will save to,
        // as it stood when the race started.
        let standing = oag_game::records::load();
        let standing = standing.get(&result_key);
        let record_target = oag_hud::RecordTarget::new(
            setup.mode,
            &setup.class,
            track_stats.as_ref(),
            standing.and_then(|record| record.best_total_ticks),
            standing.and_then(|record| record.best_lap_ticks),
        );
        let mut scene = race::Scene::new(
            gpu.device(),
            gpu.queue(),
            track_model,
            &liveries,
            collision_model,
            sky_model,
            pad_model,
            weapon_pad_model,
            gantry,
            setup.weapons_on(),
            rocket_model,
            mine_model,
            bomb_model,
            cannon_model,
            plasma_blast_models,
            bomb_blast_models,
            leach_ball_model,
            shield_cockpit,
            flare,
            leach_beam_texture,
            magstrip_wake_textures,
            noise,
            trail_blend,
            trail_shape,
            cannon_quad_textures,
            clouds,
            gpu.format(),
            size,
            anisotropy,
            visibility,
            render_profile.msaa,
            fog_volumes,
            light,
            authored_fog,
            hd_bloom,
            omega_tonemap,
            zone_grade,
            shadows,
            shadow_hulls,
        )?;
        scene.attach_ripples(ripples);
        scene.attach_mist(
            gpu.device(),
            gpu.queue(),
            gpu.format(),
            setup
                .scenery_fx
                .weather
                .as_ref()
                .and_then(|weather| weather.mist_texture.as_ref()),
        );
        // Time Trial and Speed Lap race a ghost - see `oag_game::ghosts`.
        scene.prepare_ghost(
            gpu.device(),
            gpu.queue(),
            gpu.format(),
            setup.mode,
            &liveries,
            ghost_static.as_ref(),
        );
        // Against the **surface** format, like every other renderer here, because
        // the HUD is composited into the offscreen target which shares it.
        let overlay = oag_game::hud_overlay::Overlay::new(
            gpu.device(),
            gpu.queue(),
            gpu.format(),
            &hud,
            settings.graphics.hud_scale,
        )
        .context("building the HUD overlay")?;
        // The results table, built from the same assets and drawn into the same
        // target - see `oag_game::scoreboard`, which is where the "this is ours,
        // the disc's own Race End chain is not built" argument lives. Built with
        // the race rather than when it ends: a load is the one moment a stage may
        // stall, and doing it at the finish would drop frames on the lap the
        // player is most likely to be watching.
        let scoreboard =
            oag_game::scoreboard::Overlay::new(gpu.device(), gpu.queue(), gpu.format(), &hud)
                .context("building the scoreboard overlay")?;
        // The countdown's own `<Mode3D>` model, when this mode's layout carries
        // one - see `oag_game::hud_countdown`. Built the same way `overlay`
        // just was, against the same surface format.
        let countdown = countdown_model
            .map(|(model, widget)| {
                oag_game::hud_countdown::Countdown::new(
                    gpu.device(),
                    gpu.queue(),
                    gpu.format(),
                    model,
                    &widget,
                )
            })
            .transpose()
            .context("building the countdown overlay")?;
        // The track-description panel the original lays over the flyby, built the same way.
        let track_panel = track_panel
            .map(|assets| {
                oag_game::track_panel::Overlay::new(gpu.device(), gpu.queue(), gpu.format(), assets)
            })
            .transpose()
            .context("building the track-description panel")?;
        // Read before `Race::start` takes `setup` - `--autopilot-skill`'s
        // fallback when the flag was not given.
        let difficulty = setup.difficulty;
        let (mode, seed) = (setup.mode, setup.seed);
        let mut race = race::Race::start(setup);
        race.set_boost_fov_kick(settings.graphics.boost_fov_kick);
        // The reticle projects through the same field the picture is drawn at.
        race.set_sight_fov(settings.graphics.fov);
        race.set_sight_screen(hud.space.size);
        race.set_sight_dialect(hud.art.sights);
        // Applied before the first tick, but unlike the kick this one is also
        // set again whenever the cycle button or the menu row moves it - see
        // `Session::cycle_camera_view`.
        race.set_camera_view(settings.graphics.camera_view);
        race.set_control_scheme(scheme);
        race.set_autopilot(autopilot);
        if autopilot {
            info!("--autopilot: the player's craft is being flown for them");
            let skill = autopilot_skill.unwrap_or(difficulty);
            if let Some(pilot) = autopilot_pilot {
                race.set_autopilot_pilot(pilot, skill);
            }
            if let Some(skill) = autopilot_skill {
                info!("--autopilot-skill: flying at {}", skill.name());
                race.set_autopilot_tuning(skill.tune(&oag_ai::Tuning::default()));
            }
        }
        crate::race_stage::ghost::arm(
            &mut race,
            &result_key,
            mode,
            liveries.first().map_or("", |l| l.team.as_str()),
            seed,
            oag_game::ghosts::race_options(scheme, difficulty, autopilot, None),
        );
        Ok(Box::new(RaceStage {
            scene,
            race,
            hud: overlay,
            scoreboard,
            countdown,
            track_panel,
            result_key,
            record_target,
            personal_best: None,
            result_saved: false,
            // Filled in by the caller once this stage is actually handed
            // off - see `Session::finish_loading`/`finish_race_loading`,
            // which drain `Session::campaign_cell` into here. Never known
            // here: this function has no `Session` to read it from.
            campaign_cell: None,
            campaign_difficulty: None,
            campaign_ai_skill_scale: None,
            // Not known until the leg itself finishes - see
            // `RaceStage::tournament_final_rank`'s own doc.
            tournament_final_rank: None,
            // Known here, unlike `campaign_cell` above: `load_event` already
            // resolved it onto `Loaded` itself, since it has the parsed
            // `SP.xml` `Document` in hand at exactly that point - see
            // `race::Loaded::campaign_2048_event`'s own doc.
            campaign_2048_event,
            // Built lazily once the race finishes - see
            // `crate::main::session::endrace` and `RaceStage::endrace`'s own
            // doc. Never known here for the same reason `campaign_cell`
            // isn't: this function has no `Session` to read a `Shell` from.
            endrace: None,
            // Nothing has been tried yet, so nothing has failed yet - see
            // `RaceStage::endrace_unavailable`.
            endrace_unavailable: false,
            earned_medal: None,
        }))
    }
}
