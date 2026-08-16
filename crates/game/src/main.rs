//! Runs Wipeout Pulse from the user's own disc image.
//!
//! ```sh
//! oag-game data/images/pulse-psp-usa.chd
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
//! oag-game data/images/pulse-psp-usa.chd --screenshot /tmp/menu.png \
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
//! This file is only `main` itself: read the command line, resolve everything
//! that can fail before anything is loaded, and hand off to one of the runs in
//! [`headless`] or to the window in [`app`]. The rest of the binary is the
//! eighteen modules declared below; everything that can be tested without a GPU
//! is in [`oag_game`] rather than in any of them.

use anyhow::{Context, Result, ensure};
use clap::Parser;

use oag_game::frontend;
use oag_game::{audio, boot, display, loading, menu, movie, prefetch, race, settings, source};
use oag_physics::SpeedClass;

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
#[path = "main/loading_stage.rs"]
mod loading_stage;
#[path = "main/menu_stage.rs"]
mod menu_stage;
#[path = "main/pose.rs"]
mod pose;
#[path = "main/race_stage.rs"]
mod race_stage;
#[path = "main/session.rs"]
mod session;
#[path = "main/stage.rs"]
mod stage;
#[path = "main/window.rs"]
mod window;

use crate::app::App;
use crate::args::{give_weapon, resolve_difficulty, resolve_scheme};
use crate::cli::Cli;
use crate::headless::{run_race, run_windowless};
use crate::hints::MENU_KEYS;
use crate::pose::{parse_pose, pose_from_trace};
use crate::session::Shell;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Loaded (and, on first run or a missing key, written back complete) before
    // anything else: a bad value in the file should fail immediately, not eight
    // seconds of intro later.
    let settings = settings::load()?;
    let anisotropy = cli.anisotropy.unwrap_or(settings.graphics.anisotropy);
    let render_scale = match cli.render_scale {
        Some(percent) => crate::display::Scale::try_from(percent)
            .map_err(|why| anyhow::anyhow!("--render-scale {percent}: {why}"))?,
        None => settings.graphics.render_scale,
    };
    let settings = settings::Settings {
        graphics: settings::Graphics {
            upscaler: cli.upscaler.unwrap_or(settings.graphics.upscaler),
            render_scale,
            ..settings.graphics
        },
        ..settings
    };

    // Parsed before anything is loaded, and for both ways in: the front end can
    // hand off to a race, so a misspelled class must not be discovered eight
    // seconds of intro later.
    let class = SpeedClass::from_name(&cli.class).with_context(|| {
        format!(
            "{:?} is not a speed class; try venom, flash, rapier or phantom",
            cli.class
        )
    })?;
    // Parsed here for the same reason as the speed class: `--race` goes straight
    // to a track, so a misspelled mode has to be a message about the command
    // line rather than a race that quietly runs under different rules.
    let mode = oag_race::Mode::from_name(&cli.mode).with_context(|| {
        format!(
            "{:?} is not a race mode; try time_trial, speed_lap, zone or single_race",
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

    // Resolved once, before anything opens it: both ways in need a source, and
    // "no disc image found" is a message about the command line, not something to
    // discover eight seconds of intro later.
    let source = source::resolve(cli.source.as_deref(), settings.source.image.as_deref())?;
    // Resolved beside it, but it cannot fail: no DLC is the ordinary state of a
    // copy of the game.
    let dlc = source::resolve_dlc(&cli.dlc, &settings.source.dlc);

    // Opened before either way in, because both want sound and neither owns the
    // other. **Only the device, not the music itself** - `start_music` is
    // deferred to whichever tick loop is actually about to run, windowed or
    // headless, so that a player never hears the front end before the window
    // that shows it exists. See the two call sites below and
    // `App::open`.
    let audio = audio::Audio::open(&settings.audio, cli.dump_audio.clone());
    // Surveyed once, here, because it is what decides whether the AUDIO page
    // offers MUSIC SOURCE at all - and answering it means opening every disc
    // image on the search path, which is not something to do while a menu is on
    // screen. Skipped under `--dry-run` for the same reason the music is.
    let music_discs = if cli.dry_run {
        audio::MusicDiscs::default()
    } else {
        let discs = audio::MusicDiscs::survey(&source);
        println!("audio: music discs, {}", discs.describe());
        discs
    };

    let (pose, camera) = match &cli.pose_from {
        Some(path) => {
            let (pose, camera) =
                pose_from_trace(path, cli.pose_tick, cli.no_camera, cli.camera_fov)?;
            (Some(pose), camera)
        }
        None => (
            cli.pose
                .as_deref()
                .map(parse_pose)
                .transpose()?
                .map(|(position, yaw)| race::PoseRequest::SplineAligned { position, yaw }),
            None,
        ),
    };

    let mut race_options = race::Options {
        source: source.clone(),
        dlc: dlc.clone(),
        // Passed straight through, `None` included: `race::load` resolves an
        // unnamed circuit from the title it opened, which is the only place the
        // title is known. See `race::Options::track`.
        track: cli.track.clone(),
        team: cli
            .team
            .clone()
            .unwrap_or_else(|| oag_pulse::race::DEFAULT_TEAM.to_string()),
        class,
        mode,
        // The disc's own team list arrives with the boot shell, below - it is
        // read from the plugin definition and any mounted DLC pack, so it
        // cannot be known here. Empty until then, which is the whole grid in
        // the player's hull.
        opponent_teams: Vec::new(),
        ribbon: cli.ribbon,
        collision: cli.collision,
        lod: cli.lod.unwrap_or(settings.graphics.lod),
        // **From the settings here, not only in the menu path.** `--race`
        // bypasses the menus entirely, and a difficulty wired only into
        // `LaunchRace` would be silently ignored by the flag most testing uses.
        // Same shape as the scheme: an unrecognised token is reported and the
        // default is used rather than failing the boot.
        difficulty: resolve_difficulty(&settings),
        opponents: cli.opponents,
        seed: cli.seed,
        pose,
        camera,
    };

    // Before `boot::load`, deliberately: the front end's load parses the front-end
    // XML, every language plugin and a string table, and may shell out to `ffmpeg`
    // to transcode the intro. Going straight to a race needs none of it.
    if cli.race {
        // `--prefetch` is a front-end thing, and saying so is better than
        // quietly doing nothing. `--race` exists to skip the boot sequence, and
        // hanging its exit on ten minutes of `ffmpeg` would invert the one
        // thing it is for.
        if cli.prefetch {
            println!(
                "--prefetch has no effect with --race: it converts the front end's movies and \
                 sounds, which a race never opens. Run it without --race once."
            );
        }
        // Same reasoning one screen along: the loading screen is what a player
        // waits on while that conversion runs, so on a route that does not run
        // it there is nothing for the screen to be about.
        if cli.loading_screen.is_some() {
            println!("--loading-screen has no effect with --race; run it without --race.");
        }
        return run_race(
            &cli,
            race_options,
            &settings,
            anisotropy,
            audio,
            music_discs,
        );
    }

    let leg = if cli.reel {
        frontend::Leg::DevPubReel
    } else {
        frontend::Leg::LogoFmv
    };
    let options = boot::Options {
        source: source.clone(),
        dlc: dlc.clone(),
        // The string table follows the saved language, so a player who picked
        // French once reads French from the next boot rather than only having
        // the picker skipped.
        language: settings.language.clone(),
        leg,
        // `None` when `--movie` was not given: which entry that is depends on
        // the source's own title, not known here yet, so `boot::load` resolves
        // it once the source is open. See `boot::Options::movie`.
        movie: cli.movie.clone(),
        cache: cli.cache.clone().unwrap_or_else(boot::default_cache_dir),
        audio_cache: boot::default_audio_cache_dir(),
        // Every frame by default: the movie the disc plays is 1200 frames long
        // and its last one is the Wipeout Pulse logo, so a cap would stop the
        // sequence before the thing it exists to show.
        extent: cli
            .movie_frames
            .map_or(movie::Extent::Whole, movie::Extent::Frames),
        no_video: cli.no_video,
        refresh_video: cli.refresh_video,
        prefer_av1_cache: cli.prefer_av1_cache,
    };

    // Every leg with no window loads the whole boot here and now, blocking, and
    // then leaves. Only a window has anywhere to *show* a wait, so only a
    // window earns the machinery below that defers one.
    if cli.dry_run || cli.screenshot.is_some() {
        return run_windowless(
            cli,
            &options,
            &settings,
            race_options,
            anisotropy,
            audio,
            music_discs,
        );
    }

    // **The cheap half of the boot, and the only part a window waits on.**
    // Measured on the EU disc: 0.05 s here against 4.9 s for the movies, which
    // `MediaWorker` now runs on a thread of its own while winit opens the
    // window and the loading screen goes up over it. Before this split the
    // whole 5 seconds ran before winit had been asked for a window at all, so
    // there was nothing on screen to say the game had started - which is the
    // bug this shape exists to fix. See `boot::Shell`.
    let (mut boot_shell, archives) = boot::load_shell(&options)?;
    // Drained rather than iterated: `boot::assemble` appends its own lines to
    // this same list, and the hand-off prints what it finds there. Leaving
    // these in would print the whole first half twice, seconds apart, which
    // reads as the disc having been opened again.
    for line in boot_shell.report.drain(..) {
        println!("{line}");
    }
    let media = boot::MediaWorker::spawn(archives, &boot_shell, &options);

    // Parsed here rather than when the menus open, so a broken definition is a
    // startup error and not something a player meets after the intro.
    let definition = match cli.menu.as_deref() {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?;
            menu::Definition::parse(&text).with_context(|| format!("parsing {}", path.display()))?
        }
        None => menu::Definition::parse(menu::BUILT_IN)
            .context("parsing the built-in menu definition")?,
    };
    // The three lists that come off the disc rather than out of the definition:
    // what is raceable, who can be raced for, and what languages exist. All
    // carry a label the player reads and a value the settings file stores, and
    // on a circuit or a team those are different strings - `16_Track` and
    // `Mantis` against their localised names, which are shipped content and
    // only ever live in memory.
    // Every team the player's own source declares, in the definition's file
    // order, DLC packs included. `livery::teams_for_slots` decides which slot
    // flies which; that ordering is this project's, not the original's.
    race_options.opponent_teams = boot_shell
        .teams
        .iter()
        .map(|team| team.id.clone())
        .collect();

    let shell = Shell {
        definition,
        modes: menu::mode_choices(&boot_shell.strings),
        teams: boot_shell
            .teams
            .iter()
            .map(|team| menu::Choice::labelled(&team.id, boot_shell.strings.get_or_id(&team.id)))
            .collect(),
        tracks: boot_shell
            .tracks
            .iter()
            .map(|track| {
                (
                    track.clone(),
                    boot_shell.strings.get_or_id(&track.id).to_string(),
                )
            })
            .collect(),
        languages: boot_shell
            .languages
            .iter()
            .map(|language| menu::Choice::labelled(&language.name, &language.native_name))
            .collect(),
        font: boot_shell.font.clone(),
        menu_skin: boot_shell.menu_skin,
        menu_font: boot_shell.menu_font.clone(),
        sprites: boot_shell.sprites.clone(),
    };

    println!("\n{MENU_KEYS}");

    // Unconditional now, where it used to be `--prefetch` only. Its two extra
    // archive reads used to buy nothing on a boot that went straight to the
    // front end; every windowed boot now shows the loading screen while the
    // movies decode, so every windowed boot needs the tips and the glow strip.
    let loading_assets = {
        let assets = loading::Assets::load(&source, &boot_shell.strings);
        for note in &assets.notes {
            println!("{note}");
        }
        assets
    };

    let event_loop = EventLoop::new()?;
    // Poll rather than Wait: the intro is animated whether or not input arrives.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        boot_shell: Some(boot_shell),
        media: Some(media),
        boot_overlay: cli.overlay,
        pick_language: cli.pick_language,
        give: give_weapon(cli.give.as_deref())?,
        race: None,
        race_options,
        trace: cli.trace,
        log_every: cli.log_every,
        anisotropy,
        scheme: resolve_scheme(&cli, &settings),
        settings,
        shell: Some(shell),
        audio: Some(audio),
        music_discs,
        // Asked for, not started: see `App::prefetch`.
        prefetch: cli.prefetch.then(|| prefetch::Options {
            source: source.clone(),
            movies: options.cache.clone(),
            audio: boot::default_audio_cache_dir(),
            refresh_video: cli.refresh_video,
        }),
        loading_assets,
        state: None,
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
use crate::{args::parse_progress, menu_stage::menu_playhead};

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
