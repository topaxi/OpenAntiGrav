//! Everything a run needs that depends on *which* source it opened.
//!
//! This used to be the middle of `main`, run once as soon as
//! [`oag_source::source::resolve`] had answered. It is a module of its own now
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
use log::debug;

use oag_game::{boot, loading, movie, prefetch, settings};
use oag_raceplay as race;
use oag_ui::frontend;
use oag_ui::{menu, strings};

use crate::args::{resolve_difficulty, resolve_scheme};
use crate::cli::Cli;
use crate::hints;
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
    pub(crate) class: String,
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
    pub(crate) music_discs: oag_sound::MusicDiscs,
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
                .unwrap_or_else(oag_source::cache::default_cache_dir),
            audio_cache: oag_source::cache::default_audio_cache_dir(),
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
            // Passed straight through, `None` included: a team with no
            // `HullVariant` axis, or a stem it does not offer, is the same
            // "unrecognised, race the baseline and say so" shape `team`
            // and `track` already resolve at. See
            // `race::Options::hull_variant`.
            hull_variant: self.cli.variant.clone(),
            // `race::Options::skin` - the player's own paint job, resolved
            // against what the team declares once the source is open.
            skin: self.cli.skin.clone(),
            class: self.class.clone(),
            mode: self.mode,
            // **From the settings here too, the same reason `difficulty`
            // below is** - `--race` has its own `settings::Settings`, loaded
            // the same way the menus' does, and a `None` baked in here is
            // what made `race::load` fall back to the chain's own default
            // language regardless of what the player had saved. Kept in
            // step with the *live* value at every menu-driven launch by
            // `Session::launch_race`, which overwrites this same field from
            // `self.settings.language` right before `race::load` runs - see
            // that function's own doc for why a value set once here would
            // go stale the moment the OPTIONS page's LANGUAGE row changes it.
            language: self.settings.language.clone(),
            // The saved KILLS row, for an Eliminator only; no CLI flag names
            // one. See `race::Options::eliminator_kill_target`.
            eliminator_kill_target: self.settings.race.eliminator_kill_target(self.mode),
            // `None` for the same reason - the campaign is the only writer
            // of this field. See `race::Options::laps_override`.
            laps_override: None,
            // The saved WEAPONS row, for a single race only. See
            // `race::Options::weapons_override`.
            weapons_override: self.settings.race.weapons_override(self.mode),
            zone_stage: self.cli.zone_stage,
            opponent_teams,
            // No CLI flag or menu screen offers a per-slot override; only
            // `race::load_event`'s own 2048-campaign resolution sets this.
            // See `race::Options::grid_teams`.
            grid_teams: Vec::new(),
            // See `race::Options::zone_model`: off until the Zone road shader is read.
            zone_model: false,
            hull_shine: !self.cli.no_hull_shine,
            hull_wreck: !self.cli.wreck.no_hull_wreck,
            track_shine: !self.cli.no_track_shine,
            ribbon: self.cli.ribbon,
            collision: self.cli.collision,
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
    pub(crate) fn music_discs(&self, source: &str) -> oag_sound::MusicDiscs {
        if self.cli.dry_run {
            return oag_sound::MusicDiscs::default();
        }
        let discs = oag_sound::MusicDiscs::survey(source, &oag_game::sound::GameLibrary);
        debug!("audio: music discs, {}", discs.describe());
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

        let (mut boot_shell, mut archives, title) = boot::load_shell(&options)?;
        log::info!("{}: {source}", title.name);
        // Drained rather than iterated: `boot::assemble` appends its own lines
        // to this same list, and the hand-off prints what it finds there.
        // Leaving these in would print the whole first half twice, seconds
        // apart, which reads as the disc having been opened again.
        oag_raceplay::loader_log::lines(boot_shell.report.drain(..));
        // Read while the archives are still in hand: the media worker takes
        // them next. A few kilobytes, and silence on a title with no bank.
        let menu_sfx = oag_sound::sfx::MenuSfx::load(
            &mut archives,
            title.race.sounds,
            title
                .race
                .zone_announcer
                .map_or(oag_title::SequenceTick::Unknown, |z| z.tick),
            // The style the served front end is in, which picks HD's
            // `accept_fury`/`reject_fury`. See `boot::sprites::fury_style`.
            boot_shell
                .frame
                .blocks
                .as_ref()
                .is_some_and(|blocks| blocks.fury),
        );
        oag_raceplay::loader_log::lines(menu_sfx.report());
        let media = boot::MediaWorker::spawn(archives, &boot_shell, &options);

        // Every team the player's own source declares, in the definition's file
        // order, DLC packs included. `livery::teams_for_slots` decides which
        // slot flies which, with Pulse's own roster draw off the race seed.
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
        let mut definition = self.definition.clone();
        // A no-op on every title but Wipeout Pure and Wipeout Pulse, whose
        // race defaults declare no variant axis at all - see
        // `menu::Definition::drop_unavailable_race_variant`.
        definition.drop_unavailable_race_variant(title);
        // And the three rows the race box's own screens pick, on a title
        // that authors them - see `menu::Definition::drop_rows_picked_on_screen`.
        definition.drop_rows_picked_on_screen(title);
        // A phone app is left, not quit - see `menu::Definition::drop_quit`.
        if cfg!(target_os = "android") {
            definition.drop_quit();
        }
        // A page has no monitor to pick and one present mode, and a desktop
        // has no canvas: see `settings::web`.
        if cfg!(target_arch = "wasm32") {
            definition.drop_settings(&settings::web::HIDDEN_ROWS);
            definition.drop_settings(&["display.window_size"]);
        } else {
            definition.drop_settings(&settings::web::WEB_ONLY_ROWS);
        }
        // Only a touchscreen has an overlay to set up.
        if !oag_game::touch_controls::available() {
            definition.drop_touch_controls();
        }
        // Built through `Shell::from_boot` rather than a literal here, so
        // this boot and a live LANGUAGE-row switch
        // (`Session::resupply_language`) read the same `boot::Shell` the
        // same way - see that function's own doc.
        let mut shell = Shell::from_boot(title, definition, &boot_shell);
        shell.menu_sfx = Some(menu_sfx);

        println!("\n{}", hints::menu_keys(&boot_shell.strings));

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
            oag_raceplay::loader_log::lines(&assets.notes);
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
                audio: oag_source::cache::default_audio_cache_dir(),
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
/// one - see `oag_ui::strings::project_table` for why this, and not the
/// disc's own `StringTable`, is what a caller this early can ever have.
///
/// # Errors
///
/// A `--menu` file that will not read, or a definition that will not parse.
pub(crate) fn definition(cli: &Cli, language: Option<&str>) -> Result<menu::Definition> {
    let strings = strings::project_table(language);
    match cli.menu_args.menu.as_deref() {
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
