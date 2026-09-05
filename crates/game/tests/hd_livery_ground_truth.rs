//! Eight craft, eight teams, off a real Wipeout HD / Fury disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all hd_livery
//! ```
//!
//! # The counterpart of `livery_ground_truth.rs`, and why it is a second file
//!
//! The PSP test asserts the same feature on the same code, so on the face of it
//! this is a fixture swap. It is not, because the thing that was broken on HD is
//! upstream of everything that test covers: `race::load` read
//! `Data\Plugins\PI001\Definition.xml` whatever title it had opened, and HD
//! **names** its game plugin where the PSP titles number it. The read missed,
//! the roster came back empty, and `livery::teams_for_slots` gave every slot the
//! player's team - which is exactly what a working grid looks like on a disc
//! that genuinely declares one team, so nothing failed and nothing said so.
//!
//! So what this file is really pinning is
//! [`oag_title::Title::plugin_definition`]: that the roster is read off the
//! *title's* own definition, and that HD's twelve declared teams reach the grid
//! as eight different craft.
//!
//! # Two things differ from the PSP grid, both of them the disc's
//!
//! - **The hulls come out of a `.rcsmodel`**, not out of the `.vex`, so a
//!   triangle count here exercises `mesh::rcs` rather than `mesh::build`.
//! - **The locators are in `Locators.vex`** beside the hull rather than in it,
//!   which is the sibling read `livery::locators` falls back to. There is no
//!   `shipboost.vex` on this disc under that name, so the plume half of the PSP
//!   test has no counterpart here and is deliberately not asserted.

use std::path::{Path, PathBuf};

use oag_game::race;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");
    if path.exists() {
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

/// A single race, which is the mode that fields a grid.
fn load() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        // Deliberately empty, exactly as in the PSP file: `load` then reads the
        // disc's own plugin definition, which is the path `--race`, a capture
        // and this test all take.
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// What the disc declares, before a race is asked for anything.
///
/// Separate from the grid assertions because it is the fact the grid rests on,
/// and because a failure here says "the definition moved" where a failure below
/// says "the liveries did".
#[test]
#[ignore = "needs a disc image"]
fn the_disc_declares_twelve_teams_under_hds_own_plugin_name() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("mounting the disc");
    let name = oag_hd::TITLE.plugin_definition;
    let blob = archives
        .read_name(name)
        .expect("the game plugin definition");
    let definition = oag_formats::fexml::text(&blob).expect("reading it as text");

    let teams = oag_game::catalogue::teams(&definition);
    assert_eq!(
        teams.len(),
        12,
        "eight base teams and Fury's four, as `DATA00`'s copy declares them: {:?}",
        teams
            .iter()
            .map(|team| team.id.as_str())
            .collect::<Vec<_>>()
    );

    // Every declared id has to reach a real directory, or the roster is a list
    // of names rather than a grid. `location` is the disc's own spelling and
    // the ids are capitalised where the manifest stores them lowercase - the
    // PSARC lookup folds case, and this is what asserts that it does.
    for team in &teams {
        let ship = format!(r"{}\Ship.vex", team.location);
        assert!(
            archives.locate(&ship).is_some(),
            "{}: declared at {} and no ship there",
            team.id,
            team.location
        );
        assert!(
            archives
                .locate(&oag_formats::handling::entry_name(&team.id))
                .is_some(),
            "{}: declared and no handling stats",
            team.id
        );
    }

    // Pulse's name, on an HD source, finds nothing. This is the bug the field
    // exists for, asserted directly rather than inferred from an empty grid.
    assert!(
        archives
            .locate(oag_pulse::names::GAME_PLUGIN_DEFINITION)
            .is_none(),
        "Pulse's numbered plugin resolves here after all, which would make \
         `Title::plugin_definition` unnecessary rather than wrong"
    );
}

#[test]
#[ignore = "needs a disc image"]
fn every_grid_slot_gets_its_own_teams_hull() {
    let Some(loaded) = load() else {
        return;
    };
    assert_eq!(
        loaded.liveries.len(),
        oag_gameplay::MAX_SHIPS,
        "one livery per grid slot"
    );

    let teams: Vec<&str> = loaded
        .liveries
        .iter()
        .map(|livery| livery.team.as_str())
        .collect();
    let distinct: std::collections::BTreeSet<&&str> = teams.iter().collect();
    assert_eq!(
        distinct.len(),
        oag_gameplay::MAX_SHIPS,
        "the disc declares twelve teams and the grid is eight, so every slot \
         should fly its own: {teams:?}"
    );

    // The feature, not the plumbing: eight *different models*. On this title
    // that also means eight `.rcsmodel` reads that each found their sibling -
    // a hull whose geometry file is missing draws nothing and reports it, and
    // would land here as a shared triangle count of zero.
    let shapes: std::collections::BTreeSet<usize> = loaded
        .liveries
        .iter()
        .map(|livery| livery.hull.indices.len() / 3)
        .collect();
    assert!(
        !shapes.contains(&0),
        "a hull drew no triangles at all, which is what a missing `.rcsmodel` \
         looks like: {:?}",
        loaded
            .liveries
            .iter()
            .map(|livery| (livery.team.as_str(), livery.hull.indices.len() / 3))
            .collect::<Vec<_>>()
    );
    assert!(
        shapes.len() >= 6,
        "the grid drew {} distinct hull(s) across eight teams, so most craft are \
         wearing somebody else's ship: {:?}",
        shapes.len(),
        loaded
            .liveries
            .iter()
            .map(|livery| (livery.team.as_str(), livery.hull.indices.len() / 3))
            .collect::<Vec<_>>()
    );
}

/// The other half of a livery, and the one a shared hull hid: each team's
/// exhaust hangs off *its own* nozzle - here read out of the `Locators.vex`
/// beside the hull rather than out of the hull itself.
#[test]
#[ignore = "needs a disc image"]
fn every_hull_carries_its_own_nozzle() {
    let Some(loaded) = load() else {
        return;
    };
    for livery in &loaded.liveries {
        assert!(
            livery.nozzle.is_some(),
            "{}: no Engine Flare locator, so this craft would burn nothing",
            livery.team
        );
        assert!(
            !livery.collision_fx.is_empty(),
            "{}: no Ship Collision Fx locators, so its sparks have no anchor",
            livery.team
        );
    }

    let nozzles: std::collections::BTreeSet<[u32; 3]> = loaded
        .liveries
        .iter()
        .filter_map(|livery| livery.nozzle)
        .map(|at| [at.x.to_bits(), at.y.to_bits(), at.z.to_bits()])
        .collect();
    assert!(
        nozzles.len() > 1,
        "all eight nozzles are at the same point, which is what a single shared \
         locator looks like: {:?}",
        loaded
            .liveries
            .iter()
            .map(|livery| (livery.team.as_str(), livery.nozzle))
            .collect::<Vec<_>>()
    );
}

/// The player flies what the player picked, whatever the grid does around them.
#[test]
#[ignore = "needs a disc image"]
fn slot_zero_is_the_team_the_options_asked_for() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_hd::TITLE.plugin_definition)
        .expect("the game plugin definition");
    let definition = oag_formats::fexml::text(&blob).expect("reading it as text");
    let teams = oag_game::catalogue::teams(&definition);
    assert!(teams.len() > 1, "the disc declares more than one team");

    // The *last* team, so a pass could not come from it happening to be first.
    // On this disc that is one of Fury's four, which are also the four that live
    // in `DATA03` - the archive carrying no circuit at all.
    let chosen = teams.last().expect("a team").id.clone();
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        team: Some(chosen.clone()),
        ..race::Options::default()
    })
    .expect("loading the race");

    assert_eq!(loaded.liveries[0].team, chosen, "slot 0 is the player's");
    assert!(
        loaded.liveries[1..]
            .iter()
            .all(|livery| livery.team != chosen),
        "the player's own team should not also be flown by an opponent: {:?}",
        loaded
            .liveries
            .iter()
            .map(|livery| livery.team.as_str())
            .collect::<Vec<_>>()
    );
}
