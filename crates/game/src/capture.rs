//! Runs the boot sequence headless and writes one frame to a PNG.
//!
//! It works over SSH and in CI, shows the boot sequence and the menu without a display, and goes
//! through **the same** renderer the window does, so what it captures is what the window draws.
//! A separate capture path would prove nothing.

use anyhow::{Context, Result};
use log::debug;
use oag_mesh::mesh_render::Anisotropy;

use crate::boot::Boot;
use crate::input::Input;
use crate::render::{Renderer, VideoFormat};
use oag_raceplay as race;
use oag_ui::frontend::Draw;
mod campaign_page;
mod card;
mod endrace_page;
mod endrace_touch_page;
mod loading;
mod menu_page;
mod offscreen;
mod presented;
use campaign_page::{campaign_kind, campaign_page};
use endrace_page::endrace_kind;
pub use loading::{LoadingOptions, draw_wave, loading};
use menu_page::{
    PreviewRequest, draw_preview, menu_page, open_for_previews, picker_kind, picker_page,
    picker_stills,
};
use offscreen::{offscreen, read_back, touch_preview, write_png};

/// What to capture.
#[derive(Debug, Clone)]
pub struct Options {
    /// Where to write the PNG.
    pub path: std::path::PathBuf,
    /// Capture the frame the way a window presents it - through the render
    /// scale, the upscaler, the grade and the aspect bars.
    ///
    /// On the race hand-off, the whole path. On the front end there is no 3D
    /// scene, so no render scale and no upscaler - since
    /// [ADR-0038](../../../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)
    /// that is correct rather than a gap - and what this puts in the way is
    /// what a windowed front-end frame gets from `Framebuffer::composite`:
    /// the grade and the screen filter. See `capture::presented`.
    pub presented: bool,
    /// Run until this state is current, then capture.
    pub until: Option<String>,
    /// Run at least this many ticks first.
    ///
    /// A run that reaches `Launch Game` with a race to hand off to spends what is
    /// left of them on the race instead.
    pub ticks: u32,
    /// Pins the animation clock, in seconds, instead of deriving it from the
    /// tick. `None` derives it. See `Cli::anim_seconds`.
    pub anim_seconds: Option<f32>,
    /// Plays this static path of the Fury menu backdrop instead of the
    /// picker's roll, with [`Self::anim_seconds`] as the time into it - the
    /// pair that lines a `--menu-page` capture up with an RPCS3 frame whose
    /// path and clock were read. See `Cli::fury_path`.
    pub fury_path: Option<usize>,
    /// Offset the camera by a sub-pixel each frame, on the race this hands off
    /// to. `--camera-jitter`; see `crate::race_capture::CaptureOptions::camera_jitter`.
    ///
    /// Carried rather than dropped because `--screenshot` reaching a race
    /// through `Launch Game` is one of the two ways a capture gets a race at
    /// all, and a flag honoured on one route and silently ignored on the other
    /// is how a comparison ends up differing by something nobody named. It
    /// reaches nothing on the front end itself: no stage there draws a 3D
    /// scene, per ADR-0038.
    pub camera_jitter: bool,
    /// Buttons held on every tick.
    pub held: u32,
    /// Buttons pressed and released on alternating ticks.
    ///
    /// A held button only produces one rising edge, so reaching a state that
    /// needs two presses - skip the intro, then pick a language - needs the
    /// button to be let go of in between.
    pub pressed: u32,
    /// Which control scheme the race this hands off to is driven with.
    pub scheme: oag_gameplay::ControlScheme,
    /// Print exits as well as entries.
    pub trace: bool,
    /// The race `Launch Game` hands off to, if this capture should follow it
    /// there.
    ///
    /// `None` captures the front end and nothing else, which is what a run that
    /// never reaches `Launch Game` does anyway.
    pub race: Option<race::Options>,
    /// `--event`: the 2048 campaign event a `--menu-page endrace-summary` races for its result.
    pub event: Option<String>,
    /// With that handoff, print a telemetry line every this many ticks.
    pub log_every: u32,
    /// `--give`: keep the player's pickup slot topped up. See
    /// `crate::race_capture::CaptureOptions::give`.
    pub give: Option<oag_tables::weapons::Weapon>,
    /// `--autopilot`: fly the race this hands off to with an opponent's
    /// driver. See `crate::race_capture::CaptureOptions::autopilot`.
    pub autopilot: bool,
    /// `--autopilot-pilot`. See `crate::race_capture::CaptureOptions::autopilot_pilot`.
    pub autopilot_pilot: Option<oag_ai::Pilot>,
    /// `--autopilot-skill`. See `crate::race_capture::CaptureOptions::autopilot_skill`.
    pub autopilot_skill: Option<oag_ai::Difficulty>,
    /// Image size.
    pub size: (u32, u32),
    /// Render one named screen straight out of the XML and stop.
    ///
    /// A debugging view, not a step of the sequence: it does not run the state
    /// machine, take input or advance the movie. It exists so a screen the boot
    /// order does not reach yet - most of them - can still be looked at. See
    /// [`oag_ui::frontend::Frontend::draw_screen`].
    pub screen: Option<String>,
    /// With [`Options::screen`], seconds since it appeared - `None` draws it
    /// settled. `Title Screen`'s own `<Animation><Key>` wipe and `Show
    /// Logo`'s `pulse="true"` throb both need a clock to show anything but
    /// their own settled end state; see
    /// [`oag_ui::frontend::Frontend::draw_screen_at`].
    pub screen_seconds: Option<f32>,
    /// Anisotropic filtering level, only relevant if the handoff to
    /// [`Options::race`] happens.
    pub anisotropy: Anisotropy,
    /// Draw one page of **our own** menus instead of the sequence.
    ///
    /// The same kind of debugging view [`Options::screen`] is, for the other
    /// tree: a page id from `assets/ui/menu.toml`. It exists so a menu can be
    /// looked at without launching the game and walking to it, which is what
    /// iterating on a layout otherwise costs. Takes no input and runs no state
    /// machine.
    pub menu_page: Option<String>,
    /// How far into arriving that page is, `0..=1`. `None` draws it settled.
    ///
    /// See the CLI flag's own docs for why a still needs this at all.
    pub menu_anim_phase: Option<f32>,
    /// With `--menu-page track-select`/`ship-select`, seconds since the
    /// screen opened - `None` draws it settled. See the CLI flag's own docs;
    /// `--menu-anim-phase`'s equivalent for the race box's own two screens,
    /// which are not `assets/ui/menu.toml` pages and so do not reach it.
    pub menu_picker_seconds: Option<f32>,
    /// Which modal prompt to draw over that page: `rename`, `rename-note`,
    /// `delete` or `delete-built-in`.
    ///
    /// The same argument [`Self::menu_anim_phase`] makes, one step stronger. A
    /// prompt exists because a row was *activated*, and this path runs no
    /// state machine and calls no `Menu::update` - so the on-screen keyboard
    /// can never appear here on its own, and without this its layout is
    /// reviewable only by playing the game on a machine with a display.
    pub menu_prompt: Option<String>,
    /// The persisted settings, so `--menu-page` draws the rows a player would
    /// see rather than each list's first entry.
    pub settings: crate::settings::Settings,
    /// Which Pulse releases this machine has, so the MUSIC SOURCE row draws
    /// the same way it would in a live menu: with three values on a machine
    /// that has both discs, and empty on one that does not.
    pub music_discs: oag_sound::MusicDiscs,
}

/// How many ticks the runner will take before giving up on `until`.
///
/// The boot movie is forty seconds, and the `--reel` leg is eight plus three
/// two-second holds, so a minute of simulated time covers either and is still
/// bounded. **That arithmetic only holds under the tick clock.** On a machine
/// with a real audio device, an `--until` capture with nothing skipping the
/// movie is audio-clocked per ADR-0019 (see
/// `oag_sound::Audio::movie_playhead`): the headless loop runs far faster
/// than real time, so the movie's real-time position barely advances inside
/// this many ticks and the run never leaves `LogoFMV`. Confirmed directly -
/// the same capture that fails here reaches `Language Selection` in 2,403
/// ticks with `--no-audio`, matching this arithmetic. Pass `--no-audio` (or
/// run somewhere with no device, which is what CI does) for a capture whose
/// `--until` target is only reached by letting a movie play out; one that
/// also holds or presses a skip button reaches its target long before the
/// movie's own clock matters. Raising this ceiling would not fix an
/// audio-clocked run - it would only make every capture slower.
const MAX_TICKS: u32 = 60 * 60;

/// Runs the sequence and writes one frame.
///
/// `audio` is stepped once per simulation tick, in the same loop the front end
/// is stepped in. It is threaded through rather than made here because the
/// handoff to [`crate::race_capture::capture`] continues into the *same* buffer: a
/// `--dump-audio` run that reaches `Launch Game` would otherwise lose whichever
/// leg made its own. The caller writes the file once, after both.
pub fn run(
    loaded: Boot,
    video_format: Option<VideoFormat>,
    options: &Options,
    audio: &mut oag_sound::Audio,
) -> Result<()> {
    let Boot {
        title,
        platform,
        languages,
        strings,
        tracks,
        teams,
        race_setup,
        mut frontend,
        movie,
        movie_sound,
        after_language_movie,
        after_language_movie_sound,
        backdrop,
        font,
        // `mut`: the track-select still extends it with the circuit's own
        // cards before the renderer is built from it, below.
        mut sprites,
        menu_skin,
        // Renamed on the way in: `frame` is taken in this function by the
        // *movie* frame a backdrop is showing, and two things called `frame` in
        // one scope is how the wrong one gets passed.
        frame: menu_frame,
        nav_legend,
        ticker,
        fury_backdrop,
        track_select,
        ship_select,
        circuit_names,
        menu_font,
        title_font,
        buttons_font,
        face_scales,
        ..
    } = loaded;

    // Checked before anything is printed or stepped: a name that matches
    // nothing would otherwise draw the backdrop and nothing else, which looks
    // exactly like a screen that is empty. The list of what does exist is short
    // enough to just print.
    if let Some(name) = &options.screen
        && !frontend
            .screens()
            .screens
            .iter()
            .any(|s| s.name == *name || s.path == *name)
    {
        anyhow::bail!(
            "no screen named {name:?}; this XML has {}",
            frontend
                .screens()
                .screens
                .iter()
                .map(|s| format!("{:?}", s.path))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let dt = 1.0 / 60.0;
    let mut input = Input::new();
    let mut ticks = 0u32;

    // Each movie's sound keyed to the screen that plays it, the same way
    // `App::tick` installs them - kept in step with that loop deliberately, since
    // a movie sounding on one path and not the other is the divergence
    // `movie_playhead`'s own doc comment warns about.
    //
    // The boot screen gets no `Enter` event, being where the machine starts, so
    // its own movie's sound (if it has one) starts here. On Pulse that is the
    // intro; on Pure the boot screen is the picker and plays nothing.
    let movie_states = frontend.movie_states();
    let mut pending_sound: Vec<(&'static str, Option<oag_music::at3::Pcm>)> = Vec::new();
    for (at, sound) in [(0usize, movie_sound), (1, after_language_movie_sound)] {
        if let Some(state) = movie_states.get(at).copied() {
            pending_sound.push((state, sound));
        }
    }
    if let Some(current) = frontend.machine().current().map(str::to_string)
        && let Some(at) = pending_sound
            .iter()
            .position(|(state, _)| *state == current)
    {
        let (state, sound) = pending_sound.remove(at);
        audio.start_boot_movie(state, sound);
    }

    // Those two draw one thing and step nothing, so no movie ever ends to hand
    // the music its cue the way the loop below does. A `--dump-audio` run of
    // either would otherwise be silence.
    if options.screen.is_some() || options.menu_page.is_some() {
        audio.start_music(
            &options.music_discs,
            options.settings.audio.music_source,
            &oag_source::cache::default_audio_cache_dir(),
        );
    }

    // `--screen` and `--menu-page` both draw one thing and nothing else, so the
    // sequence is not run at all: stepping it would only move the state machine
    // somewhere the capture then ignores.
    let mut card = card::CardTarget::parse(options.until.as_deref());
    while options.screen.is_none() && options.menu_page.is_none() {
        // `Launch Game` ends the front end's leg whatever `until` and `ticks` say,
        // so the ticks they asked for are spent on the race rather than on a state
        // whose whole content is the word LAUNCH GAME.
        if options.race.is_some() && frontend.is_finished() {
            break;
        }

        let reached = match &card {
            Some(card) => card.reached(&frontend),
            None => options
                .until
                .as_deref()
                .is_some_and(|name| frontend.machine().is(name)),
        };
        if reached && ticks >= options.ticks {
            break;
        }
        if options.until.is_none() && ticks >= options.ticks {
            break;
        }
        if ticks >= MAX_TICKS {
            if let Some(name) = &options.until {
                anyhow::bail!(
                    "never reached {name:?} in {MAX_TICKS} ticks; got as far as {:?}",
                    frontend.machine().current().unwrap_or("nothing")
                );
            }
            break;
        }

        let pressed = match card.as_mut() {
            Some(card) => card.step(&mut frontend, options.pressed)?,
            None => options.pressed,
        };
        let pulse = if ticks.is_multiple_of(2) { pressed } else { 0 };
        input.begin_frame(options.held | pulse);
        // The playhead is read **before** the mixer is advanced, so it is where
        // the sound had got to at the end of the previous tick - the last
        // moment it is a measurement rather than a prediction. See
        // `movie::Player::follow`.
        let events = frontend.update(dt, &mut input, audio.movie_playhead());
        // Every entered screen offered its own movie's track, the same handover
        // `App::tick` does.
        for event in &events {
            if let oag_ui::state_machine::Event::Enter(name) = event
                && let Some(at) = pending_sound.iter().position(|(state, _)| state == name)
            {
                let (state, sound) = pending_sound.remove(at);
                audio.start_boot_movie(state, sound);
            }
        }
        // In the tick loop, next to the state machine it belongs to. See
        // [`oag_sound`] for why nothing here is per frame.
        audio.tick();
        // The movie's sound outlives neither leg: see
        // `Frontend::is_playing_movie` for why the state is what is asked
        // rather than the player.
        if !frontend.is_playing_movie() {
            audio.stop_movie();
        }
        // The menu music's cue is **no movies left**, kept in step with
        // `App::tick`'s own: "not in a movie" is true before a picker-first
        // title's reel has played at all, and starting the loop there puts it
        // under the reel.
        if !frontend.is_playing_movie() && pending_sound.is_empty() {
            audio.start_music(
                &options.music_discs,
                options.settings.audio.music_source,
                &oag_source::cache::default_audio_cache_dir(),
            );
        }
        crate::report(&events, options.trace);
        oag_raceplay::loader_log::lines(frontend.take_notes());
        ticks += 1;
    }

    match &options.screen {
        Some(name) => println!("drawing screen {name:?} from the XML, sequence not run"),
        None => println!(
            "after {ticks} tick(s), state {:?}",
            frontend.machine().current().unwrap_or("nothing")
        ),
    }

    let (width, height) = options.size;

    // The handoff, and the only place a capture leaves the front end. What it
    // writes is a race frame drawn through the same scene the window draws, for
    // the same reason the rest of this file goes through the front end's own
    // renderer.
    if frontend.is_finished()
        && let Some(race_options) = &options.race
    {
        // Wipeout 2048's own front end says where it was going - a campaign
        // event, or one of this build's two menu pages, which a capture has
        // no menus to open and so reports and races on the options given.
        // See `oag_ui::frontend::Launch`.
        let loaded = match frontend.launch() {
            Some(oag_ui::frontend::Launch::Event(name)) => {
                debug!("Launch 2048: campaign event {name:?}");
                race::load_event(race_options, name)?
            }
            Some(other) => {
                debug!(
                    "Launch 2048 asked for {other:?}, which is a menu page; a capture has no \
                     menus, so this races the options given"
                );
                race::load(race_options)?
            }
            None => race::load(race_options)?,
        };
        oag_raceplay::loader_log::lines(&loaded.report);
        // The same handoff `App::launch_race` makes: the menu voice this loop
        // started above stops, and the race playlist takes over - one music
        // rule for every way a race is reached, screenshot captures included.
        audio.start_race_music(
            &options.music_discs,
            options.settings.audio.music_source,
            &oag_source::cache::default_audio_cache_dir(),
        );
        // The five render-profile settings, resolved against this title -
        // see `crate::settings::RenderProfile`. Read once rather than inline
        // below, since `anti_aliasing` is needed twice: once for the scene
        // pipeline and once for `Presentation`'s own resolve pass.
        let render_profile = options
            .settings
            .render_profiles
            .get(&crate::settings::profile_key(title, platform))
            .cloned()
            .unwrap_or_default();
        // Read-only, off whatever `<config dir>/oag/records.toml` already holds - see
        // `crate::race_capture::CaptureOptions::previous_best`'s own doc for why.
        let previous_best = crate::records::load()
            .get(&crate::records::Key::new(
                loaded.title.name,
                race_options
                    .track
                    .as_deref()
                    .or(Some(loaded.title.race.track)),
                loaded.setup.mode.name(),
                &loaded.setup.class,
            ))
            .cloned();
        return crate::race_capture::capture(
            loaded,
            &crate::race_capture::CaptureOptions {
                gpu: None,
                aspect: options.settings.display.aspect,
                path: options.path.clone(),
                ticks: options.ticks.saturating_sub(ticks),
                held: options.held,
                pressed: options.pressed,
                input_script: None,
                autopilot: options.autopilot,
                autopilot_pilot: options.autopilot_pilot,
                autopilot_skill: options.autopilot_skill,
                force_shake: None,
                intro_ticks: 0,
                force_wreck: None,
                force_hit: None,
                force_leach_lock: None,
                force_bomb_trip: None,
                force_shield: Vec::new(),
                medals: Default::default(),
                scheme: options.scheme,
                size: (width, height),
                log_every: options.log_every,
                give: options.give,
                anisotropy: options.anisotropy,
                renderer: options.settings.graphics.renderer.clone(),
                fov: options.settings.graphics.fov,
                frustum_culling: options.settings.graphics.frustum_culling,
                pvs_culling: options.settings.graphics.pvs_culling,
                anim_seconds: options.anim_seconds,
                camera_jitter: options.camera_jitter,
                boost_fov_kick: options.settings.graphics.boost_fov_kick,
                camera_view: options.settings.graphics.camera_view,
                msaa: render_profile.msaa,
                motion_blur: render_profile.motion_blur,
                motion_blur_resolution: render_profile.motion_blur_resolution,
                hud_scale: options.settings.graphics.hud_scale,
                shadows: render_profile.shadows,
                model_detail: render_profile.model_detail,
                texture_detail: render_profile.texture_detail,
                // The front-end capture path never poses a ship, so there is
                // nothing for a forced boost state to be aged relative to.
                pose_boost: None,
                pose_intensity: None,
                pose_speed: None,
                presented: options.presented.then_some(crate::race_capture::Presented {
                    render_scale: render_profile.render_scale,
                    presentation: oag_present::upscale::Presentation {
                        reconstruction: render_profile.reconstruction,
                        sharpness: render_profile.upscale_sharpness.stops(),
                        brightness: options.settings.display.brightness,
                        gamma: options.settings.display.gamma,
                    },
                }),
                // No CLI flag on this path for either: see `main/headless.rs`.
                zone_spectrum_test: false,
                touch_demo: None,
                previous_best,
                // No flag for it on this path; see `main/headless.rs`.
                ghost: race::GhostCapture::default(),
                screen_filter:
                    crate::screen::Catalogue::load(crate::screen::Catalogue::directory())
                        .get(&render_profile.screen_filter)
                        .cloned(),
                screen_filter_strength: render_profile.screen_filter_strength,
            },
            audio,
        );
    }

    // **`--menu-page`'s picture comes off a different movie than the sequence's
    // does.** A menu page's `Draw::Video` is the disc's looping menu backdrop,
    // not the intro reel, so the movie the frame is read from and the plane
    // geometry the pipeline is built for both have to be the backdrop's. Getting
    // that wrong is not a compile error and not a crash: it reads a frame of the
    // intro into planes sized for the backdrop, and the flag quietly stops
    // showing what a player would see - which is exactly what this module exists
    // not to do.
    //
    // Frame zero and not a moving one: a capture is one picture, and there is
    // nothing here for a playhead to be advanced by.
    // The grid the chosen list's rects are in - the source's, on all three
    // paths. `--menu-page` used to pin `Space::PSP` here on the grounds that
    // this project's menu layout is authored at 480x272; that is still where the
    // layout is *written*, but `menu::Skin` now scales it into the source's grid
    // so that the disc's own `FEGlobals` and its own faces are drawn at the
    // numbers the disc states. A capture that got this wrong would disagree with
    // the window, which is the divergence this module exists to prevent - so
    // this reads the same value `Session::open_menus` hands its renderer.
    let space = frontend.space();
    // The selection screens' preview mesh, owed after the draw list - see
    // `draw_preview`. `None` on every other page.
    let mut preview_request: Option<PreviewRequest> = None;
    let mut flyer_shot: Option<campaign_page::FlyerShot> = None;
    // The footer ticker's own clip, `(index in `list`, left, right)` in
    // screen space - set only by the ordinary `--menu-page` arm below
    // (`menu_page`'s own return), since it is the only page kind that reads
    // a ticker at all. `None` everywhere else, which `renderer.render`'s own
    // `clip` parameter already treats as "nothing to clip".
    let mut ticker_clip: Option<(usize, f32, f32)> = None;
    let (mut movie, video_format, list, space) = match (&options.menu_page, &options.screen) {
        (Some(page), _) => {
            let showing = backdrop.as_ref().filter(|movie| movie.frames.is_some());
            let frame = showing.map(|movie| oag_ui::menu::Backdrop {
                rect: oag_display::space::pillarbox_in(space, movie.display_aspect),
                frame: 0,
                // Position zero with the frame, there being no playhead here to
                // have got anywhere: a capture reads the frame straight out of
                // the `FrameStore` rather than off a `Feed`.
                position: 0,
            });
            let format = showing.and_then(VideoFormat::of);
            // The movie where there is one, else the style's backdrop for this
            // page - see `MenuBackdrop::still`.
            let frame = frame.map(oag_ui::menu::Picture::from).or_else(|| {
                fury_backdrop.as_deref()?.still(
                    page,
                    oag_display::display::viewport(
                        (width, height),
                        options.settings.display.aspect,
                    ),
                    options.anim_seconds,
                    options.fury_path,
                )
            });
            // The race box's two selection screens are not pages of our
            // menu tree - they are the disc's own screens, opened over it -
            // so they are drawn by their own builder, off the same boot.
            if let Some(kind) = picker_kind(page) {
                let layout = match kind {
                    oag_ui_screens::picker::Kind::Track => track_select.as_ref(),
                    oag_ui_screens::picker::Kind::Ship => ship_select.as_ref(),
                }
                .with_context(|| format!("{} authors no {kind:?} selection screen", title.name))?;
                let skin = oag_ui::menu::Skin::new(
                    menu_skin,
                    space,
                    menu_font.as_ref().unwrap_or(&font).line_height,
                );
                // The selected circuit's lap, off the disc - the number the
                // live screen's worker measures. One read; a capture with
                // no source to open keeps the dash.
                let mut archives = match options.race.as_ref().map(open_for_previews) {
                    Some(Ok(archives)) => Some(archives),
                    Some(Err(error)) => {
                        log::warn!("{error:#} - no distance measured, no slideshow");
                        None
                    }
                    None => None,
                };
                // The live screen's list: locked circuits are absent.
                let tracks = crate::unlock::offered_on(kind, archives.as_mut(), title, &tracks);
                let selected = tracks
                    .iter()
                    .find(|track| track.id == options.settings.race.track);
                let distance = match (kind, archives.as_mut(), selected) {
                    (oag_ui_screens::picker::Kind::Track, Some(archives), Some(track)) => archives
                        .read_name(&track.entry_name())
                        .ok()
                        .and_then(|blob| race::circuit_length(&blob).ok()),
                    _ => None,
                };
                let (stills, mode3d_model) = picker_stills(
                    kind,
                    &options.settings,
                    selected,
                    &tracks,
                    &teams,
                    archives.as_mut(),
                    frontend.screens(),
                    &strings,
                    &mut sprites,
                    options.menu_picker_seconds,
                );
                let (mut list, request) = picker_page(
                    kind,
                    &circuit_names,
                    layout,
                    &options.settings,
                    title,
                    &tracks,
                    &teams,
                    &strings,
                    frame,
                    &skin,
                    &menu_frame,
                    &sprites,
                    &|text| oag_ui::font::measure(menu_font.as_ref().unwrap_or(&font), text),
                    distance,
                    options.menu_picker_seconds,
                );
                list.extend(stills);
                // HD's ship screen's footer legend, as the live one draws it
                // (`main::menu_stage::footer::hd_nav`).
                if let Some(legend) = nav_legend.as_ref().filter(|_| layout.is_hd()) {
                    let measure = |text: &str| oag_ui::font::measure(&font, text);
                    let faces = oag_ui_screens::picker::FaceScales::default();
                    list.extend(legend.draw_gated(&faces, &measure, true));
                }
                // Same disc read `picker_stills` already made for the
                // hexagonal window's cards - reused here rather than read
                // twice, since the two share one `screen.xml`.
                let request = request.map(|r| PreviewRequest {
                    mode3d: mode3d_model,
                    ..r
                });
                // A title that previews with stills alone has no mesh to
                // ask for, and asking would log a miss for a file that is
                // not supposed to be there - see
                // `oag_title::FrontEnd::preview_meshes`.
                preview_request = title
                    .front_end
                    .is_some_and(|front_end| front_end.preview_meshes)
                    .then_some(request)
                    .flatten();
                (backdrop, format, list, space)
            } else if let Some(kind) = campaign_kind(page) {
                // The Race Campaign's own two screens - the disc's, opened
                // over the menus the same way the race box's are, not pages
                // of our own tree. See `oag_game::campaign` and
                // `crate::main::session::campaign` for the live flow this is
                // a still of.
                let mut archives = match options.race.as_ref() {
                    Some(race) => open_for_previews(race)?,
                    None => anyhow::bail!(
                        "--menu-page grid-select/cell-select needs --race options open"
                    ),
                };
                let faces = oag_ui_screens::picker::FaceScales {
                    default: menu_font.as_ref().map_or(
                        oag_ui_screens::picker::FaceScales::default().default,
                        |menu| font.line_height / menu.line_height,
                    ),
                    ..oag_ui_screens::picker::FaceScales::default()
                };
                let skin = oag_ui::menu::Skin::new(
                    menu_skin,
                    space,
                    menu_font.as_ref().unwrap_or(&font).line_height,
                );
                let globals: Vec<(&str, &str)> = frontend
                    .screens()
                    .globals
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str()))
                    .collect();
                // `Campaign Selection`'s own four idstrings, the same
                // `DATA06`-only overlay `crate::main::session::campaign::open_campaign`
                // applies for a live session - without it this still shows
                // `FE_RC_SELECT`/`FE_CAMPSEL_MODES`/`FE_RC_FURY`/`FE_RC_HD` as
                // raw ids, since `strings` above resolves through the general
                // precedence, which (see `oag_game::campaign::hd_selection_string_overlay`'s
                // own doc) none of `DATA00`/`DATA01`/`DATA02`/`DATA03`/`DATA05`
                // carry any of the four in.
                let entries_path = oag_ui::language::load::chosen_language(
                    &languages,
                    options.settings.language.as_deref(),
                )
                .and_then(|language| language.entries.clone());
                // Read-only, the same `crate::records::load()` call the
                // `records` `--menu-page` arm above already makes - see
                // `campaign_page`'s own `records` parameter doc.
                let records = crate::records::load();
                let (list, shot) = campaign_page(
                    kind,
                    &mut archives,
                    &strings,
                    &circuit_names,
                    entries_path.as_deref(),
                    faces,
                    [space.size.0, space.size.1],
                    frame,
                    &skin,
                    &menu_frame,
                    &mut sprites,
                    &globals,
                    title,
                    // `Default`-role, not `menu_font.unwrap_or(&font)`: this
                    // measures `FE_CONFIRM`'s own shrink-to-fit, and
                    // `NavigationLegend::draw` routes that text through the
                    // `Default`-role atlas now (`Draw::in_role`, see
                    // `boot::fonts::face_atlas_slot`) - mirrors
                    // `crate::main::menu_stage::MenuStage`'s own
                    // `default_measure`, which this capture-only path has to
                    // match by hand rather than share, having no `MenuStage`
                    // of its own to read `default_atlas` off.
                    &|text| oag_ui::font::measure(&font, text),
                    &records,
                )?;
                flyer_shot = shot;
                (backdrop, video_format, list, space)
            } else if let Some(kind) = endrace_kind(page) {
                // The EndRace screens, opened off the disc's own source - see
                // `endrace_page::capture`.
                let (list, trophy) = endrace_page::capture(
                    kind,
                    options.race.as_ref(),
                    menu_font.as_ref(),
                    &font,
                    menu_skin,
                    space,
                    &frontend.screens().globals,
                    &strings,
                    frame,
                    &menu_frame,
                    &mut sprites,
                    title,
                    (&face_scales, options.event.as_deref()),
                )?;
                preview_request = trophy;
                (backdrop, video_format, list, space)
            } else {
                // Read-only, off whatever `<config dir>/oag/records.toml`
                // already holds - the same "read, never write" rule
                // `crate::race_capture::CaptureOptions::previous_best` follows a few lines
                // above this arm's own sibling, and for the same reason:
                // this still shows a player their own stored times, never a
                // seeded or invented one. A capture with no file, or one
                // this machine has never raced under, draws every class's
                // `-` placeholder rather than a shorter table.
                let records = crate::records::load();
                // The same rotation the Race Campaign's own footer reads -
                // see `crate::records::ticker_tips`'s own doc for why one
                // function answers for both, and for the live session's own
                // ordinary menu pages besides.
                let ticker_tips = crate::records::ticker_tips(&strings, &records);
                let (list, clip) = menu_page(
                    &options.settings,
                    options.anisotropy,
                    title,
                    platform,
                    page,
                    &tracks,
                    &teams,
                    &race_setup,
                    &languages,
                    &strings,
                    &options.music_discs,
                    &records,
                    frame,
                    // The face the rows are drawn in, not the front end's
                    // default: the pitch comes off its line height, so reading
                    // the wrong one spaces the rows for a font nothing draws.
                    &oag_ui::menu::Skin::new(
                        menu_skin,
                        space,
                        menu_font.as_ref().unwrap_or(&font).line_height,
                    ),
                    &|text| oag_ui::font::measure(menu_font.as_ref().unwrap_or(&font), text),
                    &menu_frame,
                    options.menu_anim_phase,
                    options.menu_prompt.as_deref(),
                    options.race.as_ref().map(|r| r.source.as_str()),
                    nav_legend.as_ref(),
                    &|text| oag_ui::font::measure(&font, text),
                    ticker.as_ref(),
                    &ticker_tips,
                )?;
                ticker_clip = clip;
                (backdrop, format, list, space)
            }
        }
        (None, Some(name)) => {
            let list = match options.screen_seconds {
                Some(seconds) => frontend.draw_screen_at(name, f64::from(seconds)),
                None => frontend.draw_screen(name),
            };
            (movie, video_format, list, frontend.space())
        }
        (None, None) => {
            let list = frontend.draw_list();
            // **The sequence itself now has the same split.** Its last screen,
            // `Show Logo`, sits on the looping backdrop rather than on the
            // intro - so a capture that stops there has to read its frame from
            // the backdrop and build its planes for the backdrop, exactly as
            // `--menu-page` does. The draw says which movie it means, so this
            // follows the list rather than guessing from the state.
            match video_source(&list) {
                Some(oag_ui::frontend::Video::Backdrop) => {
                    let showing = backdrop.as_ref().filter(|movie| movie.frames.is_some());
                    let format = showing.and_then(VideoFormat::of);
                    (backdrop, format, list, frontend.space())
                }
                // **`Video::Intro` names two different movies over a boot.** The
                // first one, and - on a title whose chain has two - the second.
                // The draw cannot say which, the variant being reused rather
                // than a third added, so the state picks, exactly as
                // `App::tick`'s own feed swap does. Without this the capture
                // reads the *first* movie's frame at the *second* movie's
                // playhead: the wrong picture, silently, which is the class of
                // mistake this module's own docs above exist to rule out.
                //
                // **Asked as "is this the chain's second movie screen?"**, which
                // is the same question `boot::assemble` answers when it hands
                // each step its plan, and so cannot disagree with it.
                //
                // It used to ask whether the state was the one the picker
                // confirms into, and that was **never the same question** -
                // `language_confirm_target` is the step *after* the picker,
                // which on Pure is `Developer Publisher Screen`, the screen that
                // plays movie **zero**. So the old form selected the second
                // movie for the first movie's screen. Verified against the disc
                // rather than reasoned about: a capture of Pure's dev/pub screen
                // is byte-for-byte identical either way, both cuts being 480x272
                // and that screen's parent fills covering the frame at the tick
                // this stops on - which is exactly why it survived.
                //
                // On Wipeout HD it does not survive. Its chain has one movie and
                // its `Studio Logo` *is* the after-language screen, so the old
                // form handed this path `after_language_movie`, which is `None`
                // there: the renderer was built with no video pipeline and every
                // capture of the logo reel came out black with the frame counter
                // drawn over it.
                Some(oag_ui::frontend::Video::Intro)
                    if frontend
                        .machine()
                        .current()
                        .is_some_and(|state| frontend.movie_states().get(1) == Some(&state)) =>
                {
                    let showing = after_language_movie
                        .as_ref()
                        .filter(|movie| movie.frames.is_some());
                    let format = showing.and_then(VideoFormat::of);
                    (after_language_movie, format, list, frontend.space())
                }
                _ => (movie, video_format, list, frontend.space()),
            }
        }
    };

    // The same adapter the window would have drawn with, so a screenshot is a
    // picture of what a player sees rather than of whatever wgpu picked here.
    // No surface to be compatible with, which is the only difference.
    let instance = crate::adapter::instance();
    let adapter =
        crate::adapter::choose(&instance, None, &options.settings.graphics.renderer)?.adapter;
    let (device, queue) = pollster::block_on(adapter.request_device(
        &oag_mesh::mesh_render::device_descriptor("oag-game offscreen", &adapter),
    ))
    .context("requesting the device")?;

    // Rgba8Unorm rather than the surface's sRGB format: the readback is written
    // straight into a PNG, so a second gamma encode would double-correct.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    // Menu pages draw in the title's own menu face; every other capture in
    // this path - a boot screen, the picker - draws in the default one. The
    // face has to match the one whose line height set the row pitch above,
    // or the rows are spaced for a font nothing draws them in.
    let face = if options.menu_page.is_some() {
        menu_font.clone().unwrap_or_else(|| font.clone())
    } else {
        font.clone()
    };
    let mut renderer = Renderer::new(&device, &queue, format, video_format, face, &sprites)?;
    renderer.set_space(space);
    if let Some(assets) = &fury_backdrop {
        assets.install(&mut renderer, &device, &queue);
    }
    // On Pulse (both PSP pressings and the PS2 port) this loads the
    // `Default`-role atlas rather than a title role, so
    // `oag_ui_screens::campaign::footer`'s `Draw::FacedText` draws the same
    // mixed-case body face here as the live window does - see
    // `boot::fonts::face_atlas_slot`. Pure now loads its own `Title`-role
    // atlas instead, which resolves to the identical `.fnt` `Default` does
    // for that title, so the slot's contents are unchanged even though its
    // label is. Unconditional, so a menu-page capture and the live window
    // (`session::menus`) build the same picture whether or not this run's
    // draw list ever reaches `oag_ui::menu::draw_list`/`picker::draw_list`.
    let (face_atlas, face_role) =
        crate::boot::fonts::face_atlas_slot(menu_skin, &font, title_font.clone());
    renderer.set_face_atlas(&device, &queue, face_atlas, face_role);
    // The buttons atlas is not routed through `face_atlas_slot` at all - it
    // is a third, independent slot, not a substitute for `Title`/`Default`
    // in the one `face_atlas_slot` already picks between. See
    // `crate::render::Renderer::set_buttons_atlas`'s own doc.
    renderer.set_buttons_atlas(&device, &queue, buttons_font);
    crate::prompts::install(&mut renderer, title, &options.settings);

    if let (Some(frames), Some(wanted)) = (
        movie.as_mut().and_then(|movie| movie.frames.as_mut()),
        video_frame(&list),
    ) {
        let mut picture = crate::movie::VideoFrame::default();
        frames.read_frame(wanted.min(frames.len - 1), &mut picture)?;
        renderer.upload_frame(&queue, &picture)?;
    }

    let target = offscreen(&device, format, width, height);
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    // Under `--presented`, the stage draws into a presentation target and the
    // grade and the screen filter put it on `view` at the end - see
    // `presented`. Otherwise it draws straight into `view`.
    let presented = presented::Presented::new(
        &device,
        format,
        (width, height),
        &options.settings,
        title,
        platform,
        space,
        options.presented,
    )?;
    let drawn = presented
        .as_ref()
        .map_or(&view, |presented| presented.view());

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("capture"),
    });
    // Shaped the same way a window is, for the same reason the race capture is:
    // a screenshot should frame what a player would have seen at that size. At
    // the default `--size`, which is the PSP's own shape, every aspect fills the
    // frame and nothing changes.
    // A capture is one static frame with no `MenuStage` clock behind it, so
    // there is nothing here for a value marquee to be mid-scroll of -
    // `ticker_clip` is the one thing that still needs a clip (see
    // `capture::menu_page`'s own doc).
    campaign_page::render_frame(
        flyer_shot.as_ref(),
        &mut renderer,
        (&device, &queue, format),
        &mut encoder,
        drawn,
        &list,
        oag_display::display::viewport((width, height), options.settings.display.aspect),
        ((width, height), space),
        ticker_clip,
    );
    if let (Some(request), Some(race)) = (preview_request, &options.race) {
        draw_preview(
            &device,
            &queue,
            &mut encoder,
            drawn,
            oag_display::display::viewport((width, height), options.settings.display.aspect),
            (width, height),
            space,
            race,
            &request,
            options.anisotropy,
        );
    }
    if let Some(presented) = presented {
        presented.composite(&device, &queue, &mut encoder, &view);
    }
    touch_preview(options, &device, &queue, &mut encoder, &target)?;
    let pixels = read_back(&device, &queue, encoder, &target, width, height)?;
    write_png(&options.path, width, height, &pixels)
}

fn video_frame(list: &[Draw]) -> Option<usize> {
    list.iter().find_map(|draw| match draw {
        Draw::Video { frame, .. } => Some(*frame),
        _ => None,
    })
}

/// Which movie the list's video draw wants a frame of, if it has one.
fn video_source(list: &[Draw]) -> Option<oag_ui::frontend::Video> {
    list.iter().find_map(|draw| match draw {
        Draw::Video { source, .. } => Some(*source),
        _ => None,
    })
}
