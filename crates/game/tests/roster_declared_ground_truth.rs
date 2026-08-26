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

use oag_game::{boot, frontend};

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

/// Which sources lose a declared team to the raceability filter, measured.
///
/// `load_teams` filters the declared list against both files a race reads and
/// reports the drop. On the four Pulse-family and HD sources it drops nothing.
/// **Both Pure pressings declare ten and keep nine**, and the one they lose is
/// the same on each: `AG Systems`, declared with a space where its own
/// `location` says `Data\Ships\AG_Systems`. `raceable` composes both entry
/// names from the team's *id*, so the space is carried into the path, and a
/// founding team is silently absent from Pure's menu on both pressings.
///
/// Pinned rather than asserted away, because the number is the evidence: nine
/// is what this build currently offers on Pure and ten is what the disc says,
/// so a fix moves this to ten and a regression moves it to eight. Measured
/// 2026-08-26 against `pure-psp-usa.chd` and `pure-psp-eu.chd`, confidence 95 -
/// the id and the location are both read straight off the disc's own
/// `Definition.xml`, and the two pressings agree.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn only_pure_loses_a_declared_team_and_it_loses_one() {
    for (label, image) in images() {
        let shell = shell(label, &image);
        let dropped = shell
            .report
            .iter()
            .any(|line| line.contains("declared team(s) have no ship"));
        if label.starts_with("pure-") {
            assert!(
                dropped,
                "{label}: no team is dropped any more. If `raceable` learned to \
                 read the declared location, this file's docs and the count \
                 below need updating rather than this assertion deleting: {:#?}",
                shell.report
            );
            assert_eq!(
                shell.teams.len(),
                9,
                "{label}: nine of the ten declared is what a build that composes \
                 the path from the id offers: {:#?}",
                shell.report
            );
            let ids: Vec<&str> = shell.teams.iter().map(|team| team.id.as_str()).collect();
            assert!(
                !ids.contains(&"AG Systems"),
                "{label}: the team this measurement is about is present, so the \
                 count of nine now means something else: {ids:?}"
            );
        } else {
            assert!(
                !dropped,
                "{label}: a declared team lost its ship or its handling stats, \
                 which no source outside Pure has ever done: {:#?}",
                shell.report
            );
        }
    }
}
