//! Everything a run needs that depends on *which* source it opened.
//!
//! This used to be the middle of `main`, run once as soon as
//! [`oag_game::source::resolve`] had answered. It is a module of its own now
//! because the answer can arrive later: with no source named and several images
//! on the search path, [`oag_game::launcher`] puts them on screen and the pick
//! comes from a window that is already open. Both routes run the same code
//! here, so a launched boot and a named one cannot drift apart.
//!
//! The split is by *dependency*, not by tidiness. [`Pending`] is what the
//! command line and the settings file decided, none of which needs a disc;
//! [`Prepared`] is what only a source can produce. `main` keeps everything
//! that is neither - the mixer, which opens against a device rather than a
//! disc, and the menu definition, which is ours and must fail before a window
//! whichever route is taken.

use std::path::PathBuf;

use anyhow::{Context, Result};
use log::info;

use oag_game::frontend;
use oag_game::{audio, boot, catalogue, loading, menu, movie, prefetch, race, settings, strings};
use oag_physics::SpeedClass;

use crate::args::{resolve_difficulty, resolve_scheme};
use crate::cli::Cli;
use crate::hints::MENU_KEYS;
use crate::session::Shell;

/// What the command line settled that does not depend on a disc.
///
/// Held rather than consumed so that both routes can read it: the eager one
/// once, the launcher's once per pick - and a player who backs out of a boot
/// and picks a different image would read it again.
pub(crate) struct Pending {
    pub(crate) cli: Cli,
    pub(crate) settings: settings::Settings,
    /// Our own menu tree, parsed in `main` so that a broken `--menu` file is a
    /// startup error on every route rather than something a player meets after
    /// the intro.
    pub(crate) definition: menu::Definition,
    pub(crate) dlc: Vec<PathBuf>,
    pub(crate) class: SpeedClass,
    pub(crate) mode: oag_race::Mode,
    pub(crate) leg: frontend::Leg,
    pub(crate) pose: Option<race::PoseRequest>,
    pub(crate) camera: Option<race::CameraOverride>,
}

/// What only a source can produce.
pub(crate) struct Prepared {
    /// The cheap half of the boot - 0.05 s on the EU disc, against 4.9 s for
    /// the movies still decoding on [`Self::media`].
    pub(crate) boot_shell: boot::Shell,
    pub(crate) media: boot::MediaWorker,
    pub(crate) shell: Shell,
    pub(crate) loading_assets: loading::Assets,
    pub(crate) race_options: race::Options,
    pub(crate) music_discs: audio::MusicDiscs,
    /// What `--prefetch` asked for, not started. See `Session::start_prefetch`.
    pub(crate) prefetch: Option<prefetch::Options>,
}

impl Pending {
    /// The boot sequence's own options for one source.
    pub(crate) fn boot_options(&self, source: &str) -> boot::Options {
        boot::Options {
            source: source.to_string(),
            dlc: self.dlc.clone(),
            // The string table follows the saved language, so a player who
            // picked French once reads French from the next boot rather than
            // only having the picker skipped.
            language: self.settings.language.clone(),
            leg: self.leg,
            // `None` when `--movie` was not given: which entry that is depends
            // on the source's own title, not known here yet, so `boot::load`
            // resolves it once the source is open. See `boot::Options::movie`.
            movie: self.cli.movie.clone(),
            cache: self
                .cli
                .cache
                .clone()
                .unwrap_or_else(boot::default_cache_dir),
            audio_cache: boot::default_audio_cache_dir(),
            // Every frame by default: the movie the disc plays is 1200 frames
            // long and its last one is the Wipeout Pulse logo, so a cap would
            // stop the sequence before the thing it exists to show.
            extent: self
                .cli
                .movie_frames
                .map_or(movie::Extent::Whole, movie::Extent::Frames),
            no_video: self.cli.no_video,
            refresh_video: self.cli.refresh_video,
            prefer_av1_cache: self.cli.prefer_av1_cache,
        }
    }

    /// What a race off this source is flown on.
    ///
    /// `opponent_teams` comes from the boot shell on the windowed route and is
    /// empty everywhere else, which is the whole grid in the player's own hull -
    /// see [`race::Options::opponent_teams`].
    pub(crate) fn race_options(&self, source: &str, opponent_teams: Vec<String>) -> race::Options {
        race::Options {
            source: source.to_string(),
            // `None` outside a Race Remix - the ordinary CLI/menu route names
            // one source, and `race::load` treats that as "craft comes from
            // the same place as the track". See `race::Options::craft_source`.
            craft_source: self.cli.craft_source.clone(),
            dlc: self.dlc.clone(),
            // Passed straight through, `None` included: `race::load` resolves an
            // unnamed circuit from the title it opened, which is the only place
            // the title is known. See `race::Options::track`.
            track: self.cli.track.clone(),
            // Passed straight through like `track` above, `None` included:
            // `race::load` resolves an unnamed team from the title it opened,
            // which is the only place the title is known. See
            // `race::Options::team`.
            team: self.cli.team.clone(),
            class: self.class,
            mode: self.mode,
            zone_stage: self.cli.zone_stage,
            opponent_teams,
            ribbon: self.cli.ribbon,
            collision: self.cli.collision,
            lod: self.cli.lod.unwrap_or(self.settings.graphics.lod),
            // **From the settings here, not only in the menu path.** `--race`
            // bypasses the menus entirely, and a difficulty wired only into
            // `LaunchRace` would be silently ignored by the flag most testing
            // uses. Same shape as the scheme: an unrecognised token is reported
            // and the default is used rather than failing the boot.
            difficulty: resolve_difficulty(&self.settings),
            opponents: self.cli.opponents,
            trail_sparks: self.cli.trail_sparks,
            seed: self.cli.seed,
            pose: self.pose,
            camera: self.camera,
        }
    }

    /// The control scheme every race this run starts is driven with.
    pub(crate) fn scheme(&self) -> oag_gameplay::ControlScheme {
        resolve_scheme(&self.cli, &self.settings)
    }

    /// Which releases this machine has, for the music source.
    ///
    /// Answering it means opening every disc image on the search path, which is
    /// not something to do while a menu is on screen - so it happens here, with
    /// the rest of the load, rather than when the AUDIO page is opened. Skipped
    /// under `--dry-run` for the same reason the music is.
    pub(crate) fn music_discs(&self, source: &str) -> audio::MusicDiscs {
        if self.cli.dry_run {
            return audio::MusicDiscs::default();
        }
        let discs = audio::MusicDiscs::survey(source);
        info!("audio: music discs, {}", discs.describe());
        discs
    }

    /// The whole windowed load for one source.
    ///
    /// **The cheap half of the boot, and the only part a window waits on.**
    /// Measured on the EU disc: 0.05 s here against 4.9 s for the movies, which
    /// `MediaWorker` runs on a thread of its own while winit opens the window
    /// and the loading screen goes up over it. Before this split the whole 5
    /// seconds ran before winit had been asked for a window at all, so there
    /// was nothing on screen to say the game had started.
    ///
    /// # Errors
    ///
    /// Propagates [`boot::load_shell`].
    pub(crate) fn windowed(&self, source: &str) -> Result<Prepared> {
        let music_discs = self.music_discs(source);
        let options = self.boot_options(source);

        let (mut boot_shell, archives, title) = boot::load_shell(&options)?;
        // Drained rather than iterated: `boot::assemble` appends its own lines
        // to this same list, and the hand-off prints what it finds there.
        // Leaving these in would print the whole first half twice, seconds
        // apart, which reads as the disc having been opened again.
        for line in boot_shell.report.drain(..) {
            info!("{line}");
        }
        let media = boot::MediaWorker::spawn(archives, &boot_shell, &options);

        // Every team the player's own source declares, in the definition's file
        // order, DLC packs included. `livery::teams_for_slots` decides which
        // slot flies which; that ordering is this project's, not the original's.
        let race_options = self.race_options(
            source,
            boot_shell
                .teams
                .iter()
                .map(|team| team.id.clone())
                .collect(),
        );

        // The three lists that come off the disc rather than out of the
        // definition: what is raceable, who can be raced for, and what
        // languages exist. All carry a label the player reads and a value the
        // settings file stores, and on a circuit or a team those are different
        // strings - `16_Track` and `Mantis` against their localised names,
        // which are shipped content and only ever live in memory.
        let shell = Shell {
            definition: self.definition.clone(),
            title,
            strings: boot_shell.strings.clone(),
            entries: boot_shell.entries.clone(),
            modes: menu::mode_choices(&boot_shell.strings),
            // The disc's own names for its stylings, so the row offers what the
            // source has rather than a list this build holds.
            front_end_styles: boot_shell
                .loading
                .map(|loading| {
                    loading
                        .features
                        .iter()
                        .map(|style| menu::Choice::plain(style.name))
                        .collect()
                })
                .unwrap_or_default(),
            teams: boot_shell
                .teams
                .iter()
                .map(|team| menu::Choice::labelled(&team.id, team.label(&boot_shell.strings)))
                .collect(),
            tracks: boot_shell
                .tracks
                .iter()
                .map(|track| {
                    (
                        track.clone(),
                        catalogue::label(
                            track,
                            &boot_shell.circuit_names,
                            &boot_shell.strings,
                            &boot_shell.tracks,
                        ),
                    )
                })
                .collect(),
            zone_tracks: boot_shell
                .zone_tracks
                .iter()
                .map(|track| {
                    (
                        track.clone(),
                        catalogue::label(
                            track,
                            &boot_shell.circuit_names,
                            &boot_shell.strings,
                            &boot_shell.zone_tracks,
                        ),
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
            space: boot_shell.space,
            menu_font: boot_shell.menu_font.clone(),
            sprites: boot_shell.sprites.clone(),
            // The disc's own chrome, read by the boot itself - which is where
            // the parsed XML, the sheet and the grid were all in hand.
            frame: boot_shell.frame.clone(),
        };

        println!("\n{MENU_KEYS}");

        // Unconditional, where it used to be `--prefetch` only. Its two extra
        // archive reads used to buy nothing on a boot that went straight to the
        // front end; every windowed boot now shows the loading screen while the
        // movies decode, so every windowed boot needs the tips and the glow
        // strip.
        let loading_assets = {
            let assets = loading::Assets::load(
                source,
                &boot_shell.strings,
                boot_shell.entries.as_deref(),
                crate::args::style_of(&self.settings),
            );
            for note in &assets.notes {
                info!("{note}");
            }
            assets
        };

        Ok(Prepared {
            boot_shell,
            media,
            shell,
            loading_assets,
            race_options,
            music_discs,
            // Asked for, not started: see `Session::start_prefetch`.
            prefetch: self.cli.prefetch.then(|| prefetch::Options {
                source: source.to_string(),
                movies: options.cache.clone(),
                audio: boot::default_audio_cache_dir(),
                refresh_video: self.cli.refresh_video,
            }),
        })
    }
}

/// Parses our own menu tree, or the one `--menu` names.
///
/// Here rather than in `main`'s body only because it is the last thing between
/// the command line and a source; it reads no disc and depends on nothing this
/// module produces. `language` resolves a row's `string_id`, when it names
/// one - see `crate::strings::project_table` for why this, and not the
/// disc's own `StringTable`, is what a caller this early can ever have.
///
/// # Errors
///
/// A `--menu` file that will not read, or a definition that will not parse.
pub(crate) fn definition(cli: &Cli, language: Option<&str>) -> Result<menu::Definition> {
    let strings = strings::project_table(language);
    match cli.menu.as_deref() {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?;
            menu::Definition::parse(&text, &strings)
                .with_context(|| format!("parsing {}", path.display()))
        }
        None => menu::Definition::parse(menu::BUILT_IN, &strings)
            .context("parsing the built-in menu definition"),
    }
}
