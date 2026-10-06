//! The three runs that open no window.
//!
//! `--screenshot` without a display, `--trace-out`, and `--race` captured
//! headless. Each takes the whole boot in one blocking call rather than
//! deferring the movies onto a worker, because a capture must not race a
//! worker for the frames it is about to draw.

use anyhow::{Context, Result};
use log::{debug, warn};

use oag_game::{boot, capture, loading, prefetch, records, settings};
use oag_gameplay::ControlScheme;
use oag_mesh::mesh_render::Anisotropy;
use oag_raceplay as race;
use oag_ui::frontend::EarnedTier;
use oag_ui::strings;

use winit::event_loop::{ControlFlow, EventLoop};

use crate::app::App;
use crate::args::{
    autopilot_pilot, button_mask, give_weapon, parse_progress, parse_size, parse_step,
    resolve_scheme,
};
use crate::cli::Cli;
use crate::hints;
use crate::session::menus::combine_variant;

/// Every leg that never opens a window: `--dry-run` and the three captures.
///
/// **Split off from `main` because the boot is loaded differently here.** A
/// window defers the movies onto a worker and covers the wait with the loading
/// screen (`boot::MediaWorker`); none of these legs has anywhere to show that,
/// and a capture must not race a worker for the frames it is about to draw, so
/// every one of them takes the whole boot in one blocking `boot::load` exactly
/// as the game always did. The body below is that former part of `main`,
/// unchanged.
pub(crate) fn run_windowless(
    cli: &Cli,
    options: &boot::Options,
    settings: &settings::Settings,
    race_options: race::Options,
    anisotropy: Anisotropy,
    mut audio: oag_sound::Audio,
    music_discs: oag_sound::MusicDiscs,
) -> Result<()> {
    let mut loaded = boot::load(options)?;
    log::info!("{}: {}", loaded.title.name, options.source);
    if cli.overlay {
        loaded.frontend.set_overlay(true);
    }
    // A language chosen on an earlier run skips the picker. Reported either
    // way: silently not asking is indistinguishable from a broken picker, and
    // silently asking again is indistinguishable from a setting that did not
    // save.
    match (cli.pick_language, settings.language.as_deref()) {
        (false, Some(name)) if loaded.frontend.preselect_language(name) => {
            debug!("language {name} from settings, skipping the picker");
        }
        (false, Some(name)) => {
            warn!("this source does not offer {name:?}, so the picker is shown");
        }
        // A fresh run with no settings language yet: on HD alone, this screen
        // never presents to a player, so it must not be the one this build
        // waits on - see `Frontend::skip_never_shown_picker`.
        (false, None) if loaded.frontend.skip_never_shown_picker(loaded.title) => {
            debug!("HD's own language picker never presents to a player; defaulting to English");
        }
        _ => {}
    }
    oag_raceplay::loader_log::lines(&loaded.report);

    // Wipeout 2048's own career, read the same way `Session::finish_loading`
    // does - see that function's own comment. **Read-only**, the same
    // exception `oag_game::race_capture::CaptureOptions::previous_best` a few lines down
    // already makes for this exact file: a capture never calls
    // `records::save`, so reading `records::load()` here cannot write a
    // player's own `records.toml`, only reflect what is already in it. A
    // no-op on every title but 2048, whose `campaign_events()` is the only
    // one ever non-empty.
    let title_name = loaded.title.name;
    let saved = records::load();
    loaded.frontend.refresh_campaign_progress(|name| {
        saved
            .campaign_medal(title_name, name)
            .and_then(|row| row.best_medal)
            .map(|medal| match medal {
                records::Medal::Gold => EarnedTier::Elite,
                records::Medal::Silver | records::Medal::Bronze => EarnedTier::Pass,
            })
    });
    // `race_options.team` when the CLI named one (`--team`/`--variant`,
    // already combined by the time it reaches here) - the same value
    // `race::load_event` below actually races, so a `--press` capture's own
    // craft-restriction gate checks against what will really load rather
    // than a `settings.race` default the CLI is about to override. Falls
    // back to that default when the CLI left `--team` unset, same as the
    // windowed boot.
    loaded
        .frontend
        .seed_craft(race_options.team.clone().unwrap_or_else(|| {
            combine_variant(loaded.title, &settings.race.team, &settings.race.variant).0
        }));

    if cli.dry_run {
        return Ok(());
    }

    // **After `boot::load`, and that is the whole of why it is here.** The boot
    // sequence transcodes the intro and the backdrop itself, lazily, and the
    // prefetch worker would convert those same two movies - two `ffmpeg`
    // processes writing one cache file, which is a corrupt file rather than a
    // race that resolves. Started once boot has finished, they are already
    // cached and the worker's planning pass skips them by name.
    //
    // `--refresh-video` turns that skip off, so with both flags the worker
    // converts the boot's own two reels a second time. Wasteful - a couple of
    // the ten minutes - but still ordered, which is the property this comment is
    // actually about: the boot has finished with the cache before the worker
    // touches it either way.
    let mut prefetch = cli.prefetch.then(|| {
        prefetch::Prefetch::spawn(prefetch::Options {
            source: options.source.clone(),
            movies: options.cache.clone(),
            audio: oag_source::cache::default_audio_cache_dir(),
            refresh_video: cli.refresh_video,
        })
    });

    let scheme = resolve_scheme(cli, settings);

    // A source with no movie at all - which is every PS2 source, whose intro is
    // an MPEG-2 program stream outside the archives - has no video format
    // either, and the front end draws without one. A source with *some* movie
    // gets a pipeline sized for it even when the first boot leg is not the one
    // that plays it; see `Boot::video_format`.
    let video_format = loaded.video_format();

    // Before the sequence's own capture, because it is a different picture
    // rather than a variation on that one: it runs no state machine, opens no
    // movie and reaches the GPU through `capture::loading`.
    if let (Some(path), Some(spec)) = (&cli.screenshot, &cli.loading_screen) {
        let mut progress = parse_progress(spec)?;
        let phase = parse_step(cli.loading_step.as_deref())?;
        // A race load counts nothing and names the circuit, which is a shape
        // `--loading-screen DONE/TOTAL` cannot express - see
        // `loading::Phase::Race`. So the counts are dropped and the line the
        // other phases put an entry name on takes `--track`'s, which is a real
        // name off the command line rather than one invented here. Without
        // `--track` the line is simply absent, as it is on a run whose caller
        // had no name to give.
        if phase == loading::Phase::Race {
            progress.total = 0;
            progress.done = 0;
            // The circuit's own name where `--track` names one the source
            // offers, which is what the window's race load puts on this line
            // (see `Session::launch_race`); a path the catalogue does not know
            // is shown as given.
            progress.current = cli.track.as_ref().map(|entry| {
                loaded
                    .tracks
                    .iter()
                    .find(|track| track.entry_name() == *entry)
                    .map_or_else(
                        || entry.clone(),
                        |track| {
                            oag_raceplay::catalogue::label(
                                track,
                                &loaded.circuit_names,
                                &loaded.strings,
                                &loaded.tracks,
                            )
                        },
                    )
            });
        }
        let assets = loading::Assets::load(
            &options.source,
            &loaded.strings,
            loaded.entries.as_deref(),
            crate::args::style_of(settings),
        );
        oag_raceplay::loader_log::lines(&assets.notes);
        capture::loading(
            &assets,
            loaded.font.clone(),
            &loaded.sprites,
            &capture::LoadingOptions {
                path: path.clone(),
                size: parse_size(&cli.size)?,
                ticks: cli.ticks,
                progress,
                phase,
                live: (cli.loading_live && phase == loading::Phase::Race)
                    .then(|| race_options.clone()),
                renderer: settings.graphics.renderer.clone(),
                aspect: settings.display.aspect,
            },
        )?;
        if let Some(prefetch) = &mut prefetch {
            prefetch.join();
        }
        return Ok(());
    }

    if let Some(path) = cli.screenshot.clone() {
        // No `start_music` here: this leg runs the boot sequence, so the music
        // waits behind the intro exactly as the window's does, and `capture::run`
        // starts it from its own tick loop. See `Audio::start_music`.
        capture::run(
            loaded,
            video_format,
            &capture::Options {
                give: give_weapon(cli.give.as_deref())?,
                autopilot: cli.autopilot,
                autopilot_pilot: autopilot_pilot(cli.autopilot_pilot.as_deref())?,
                autopilot_skill: cli.autopilot_skill,
                path,
                until: cli.until.clone(),
                ticks: cli.ticks,
                anim_seconds: cli.anim_seconds,
                fury_path: cli.fury_path,
                camera_jitter: cli.camera_jitter,
                held: button_mask(cli.hold.as_deref()),
                pressed: button_mask(cli.press.as_deref()),
                scheme,
                trace: cli.trace,
                race: Some(race_options),
                log_every: cli.log_every,
                size: parse_size(&cli.size)?,
                screen: cli.screen.clone(),
                screen_seconds: cli.screen_seconds,
                menu_page: cli.menu_args.menu_page.clone(),
                menu_anim_phase: cli.menu_args.menu_anim_phase,
                menu_picker_seconds: cli.menu_args.menu_picker_seconds,
                menu_prompt: cli.menu_args.menu_prompt.clone(),
                presented: cli.presented,
                // The clone is overridden rather than `settings` itself, so
                // `--camera-view` reaches the race this capture may hand over to
                // (`capture::run` builds its `CaptureOptions` from this block)
                // without ever being written back to the settings file. The same
                // one flag then covers both screenshot paths - this one and
                // `--race --screenshot` - and neither persists it.
                settings: settings::Settings {
                    graphics: settings::Graphics {
                        camera_view: cli.camera_view.unwrap_or(settings.graphics.camera_view),
                        pvs_culling: cli.pvs.unwrap_or(settings.graphics.pvs_culling),
                        ..settings.graphics.clone()
                    },
                    ..settings.clone()
                },
                music_discs: music_discs.clone(),
                anisotropy,
            },
            &mut audio,
        )?;
        // After both legs, and only here: the capture above may have handed off
        // to a race that went on filling the same buffer, so writing inside
        // either one would truncate the WAV to whichever leg wrote it.
        audio.finish()?;
        // A capture is over in seconds and the conversion is not, so this is
        // where `--prefetch` actually waits. Joined rather than dropped,
        // because a headless run is how the whole cache gets filled in the
        // first place.
        if let Some(prefetch) = &mut prefetch {
            prefetch.join();
        }
        return Ok(());
    }
    unreachable!("every branch above returns")
}

/// Our per-tick state, in the capture's own columns, written to `path`.
///
/// # Driving the run
///
/// `--input-script` supersedes `--hold`/`--press` from tick 0, the same way
/// `oag_game::race_capture::CaptureOptions::input_script` documents its own field: with a
/// script, the two masks are read only for the ticks past its end.
///
/// **This loop used to read `cli.hold`/`cli.press` unconditionally and never
/// look at `cli.input_script` at all**, so a scripted `--trace-out` run
/// silently wrote a CSV of a craft sitting still - no error, right row count,
/// right columns, `throttle`/`steer` zero throughout.
/// `race::HeldButtons::advance` is the one place both this loop and
/// `oag_game::race_capture::capture`'s tick loop make the script-or-mask choice now.
///
/// **Only the script's button mask reaches the ship** - `stick_x`, `stick_y`
/// and the two `airbrake_*` analog fields a script can author are dropped
/// here the same way `oag_game::race_capture::capture`'s tick loop already dropped them; see
/// `HeldButtons::advance`'s own doc for why and what reads them instead.
///
/// # Which of our values stands for which recovered field
///
/// This mapping is a **claim**, not a convenience: differencing two columns that
/// do not mean the same thing produces a number that looks like a measurement
/// and is not. Each row is one field of the original's flare object, at the
/// offset `scripts/psp_trace_fields.py`'s `FLARE_FIELDS` reads it from.
///
/// | Column | Flare offset | Ours |
/// | --- | --- | --- |
/// | `boost_timer` | `+0xb8` | [`Exhaust::boost_timer`] |
/// | `plume_timer` | `+0x88` | [`Exhaust::plume_timer`] |
/// | `intensity` | `+0xbc` | [`Exhaust::intensity`] |
/// | `half_size` | `+0xc4` | [`Exhaust::half_size`] |
/// | `engine_on` | `+0x94` | [`Exhaust::engine_on`] |
/// | `flare_speed_kmh` | `+0x8c` | [`Exhaust::speed_kmh`] |
/// | `speed_ramp` | `+0x90` | [`Exhaust::speed_ramp`] |
/// | `boost_accum` | `+0x60` | [`Exhaust::boost_accumulator`] |
///
/// Two of them are worth stating outright, because the obvious accessor is the
/// wrong one:
///
/// - **`plume_timer` is the reveal timer, not the boost timer.** It is *not*
///   [`Exhaust::plume_visible`], which is the bit the reveal sets; the column is
///   the seconds-since-reveal that decides when that bit clears again.
/// - **`engine_on` is written as `0` or `1` here, and the capture's column is
///   neither.** The original's `+0x94` is an integer that `psp-trace.py` reads
///   through `struct.unpack("<f", ..)`, so a set value arrives as the denormal
///   `3.601337e-43`. `oag_trace::Flare::engine_on_is_set` is what both sides are
///   compared through, so writing a plain `1.0` is correct and comparable - see
///   `oag_trace::trace::FLARE_COLUMNS`.
///
/// # Row alignment
///
/// One row per tick, then one final row, mirroring `oag_game::race_capture::capture`: a row is
/// emitted **before** the tick it labels, which is `scripts/psp-trace.py`'s own
/// alignment ("the craft as this frame's update found it"), and the last row is
/// the state a `--screenshot` of the same command line would draw - after
/// `--pose-boost` has been applied, which `capture` also does after its loop.
/// So `--ticks 0` writes exactly one row: the posed state, drawing nothing.
pub(crate) fn write_trace(
    loaded: race::Loaded,
    cli: &Cli,
    scheme: ControlScheme,
    path: &std::path::Path,
) -> Result<()> {
    use oag_trace::{Flare, Frame, Trace};

    let mut race = race::Race::start(loaded.setup);
    race.set_control_scheme(scheme);

    // Built from the world rather than from `Telemetry`, which carries a summary
    // for the console and not the columns a comparison needs.
    let frame_of = |race: &race::Race, dt: f32| {
        let ship = &race.sim.world.ships[0].physics;
        let body = &ship.body;
        let exhaust = race.exhaust();
        Frame {
            tick: race.sim.world.tick,
            dt,
            grounded: ship.grounded,
            throttle: ship.thrust,
            brake: ship.brake,
            steer: ship.steer,
            airbrake_left: ship.airbrake_left,
            airbrake_right: ship.airbrake_right,
            speed_cached: body.linear_velocity.length(),
            row0: body.right(),
            up: body.up(),
            forward: body.forward(),
            position: body.position,
            velocity: body.linear_velocity,
            speed: body.linear_velocity.length(),
            // The energy pool, so a comparison can put a number on the recovered
            // contact-damage law instead of watching the bar. `oag-trace
            // replay` writes it too; the capture side is
            // `scripts/psp_trace_fields.py`'s `shield` at `entity+0x88`.
            shield: Some(ship.shield),
            flare: Some(Flare {
                boost_timer: exhaust.boost_timer(),
                plume_timer: exhaust.plume_timer(),
                intensity: exhaust.intensity(),
                half_size: exhaust.half_size(),
                engine_on: f32::from(u8::from(exhaust.engine_on())),
                speed_kmh: exhaust.speed_kmh(),
                speed_ramp: exhaust.speed_ramp(),
                boost_accumulator: exhaust.boost_accumulator(),
            }),
            ..Frame::default()
        }
    };

    let dt = oag_core::tick::TickRate::DEFAULT.dt();
    let mut trace = Trace::default();
    let mut held = race::HeldButtons::new(button_mask(cli.hold.as_deref()));
    let script = crate::args::input_script(cli.input_script.as_deref())?;
    for tick in 0..cli.ticks {
        trace.frames.push(frame_of(&race, dt));
        held.advance(
            script.as_ref(),
            button_mask(cli.press.as_deref()),
            button_mask(cli.hold.as_deref()),
            tick,
        );
        // One scripted pilot, in whichever slot the world says a person
        // flies - slot 0 for every headless run there is. A script with a
        // second pilot in it would fill a second entry here and nothing
        // downstream would change.
        let mut inputs = oag_gameplay::PlayerInputs::none();
        inputs.set(race.sim.world.primary_slot(), held.snapshot());
        race.tick(&inputs);
    }
    if let Some(age) = cli.pose_boost {
        race.force_boost_state(age, cli.pose_intensity, cli.pose_speed);
    }
    trace.frames.push(frame_of(&race, dt));

    let csv = trace.to_csv();
    std::fs::write(path, &csv)
        .with_context(|| format!("writing {} for --trace-out", path.display()))?;
    println!(
        "trace: {} row(s), {} column(s) -> {}",
        trace.len(),
        trace.columns().len(),
        path.display()
    );
    Ok(())
}

/// Loads a track and a ship and either captures one frame or opens a window.
///
/// **The settings apply here too**, even though this route never opens a menu
/// to change them with. It used to take only the aspect, which left a window
/// opened with `--race` ignoring the window size, the render scale and the
/// performance overlay that the same file was setting for every other route -
/// and the overlay is most wanted exactly here, where a track is on screen.
/// Nothing on this path writes the file back.
///
/// This paragraph sat on `write_trace` until finding G5 of the 2026-08-18
/// review - fused onto that function's own docs, leaving this one
/// undocumented and that one describing something it does not do.
pub(crate) fn run_race(
    cli: &Cli,
    options: race::Options,
    settings: &settings::Settings,
    anisotropy: Anisotropy,
    mut audio: oag_sound::Audio,
    music_discs: oag_sound::MusicDiscs,
) -> Result<()> {
    // `--event` resolves a Wipeout 2048 campaign event onto `options` before
    // loading - see `race::load_event`'s own doc comment for what it
    // overrides and why this is a second entry point rather than a new
    // `race::Options` field.
    //
    // **A screenshot opens its device first**, so the load can upload each
    // texture as it decodes it instead of holding every one for the scene -
    // `race::TextureSink`. The trace and `--dry-run` routes build no scene,
    // and the window's own device does not exist yet on the windowed route.
    let gpu = (cli.screenshot.is_some() && cli.trace_out.is_none() && !cli.dry_run)
        .then(|| oag_game::race_capture::gpu::CaptureGpu::request(&settings.graphics.renderer))
        .transpose()?;
    let loaded = {
        let sink = gpu
            .as_ref()
            .map(oag_game::race_capture::gpu::CaptureGpu::texture_sink);
        let _scope = race::TextureSink::open_if(sink.as_ref());
        match &cli.event {
            Some(name) => race::load_event(&options, name)?,
            None => race::load(&options)?,
        }
    };
    oag_raceplay::loader_log::lines(&loaded.report);

    if cli.dry_run {
        return Ok(());
    }

    let scheme = resolve_scheme(cli, settings);

    if let Some(path) = cli.trace_out.clone() {
        write_trace(loaded, cli, scheme, &path)?;
        return audio.finish();
    }

    if let Some(path) = cli.screenshot.clone() {
        // Same reasoning as the front end's own capture branch: no window to
        // stay in step with, but the music has to be running before
        // `oag_game::race_capture::capture`'s first tick. The race playlist rather than
        // `start_music`, so `--race` plays the same music a race launched from
        // the menus does - there is no menu voice here to switch away from,
        // so this simply starts the list at its first (or resumed) track.
        audio.start_race_music(
            &music_discs,
            settings.audio.music_source,
            &oag_source::cache::default_audio_cache_dir(),
        );
        // **The title's own profile**, which this leg used to have no way to
        // reach: `race::load` resolves the title to pick a default track and
        // team, and since it carries that out in `Loaded::title` there is
        // something to key `settings.render_profiles` on. Falls back to the
        // default for a title with no table entry, exactly as
        // `Session::render_profile` does.
        //
        // **The four CLI overrides are re-applied on top**, and forgetting them
        // is a bug this carried: `main.rs` applies them by walking
        // `settings.render_profiles`, which this profile was not in - so every
        // one of them was silently discarded on a headless capture, and `just
        // compare-upscalers` produced three byte-identical images while
        // reporting nothing wrong. See `Cli::apply_render_overrides`.
        let mut render_profile = settings
            .render_profiles
            .get(&settings::profile_key(loaded.title, loaded.platform))
            .cloned()
            .unwrap_or_default();
        let render_scale = match cli.render_scale {
            Some(percent) => Some(
                oag_display::display::Scale::try_from(percent)
                    .map_err(|why| anyhow::anyhow!("--render-scale {percent}: {why}"))?,
            ),
            None => None,
        };
        cli.apply_render_overrides(&mut render_profile, render_scale);
        // The same key `main::stage::build_race_stage` resolves for a real
        // session - see `oag_game::records::Key::track`'s own doc for why the
        // fallback order matters - read here only to look a row up, never to
        // write one back. See `oag_game::race_capture::CaptureOptions::previous_best`'s own doc
        // for why this capture never calls `oag_game::records::save`.
        let key = oag_game::records::Key::new(
            loaded.title.name,
            options.track.as_deref().or(Some(loaded.title.race.track)),
            loaded.setup.mode.name(),
            &loaded.setup.class,
        );
        let previous_best = oag_game::records::load().get(&key).cloned();
        let ghost = race::GhostCapture {
            race: cli.ghost.ghost.clone(),
            record: cli.ghost.record_ghost.clone().map(|path| {
                let team = loaded.liveries.first().map_or("", |l| l.team.as_str());
                let mut header = oag_game::ghosts::header(&key, team, loaded.setup.seed);
                header.options = oag_game::ghosts::race_options(
                    scheme,
                    loaded.setup.difficulty,
                    cli.autopilot,
                    give_weapon(cli.give.as_deref()).ok().flatten(),
                );
                (path, header)
            }),
        };
        oag_game::race_capture::capture(
            loaded,
            &oag_game::race_capture::CaptureOptions {
                gpu,
                give: give_weapon(cli.give.as_deref())?,
                autopilot: cli.autopilot,
                autopilot_pilot: autopilot_pilot(cli.autopilot_pilot.as_deref())?,
                autopilot_skill: cli.autopilot_skill,
                force_shake: crate::args::force_shake(cli.force_shake.as_deref())?,
                intro_ticks: cli.intro.intro_ticks,
                force_wreck: crate::args::force_wreck(cli.wreck.force_wreck.as_deref())?,
                force_shield: crate::args::force_shield(&cli.wreck.force_shield)?,
                force_medal: crate::args::force_medal(&cli.wreck.force_medal)?,
                path,
                ticks: cli.ticks,
                held: button_mask(cli.hold.as_deref()),
                pressed: button_mask(cli.press.as_deref()),
                input_script: crate::args::input_script(cli.input_script.as_deref())?,
                scheme,
                size: parse_size(&cli.size)?,
                log_every: cli.log_every,
                aspect: settings.display.aspect,
                anisotropy,
                renderer: settings.graphics.renderer.clone(),
                fov: settings.graphics.fov,
                frustum_culling: settings.graphics.frustum_culling,
                pvs_culling: cli.pvs.unwrap_or(settings.graphics.pvs_culling),
                anim_seconds: cli.anim_seconds,
                boost_fov_kick: settings.graphics.boost_fov_kick,
                camera_view: cli.camera_view.unwrap_or(settings.graphics.camera_view),
                msaa: render_profile.msaa,
                motion_blur: render_profile.motion_blur,
                motion_blur_resolution: render_profile.motion_blur_resolution,
                shadows: render_profile.shadows,
                model_detail: render_profile.model_detail,
                texture_detail: render_profile.texture_detail,
                camera_jitter: cli.camera_jitter,
                pose_boost: cli.pose_boost,
                pose_intensity: cli.pose_intensity,
                pose_speed: cli.pose_speed,
                presented: cli.presented.then_some(oag_game::race_capture::Presented {
                    render_scale: render_profile.render_scale,
                    presentation: oag_present::upscale::Presentation {
                        reconstruction: render_profile.reconstruction,
                        sharpness: render_profile.upscale_sharpness.stops(),
                        brightness: settings.display.brightness,
                        gamma: settings.display.gamma,
                    },
                }),
                zone_spectrum_test: cli.zone_spectrum_test,
                previous_best,
                ghost,
                // Resolved here rather than in `oag_game::race_capture::capture`, which has no
                // config directory in hand: the same catalogue the window
                // reads, so a `--presented` capture shows the same preset.
                screen_filter: oag_game::screen::Catalogue::load(
                    oag_game::screen::Catalogue::directory(),
                )
                .get(&render_profile.screen_filter)
                .cloned(),
                screen_filter_strength: render_profile.screen_filter_strength,
            },
            &mut audio,
        )?;
        return audio.finish();
    }

    let hint_strings = strings::project_table(settings.language.as_deref());
    println!(
        "\n{}{}",
        hints::race_keys(&hint_strings),
        hints::esc_quits(&hint_strings)
    );

    let give = give_weapon(cli.give.as_deref())?;

    let event_loop = EventLoop::new()?;
    // Poll rather than Wait: the simulation runs whether or not input arrives.
    event_loop.set_control_flow(ControlFlow::Poll);
    // Read before `loaded` is moved into the app below.
    let race_title = loaded.title;
    let race_platform = loaded.platform;
    let mut app = App {
        pvs_culling: cli.pvs,
        camera_jitter: cli.camera_jitter,
        boot_shell: None,
        media: None,
        boot_overlay: false,
        pick_language: false,
        // `--race` never shows the loading screen, so there is no
        // transition to time.
        measure_race_load: None,
        give,
        no_intro: cli.intro.no_intro,
        autopilot: cli.autopilot,
        autopilot_pilot: autopilot_pilot(cli.autopilot_pilot.as_deref())?,
        autopilot_skill: cli.autopilot_skill,
        anim_seconds: cli.anim_seconds,
        race: Some(loaded),
        race_options: Some(options),
        trace: cli.trace,
        log_every: cli.log_every,
        anisotropy,
        scheme,
        settings: settings.clone(),
        // `--race` opens a window straight onto a track: no front end, so no
        // font and no sprite sheet, so no menu tree and no circuit list. The
        // settings still apply, they just cannot be changed from here - which
        // is what makes escape quit on this route rather than back out.
        shell: None,
        // **What makes the sentence above true.** `Session::render_profile`
        // keys on the shell's title, and with no shell it fell all the way to
        // `RenderProfile::default()` - so this route ignored the render scale,
        // the reconstruction, the MSAA level, the motion blur *and* every CLI
        // flag that overrides them, silently. Measured before the fix:
        // `--render-scale 100` and `--render-scale 200` both gave a 0.80 ms
        // scene pass and zero FSR 3.1 chain readings.
        race_title: Some(race_title),
        race_platform: Some(race_platform),
        audio: Some(audio),
        music_discs,
        // `--race` says up front that it ignores `--prefetch` - that converts
        // the front end's movies and sounds, which a race never opens - so
        // there is nothing to wait on here and no loading screen to wait with.
        prefetch: None,
        // Nor anything to draw one with: this route never opens the archives
        // the tips and the glow strip come out of.
        loading_assets: loading::Assets::default(),
        // `--race` names its own source and is refused alongside `--launcher`,
        // so there is nothing here to choose and nothing left to prepare.
        launcher: None,
        pending: None,
        state: None,
    };
    event_loop.run_app(&mut app)?;
    app.finish_audio()
}

// `#[path]` is needed here for the same reason `main.rs` gives for its own
// `#[path]`ed children: this module was itself loaded via `#[path =
// "main/headless.rs"]`, so a bare `mod tests;` would resolve *next to* it, at
// `src/main/tests.rs` - which exists already, as `main.rs`'s own tests
// module - rather than under it at `src/main/headless/tests.rs`, where this
// one actually lives.
#[cfg(test)]
#[path = "headless/tests.rs"]
mod tests;
