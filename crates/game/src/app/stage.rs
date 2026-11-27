//! What is on screen, as one enum over the four things it can be.
//!
//! [`Stage`] itself is only the switch; each variant is a module of its own
//! beside this one - [`crate::app::menu_stage`], [`crate::app::loading_stage`],
//! [`crate::app::frontend_stage`] and [`crate::app::race_stage`] - because a stage owns
//! its own assets and its own update. They are siblings rather than children
//! named `stage::menu` and the rest: the library already owns `menu`,
//! `loading`, `frontend` and `race`, and a child module of any of those names
//! shadows the import the body right here needs.

use anyhow::{Context, Result};
use log::info;

use crate::render::{Renderer, VideoFormat};
use crate::{boot, loading, movie, settings};
use oag_gameplay::ControlScheme;
use oag_mesh::mesh_render::Anisotropy;
use oag_raceplay as race;

use crate::app::frontend_stage::{FrontendStage, PendingMovie};
use crate::app::gpu::{Gpu, GpuContext};
use crate::app::launcher_stage::LauncherStage;
use crate::app::loading_stage::LoadingStage;
use crate::app::menu_stage::MenuStage;
use crate::app::race_stage::RaceStage;

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
    pub(crate) fn launcher(gpu: &Gpu, launcher: crate::launcher::Launcher) -> Result<Self> {
        let renderer = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.output.config.format,
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
        media: Option<boot::worker::MediaWorker>,
        assets: &loading::Assets,
        trace: bool,
        // The player's chosen language, so this screen's own invented prose
        // can be overridden the same way the disc's own strings already are
        // elsewhere - see `loading::Screen::new`. Both callers have a
        // `Settings` open already; this screen goes up before any disc's own
        // language plugin does, so `Settings::language` is the only source
        // there is.
        language: Option<&str>,
        // `[graphics] hud_safe_area`: the screen is drawn inside the title's `Safe Area
        // Setting`, as the original draws it.
        hud_safe_area: Option<u8>,
    ) -> Result<Self> {
        // The **disc's** font, not the built-in 5x7 set: the tips are the
        // disc's own prose in the player's own language, and `push_text` skips
        // a glyph the atlas has no cell for - so accented text would silently
        // lose characters rather than fail. No video pipeline, this screen
        // having no movie; there is no loading movie in either build.
        let mut renderer = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.output.config.format,
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
        renderer.set_face_atlas(
            &gpu.device,
            &gpu.queue,
            shell.title_font.clone(),
            shell.title_font.as_ref().map(|_| "Title"),
        );
        let safe_area = shell.title.hud_art.safe_area_scale(hud_safe_area);
        let mut screen = loading::Screen::new(assets, line_height, 0, language);
        screen.set_safe_area(safe_area);
        renderer.set_space(screen.space());
        // Sample count 1, matching `upscale::Framebuffer`'s target, which is
        // what this draws into.
        let wave = oag_render::loading::Pipeline::new(
            &gpu.device,
            &gpu.queue,
            gpu.output.config.format,
            &assets.strip,
            1,
        );
        Ok(Self::Loading(Box::new(LoadingStage {
            atlas: shell.font.clone(),
            renderer,
            wave,
            // Seed 0 on the boot screen: it goes up once per run, and the
            // title that has features does not draw this one anyway.
            screen,
            shell: Some(shell),
            media,
            race: None,
            build: None,
            built_race: None,
            music: None,
            trace,
            icon: None,
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
        music: Option<oag_sound::MusicFetchWorker>,
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
        // The title's `Title`-role face, which a layout's caption is drawn in, and the
        // title's Safe Area for the player's `hud_safe_area`. See `Stage::loading`.
        title_font: Option<&oag_ui::font::Atlas>,
        safe_area: f32,
        // What the track being raced supplies of its own loading screen, when it is a track
        // file with a `[loading]` table.
        track: Option<&loading::TrackLoading>,
        // The card the race was launched from, drawn behind the box on a title that does.
        card: Option<oag_ui::frontend::LaunchedCard>,
    ) -> Result<Self> {
        // The mode icon's clip, where the title maps one for this mode: the
        // renderer's planes are sized from its header, and the clip itself is
        // opened on a thread of its own (`crate::clip::Player`).
        let clip = loading::icon::clip(assets, mode);
        // The track's own picture goes under the title's sheet when it supplies one.
        let own_sheet = track.and_then(|track| track.sheet(assets));
        let mut renderer = Renderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.output.config.format,
            clip.and_then(loading::icon::format),
            font.clone(),
            // The illustration's own sheet where the title ships one; see
            // `Stage::loading`, which chooses the same way and for the same
            // reason.
            own_sheet
                .as_ref()
                .map(|(sheet, _)| sheet)
                .or(assets.art.as_ref().map(|art| &art.sheet))
                .unwrap_or(sprites),
        )?;
        renderer.set_face_atlas(
            &gpu.device,
            &gpu.queue,
            title_font.cloned(),
            title_font.map(|_| "Title"),
        );
        let mut screen = loading::Screen::for_mode(assets, font.line_height, draw, language, mode);
        screen.set_safe_area(safe_area);
        match card {
            Some(card) => screen.set_card(card, assets.game_tips.as_ref(), draw),
            None if assets.game_tips.is_some() => log::debug!(
                "race load: this race was not launched from an event card, so the box and the \
                 meter draw over the ground and no card is made up"
            ),
            None => {}
        }
        if let Some(track) = track {
            for note in &track.notes {
                log::warn!("{note}");
            }
            screen.apply_track(track, own_sheet.as_ref().map(|(_, rect)| *rect));
        }
        renderer.set_space(screen.space());
        let wave = oag_render::loading::Pipeline::new(
            &gpu.device,
            &gpu.queue,
            gpu.output.config.format,
            &assets.strip,
            1,
        );
        Ok(Self::Loading(Box::new(LoadingStage {
            atlas: font.clone(),
            renderer,
            wave,
            screen,
            shell: None,
            media: None,
            race: Some(worker),
            build: None,
            built_race: None,
            music,
            trace,
            icon: clip
                .filter(|clip| loading::icon::format(clip).is_some())
                .map(|clip| {
                    let clip = clip.clone();
                    let cache = oag_source::cache::default_cache_dir();
                    // Looped: the original's panel plays the clip over and over for
                    // as long as the load takes (two RPCS3 grabs 2 s apart are on
                    // different beats of it).
                    crate::clip::Player::spawn("loading screen mode icon", move || {
                        loading::icon::open(&clip, &cache)
                    })
                }),
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
        let mut renderer = frontend_renderer(
            &gpu.device,
            &gpu.queue,
            gpu.output.config.format,
            video_format,
            &FrontendFaces {
                font: &loaded.font,
                sprites: &loaded.sprites,
                skin: loaded.menu_skin,
                menu_font: loaded.menu_font.as_ref(),
                title_font: loaded.title_font.as_ref(),
                buttons_font: loaded.buttons_font.as_ref(),
            },
        )?;
        // The sequence's rects are in its source's grid, and the renderer maps
        // rects onto the viewport - so it has to be told which grid, or a PS2
        // screen's widgets are drawn a third again too big. See
        // `oag_display::space::Space`.
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
        loaded
            .frontend
            .set_option_defaults(crate::settings::option_defaults());
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
            team_hull: None,
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
    ///
    /// Never inlined: the `RaceStage` it builds (about 350 KiB) and its
    /// temporaries sit on the stack before they are boxed, and inlined they
    /// would sit in `App::open_with`'s frame on every leg, the browser's front
    /// end included, which never takes this one (`docs/tools/web.md`, "The
    /// page's stack").
    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    pub(crate) fn race(
        gpu: &Gpu,
        loaded: Box<race::Loaded>,
        size: (u32, u32),
        anisotropy: Anisotropy,
        settings: &settings::Settings,
        render_profile: &settings::RenderProfile,
        scheme: ControlScheme,
        autopilot: bool,
        autopilot_pilot: Option<oag_ai::Pilot>,
        autopilot_skill: Option<oag_ai::Difficulty>,
        // The entry name `loaded` was actually built from, for
        // `crate::records::Key` - see `build_race_stage`'s own parameter
        // of the same name. Not on `race::Loaded` itself: that is the
        // *result* of a load, and the request is what a record has to be
        // keyed against, the same distinction `race::Options::track` and
        // `race::Loaded::title` already draw for "what was asked" against
        // "what came back".
        track_entry: Option<&str>,
    ) -> Result<Self> {
        // The scene's shaders are compiled below this call, and `naga`'s parser recurses deeply
        // enough that the 2 MiB `android_main` thread overflowed under the frames already
        // above it (`docs/tools/android.md`, "The stack"). Every `--race` builds on a stack
        // sized for that, so no platform runs a path another one does not.
        let handles = gpu.handles();
        // Boxed all the way from `App::race`, so neither this frame, the closure nor the
        // copies of it the thread's start makes carry the 62 KB it points at.
        let build = || {
            Self::build_race_stage(
                &handles,
                *loaded,
                size,
                anisotropy,
                settings,
                render_profile,
                scheme,
                autopilot,
                autopilot_pilot,
                autopilot_skill,
                track_entry,
            )
        };
        #[cfg(not(target_arch = "wasm32"))]
        let built = oag_thread::on_big_stack("race-stage", build)
            .context("starting the race stage's thread")??;
        #[cfg(target_arch = "wasm32")]
        let built = build()?;
        Ok(Self::Race(built))
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
        // `crate::app::args::autopilot_pilot` - roster lookup needs a filesystem
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
        // `crate::records::Key` wants the disc's own path. See
        // `crate::records::Key::track`'s own doc.
        track_entry: Option<&str>,
    ) -> Result<Box<RaceStage>> {
        let race::Loaded {
            setup,
            view,
            modules,
            title,
            hud,
            track_panel,
            demo_overlay,
            track_model,
            behind_glass,
            liveries,
            collision_model,
            sky_model,
            pad_model,
            weapon_pad_model,
            billboards,
            rocket_model,
            mine_model,
            bomb_model,
            cannon_model,
            plasma_blast_models,
            bomb_blast_models,
            leach_ball_model,
            shield_cockpit,
            countdown_model,
            medal_models,
            fog_volumes,
            light,
            circuit_lighting,
            probes,
            authored_fog,
            hd_bloom,
            omega_tonemap,
            wo_composite,
            zone_grade,
            visibility,
            flare,
            leach_beam_texture,
            ribbon_textures,
            noise,
            trail_blend,
            trail_shape,
            cannon_quad_textures,
            clouds,
            shadows,
            campaign_2048_event,
            track_stats,
            ghost_static,
            ripples,
            started,
            circuit,
            ..
        } = loaded;
        // Read before `setup` moves into `race::Race::start` below - `mode`
        // is `Copy` and would survive that move, but `class` would not.
        //
        // A run that named no circuit files under the one `race::load` resolved
        // (`Loaded::circuit`): the title's default, or the first circuit the
        // source ships when the default is not in it. A foreign track is its own
        // circuit: filed under the disc's default it would take that circuit's
        // best lap and ghost.
        let foreign_key = setup.foreign.as_ref().map(|id| format!("foreign:{id}"));
        let result_key = crate::records::Key::raced(
            title.name,
            foreign_key.as_deref().or(track_entry),
            &circuit,
            setup.mode.name(),
            &setup.class,
        );
        // What the HUD's `RECORD` readout chases, read once here so the
        // standing best cannot move mid-race: the row this race will save to,
        // as it stood when the race started.
        let standing = crate::records::load();
        let standing = standing.get(&result_key);
        let record_target = oag_hud::RecordTarget::new(
            setup.mode,
            &setup.class,
            track_stats.as_ref(),
            standing.and_then(|record| record.best_total_ticks),
            standing.and_then(|record| record.best_lap_ticks),
        );
        let mut scene = Box::new(race::Scene::new(
            gpu.device(),
            gpu.queue(),
            track_model,
            &liveries,
            collision_model,
            sky_model,
            pad_model,
            weapon_pad_model,
            billboards,
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
            ribbon_textures,
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
            circuit_lighting,
            probes,
            authored_fog,
            hd_bloom,
            omega_tonemap,
            wo_composite,
            zone_grade,
            shadows,
            behind_glass,
        )?);
        scene.attach_ripples(ripples);
        scene.attach_mist(
            gpu.device(),
            gpu.queue(),
            gpu.format(),
            view.scenery_fx
                .weather
                .as_ref()
                .and_then(|weather| weather.mist_texture.as_ref()),
        );
        scene.attach_bloom_order(view.scenery_fx.bloom_under_hud);
        scene.attach_lens_flare(
            gpu.device(),
            gpu.queue(),
            gpu.format(),
            view.scenery_fx.lens_flare.as_deref(),
        );
        scene.attach_mod_shaders(gpu.device(), gpu.queue(), view.handles.shaders().list());
        // Time Trial and Speed Lap race a ghost - see `crate::ghosts`.
        scene.prepare_ghost(
            gpu.device(),
            gpu.queue(),
            gpu.format(),
            setup.mode,
            &liveries,
            ghost_static.as_ref(),
        );
        // Each overlay is built in a function of its own, so the copies a `?`, a
        // `.context()` and a `.map()` make of it in a debug build live in a frame
        // that is gone before the next one is built, and this one holds a pointer.
        let overlay = hud_overlay(gpu, &hud, settings)?;
        let scoreboard = scoreboard_overlay(gpu, &hud)?;
        let countdown = countdown_overlay(gpu, countdown_model)?;
        let medals = (!medal_models.is_empty()).then(|| {
            Box::new(crate::hud_medal::Medals::new(
                gpu.device(),
                gpu.queue(),
                gpu.format(),
                medal_models,
            ))
        });
        let track_panel = track_panel_overlay(gpu, track_panel)?;
        let demo_overlay = demo_screen_overlay(gpu, demo_overlay)?;
        // Read before `Race::start` takes `setup` - `--autopilot-skill`'s
        // fallback when the flag was not given.
        let difficulty = setup.difficulty;
        let (mode, seed) = (setup.mode, setup.seed);
        // Photo mode's rewind starts a second race from this when it is first used.
        let keeper_setup =
            crate::photo::WRITES.then(|| (setup.clone(), view.clone(), modules.clone()));
        // Started on the web's load worker already, from a copy of the same
        // setup (`race::Loaded::started`).
        let mut race = match started {
            Some(race) => race,
            None => Box::new(race::Race::start(setup, view, modules)),
        };
        race.set_boost_fov_kick(settings.graphics.boost_fov_kick);
        // The reticle projects through the same field the picture is drawn at.
        race.set_sight_fov(settings.graphics.fov);
        race.watch_laps(&hud, standing.and_then(|record| record.best_lap_ticks));
        // Applied before the first tick, but unlike the kick this one is also
        // set again whenever the cycle button or the menu row moves it - see
        // `Session::cycle_camera_view`.
        race.set_camera_view(settings.graphics.camera_view);
        let launch = race::Launch {
            scheme,
            difficulty,
            autopilot,
            pilot_assist: settings.controls.pilot_assist,
            original_ai: None,
            campaign_skill: None,
            skill_level: None,
        };
        race.core.sim.apply_launch(&launch);
        if let Some(progress) = campaign_2048_event.as_ref() {
            race.core.sim.arm_event_xp(progress);
        }
        if autopilot {
            info!("--autopilot: the player's craft is being flown for them");
            let skill = autopilot_skill.unwrap_or(difficulty);
            if let Some(pilot) = autopilot_pilot {
                race.core.sim.set_autopilot_pilot(pilot, skill);
            }
            if let Some(skill) = autopilot_skill {
                info!("--autopilot-skill: flying at {}", skill.name());
                race.core
                    .sim
                    .set_autopilot_tuning(skill.tune(&oag_ai::Tuning::default()));
            }
        }
        race.set_ghost_visible(settings.display.ghost_ship_shown(title));
        race.set_player_tags(settings.display.multiplayer_tags_shown(title));
        crate::split_screen::name_first_seat(&mut race, &settings.player, &title.profile);
        let team = liveries.first().map_or("", |l| l.team.as_str());
        let replay_header =
            crate::app::race_stage::ghost::header_for(&result_key, team, seed, launch.to_table());
        crate::app::race_stage::ghost::arm(
            &mut race,
            &result_key,
            mode,
            settings.display.ghost_auto_loads(title),
            team,
            seed,
            launch.to_table(),
        );
        let keeper = keeper_setup.map(|(setup, view, modules)| {
            Box::new(race::timeline::Keeper::new(setup, view, modules, &race))
        });
        crate::texture_cache::report_load();
        Ok(Box::new(RaceStage {
            scene,
            race,
            keeper,
            hud: overlay,
            scoreboard,
            countdown,
            medals,
            resume_art: None,
            resume_go: String::new(),
            track_panel,
            demo_overlay,
            circuit: circuit.clone(),
            result_key,
            record_target,
            personal_best: None,
            score_record: None,
            result_saved: false,
            // Filled in by the caller once this stage is actually handed
            // off - see `Session::finish_loading`/`finish_race_loading`,
            // which drain `Session::campaign_cell` into here. Never known
            // here: this function has no `Session` to read it from.
            campaign_cell: None,
            title,
            campaign_custom: None,
            campaign_difficulty: None,
            campaign_ai_skill_scale: None,
            // Not known until the leg itself finishes - see
            // `RaceStage::tournament_final_rank`'s own doc.
            tournament_final_rank: None,
            shared_frame_work: false,
            // Known here, unlike `campaign_cell` above: `load_event` already
            // resolved it onto `Loaded` itself, since it has the parsed
            // `SP.xml` `Document` in hand at exactly that point - see
            // `race::Loaded::campaign_2048_event`'s own doc.
            campaign_2048_event,
            xp_banked: None,
            // Built lazily once the race finishes - see
            // `crate::app::session::endrace` and `RaceStage::endrace`'s own
            // doc. Never known here for the same reason `campaign_cell`
            // isn't: this function has no `Session` to read a `Shell` from.
            endrace: None,
            // Nothing has been tried yet, so nothing has failed yet - see
            // `RaceStage::endrace_unavailable`.
            endrace_unavailable: false,
            endrace_prefetch: None,
            endrace_prefetch_started: false,
            endrace_cancel: std::sync::Arc::default(),
            earned_medal: None,
            replay_header,
            auto_replay: false,
            watching: false,
            demo: None,
            benchmark: None,
        }))
    }
}

fn hud_overlay(
    gpu: &impl GpuContext,
    hud: &oag_hud::Assets,
    settings: &settings::Settings,
) -> Result<Option<Box<crate::hud_overlay::Overlay>>> {
    // Against the **surface** format, like every other renderer here, because
    // the HUD is composited into the offscreen target which shares it.
    Ok(crate::hud_overlay::Overlay::new(
        gpu.device(),
        gpu.queue(),
        gpu.format(),
        hud,
        settings.graphics.hud_scale,
    )
    .context("building the HUD overlay")?
    .map(|mut overlay| {
        overlay.set_dynamic_hud(
            settings.display.dynamic_hud,
            settings.graphics.camera_view == oag_display::display::CameraView::Internal,
        );
        Box::new(overlay.with_safe_area(settings.graphics.hud_safe_area))
    }))
}

/// The results table, built from the same assets and drawn into the same
/// target - see `crate::scoreboard`. Built with the race rather than when it
/// ends: a load is the one moment a stage may stall, and doing it at the finish
/// would drop frames on the lap the player is most likely to be watching.
fn scoreboard_overlay(
    gpu: &impl GpuContext,
    hud: &oag_hud::Assets,
) -> Result<Box<crate::scoreboard::Overlay>> {
    Ok(Box::new(
        crate::scoreboard::Overlay::new(gpu.device(), gpu.queue(), gpu.format(), hud)
            .context("building the scoreboard overlay")?,
    ))
}

/// The countdown's own `<Mode3D>` model, when this mode's layout carries one -
/// see `crate::hud_countdown`.
fn countdown_overlay(
    gpu: &impl GpuContext,
    model: Option<(oag_mesh::mesh::Model, oag_hud::Model)>,
) -> Result<Option<Box<crate::hud_countdown::Countdown>>> {
    model
        .map(|(model, widget)| {
            crate::hud_countdown::Countdown::new(
                gpu.device(),
                gpu.queue(),
                gpu.format(),
                model,
                &widget,
            )
            .map(Box::new)
        })
        .transpose()
        .context("building the countdown overlay")
}

/// The track-description panel the original lays over the flyby.
fn track_panel_overlay(
    gpu: &impl GpuContext,
    assets: Option<race::track_panel::Assets>,
) -> Result<Option<Box<crate::track_panel::Overlay>>> {
    assets
        .map(|assets| {
            crate::track_panel::Overlay::new(gpu.device(), gpu.queue(), gpu.format(), assets)
                .map(Box::new)
        })
        .transpose()
        .context("building the track-description panel")
}

/// The attract demo's screen.
fn demo_screen_overlay(
    gpu: &impl GpuContext,
    assets: Option<race::demo_overlay::Assets>,
) -> Result<Option<Box<crate::demo_overlay::Overlay>>> {
    assets
        .map(|assets| {
            crate::demo_overlay::Overlay::new(gpu.device(), gpu.queue(), gpu.format(), &assets)
                .map(Box::new)
        })
        .transpose()
        .context("building the demo screen")
}

/// What the boot sequence's renderer is built from: [`boot::Boot`]'s font, sprites, menu skin and
/// the two optional faces.
pub(crate) struct FrontendFaces<'a> {
    pub(crate) font: &'a oag_ui::font::Atlas,
    pub(crate) sprites: &'a oag_hud::sprite::Sheet,
    pub(crate) skin: &'static oag_title::MenuSkin,
    pub(crate) menu_font: Option<&'a oag_ui::font::Atlas>,
    pub(crate) title_font: Option<&'a oag_ui::font::Atlas>,
    pub(crate) buttons_font: Option<&'a oag_ui::font::Atlas>,
}

/// The renderer of the boot sequence, with both glyph slots the title's front end draws with
/// (`boot::fonts::install_faces`, the call `--screen`, the menus and a language switch make too),
/// so the live title screen draws the faces and pens a capture of it does.
pub(crate) fn frontend_renderer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    video_format: Option<VideoFormat>,
    faces: &FrontendFaces<'_>,
) -> Result<Renderer> {
    let mut renderer = Renderer::new(
        device,
        queue,
        format,
        video_format,
        faces.font.clone(),
        faces.sprites,
    )?;
    boot::fonts::install_faces(
        &mut renderer,
        device,
        queue,
        faces.skin,
        faces.font,
        faces.title_font.cloned(),
        faces.buttons_font.cloned(),
    );
    boot::fonts::install_boot_menu_face(&mut renderer, device, queue, faces.skin, faces.menu_font);
    Ok(renderer)
}

#[cfg(test)]
mod tests;
