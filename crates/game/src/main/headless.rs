//! The three runs that open no window.
//!
//! `--screenshot` without a display, `--trace-out`, and `--race` captured
//! headless. Each takes the whole boot in one blocking call rather than
//! deferring the movies onto a worker, because a capture must not race a
//! worker for the frames it is about to draw.

use anyhow::{Context, Result};
use log::{info, warn};

use oag_game::{audio, boot, capture, loading, prefetch, race, settings};
use oag_gameplay::ControlScheme;
use oag_render::mesh_render::Anisotropy;

use winit::event_loop::{ControlFlow, EventLoop};

use crate::app::App;
use crate::args::{
    button_mask, give_weapon, parse_progress, parse_size, parse_step, resolve_scheme,
};
use crate::cli::Cli;
use crate::hints::{ESC_QUITS, RACE_KEYS};

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
    mut audio: audio::Audio,
    music_discs: audio::MusicDiscs,
) -> Result<()> {
    let mut loaded = boot::load(options)?;
    if cli.overlay {
        loaded.frontend.set_overlay(true);
    }
    // A language chosen on an earlier run skips the picker. Reported either
    // way: silently not asking is indistinguishable from a broken picker, and
    // silently asking again is indistinguishable from a setting that did not
    // save.
    match (cli.pick_language, settings.language.as_deref()) {
        (false, Some(name)) if loaded.frontend.preselect_language(name) => {
            info!("language {name} from settings, skipping the picker");
        }
        (false, Some(name)) => {
            warn!("this source does not offer {name:?}, so the picker is shown");
        }
        _ => {}
    }
    for line in &loaded.report {
        info!("{line}");
    }

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
            audio: boot::default_audio_cache_dir(),
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
            progress.current = cli.track.clone();
        }
        let assets = loading::Assets::load(
            &options.source,
            &loaded.strings,
            loaded.entries.as_deref(),
            crate::args::style_of(settings),
        );
        for note in &assets.notes {
            info!("{note}");
        }
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
                path,
                until: cli.until.clone(),
                ticks: cli.ticks,
                anim_seconds: cli.anim_seconds,
                held: button_mask(cli.hold.as_deref()),
                pressed: button_mask(cli.press.as_deref()),
                scheme,
                trace: cli.trace,
                race: Some(race_options),
                log_every: cli.log_every,
                size: parse_size(&cli.size)?,
                screen: cli.screen.clone(),
                menu_page: cli.menu_page.clone(),
                menu_anim_phase: cli.menu_anim_phase,
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
/// One row per tick, then one final row, mirroring `race::capture`: a row is
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
        let ship = &race.world.ships[0].physics;
        let body = &ship.body;
        let exhaust = race.exhaust();
        Frame {
            tick: race.world.tick,
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
    for tick in 0..cli.ticks {
        trace.frames.push(frame_of(&race, dt));
        held.pulse(
            button_mask(cli.press.as_deref()),
            button_mask(cli.hold.as_deref()),
            tick.is_multiple_of(2),
        );
        let snapshot = held.snapshot();
        race.tick(&snapshot);
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
    mut audio: audio::Audio,
    music_discs: audio::MusicDiscs,
) -> Result<()> {
    let loaded = race::load(&options)?;
    for line in &loaded.report {
        info!("{line}");
    }

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
        // `race::capture`'s first tick. The race playlist rather than
        // `start_music`, so `--race` plays the same music a race launched from
        // the menus does - there is no menu voice here to switch away from,
        // so this simply starts the list at its first (or resumed) track.
        audio.start_race_music(
            &music_discs,
            settings.audio.music_source,
            &boot::default_audio_cache_dir(),
        );
        race::capture(
            loaded,
            &race::CaptureOptions {
                give: give_weapon(cli.give.as_deref())?,
                autopilot: cli.autopilot,
                path,
                ticks: cli.ticks,
                held: button_mask(cli.hold.as_deref()),
                pressed: button_mask(cli.press.as_deref()),
                input_script: cli
                    .input_script
                    .as_deref()
                    .map(|path| -> anyhow::Result<_> {
                        let text = std::fs::read_to_string(path)
                            .with_context(|| format!("reading {}", path.display()))?;
                        oag_trace::script::Script::parse(&text)
                            .map_err(|e| anyhow::anyhow!("parsing {}: {e}", path.display()))
                    })
                    .transpose()?,
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
                bloom: settings.graphics.bloom,
                boost_fov_kick: settings.graphics.boost_fov_kick,
                camera_view: cli.camera_view.unwrap_or(settings.graphics.camera_view),
                anti_aliasing: settings.graphics.anti_aliasing,
                motion_blur: settings.graphics.motion_blur,
                pose_boost: cli.pose_boost,
                pose_intensity: cli.pose_intensity,
                pose_speed: cli.pose_speed,
                presented: cli.presented.then_some(race::Presented {
                    render_scale: settings.graphics.render_scale,
                    presentation: oag_game::upscale::Presentation {
                        upscaler: settings.graphics.upscaler,
                        sharpness: settings.graphics.upscale_sharpness.stops(),
                        anti_aliasing: settings.graphics.anti_aliasing,
                        brightness: settings.display.brightness,
                        gamma: settings.display.gamma,
                    },
                }),
            },
            &mut audio,
        )?;
        return audio.finish();
    }

    println!("\n{RACE_KEYS}{ESC_QUITS}");

    let give = give_weapon(cli.give.as_deref())?;

    let event_loop = EventLoop::new()?;
    // Poll rather than Wait: the simulation runs whether or not input arrives.
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        pvs_culling: cli.pvs,
        boot_shell: None,
        media: None,
        boot_overlay: false,
        pick_language: false,
        give,
        autopilot: cli.autopilot,
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
