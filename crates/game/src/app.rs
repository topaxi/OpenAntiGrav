use crate::{launcher, loading, settings};
use anyhow::{Context, Result, ensure};
use clap::Parser;
use log::{info, warn};
use oag_raceplay as race;
use oag_source::source;
use oag_ui::frontend;

use winit::event_loop::{ControlFlow, EventLoop};

mod args;
mod campaign_editor_stage;
mod campaign_stage;
mod cli;
pub(crate) mod clipboard;
mod device_loss;
#[cfg(not(target_arch = "wasm32"))]
mod drive;
mod from_png;
mod frontend_stage;
mod gpu;
mod handler;
mod headless;
pub(crate) mod hints;
mod join_stage;
mod launcher_stage;
mod loading_stage;
mod menu_stage;
mod overlay;
mod picker_stage;
mod pointer;
mod pose;
mod prepare;
mod pure_chain_stage;
mod race_build;
mod race_stage;
mod rebind;
mod records_page;
mod seat_assign_stage;
mod session;
mod soft_keyboard;
mod stage;
mod touch;
mod typing;
#[cfg(target_os = "linux")]
mod wallpaper;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub mod web;
mod window;
mod window_layout_stage;
mod windows;
#[cfg(target_os = "linux")]
mod x11_refresh;

use crate::app::args::{autopilot_pilot, give_weapon};
use crate::app::cli::Cli;
use crate::app::handler::App;
use crate::app::headless::{race_launch, run_windowless};
use crate::app::pose::{
    capture_hud, field_from_csv, parse_camera_pose, parse_pose, pose_from_trace,
};

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
    oag_input::pad::silence_on_panic();
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
        .boot
        .source
        .clone()
        .or_else(|| std::env::var("OAG_IMAGE").ok())
        .or_else(|| settings.source.image.clone());
    let header =
        crate::logfile::header(source.as_deref(), &|key| std::env::var(key).ok(), &|path| {
            path.exists()
        });
    if let Err(why) = oag_log::attach(&file, settings.log.filter.as_deref(), &header) {
        warn!("not writing a log file at {}: {why}", file.display());
    }
}

/// The desktop and iOS entry point: reads the command line and runs it.
pub fn main() -> Result<()> {
    // iOS has no command line (and Xcode passes its own flags) and starts in the
    // read-only bundle: settle the working directory before anything reads it.
    #[cfg(target_os = "ios")]
    oag_mobile::ios::prepare();
    init_logging();
    #[cfg(target_os = "ios")]
    let cli = Cli::parse_from(["oag-game"]);
    #[cfg(not(target_os = "ios"))]
    let cli = Cli::parse();
    let result = run(cli);
    // Every route out of `run` has dropped its session by now; a worker still inside the driver
    // must finish before the process exit tears the driver down under it.
    oag_thread::gpu_workers::finish();
    oag_log::flush();
    result
}

/// The NativeActivity body: `liboag_android.so` (`just apk`, the `oag_android`
/// example) exports the `android_main` symbol and calls this. See
/// `docs/tools/android.md`; the storage setup and the event loop are
/// `oag_mobile::android`.
#[cfg(target_os = "android")]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    oag_mobile::android::prepare(app);
    init_logging();
    match oag_mobile::android::install_official_mods(std::path::Path::new(oag_mod::OFFICIAL)) {
        Ok(done) => log::debug!("official mods in {}: {done:?}", oag_mod::OFFICIAL),
        Err(why) => log::warn!("official mods not installed: {why}"),
    }
    let args = oag_mobile::android::take_launch_args();
    if !args.is_empty() {
        log::info!(
            "launch arguments from {}: {args:?}",
            oag_mobile::android::LAUNCH_ARGS_FILE
        );
    }
    let result = match Cli::try_parse_from(std::iter::once("oag-game".to_owned()).chain(args)) {
        Ok(cli) => run(cli),
        Err(why) => Err(anyhow::anyhow!("launch arguments: {why}")),
    };
    oag_thread::gpu_workers::finish();
    oag_log::flush();
    if let Err(why) = result {
        log::error!("oag-game exited: {why:#}");
    }
}

/// The window system's event loop: winit's default everywhere but Android,
/// where it has to be handed the activity.
fn new_event_loop() -> Result<EventLoop<()>> {
    #[cfg(target_os = "android")]
    return oag_mobile::android::event_loop().map_err(anyhow::Error::msg);
    #[cfg(not(target_os = "android"))]
    Ok(EventLoop::new()?)
}

/// The command line, run: to the end when nothing needs a window, otherwise as far as the window.
fn run(mut cli: Cli) -> Result<()> {
    cli::benchmark::resolve_title(&cli.benchmark, &mut cli.boot.source)?;
    match launch(cli)? {
        Launch::Done => Ok(()),
        Launch::Window(window) => run_window(*window),
    }
}

/// What [`launch`] leaves for [`run_window`].
pub(crate) enum Launch {
    /// Everything the command line asked for has run.
    Done,
    /// A window's event loop is next, on this app.
    Window(Box<Windowed>),
}

/// An [`App`] built and waiting for its window.
pub(crate) struct Windowed {
    app: Box<App>,
    drive: Option<crate::drive::Script>,
    /// `--launcher` was given, so a script is meant to walk the chooser.
    scripted_launcher: bool,
    state_log: Option<std::path::PathBuf>,
}

impl Windowed {
    /// A window for `app` with no scripted session to run in its place.
    pub(crate) fn new(app: App) -> Self {
        Self {
            app: Box::new(app),
            drive: None,
            scripted_launcher: false,
            state_log: None,
        }
    }
}

/// Builds the window's app, or runs whatever needs no window.
///
/// **Kept apart from the loop on purpose.** This function inlines every route's locals, the
/// command line, the settings, a race's [`App`] and its capture options, into one frame of about
/// 470 KB. Run in the same function as the event loop, that frame stayed under every callback of
/// the game, and on Android's 2 MiB `android_main` stack it left no room for `naga`'s parser
/// (`docs/tools/android.md`, "The stack"). Returning the app boxed ends the frame before the loop
/// starts.
#[inline(never)]
fn launch(mut cli: Cli) -> Result<Launch> {
    // Ahead of settings and the disc search, deliberately: rasterising the
    // icon needs neither, and `just install-desktop-file` /
    // `scripts/build-appimage.sh` both call this on machines that may have no
    // disc image configured at all. See `crate::icon` and
    // `crate::app::window::window_icon`, the winit-facing use of the same function.
    if let Some(path) = &cli.icon.write_icon {
        // **Validated here, because `rasterize`'s `expect` is not a CLI error
        // message.** That function's own docs say no player input reaches it;
        // `--icon-size` does, and `--write-icon out.png --icon-size 0` panicked
        // rather than saying what was wrong.
        anyhow::ensure!(cli.icon.icon_size > 0, "--icon-size must be at least 1");
        let crate::icon::Rgba {
            width,
            height,
            pixels,
        } = crate::icon::rasterize(cli.icon.icon_size);
        let png = oag_texture::png::encode_rgba(width, height, &pixels);
        std::fs::write(path, png).with_context(|| format!("writing {}", path.display()))?;
        println!("wrote {}x{} {}", width, height, path.display());
        return Ok(Launch::Done);
    }

    if let Some(path) = &cli.shot.png_info {
        return from_png::info(path).map(|()| Launch::Done);
    }
    let (restage, blurred) = match cli.shot.from_png.clone() {
        Some(path) => {
            let (record, blurred) = from_png::prepare(&mut cli, &path)?;
            (Some(record), blurred)
        }
        None => (None, None),
    };

    // Loaded (and, on first run or a missing key, written back complete) before
    // anything else: a bad value in the file should fail immediately, not eight
    // seconds of intro later.
    let mut settings = settings::load()?;
    attach_log_file(&cli, &settings);
    oag_ffmpeg::configure(oag_ffmpeg::Policy {
        use_pinned: settings.ffmpeg.use_pinned,
        root: Some(oag_source::cache::default_ffmpeg_dir()),
    });
    crate::texture_cache::apply(settings.graphics.texture_compression);
    if cli.diagnostics.fetch_ffmpeg {
        return crate::ffmpeg_cli::run(
            &oag_source::cache::default_ffmpeg_dir(),
            settings.ffmpeg.use_pinned,
        )
        .map(|()| Launch::Done);
    }
    if cli.diagnostics.probe_pad {
        info!("--probe-pad: printing pad events until stopped; no window opens");
        return oag_input::probe::run(&mut std::io::stdout(), || false)
            .map(|()| Launch::Done)
            .map_err(anyhow::Error::msg);
    }
    #[cfg(feature = "dev-cli")]
    if let Some(pattern) = &cli.dev.rumble_pattern {
        return crate::rumble::probe(&crate::rumble::Probe {
            pattern,
            source: cli
                .boot
                .source
                .as_deref()
                .or(settings.source.image.as_deref()),
            token: &settings.controls.rumble,
            hold: cli.dev.rumble_hold,
        })
        .map(|()| Launch::Done);
    }
    let anisotropy = cli
        .render
        .anisotropy
        .unwrap_or(settings.graphics.anisotropy);
    // A CLI render-profile flag overrides *every* title's entry for this run
    // only - never persisted, the same footing `anisotropy` above is already
    // on. There is no title open yet this early, so there is no one title to
    // single out; a flag naming one and not the rest would silently leave the
    // rest of a play session's titles unaffected the moment a different disc
    // is chosen, which is not what typing the flag once means.
    let render_scale = match cli.render.render_scale {
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
    // against the four Pulse authors, so `--class vector` reaches a
    // Wipeout Pure race. This is spell-checking only: whether *this* disc's
    // files author the rung is settled by `race::load`, which names what the
    // file does carry when they do not.
    let class = cli.race_setup.class.trim().to_string();
    anyhow::ensure!(
        oag_title::SpeedClasses::is_measured_name(&class),
        "{:?} is not a speed class; try {}",
        cli.race_setup.class,
        oag_title::SpeedClasses::MEASURED.join(", ").to_lowercase()
    );
    // Parsed here for the same reason as the speed class: `--race` goes straight
    // to a track, so a misspelled mode has to be a message about the command
    // line rather than a race that quietly runs under different rules.
    let mode = oag_race::Mode::from_name(&cli.race_setup.mode).with_context(|| {
        format!(
            "{:?} is not a race mode; try time_trial, speed_lap, zone, single_race, eliminator, \
             head_to_head or detonator",
            cli.race_setup.mode
        )
    })?;
    // Head to Head is two craft, so it seats two people at most. Refused by name here,
    // where the mode and the head count (`--players` or the windows' seats) are both
    // known, rather than fielding a race that cannot be what was asked for.
    // **Chosen, not measured**: see `Mode::max_people`.
    if let Some(most) = mode.max_people() {
        let people = cli.driver.people();
        anyhow::ensure!(
            people <= most,
            "{} takes at most {most} people and {people} are seated (--players or \
             --window-seats)",
            mode.name()
        );
    }
    // A windowed run may dump too (a scripted front-end walk is the only way
    // to hear the session's own cues). The buffer grows for as long as the
    // window stays open, about 384 KB a second, so it belongs to a run that
    // ends itself or is closed by hand, never a player's session.
    // Refused for the same reason `--screen` and `--menu-page` are capture-only:
    // there is no window route that draws a *stated* conversion state, and
    // silently ignoring the flag would look like a screen that does not work.
    ensure!(
        cli.loading.loading_screen.is_none() || cli.screenshot.is_some(),
        "--loading-screen needs --screenshot: it draws one frame of the loading \
         screen at a stated state. A window shows the real one under --prefetch."
    );

    // Refused rather than ignored: the chooser is a screen, and every one of
    // these three routes is defined by not opening a window to show it on.
    ensure!(
        !cli.boot.launcher || !(cli.race || cli.diagnostics.dry_run || cli.screenshot.is_some()),
        "--launcher needs a window: it is a screen for choosing a disc image, and \
         --race, --dry-run and --screenshot all name their own source and open none. \
         Run it on its own, or name an image directly."
    );

    // Resolved beside the source below, but it cannot fail: no DLC is the
    // ordinary state of a copy of the game.
    let dlc = source::resolve_dlc(&cli.race_setup.dlc, &settings.source.dlc);
    crate::unlock::set_unlock_all(cli.unlock_all || settings.dev.unlocks_everything());

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
    let mut audio = oag_sound::Audio::open(
        &settings.audio,
        cli.audio.dump_audio.clone(),
        tap.as_ref(),
        buffer,
        cli.audio.no_audio,
    );
    audio.set_soundtrack(!cli.audio.no_soundtrack);
    // The shuffle is presentation randomness: the clock seeds it, the simulation never sees it.
    audio.seed_music_shuffle(
        web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos() as u64),
    );

    let (pose, camera) = match &cli.placement.pose_from {
        Some(path) => {
            let (pose, camera) = pose_from_trace(
                path,
                cli.capture_view.pose_tick,
                cli.capture_view.no_camera,
                cli.capture_view.camera_fov,
            )?;
            (Some(pose), camera)
        }
        // `--camera-pose` is a camera and nothing else, so it composes with
        // `--pose` rather than replacing it: an RPCS3 capture gives the camera
        // exactly and the craft only as whatever the frame shows.
        None => (
            cli.placement
                .pose
                .as_deref()
                .map(parse_pose)
                .transpose()?
                .map(|(position, yaw)| race::PoseRequest::SplineAligned { position, yaw }),
            cli.capture_view
                .camera_pose
                .as_deref()
                .map(parse_camera_pose)
                .transpose()?,
        ),
    };
    // Applied after either way in, because both can produce a camera and the
    // flag means the same thing to each.
    let camera = camera.map(|camera| race::CameraOverride {
        fov_deg: cli.capture_view.camera_fov.or(camera.fov_deg),
        ..camera
    });

    let field = cli
        .placement
        .field_from
        .as_deref()
        .map(field_from_csv)
        .transpose()?;
    // A field file with a slot 0 row carries the player's exact pose, which wins over `--pose`.
    let pose = field
        .as_ref()
        .and_then(|field| field.player)
        .map(race::PoseRequest::Exact)
        .or(pose);
    let field = field.map(|field| field.opponents).unwrap_or_default();

    let hud_state = capture_hud(&cli)?;

    let leg = if cli.boot.reel {
        frontend::Leg::DevPubReel
    } else {
        frontend::Leg::LogoFmv
    };

    // Everything the command line decided that no disc is needed for. The
    // source is deliberately not in here: on the launcher route it is not known
    // until a window has been open for a while. See `crate::app::prepare`.
    // A foreign track that cannot be found is a refusal, never the disc's
    // default circuit raced instead.
    #[cfg(feature = "ballisticng")]
    let workshop_track = cli
        .race_setup
        .ballisticng_track
        .as_deref()
        .map(|name| {
            oag_source::tracks::resolve(name).ok_or_else(|| {
                let found: Vec<String> = oag_source::tracks::all()
                    .iter()
                    .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                    .collect();
                anyhow::anyhow!(
                    "--ballisticng-track {name}: no .wtrk at that path and no unique name match; \
                     searched {:?} and {:?}, found {found:?}",
                    oag_source::tracks::search_path(),
                    oag_source::ballisticng::search_path()
                )
            })
        })
        .transpose()?;
    #[cfg(not(feature = "ballisticng"))]
    let workshop_track: Option<std::path::PathBuf> = None;
    let foreign_entry = workshop_track
        .or_else(|| cli.race_setup.track_file.clone())
        .map(|path| race::foreign::entry(&path))
        .transpose()?;
    let (definition, menu_source) =
        prepare::definition(&cli, settings.language.as_deref(), &settings.mods)?;
    let pending = prepare::Pending {
        foreign_entry,
        definition,
        menu_source,
        settings,
        dlc,
        class,
        mode,
        leg,
        pose,
        field,
        hud_state,
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
    // `--demo` with no window: `--dry-run` flies demos back to back, `--screenshot` captures the
    // first one's frame through the ordinary race capture. See `docs/ui/attract-demo.md`.
    if cli.demo.mode().is_some() && (cli.diagnostics.dry_run || cli.screenshot.is_some()) {
        let source = source::resolve(cli.boot.source.as_deref(), settings.source.image.as_deref())?;
        let music_discs = pending.music_discs(&source);
        let mut options = pending.race_options(&source, Vec::new());
        let catalogue = crate::title_lists::catalogue(&source)?;
        let config = oag_source::title::open_source(&source, Vec::new(), Vec::new())
            .ok()
            .and_then(|mut opened| {
                crate::boot::demo_config::load(&mut opened.archives, opened.title, &mut Vec::new())
            });
        let length = if cli.capture.ticks > 0 {
            cli.capture.ticks
        } else {
            u32::MAX
        };
        let seed = options.seed.unwrap_or_else(crate::race_seed::fresh);
        let unlocked = crate::unlock::unlock_all();
        let playable = |track: &race::catalogue::Track| unlocked || track.unlock_grid.is_none();
        if cli.diagnostics.dry_run {
            for (n, report) in crate::demo::chain::run(
                &options,
                &catalogue,
                config.as_ref(),
                playable,
                seed,
                2,
                length,
            )?
            .iter()
            .enumerate()
            {
                println!("demo {}: {report}", n + 1);
            }
            return Ok(Launch::Done);
        }
        let track = catalogue
            .tracks
            .iter()
            .map(|(track, _)| track)
            .find(|track| playable(track));
        let team = catalogue.teams.first();
        options.mode = oag_race::Mode::SingleRace;
        options.track = track.map(race::catalogue::Track::entry_name);
        options.team = team.map(|team| team.value.clone());
        return race_launch(
            cli,
            options,
            settings,
            anisotropy,
            audio,
            music_discs,
            headless::RaceRoute::default(),
        );
    }

    // `--connect` is a race too: the server's.
    if cli.race || cli.net.connect.is_some() || cli.net.online.is_some() {
        // `--prefetch` is a front-end thing, and saying so is better than
        // quietly doing nothing. `--race` exists to skip the boot sequence, and
        // hanging its exit on ten minutes of `ffmpeg` would invert the one
        // thing it is for.
        if cli.boot.prefetch {
            warn!(
                "--prefetch has no effect with --race: it converts the front end's movies and \
                 sounds, which a race never opens. Run it without --race once."
            );
        }
        // Same reasoning one screen along: the loading screen is what a player
        // waits on while that conversion runs, so on a route that does not run
        // it there is nothing for the screen to be about.
        if cli.loading.loading_screen.is_some() {
            warn!("--loading-screen has no effect with --race; run it without --race.");
        }
        let source = source::resolve(cli.boot.source.as_deref(), settings.source.image.as_deref())?;
        let music_discs = pending.music_discs(&source);
        // Joined before the load: the race to load is the server's.
        #[cfg(not(target_arch = "wasm32"))]
        let net = cli
            .net
            .connect(&source)?
            .map(|connect| crate::net_play::NetPlay::connect(&connect))
            .transpose()?;
        #[cfg(target_arch = "wasm32")]
        let net: Option<crate::net_play::NetPlay> = {
            // Refuses `--connect` by name; the browser build never joins a server.
            cli.net.connect(&source)?;
            None
        };
        let mut options = pending.race_options(&source, Vec::new());
        // The rules modules of the mods that are on. None in a network race (the host's
        // race runs vanilla, and the lobby's fingerprint leaves the examples out), an
        // original campaign's cell (`--campaign-cell`) or a 2048 event (`--event`).
        options.campaign = cli.wreck.campaign_cell.is_some() || cli.race_setup.event.is_some();
        #[cfg(feature = "extensions")]
        {
            let vanilla = net.is_some()
                || cli.wreck.campaign_cell.is_some()
                || cli.race_setup.event.is_some();
            crate::mod_extensions::attach(
                &mut options,
                &settings.mods,
                &settings.mod_settings,
                vanilla,
            );
        }
        crate::texture_pack::attach(&mut options, &settings.mods);
        if let Some(record) = &restage {
            options.apply_replay(&record.replay.header)?;
        }
        let options = match &net {
            Some(net) => net.race_options(options)?,
            None => options,
        };
        return race_launch(
            cli,
            options,
            settings,
            anisotropy,
            audio,
            music_discs,
            headless::RaceRoute {
                net,
                restage,
                blurred,
            },
        );
    }

    // Every leg with no window loads the whole boot here and now, blocking, and
    // then leaves. Only a window has anywhere to *show* a wait, so only a
    // window earns the machinery below that defers one.
    if cli.diagnostics.dry_run || cli.screenshot.is_some() {
        let source = source::resolve(cli.boot.source.as_deref(), settings.source.image.as_deref())?;
        let music_discs = pending.music_discs(&source);
        // A `--demo` capture's race is the demo's, so the title's demo screen is read for it; the
        // windowed session's own options never are (a demo's `Session::demo_options` says so).
        let mut options = pending.race_options(&source, Vec::new());
        options.demo = pending.cli.demo.mode().is_some();
        return run_windowless(
            &pending.cli,
            &pending.boot_options(&source),
            settings,
            options,
            anisotropy,
            audio,
            music_discs,
        )
        .map(|()| Launch::Done);
    }

    // Which way in this window takes: straight to a boot, or the chooser first.
    //
    // The chooser is offered only when nothing was *stated* - see
    // `source::explicit` - and only when there is a choice to make. One image
    // is not a choice, and a screen that has to be dismissed before every boot
    // would be a step added to the common case for nothing.
    let chosen = source::explicit(cli.boot.source.as_deref(), settings.source.image.as_deref())?;
    let launcher = match &chosen {
        // A stated source that is an encrypted image with no key is the one
        // stated source that still gets the screen: the prompt is the fix.
        Some(source) if !cli.boot.launcher && !launcher::needs_key(source) => None,
        Some(source) if !cli.boot.launcher => {
            let rows = launcher::survey(&[std::path::PathBuf::from(source)]);
            Some(launcher::Launcher::new(rows).with_paste(crate::app::clipboard::AVAILABLE))
        }
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
            // One image is no choice - unless it is locked, when the screen is
            // how its key is asked for. **None at all is never a chooser**, not
            // even under `--launcher`: an empty list offers nothing but escape.
            // It is the not-found screen below instead (a double-click never shows the
            // terminal message from `resolve`), and the same message still goes to the
            // log.
            let locked = rows
                .iter()
                .any(|row| matches!(row.state, launcher::State::NeedsKey));
            if rows.len() > 1 || (cli.boot.launcher && !rows.is_empty()) || locked {
                Some(launcher::Launcher::new(rows).with_paste(crate::app::clipboard::AVAILABLE))
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
            } else if rows.is_empty() && cfg!(target_os = "ios") {
                // The bundle is read-only and the container's path is a UUID:
                // the screen names the folder as the Files app shows it.
                #[cfg(target_os = "ios")]
                let folder = oag_mobile::ios::IMAGES_IN_FILES;
                #[cfg(not(target_os = "ios"))]
                let folder = "";
                log::error!("no disc image found; copy one to {folder}");
                Some(launcher::Launcher::not_found(
                    launcher::not_found_notice_desktop(folder),
                ))
            } else if rows.is_empty() && chosen.is_none() {
                // A double-click has no terminal for `resolve`'s message either,
                // and its window would close before it could be read: the
                // screen names the folder to use, and the log keeps the full
                // list of what was searched.
                if let Err(error) = source::resolve(None, None) {
                    log::error!("{error}");
                }
                let folder = dirs::data_dir().map_or_else(
                    || "data/images".to_string(),
                    |dir| dir.join("oag").join("images").display().to_string(),
                );
                Some(launcher::Launcher::not_found(
                    launcher::not_found_notice_desktop(&folder),
                ))
            } else {
                None
            }
        }
    };

    // Cards unless the list was asked for. The not-found screen and the disc-key
    // prompt look the same either way.
    let cards = launcher::wants_cards(cli.boot.launcher_list, settings.launcher.style);
    let launcher = launcher.map(|chooser| {
        if !cards {
            return chooser;
        }
        let table = oag_ui::strings::project_table(settings.language.as_deref());
        chooser.with_cards(
            launcher::cards::Words::from_table(&table),
            settings.launcher.pictures.clone(),
        )
    });

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
    let give = give_weapon(cli.give())?;
    let autopilot_pilot = autopilot_pilot(cli.driver.autopilot_pilot.as_deref())?;
    let autopilot_skill = cli.driver.autopilot_skill;
    let ai_choice = cli.driver.ai;
    let players = cli.driver.players;
    if let Some(note) = cli.driver.gamescope_fold_note() {
        log::info!("{note}");
    }
    let window_layout = cli.driver.window_layout()?;
    let windows_fullscreen = cli.driver.windows_fullscreen;
    let split_cut = cli.driver.cut();
    let (boot_overlay, pick_language, autopilot, trace, log_every) = (
        cli.overlay,
        cli.diagnostics.pick_language,
        cli.driver.autopilot,
        cli.diagnostics.trace,
        cli.race_run.log_every,
    );
    let anim_seconds = cli.anim_seconds;
    let settings = settings.clone();

    // `--drive` runs the same session with no window and so no event loop: made after it.
    let drive = cli.dev.drive()?;
    let scripted_launcher = cli.boot.launcher;
    let state_log = cli.dev.state_log.clone();

    let app = App {
        pvs_culling: cli.render.pvs,
        camera_jitter: cli.render.camera_jitter,
        boot_shell,
        media,
        boot_overlay,
        pick_language,
        measure_race_load: cli.measure.measure_race_load,
        demo: cli.demo.mode(),
        demo_limit: cli.demo.limit(),
        benchmark: cli.benchmark.request(),
        benchmark_ticks: (cli.capture.ticks > 0).then_some(cli.capture.ticks),
        replay_png: cli.shot.replay_png.clone(),
        give,
        no_intro: cli.intro.no_intro,
        original_menu: cli.original_menu,
        dev: cli.dev.dev(cli.driver.people())?,
        net: None,
        net_bot: cli.net.net_bot,
        prompt_style: cli
            .menu_args
            .prompt_style
            .as_deref()
            .and_then(oag_input::prompt::PromptStyle::from_name),
        autopilot,
        autopilot_pilot,
        autopilot_skill,
        ai_choice,
        players,
        window_layout,
        windows_fullscreen,
        split_cut,
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
        // only `--race`, which is `headless::race_launch`, has neither.
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
        occluded: false,
    };
    Ok(Launch::Window(Box::new(Windowed {
        app: Box::new(app),
        drive,
        scripted_launcher,
        state_log,
    })))
}

/// The window's loop, or the scripted session or wallpaper that stands in for it.
#[inline(never)]
fn run_window(window: Windowed) -> Result<()> {
    #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
    let Windowed {
        mut app,
        drive,
        scripted_launcher,
        state_log,
    } = window;
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(script) = drive {
        oag_sound::probe::mark_frame_thread();
        return crate::app::drive::run(&mut app, script, scripted_launcher, state_log.as_deref());
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (drive, state_log, scripted_launcher);
    // A wallpaper on a compositor with a layer shell is the desktop background, with no window.
    #[cfg(target_os = "linux")]
    if app.demo == Some(true) {
        oag_sound::probe::mark_frame_thread();
        if crate::app::wallpaper::run(&mut app)? {
            app.finish_audio()?;
            app.finish_prefetch();
            return Ok(());
        }
    }
    let event_loop = new_event_loop()?;
    // Poll rather than Wait: the intro is animated whether or not input arrives.
    event_loop.set_control_flow(ControlFlow::Poll);
    // From here on this thread draws frames, so music work on it is logged with
    // its duration (`oag_sound::probe`).
    oag_sound::probe::mark_frame_thread();
    // The browser owns the loop: `spawn_app` returns at once and the page's
    // own frame callbacks drive `app` from then on, so there is no "after" to
    // finish audio or a prefetch in (the web build has neither).
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(*app);
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        event_loop.run_app(&mut *app)?;
        app.finish_audio()?;
        app.finish_prefetch();
        Ok(())
    }
}

/// What [`tests`] reaches through its own `use super::*`, and `main` itself does
/// not.
///
/// A private `use` in the crate root is visible to every module below it, so
/// naming them here is what let the split leave the test file's body untouched -
/// the tests still call `menu_playhead` and `parse_progress` unqualified,
/// exactly as they did when all three lived in this file.
#[cfg(test)]
use crate::app::{
    args::parse_progress, frontend_stage::HeldFrame, menu_stage::menu_playhead, session::Session,
    session::menus::backdrop_seed,
};
/// Same reasoning, for a library module `main` itself no longer names: the
/// windowed load moved into [`prepare`] and took `movie` with it. `menu` and
/// `language` are here for [`Session::maybe_begin_binding`]'s own tests, which
/// need a [`menu::Menu`] and nothing else this file already imports names one.
#[cfg(test)]
use crate::movie;
#[cfg(test)]
use oag_input::Controls;
#[cfg(test)]
use oag_ui::{language, menu};

#[cfg(test)]
mod tests;
