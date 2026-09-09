//! Every disc present declares its own roster, so nothing stands in for it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(roster_declared_ground_truth)'
//! ```
//!
//! # What this is for
//!
//! `boot::load_teams` used to carry a fallback: a source declaring no team at
//! all got Pulse's eight, filtered against what the archives really held. It
//! was the last of the five Pulse literals `boot.rs` applied to every source -
//! the other four became title axes (`oag_title::FrontEnd::root`, its
//! `language_plugins`, `oag_title::Title::plugin_definition`) or stopped being
//! constants at all (the body face, resolved off the language plugins by
//! `boot::load_font`).
//!
//! It was **removed rather than moved**, and this file is why that is safe to
//! say. A remembered roster is the hand-transcribed-table failure CLAUDE.md
//! names: on Wipeout HD it did fire, for a whole release, and eight teams in a
//! menu is what a working front end looks like - which is exactly why nothing
//! failed and nobody noticed until `plugin_definition` became per-title. A
//! title package cannot state the right answer for that branch either, since no
//! title has a *measured* roster for the definition-unreadable case; Pure says
//! as much where a constant would go (`oag_pure::names::handling_stats`).
//!
//! So the assertion is the shape, not a table of team names: every image
//! present declares a roster off its own `Definition.xml`, and every one of
//! those teams resolves to a ship on that source. What the counts are is each
//! title's own business and is pinned per-title elsewhere -
//! `hd_boot_ground_truth` asserts HD's twelve by name.

use std::path::{Path, PathBuf};

use oag_game::boot;
use oag_ui::frontend;

/// Every image present, as `(label, path)`.
///
/// A missing image is skipped rather than failed, the way the other
/// ground-truth files here do it, unless `OAG_REQUIRE_GAME_DATA` says the
/// caller expects them.
fn images() -> Vec<(&'static str, PathBuf)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for (label, name) in [
        ("pulse-psp-eu", "data/images/pulse-psp-eu.chd"),
        ("pulse-psp-usa", "data/images/pulse-psp-usa.chd"),
        ("pulse-ps2-eu", "data/images/pulse-ps2-eu.chd"),
        ("pure-psp-eu", "data/images/pure-psp-eu.chd"),
        ("pure-psp-usa", "data/images/pure-psp-usa.chd"),
        ("hdfury-ps3-eu", "data/images/hdfury-ps3-eu-dec.iso"),
    ] {
        let path = root.join(name);
        if path.exists() {
            found.push((label, path));
        } else {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but {} is missing",
                path.display()
            );
            println!("skipping: {} not present", path.display());
        }
    }
    found
}

/// Boots the shell alone - no movies, which is the slow half.
fn shell(label: &str, image: &Path) -> boot::Shell {
    let options = boot::Options {
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-roster-ground-truth"),
        audio_cache: boot::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    boot::load_shell(&options)
        .unwrap_or_else(|e| panic!("{label}: booting the shell: {e:#}"))
        .0
}

/// No source present reaches the boot with an empty roster.
///
/// The direct check on the removed branch's own condition. It fired on exactly
/// one source in this project's history - Wipeout HD, while `boot::definitions`
/// still asked every title for Pulse's `Data\Plugins\PI001\Definition.xml` -
/// and that is fixed at the source rather than papered over.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_disc_declares_a_roster_of_its_own() {
    for (label, image) in images() {
        let shell = shell(label, &image);
        assert!(
            !shell.teams.is_empty(),
            "{label}: the roster came back empty, so this source would have had \
             nothing to race: {:#?}",
            shell.report
        );
    }
}

/// The roster is the disc's, and the boot report says so.
///
/// Asserted from the report rather than only from the count, because a count
/// can be right for the wrong reason: Pulse's eight standing in on a Pulse disc
/// is indistinguishable from Pulse's eight read off it. The line naming the
/// definition is the one that tells them apart.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_boot_report_credits_the_discs_own_definition() {
    for (label, image) in images() {
        let shell = shell(label, &image);
        let credited = shell
            .report
            .iter()
            .any(|line| line.contains("team(s) over") && line.contains("definition(s)"));
        assert!(
            credited,
            "{label}: no report line says which definition the roster came off: {:#?}",
            shell.report
        );
        // The removed fallback announced itself on this wording. Nothing should
        // print it now, and a re-introduced stand-in that stayed silent would
        // still be caught by the line above.
        let stood_in = shell
            .report
            .iter()
            .any(|line| line.contains("no team was declared"));
        assert!(
            !stood_in,
            "{label}: a roster stood in for the disc's own: {:#?}",
            shell.report
        );
    }
}

/// No source loses a declared team to the raceability filter.
///
/// `load_teams` filters the declared list against both files a race reads and
/// reports the drop. **Both Pure pressings used to declare ten and keep nine**,
/// losing `AG Systems` - declared with a space where its own `location` says
/// `Data\Ships\AG_Systems` - because every path was composed from the declared
/// name rather than the folder. `catalogue::read_team` reads the folder now,
/// so all ten resolve and the team is back in the menu; measured 2026-08-26
/// against both pressings, and the underscore spelling was confirmed to hold
/// both the hull and the handling stats with no DLC mounted.
///
/// The count is asserted, not just the absence of a drop, because "nothing was
/// dropped" is also what an empty roster looks like.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_source_loses_a_declared_team() {
    for (label, image) in images() {
        let shell = shell(label, &image);
        assert!(
            !shell
                .report
                .iter()
                .any(|line| line.contains("declared team(s) have no ship")),
            "{label}: a declared team lost its ship or its handling stats: {:#?}",
            shell.report
        );
        if label.starts_with("pure-") {
            assert_eq!(
                shell.teams.len(),
                10,
                "{label}: ten is what this disc declares: {:#?}",
                shell.report
            );
            // Named directly, because the count alone cannot tell a restored
            // `AG_Systems` from some other team arriving in its place.
            let ids: Vec<&str> = shell.teams.iter().map(|team| team.id.as_str()).collect();
            assert!(
                ids.contains(&"AG_Systems"),
                "{label}: the team this file exists for is the folder \
                 `AG_Systems`, and it is absent: {ids:?}"
            );
        }
    }
}

/// The declared name survives as a label where it differs from the folder.
///
/// The other half of reading the id off the location: `AG_Systems` is the right
/// path component and the wrong thing to show a player. Pure's string tables go
/// unread (`load_strings` finds no entries for its language plugins), so
/// nothing else would put the space back.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_keeps_the_name_it_declares_for_the_team_whose_folder_differs() {
    for (label, image) in images() {
        if !label.starts_with("pure-") {
            continue;
        }
        let shell = shell(label, &image);
        let team = shell
            .teams
            .iter()
            .find(|team| team.id == "AG_Systems")
            .unwrap_or_else(|| panic!("{label}: the folder `AG_Systems` is not in the roster"));
        assert_eq!(
            team.name.as_deref(),
            Some("AG Systems"),
            "{label}: the declared name is what the menu falls back to here"
        );
        // Every other team on the disc spells the two the same way, and carries
        // no second spelling for that reason.
        for other in shell.teams.iter().filter(|team| team.id != "AG_Systems") {
            assert_eq!(
                other.name, None,
                "{label}: {} declares a name differing from its folder, which \
                 only AG Systems was ever measured doing",
                other.id
            );
        }
    }
}
