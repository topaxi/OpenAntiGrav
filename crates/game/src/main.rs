//! Runs Wipeout Pulse from the user's own disc image.
//!
//! ```sh
//! oag-game data/images/pulse-psp-eu.chd
//! ```
//!
//! Boots into `LogoFMV`, the screen the disc's own boot reaches: it plays
//! `Data\Movies\Intro.PMF` straight through, START or X skips it, and then the
//! Language Selection screen driven
//! by the disc's own front-end XML. Arrow keys move, Return or X selects, which
//! fires `Launch Game` - and `Launch Game` loads a track and a ship and hands the
//! window over to a race, in the same window and on the same GPU device.
//!
//! ```sh
//! # No display needed. Runs the sequence headless and writes one frame.
//! oag-game data/images/pulse-psp-eu.chd --screenshot /tmp/menu.png \
//!     --until "Language Selection" --hold start
//! ```
//!
//! `--race` is the shortcut into the second half: the same race, without booting
//! the front end first.
//!
//! ```sh
//! oag-game --race
//! oag-game --race --screenshot /tmp/race.png --ticks 600 --hold cross
//! ```
//!
//! Named no image at all, it looks for one - and if the search path holds
//! several, it says so on screen instead of picking one quietly:
//!
//! ```sh
//! oag-game            # one image found: boots it, as it always did
//! oag-game --launcher # the chooser, whatever is there
//! ```
//!
//! See [`oag_game::launcher`] and `just launch`.
//!
//! This file is only `main` itself: read the command line, resolve everything
//! that can fail before anything is loaded, and hand off to one of the runs in
//! [`headless`] or to the window in [`app`]. **"Everything that can fail" no
//! longer includes the source on the windowed route**: which disc image a run
//! opens may be a screen away, so the load that depends on it lives in
//! [`prepare`] and runs either here or from `Session::finish_launcher`. The
//! rest of the binary is the modules declared below; everything that can be
//! tested without a GPU is in [`oag_game`] rather than in any of them.

use anyhow::{Context, Result, ensure};
use clap::Parser;
use log::warn;
use oag_game::{launcher, loading, settings};
use oag_raceplay as race;
use oag_source::source;
use oag_ui::frontend;

use winit::event_loop::{ControlFlow, EventLoop};

// The bin's own modules live under `src/main/`, which a crate root cannot reach
// on its own: `mod cli;` here would resolve to `src/cli.rs`, beside the
// library's own modules. `#[path]` names the directory instead, and it is needed
// at **every** level, because a `#[path]`ed module's children resolve *next to*
// it rather than under it - `session.rs` repeats the attribute for its four.
//
// The four stage modules are `<name>_stage` rather than `stage::<name>`: the
// library already owns `menu`, `loading`, `frontend` and `race`, and a child
// module of those names shadows the import every one of them needs.
#[path = "main/app.rs"]
mod app;
#[path = "main/args.rs"]
mod args;
#[path = "main/campaign_stage.rs"]
mod campaign_stage;
#[path = "main/cli.rs"]
mod cli;
#[path = "main/frontend_stage.rs"]
mod frontend_stage;
#[path = "main/gpu.rs"]
mod gpu;
#[path = "main/headless.rs"]
mod headless;
#[path = "main/hints.rs"]
mod hints;
#[path = "main/launcher_stage.rs"]
mod launcher_stage;
#[path = "main/loading_stage.rs"]
mod loading_stage;
#[path = "main/menu_stage.rs"]
mod menu_stage;
#[path = "main/overlay.rs"]
mod overlay;
#[path = "main/picker_stage.rs"]
mod picker_stage;
#[path = "main/pointer.rs"]
mod pointer;
#[path = "main/pose.rs"]
mod pose;
#[path = "main/prepare.rs"]
mod prepare;
#[path = "main/race_build.rs"]
mod race_build;
#[path = "main/race_stage.rs"]
mod race_stage;
#[path = "main/rebind.rs"]
mod rebind;
#[path = "main/records_page.rs"]
mod records_page;
#[path = "main/session.rs"]
mod session;
#[path = "main/stage.rs"]
mod stage;
#[path = "main/typing.rs"]
mod typing;
#[path = "main/window.rs"]
mod window;

use crate::app::App;
use crate::args::{autopilot_pilot, give_weapon};
use crate::cli::Cli;
use crate::headless::{run_race, run_windowless};
use crate::pose::{parse_camera_pose, parse_pose, pose_from_trace};

/// Installs the sink every `log` call in this workspace ends up in.
///
/// **The default filter is `warn` globally with our own crates at `info`.**
/// A normal launch prints a handful of lifecycle lines (the disc found, the
/// renderer chosen, a race starting) and whatever degraded or missing, which
/// is `warn` and always shown. Everything a loader merely *did* - counts,
/// sizes, timings, one line per cue or ship - is `debug` or `trace`. The rule
/// that sorts a message into a level, and the `RUST_LOG` values that bring the
/// rest back, are in `docs/architecture/logging.md`.
///
/// Everything else stays at `warn` because it is not ours to read - at `info`
/// the graphics stack alone narrates every adapter, shader module and pipeline
/// it builds, and the engine's own lines drown in it.
///
/// **`calloop` is pinned to `error`**, and it is the one third-party crate
/// singled out by name. winit's Wayland backend removes and re-inserts a
/// key-repeat timer source on every key press
/// (`winit/src/platform_impl/linux/wayland/seat/keyboard/mod.rs`), so a loaded
/// machine routinely dispatches a batch that still holds an event for the
/// token that was just removed, and calloop warns `Received an event for
/// non-existence source` once per occurrence. The dropped event is a stale
/// repeat tick for a key this game already tracks itself through
/// `Controls::set_key`, so nothing is lost and there is nothing for a player
/// to do about it - it is upstream noise that happens to be loudest exactly
/// when a race is least able to spare the attention.
///
/// `RUST_LOG` replaces the whole expression: `RUST_LOG=warn,oag=debug` for
/// what the loaders did, `RUST_LOG=warn,oag=trace` for every line there is,
/// `RUST_LOG=warn,oag_sound=trace` for one module,
/// `RUST_LOG=warn,wgpu_core=info` to hear the graphics stack instead, or
/// `RUST_LOG=warn,calloop=warn` to put the line above back.
///
/// The format carries the level and nothing else. These lines are read by a
/// player watching a terminal, not shipped to a collector, and a timestamp and
/// a module path on each would be wider than most of the messages.
fn init_logging() {
    oag_log::install("warn,oag=info,calloop=error", oag_log::tool::FILE_FILTER);
    oag_log::install_panic_hook();
}

/// Opens the log file once the command line and the settings file are known,
/// the two things its path depends on. Lines logged since `init_logging` are
/// held and written first.
///
/// **A file is written in addition to the terminal, never instead**, and the
/// terminal is exactly what `init_logging` always made it. The file has its own,
/// more verbose filter, a timestamp and the module path on every line, and is
/// appended to across runs with entries older than a week pruned at startup: see
/// `oag_log`. `--log-file ''` or `[log] file = ""` writes none. An unwritable
/// location is a `warn` on the terminal, not a failed launch.
fn attach_log_file(cli: &Cli, settings: &settings::Settings) {
    let default = oag_log::path::default_path(
        oag_log::path::Os::here(),
        &oag_log::path::Env::from_process(),
        "oag-game",
    );
    let Some(file) = oag_log::path::resolve(
        cli.log.file.as_deref(),
        settings.log.file.as_deref(),
        default,
    ) else {
        oag_log::detach();
        return;
    };
    let source = cli
        .source
        .clone()
        .or_else(|| std::env::var("OAG_IMAGE").ok())
        .or_else(|| settings.source.image.clone());
    let header =
        oag_game::logfile::header(source.as_deref(), &|key| std::env::var(key).ok(), &|path| {
            path.exists()
        });
    if let Err(why) = oag_log::attach(&file, settings.log.filter.as_deref(), &header) {
        warn!("not writing a log file at {}: {why}", file.display());
    }
}

/// The counting allocator behind `oag-render`'s off-by-default `perf-probe`
/// feature, and the reason it is installed here rather than in a test: it has
/// to be the process's allocator to see the frame path's allocations at all.
///
/// Absent entirely without the feature - not merely inert - so a default build
/// keeps the system allocator with no wrapper in front of it.
#[cfg(feature = "perf-probe")]
#[global_allocator]
static ALLOCATOR: oag_gpu::perfprobe::Counting = oag_gpu::perfprobe::Counting;

#[cfg_attr(target_os = "android", allow(dead_code))]
fn main() -> Result<()> {
    init_logging();
    let result = run(Cli::parse());
    oag_log::flush();
    result
}

/// The NativeActivity entry point: `liboag_game.so` is this file built as a
/// cdylib (`just apk`: `cargo rustc --bin oag-game -- --crate-type cdylib`). See `docs/tools/android.md`.
///
/// There is no command line and no working directory on Android, so the app's
/// external files directory stands in for both: it becomes the current
/// directory and the XDG roots, which puts `data/images`, `data/cache`, the
/// settings, records, ghosts and the log under
/// `/storage/emulated/0/Android/data/<package>/files/` without `oag-source`
/// knowing about Android at all.
#[cfg(target_os = "android")]
#[allow(unsafe_code)] // the C ABI symbol NativeActivity looks up by name
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_env(&app);
    init_logging();
    let _ = ANDROID_APP.set(app);
    let result = run(Cli::parse_from(["oag-game"]));
    oag_log::flush();
    if let Err(why) = result {
        log::error!("oag-game exited: {why:#}");
    }
}

#[cfg(target_os = "android")]
static ANDROID_APP: std::sync::OnceLock<winit::platform::android::activity::AndroidApp> =
    std::sync::OnceLock::new();

/// Points the working directory and the XDG roots at the app's files directory.
#[cfg(target_os = "android")]
#[allow(unsafe_code)] // `set_var`, see below
fn android_env(app: &winit::platform::android::activity::AndroidApp) {
    let Some(files) = app
        .external_data_path()
        .or_else(|| app.internal_data_path())
    else {
        return;
    };
    for (name, sub) in [
        ("HOME", "home"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_STATE_HOME", "state"),
    ] {
        let dir = files.join(sub);
        let _ = std::fs::create_dir_all(&dir);
        // SAFETY: first thing `android_main` does, before any thread of ours
        // exists to race a read of the environment.
        unsafe { std::env::set_var(name, &dir) };
    }
    let _ = std::fs::create_dir_all(files.join("data").join("images"));
    let _ = std::env::set_current_dir(&files);
}

/// The window system's event loop: winit's default everywhere but Android,
/// where it has to be handed the activity.
fn new_event_loop() -> Result<EventLoop<()>> {
    #[cfg(target_os = "android")]
    {
        use winit::event_loop::EventLoopBuilder;
        use winit::platform::android::EventLoopBuilderExtAndroid;
        let app = ANDROID_APP
            .get()
            .context("no AndroidApp: not started by android_main")?
            .clone();
        Ok(EventLoopBuilder::default().with_android_app(app).build()?)
    }
    #[cfg(not(target_os = "android"))]
    Ok(EventLoop::new()?)
}

fn run(cli: Cli) -> Result<()> {
    // Ahead of settings and the disc search, deliberately: rasterising the
    // icon needs neither, and `just install-desktop-file` /
    // `scripts/build-appimage.sh` both call this on machines that may have no
    // disc image configured at all. See `oag_game::icon` and
    // `crate::window::window_icon`, the winit-facing use of the same function.
    if let Some(path) = &cli.icon.write_icon {
        // **Validated here, because `rasterize`'s `expect` is not a CLI error
        // message.** That function's own docs say no player input reaches it;
        // `--icon-size` does, and `--write-icon out.png --icon-size 0` panicked
        // rather than saying what was wrong. Finding G2 of the 2026-08-18
        // review.
        anyhow::ensure!(cli.icon.icon_size > 0, "--icon-size must be at least 1");
        let oag_game::icon::Rgba {
            width,
            height,
            pixels,
        } = oag_game::icon::rasterize(cli.icon.icon_size);
        let png = oag_texture::png::encode_rgba(width, height, &pixels);
        std::fs::write(path, png).with_context(|| format!("writing {}", path.display()))?;
        println!("wrote {}x{} {}", width, height, path.display());
        return Ok(());
    }

    // Loaded (and, on first run or a missing key, written back complete) before
    // anything else: a bad value in the file should fail immediately, not eight
    // seconds of intro later.
    let mut settings = settings::load()?;
    attach_log_file(&cli, &settings);
    let anisotropy = cli.anisotropy.unwrap_or(settings.graphics.anisotropy);
    // A CLI render-profile flag overrides *every* title's entry for this run
    // only - never persisted, the same footing `anisotropy` above is already
    // on. There is no title open yet this early, so there is no one title to
    // single out; a flag naming one and not the rest would silently leave the
    // rest of a play session's titles unaffected the moment a different disc
    // is chosen, which is not what typing the flag once means.
    let render_scale = match cli.render_scale {
        Some(percent) => Some(
            oag_display::display::Scale::try_from(percent)
                .map_err(|why| anyhow::anyhow!("--render-scale {percent}: {why}"))?,
        ),
        None => None,
    };
    for profile in settings.render_profiles.values_mut() {
        cli.apply_render_overrides(profile, render_scale);
    }

    // Parsed before anything is loaded, and for both ways in: the front end can
    // hand off to a race, so a misspelled class must not be discovered eight
    // seconds of intro later.
    // Checked against the union of every ladder read so far rather than
    // against `oag_physics::SpeedClass`'s four, so `--class vector` reaches a
    // Wipeout Pure race. This is spell-checking only: whether *this* disc's
    // files author the rung is settled by `race::load`, which names what the
    // file does carry when they do not.
    let class = cli.class.trim().to_string();
    anyhow::ensure!(
        oag_title::SpeedClasses::is_measured_name(&class),
        "{:?} is not a speed class; try {}",
        cli.class,
        oag_title::SpeedClasses::MEASURED.join(", ").to_lowercase()
    );
    // Parsed here for the same reason as the speed class: `--race` goes straight
    // to a track, so a misspelled mode has to be a message about the command
    // line rather than a race that quietly runs under different rules.
    // `head_to_head` is outside `Mode::ALL` (the RACE page has no row for it),
    // so a campaign cell is the only way a player reaches it. It is accepted
    // here as a verification aid, so a headless capture can exercise the mode.
    let mode = oag_race::Mode::from_name(&cli.mode)
        .or_else(|| (cli.mode == oag_race::Mode::Head2Head.name()).then_some(oag_race::Mode::Head2Head))
        .with_context(|| {
        format!(
            "{:?} is not a race mode; try time_trial, speed_lap, zone, single_race or head_to_head",
            cli.mode
        )
    })?;
    // Refused rather than tolerated, and the reason is memory rather than
    // tidiness: a dump accumulates every sample it renders, so a windowed run
    // that a player leaves open grows the buffer for as long as they play - a
    // quarter of a gigabyte in ten minutes, at 48 kHz stereo `f32`. A capture is
    // bounded by `--ticks`, which is the only route that ends.
    ensure!(
        cli.dump_audio.is_none() || cli.screenshot.is_some(),
        "--dump-audio needs --screenshot: the dump is as long as the run, and only \
         a capture has an end. Add --screenshot FILE --ticks N."
    );
    // Refused for the same reason `--screen` and `--menu-page` are capture-only:
    // there is no window route that draws a *stated* conversion state, and
    // silently ignoring the flag would look like a screen that does not work.
    ensure!(
        cli.loading_screen.is_none() || cli.screenshot.is_some(),
        "--loading-screen needs --screenshot: it draws one frame of the loading \
         screen at a stated state. A window shows the real one under --prefetch."
    );

    // Refused rather than ignored: the chooser is a screen, and every one of
    // these three routes is defined by not opening a window to show it on.
    ensure!(
        !cli.launcher || !(cli.race || cli.dry_run || cli.screenshot.is_some()),
        "--launcher needs a window: it is a screen for choosing a disc image, and \
         --race, --dry-run and --screenshot all name their own source and open none. \
         Run it on its own, or name an image directly."
    );

    // Resolved beside the source below, but it cannot fail: no DLC is the
    // ordinary state of a copy of the game.
    let dlc = source::resolve_dlc(&cli.dlc, &settings.source.dlc);
    oag_game::unlock::set_unlock_all(cli.unlock_all);

    // Opened before either way in, because both want sound and neither owns the
    // other. **Only the device, not the music itself** - `start_music` is
    // deferred to whichever tick loop is actually about to run, windowed or
    // headless, so that a player never hears the front end before the window
    // that shows it exists. See the two call sites below and
    // `App::open`.
    // `--tap-audio` records what the device is actually handed, which
    // `--dump-audio` cannot: that one forces the null backend by construction.
    let tap = cli.tap_audio.clone().map(|path| oag_audio::TapSpec {
        seconds: cli.tap_seconds,
        path,
    });
    // **Two and a half frames of whatever the loop is capped at, floored at
    // `oag_audio::MIN_BUFFER`.** The render-ahead queue exists to cover a stall
    // and a stall is a frame, so a fixed number of milliseconds is the wrong
    // unit: 60 ms is three and a half frames at 60 Hz and one and four fifths
    // at the 30 the menus also offer. Unlimited and anything above 60 Hz take
    // the floor.
    //
    // Read off the *configured* cap rather than the achieved rate: the ring is
    // sized when the stream is built and does not follow a frame rate that
    // turns out slower than the player asked for. That case is named in
    // `handover/` rather than papered over here.
    let buffer = settings
        .display
        .frame_limit
        .period()
        .map_or(oag_audio::MIN_BUFFER, |frame| {
            frame.mul_f32(2.5).max(oag_audio::MIN_BUFFER)
        });
    let audio = oag_sound::Audio::open(
        &settings.audio,
        cli.dump_audio.clone(),
        tap.as_ref(),
        buffer,
        cli.no_audio,
    );

    let (pose, camera) = match &cli.pose_from {
        Some(path) => {
            let (pose, camera) =
                pose_from_trace(path, cli.pose_tick, cli.no_camera, cli.camera_fov)?;
            (Some(pose), camera)
        }
        // `--camera-pose` is a camera and nothing else, so it composes with
        // `--pose` rather than replacing it: an RPCS3 capture gives the camera
        // exactly and the craft only as whatever the frame shows.
        None => (
            cli.pose
                .as_deref()
                .map(parse_pose)
                .transpose()?
                .map(|(position, yaw)| race::PoseRequest::SplineAligned { position, yaw }),
            cli.camera_pose
                .as_deref()
                .map(parse_camera_pose)
                .transpose()?,
        ),
    };
    // Applied after either way in, because both can produce a camera and the
    // flag means the same thing to each.
    let camera = camera.map(|camera| race::CameraOverride {
        fov_deg: cli.camera_fov.or(camera.fov_deg),
        ..camera
    });

    let leg = if cli.reel {
        frontend::Leg::DevPubReel
    } else {
        frontend::Leg::LogoFmv
    };

    // Everything the command line decided that no disc is needed for. The
    // source is deliberately not in here: on the launcher route it is not known
    // until a window has been open for a while. See `crate::prepare`.
    let pending = prepare::Pending {
        definition: prepare::definition(&cli, settings.language.as_deref())?,
        settings,
        dlc,
        class,
        mode,
        leg,
        pose,
        camera,
        cli,
    };
    // Read back off `pending` from here on: the command line moved into it, and
    // a second copy of any of these is a second thing to keep in step.
    let cli = &pending.cli;
    let settings = &pending.settings;

    // Before `boot::load`, deliberately: the front end's load parses the front-end
    // XML, every language plugin and a string table, and may shell out to `ffmpeg`
    // to transcode the intro. Going straight to a race needs none of it.
    //
    // Its source is resolved here rather than up with the rest of the command
    // line, because this is the first route that actually needs one: the
    // launcher route below has no single answer to resolve. Still before
    // anything opens it, so "no disc image found" stays a message about the
    // command line rather than something to discover eight seconds of intro
    // later.
    if cli.race {
        // `--prefetch` is a front-end thing, and saying so is better than
        // quietly doing nothing. `--race` exists to skip the boot sequence, and
        // hanging its exit on ten minutes of `ffmpeg` would invert the one
        // thing it is for.
        if cli.prefetch {
            warn!(
                "--prefetch has no effect with --race: it converts the front end's movies and \
                 sounds, which a race never opens. Run it without --race once."
            );
        }
        // Same reasoning one screen along: the loading screen is what a player
        // waits on while that conversion runs, so on a route that does not run
        // it there is nothing for the screen to be about.
        if cli.loading_screen.is_some() {
            warn!("--loading-screen has no effect with --race; run it without --race.");
        }
        let source = source::resolve(cli.source.as_deref(), settings.source.image.as_deref())?;
        let music_discs = pending.music_discs(&source);
        return run_race(
            cli,
            pending.race_options(&source, Vec::new()),
            settings,
            anisotropy,
            audio,
            music_discs,
        );
    }

    // Every leg with no window loads the whole boot here and now, blocking, and
    // then leaves. Only a window has anywhere to *show* a wait, so only a
    // window earns the machinery below that defers one.
    if cli.dry_run || cli.screenshot.is_some() {
        let source = source::resolve(cli.source.as_deref(), settings.source.image.as_deref())?;
        let music_discs = pending.music_discs(&source);
        return run_windowless(
            &pending.cli,
            &pending.boot_options(&source),
            settings,
            pending.race_options(&source, Vec::new()),
            anisotropy,
            audio,
            music_discs,
        );
    }

    // Which way in this window takes: straight to a boot, or the chooser first.
    //
    // The chooser is offered only when nothing was *stated* - see
    // `source::explicit` - and only when there is a choice to make. One image
    // is not a choice, and a screen that has to be dismissed before every boot
    // would be a step added to the common case for nothing.
    let chosen = source::explicit(cli.source.as_deref(), settings.source.image.as_deref())?;
    let launcher = match &chosen {
        Some(_) if !cli.launcher => None,
        _ => {
            let paths = source::candidates();
            // Silent on an empty search path: there is nothing to read, and
            // `resolve`'s own message a moment later names every directory
            // that was tried, which is the useful half of the same news.
            if !paths.is_empty() {
                println!(
                    "disc images: {} under the search path, reading each one",
                    paths.len()
                );
            }
            let rows = launcher::survey(&paths);
            for row in &rows {
                println!(
                    "  {:<14} {:<18} {}",
                    row.title(),
                    row.provenance(),
                    row.name
                );
                // The whole reason, here rather than on screen: a terminal has
                // no right edge to run off. See `launcher::advice`.
                if let Some(advice) = launcher::advice(row) {
                    println!("    {advice}");
                }
            }
            // One image is no choice, and **none at all is never a screen, not
            // even under `--launcher`**: an empty list offers nothing but
            // escape, where falling through reaches either the stated source or
            // `resolve`'s message naming every directory it looked in - which
            // for a packaged build with no image is the entire user interface.
            if rows.len() > 1 || (cli.launcher && !rows.is_empty()) {
                Some(launcher::Launcher::new(rows))
            } else if rows.is_empty() && cfg!(target_os = "android") {
                // A phone has no terminal for `resolve`'s message and its
                // window would only close, so the chooser's own screen says it.
                let images = std::env::current_dir()
                    .map_or_else(|_| "data".into(), |dir| dir.join("data").join("images"));
                let images = images.display().to_string();
                let images = images.replacen("/storage/emulated/0", "/sdcard", 1);
                log::error!("no disc image found; adb push one to {images}/");
                Some(launcher::Launcher::not_found(launcher::not_found_notice(
                    &images,
                )))
            } else {
                None
            }
        }
    };

    // With no chooser there is exactly one source and it is loaded before the
    // window, as it always was. With one, everything below waits for the pick -
    // see `Session::finish_launcher`.
    let prepared = match &launcher {
        Some(_) => None,
        None => {
            let source = match chosen {
                Some(source) => source,
                None => source::resolve(None, None)?,
            };
            Some(pending.windowed(&source)?)
        }
    };

    // Split apart here rather than carried as one: `App`'s fields are what the
    // window's first resume reaches for, and half of them are `Option` already
    // because the `--race` route has none of them either. A run that is still
    // choosing is in the same position as that one - it has no shell, no boot
    // and no race yet - so it takes the same shape.
    let (boot_shell, media, shell, loading_assets, race_options, music_discs, prefetch) =
        match prepared {
            Some(prepared) => (
                Some(prepared.boot_shell),
                Some(prepared.media),
                Some(prepared.shell),
                prepared.loading_assets,
                Some(prepared.race_options),
                prepared.music_discs,
                prepared.prefetch,
            ),
            None => (
                None,
                None,
                None,
                loading::Assets::default(),
                None,
                oag_sound::MusicDiscs::default(),
                None,
            ),
        };

    let scheme = pending.scheme();
    let give = give_weapon(cli.give.as_deref())?;
    let autopilot_pilot = autopilot_pilot(cli.autopilot_pilot.as_deref())?;
    let autopilot_skill = cli.autopilot_skill;
    let (boot_overlay, pick_language, autopilot, trace, log_every) = (
        cli.overlay,
        cli.pick_language,
        cli.autopilot,
        cli.trace,
        cli.log_every,
    );
    let anim_seconds = cli.anim_seconds;
    let settings = settings.clone();

    let event_loop = new_event_loop()?;
    // Poll rather than Wait: the intro is animated whether or not input arrives.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        pvs_culling: cli.pvs,
        camera_jitter: cli.camera_jitter,
        boot_shell,
        media,
        boot_overlay,
        pick_language,
        measure_race_load: cli.measure.measure_race_load,
        give,
        no_intro: cli.intro.no_intro,
        prompt_style: cli
            .menu_args
            .prompt_style
            .as_deref()
            .and_then(oag_input::prompt::PromptStyle::from_name),
        autopilot,
        autopilot_pilot,
        autopilot_skill,
        anim_seconds,
        race: None,
        race_options,
        trace,
        log_every,
        anisotropy,
        scheme,
        settings,
        shell,
        // Every route through `main` has a shell or is about to load one;
        // only `--race`, which is `headless::run_race`, has neither.
        race_title: None,
        race_platform: None,
        audio: Some(audio),
        music_discs,
        prefetch,
        loading_assets,
        launcher,
        // Kept whole, because a pick is what turns it into everything above.
        pending: Some(pending),
        state: None,
        suspended: false,
    };
    event_loop.run_app(&mut app)?;
    app.finish_audio()?;
    app.finish_prefetch();
    Ok(())
}

/// What [`tests`] reaches through its own `use super::*`, and `main` itself does
/// not.
///
/// A private `use` in the crate root is visible to every module below it, so
/// naming them here is what let the split leave the test file's body untouched -
/// the tests still call `menu_playhead` and `parse_progress` unqualified,
/// exactly as they did when all three lived in this file.
#[cfg(test)]
use crate::{
    args::parse_progress, frontend_stage::HeldFrame, menu_stage::menu_playhead, session::Session,
    session::menus::backdrop_seed,
};
/// Same reasoning, for a library module `main` itself no longer names: the
/// windowed load moved into [`prepare`] and took `movie` with it. `menu` and
/// `language` are here for [`Session::maybe_begin_binding`]'s own tests, which
/// need a [`menu::Menu`] and nothing else this file already imports names one.
#[cfg(test)]
use oag_game::movie;
#[cfg(test)]
use oag_input::Controls;
#[cfg(test)]
use oag_ui::{language, menu};

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
