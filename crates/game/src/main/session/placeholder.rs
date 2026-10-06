//! Booting a title with no front end this build can walk straight into this
//! build's own menus instead of refusing.
//!
//! **Wipeout 2048 no longer lands here** (2026-09-21): its front end is
//! `Some` since [ADR-0054], and `oag_game::boot::load_shell` walks its
//! declared boot chain and draws its touch grids off
//! `oag_title::FrontEnd::touch`, with `oag_ui::placeholder::MENU_SKIN`
//! standing in only for the `MenuSkin` the shell's *own* menus would need -
//! see that module's docs for why nothing may put a `MenuSkin` on
//! `oag_2048::TITLE` itself. What is left for this route is a title whose
//! `front_end` is `None` outright, or one whose front end fills neither
//! axis; a 2048 boot that fails for some other reason hands its error back
//! rather than covering it with these menus. This module never calls
//! `load_shell` for such a title at all: it opens the archives itself, reads
//! only what does not need a `MenuSkin` - the roster and the circuit list,
//! off the title's own plugin definitions - and builds the same light
//! `Session::Shell` [`Session::open_menus`] already knows how to draw.
//!
//! [ADR-0054]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md
//!
//! **A whole `Stage::Frontend`/`Stage::loading` cheaper too, not just a
//! refusal avoided.** There is no intro reel to decode and no loading-screen
//! art to read, because there is no front end to have authored either, so the
//! boot this module runs skips straight from the disc chooser to the menus -
//! [`Session::finish_launcher`] is what decides which of the two routes a
//! picked source takes.
//!
//! # The circuit list needs a second file, and that is 2048's own shape
//!
//! Every other title in the corpus keeps `PI_Team` and `PI_Track` in the one
//! file `oag_title::Title::plugin_definition` names. 2048 splits the front
//! end's plugins by kind - `oag_2048::race::SHIP_DIR`'s own module docs cover
//! why - so its teams and its circuits are two different documents, and this
//! is the one place in the engine that reads both. A title that does not
//! split them simply has no second document to add, and
//! [`plugin_documents`] drops a name that will not read the same way
//! `oag_game::boot`'s own loaders do everywhere else.

use anyhow::{Context, Result};
use log::{debug, warn};

use oag_game::loading;
use oag_hud::sprite;
use oag_raceplay::catalogue;
use oag_ui::{font, language, menu, placeholder, strings};

use crate::hints;
use crate::session::Shell;

use super::Session;

impl Session {
    /// The other route [`Session::finish_launcher`] can take: straight to the
    /// menus, for a title [`oag_game::boot::load_shell`] would refuse.
    ///
    /// `windowed_error` is what `Pending::windowed` failed with - not
    /// necessarily this title's refusal, so this re-opens `source` itself and
    /// checks. When the title it finds carries a front end after all, this was
    /// some other failure (the source vanished between the two opens, most
    /// plausibly) and `windowed_error` is what the player should see, with the
    /// pick put back so the chooser is still usable.
    ///
    /// # Errors
    ///
    /// `windowed_error` itself, on a title with a front end after all, or
    /// opening the archives a second time, on one without.
    pub(crate) fn finish_launcher_placeholder(
        &mut self,
        source: &str,
        windowed_error: anyhow::Error,
    ) -> Result<()> {
        let Some(pending) = self.pending.take() else {
            return Err(windowed_error);
        };

        let (packs, pure_packs, problems) = oag_source::dlc::packs_from_defaults(
            &pending.dlc,
            &oag_source::cache::default_dlc_cache_dir(),
        );
        let opened = match oag_source::title::open_source(source, packs, pure_packs) {
            Ok(opened) => opened,
            Err(e) => {
                self.pending = Some(pending);
                return Err(e).context("opening the archives a second time");
            }
        };
        let title = opened.title;
        if title
            .front_end
            .is_some_and(|fe| fe.menu.is_some() || fe.touch.is_some())
        {
            // Not this route's case after all: whatever `windowed_error` was
            // about, it was not a front end with nothing to draw, and the
            // pick goes back so the player can try another row.
            self.pending = Some(pending);
            return Err(windowed_error);
        }
        warn!(
            "{}: no MenuSkin-shaped menu has been read off this title's own archives \
             (front end: {}), so this is OpenAntiGrav's own placeholder menu, not {}'s \
             own - see oag_ui::placeholder",
            title.name,
            if title.front_end.is_some() {
                "read, but drawing a different vocabulary - see oag_title::FrontEnd::touch"
            } else {
                "not read at all"
            },
            title.name
        );
        for problem in &problems {
            warn!("dlc: {problem}");
        }
        let mut archives = opened.archives;
        let strings = language::StringTable::default();

        // **Two documents, not one.** See this module's own docs: 2048 is the
        // one title in the corpus that splits `PI_Team` and `PI_Track` across
        // separate plugins, and `oag_2048::names::TRACK_PLUGIN_DEFINITION` is
        // the second - harmless to ask for on a hypothetical title that keeps
        // them together, since a name that does not exist there is dropped the
        // same way any other unread plugin is.
        let documents = plugin_documents(
            &mut archives,
            &[
                title.plugin_definition,
                oag_2048::names::TRACK_PLUGIN_DEFINITION,
            ],
        );

        let declared_teams = catalogue::all_teams(&documents);
        let declared_team_count = declared_teams.len();
        let teams = raceable_teams(&archives, declared_teams);
        if teams.len() != declared_team_count {
            debug!(
                "{}: {} of {declared_team_count} declared team(s) are raceable - see \
                 oag_game::main::session::placeholder::raceable_teams for why the rest \
                 are not offered",
                title.plugin_definition,
                teams.len(),
            );
        }
        let team_choices: Vec<menu::Choice> = teams
            .iter()
            .map(|(id, label)| menu::Choice::labelled(id.clone(), label.clone()))
            .collect();

        // Filtered to what this source actually has geometry for, the same
        // check `oag_game::boot::roster::load_tracks` makes: a declared
        // circuit with no `.vex` on this source is a pack that is not mounted,
        // not a race that can be offered.
        let raceable: Vec<catalogue::Track> = catalogue::all_tracks(&documents)
            .into_iter()
            .filter(|track| archives.locate(&track.entry_name()).is_some())
            .collect();
        let circuit_names = language::CircuitNames::default();
        let tracks: Vec<(catalogue::Track, String)> = raceable
            .iter()
            .map(|track| {
                (
                    track.clone(),
                    catalogue::label(track, &circuit_names, &strings, &raceable),
                )
            })
            .collect();
        debug!(
            "{}: {} team(s), {} raceable circuit(s) over {} definition(s)",
            title.plugin_definition,
            teams.len(),
            tracks.len(),
            documents.len()
        );

        self.race_options =
            Some(pending.race_options(source, teams.iter().map(|(id, _)| id.clone()).collect()));
        self.music_discs = pending.music_discs(source);
        self.loading_assets = loading::Assets::load(source, &strings, None, None);
        self.shell = Some(Shell {
            definition: pending.definition.clone(),
            title,
            platform: archives.layout.platform,
            // 2048 is `oag_title::ZoneCircuit::SameCircuit` - a Zone race
            // runs whichever circuit is already picked, so the CIRCUIT row
            // offers the same list under either mode. See
            // `oag_2048::race::DEFAULTS.zone` and
            // `crate::main::session::Shell::tracks_for`.
            zone_tracks: tracks.clone(),
            tracks,
            teams: team_choices,
            languages: Vec::new(),
            modes: menu::mode_choices(&strings),
            kill_targets: Vec::new(),
            weapons: Vec::new(),
            entries: None,
            front_end_styles: Vec::new(),
            font: font::Atlas::build(),
            sprites: sprite::Sheet::default(),
            globals: Vec::new(),
            circuit_names: oag_ui::language::CircuitNames::default(),
            menu_skin: &placeholder::MENU_SKIN,
            space: oag_display::space::Space::default(),
            menu_font: None,
            face_scales: Vec::new(),
            title_font: None,
            buttons_font: None,
            frame: menu::Frame::default(),
            // This placeholder shell reads no front-end XML at all - see
            // the module doc - so there is no `NavigationController` to
            // find one in either.
            nav_legend: None,
            ticker: None,
            fury_backdrop: None,
            // 2048's front end is unread, so its race launches from the
            // RACE page the way every title's did before the pickers.
            track_select: None,
            ship_select: None,
            team_details: Vec::new(),
            strings,
        });
        self.prefetch_pending = None;

        self.open_menus()?;
        // Not `oag_2048`'s own window title - there is none read - but a
        // reminder this is a stand-in menu, on screen rather than only in the
        // log line above. See `oag_ui::placeholder`.
        self.gpu
            .window
            .set_title(&format!("OpenAntiGrav - {} (placeholder menu)", title.name));
        println!(
            "\n{}",
            hints::menu_keys(&strings::project_table(self.settings.language.as_deref()))
        );
        Ok(())
    }
}

/// The teams this build can actually race, as `(id, label)` - not the
/// nineteen `PI_Team` nodes 2048's own plugin declares.
///
/// **2048's own five teams are a two-level roster.** `oag_2048::race::SHIP_DIR`'s
/// own docs cover why: each team is four numbered craft, and both its ship and
/// its `handlingstats.xml` live one directory deeper than the plugin's own id
/// names - `Data\HandlingStats\Auricom2048\handlingstats.xml` is not a file on
/// this source, `Data\HandlingStats\Auricom2048\3\handlingstats.xml` is. So
/// this appends the craft number a race needs, and picks the same one
/// [`oag_2048::race::DEFAULT_TEAM`] already does - the "speed" craft - so a
/// menu pick and the `--race` default fly the same slot for the same team.
/// **This is a stand-in, not a measurement**: the original's own craft picker
/// is a menu step nothing here has read, and every team races one fixed craft
/// until it has.
///
/// **2048's other fourteen teams - its HD-derived roster - are dropped
/// entirely.** They are declared in the same plugin, ported wholesale from
/// Wipeout HD, but their `handlingstats.xml` lives beside their ship under
/// `oag_2048::race::HD_SHIP_DIR` rather than under
/// `oag_2048::race::HANDLING_DIR` - the one path
/// `oag_title::RaceDefaults::handling_dir` can name. Nothing this build
/// composes finds them, so offering one would repeat the crash this filter
/// exists to prevent. `oag_2048::race::HD_SHIP_DIR`'s own docs call this out
/// already: "what it costs is the HD-derived roster" is a fact about the
/// title, not something a menu can paper over.
fn raceable_teams(
    archives: &oag_assets::Archives,
    declared: Vec<catalogue::Team>,
) -> Vec<(String, String)> {
    /// The craft [`oag_2048::race::DEFAULT_TEAM`] already picks - see this
    /// function's own docs.
    const CRAFT: &str = "3";

    declared
        .into_iter()
        .filter(|team| team.location.starts_with(oag_2048::race::SHIP_DIR))
        .filter_map(|team| {
            let id = format!("{}\\{CRAFT}", team.id);
            let ship = oag_pulse::race::ships::entry_name_in(
                oag_2048::race::SHIP_DIR,
                &id,
                oag_pulse::race::ships::HULL,
            );
            let handling = oag_tables::handling::entry_name_in(oag_2048::race::HANDLING_DIR, &id);
            let raceable = archives.locate(&ship).is_some() && archives.locate(&handling).is_some();
            raceable.then(|| {
                let label = team.name.unwrap_or(team.id);
                (id, label)
            })
        })
        .collect()
}

/// Every named plugin definition this title's roster and circuit list are
/// declared in, followed by every mounted pack's manifest - the same
/// one-list-because-one-schema shape `oag_raceplay::catalogue::all_teams` and
/// `all_tracks` expect, built by hand rather than through
/// `oag_game::boot::roster::definitions` because that function is private to
/// the boot sequence this module deliberately does not run.
///
/// A name that will not read is dropped rather than failing the boot: the
/// same trade `oag_game::boot`'s own loaders make everywhere else.
fn plugin_documents(archives: &mut oag_assets::Archives, names: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for name in names {
        if let Ok(blob) = archives.read_name(name) {
            out.push(decode(&blob));
        }
    }
    out.extend(archives.manifests.iter().cloned());
    out
}

/// A front-end XML file's bytes as text, the same fallback
/// `oag_game::oag_ui::xml::expand` uses and for the same reason - `.fexml`'s
/// packed name dictionary on one side, a release with a non-UTF-8 language
/// name on the other. Reimplemented rather than reached for: that function is
/// `pub(crate)` to the library crate, and this module is the `[[bin]]` one.
fn decode(blob: &[u8]) -> String {
    if oag_tables::fexml::is_fexml(blob)
        && let Ok(text) = oag_tables::fexml::expand(blob)
    {
        return text;
    }
    String::from_utf8(blob.to_vec())
        .unwrap_or_else(|e| e.into_bytes().into_iter().map(char::from).collect())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    /// `data/extracted/vita/PCSF00007`, as an absolute path: a test binary's
    /// cwd is this crate's own manifest directory, not the workspace root, so
    /// every ground-truth test in this project reaches `data/` this way rather
    /// than through a bare relative string. See
    /// `crates/game/tests/boot_legal_wrap_ground_truth.rs` for the same
    /// pattern.
    fn package() -> Option<PathBuf> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("data/extracted/vita/PCSF00007");
        if path.join("base/PSP2/data.psarc").is_file() {
            return Some(path);
        }
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        println!("skipping: {} not present", path.display());
        None
    }

    /// `#[ignore]`d like every other ground-truth test: it needs the real
    /// extract under `data/extracted/vita/`, which `just test-data` runs with
    /// but an ordinary `just test` does not. Pins the two document reads this
    /// module's own docs argue for - the roster and the circuit list both
    /// coming back non-empty, off 2048's own real archives, with no window and
    /// no `Session` involved.
    #[test]
    #[ignore = "needs data/extracted/vita/PCSF00007, see data/README.md"]
    fn twenty_forty_eights_own_roster_and_circuits_both_read() {
        let Some(package) = package() else { return };
        let opened =
            oag_source::title::open_source(&package.display().to_string(), Vec::new(), Vec::new())
                .unwrap();
        assert_eq!(opened.title.name, "Wipeout 2048");
        // 2048's front end is real since ADR-0054 - its boot chain and
        // language plugins are read - but it still has no `MenuSkin`-shaped
        // menu, which is the state this whole module exists for.
        assert!(
            opened
                .title
                .front_end
                .is_some_and(|fe| fe.menu.is_none() && fe.touch.is_some())
        );

        let mut archives = opened.archives;
        let documents = plugin_documents(
            &mut archives,
            &[
                opened.title.plugin_definition,
                oag_2048::names::TRACK_PLUGIN_DEFINITION,
            ],
        );
        assert_eq!(documents.len(), 2, "both of 2048's own plugins should read");

        let declared = oag_raceplay::catalogue::all_teams(&documents);
        assert!(!declared.is_empty(), "2048 ships a real roster");
        // Pins the crash a plain `PI_Team` id produced: `--race`'s own default,
        // `feisar2048\3`, resolving via `oag_tables::handling::entry_name_in`
        // is what a chosen row must also resolve to.
        let teams = raceable_teams(&archives, declared);
        assert!(!teams.is_empty(), "the native five should all be raceable");
        assert_eq!(
            teams.len(),
            5,
            "exactly the native five, not the HD-derived fourteen"
        );
        for (id, _) in &teams {
            assert!(
                id.ends_with(r"\3"),
                "{id} should carry the speed craft's number"
            );
            let entry = oag_tables::handling::entry_name_in(oag_2048::race::HANDLING_DIR, id);
            archives
                .read_name(&entry)
                .unwrap_or_else(|e| panic!("{entry} should actually read: {e}"));
        }

        let raceable: Vec<_> = oag_raceplay::catalogue::all_tracks(&documents)
            .into_iter()
            .filter(|track| archives.locate(&track.entry_name()).is_some())
            .collect();
        assert!(
            !raceable.is_empty(),
            "the base package's own circuits should have geometry on this source"
        );
    }
}
