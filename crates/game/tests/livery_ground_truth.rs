//! Eight craft, eight teams, off a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all livery
//! ```
//!
//! # Why it needs the disc
//!
//! `crate::livery`'s unit tests cover which team lands in which slot, which is
//! arithmetic over a list of strings. What they cannot see is whether those
//! ids resolve to *different hulls* on a real source, and that is the whole
//! feature: before this landed, `race::load` read one `Ship.vex` and
//! `Scene::new` cloned it eight times, so a grid was one team's ship eight
//! times over.
//!
//! **The assertion is on geometry, and that is a deliberate choice.** Two hulls
//! could differ only in the texture painted on them - that is exactly what
//! Zone mode does, where every team's `Zone.vex` decodes to the same 1213
//! vertices and 1149 triangles. It is not what the race hulls do: the eight
//! teams the PSP disc declares range from 845 to 1,497 triangles. So counting
//! distinct triangle counts is a real test here, and would be a vacuous one if
//! pointed at Zone.

use std::path::{Path, PathBuf};

use oag_game::race;
use oag_physics::SpeedClass;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");
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
        class: SpeedClass::Venom,
        mode: oag_race::Mode::SingleRace,
        // Deliberately empty: `load` then reads the disc's own plugin
        // definition, which is the path `--race`, a capture and this test all
        // take. An entry point that forgot to pass a list is how the whole
        // grid ended up in one livery the first time.
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
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
        "the disc declares eight raceable teams and the grid is eight, so every \
         slot should fly its own: {teams:?}"
    );

    // The feature, not the plumbing: eight *different models*. Triangle counts
    // are the cheapest thing that cannot be equal by accident - see the module
    // docs on why this would be the wrong assertion for Zone.
    let shapes: std::collections::BTreeSet<usize> = loaded
        .liveries
        .iter()
        .map(|livery| livery.hull.indices.len() / 3)
        .collect();
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
/// exhaust hangs off *its own* nozzle.
///
/// A single locator carried onto eight different hulls puts the flare inside
/// the fuselage on the ones it was not authored for, which is invisible in a
/// test that only counts triangles.
#[test]
#[ignore = "needs a disc image"]
fn every_hull_carries_its_own_nozzle_and_plume() {
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
            livery.boost.is_some(),
            "{}: no boost plume beside its hull",
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
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_formats::fexml::expand(&blob).expect("expanding it");
    let teams = oag_game::catalogue::teams(&definition);
    assert!(teams.len() > 1, "the disc declares more than one team");

    // The *last* team, so a pass could not come from it happening to be first.
    let chosen = teams.last().expect("a team").id.clone();
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: SpeedClass::Venom,
        mode: oag_race::Mode::SingleRace,
        team: chosen.clone(),
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
