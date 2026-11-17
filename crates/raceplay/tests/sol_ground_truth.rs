//! Wipeout 2048's `sol`, on 2048's own package and on the Omega Collection's
//! re-ship of it: the one circuit whose `track.vex` authors collision (a lone
//! `Reset` node) but no ground.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^sol_ground_truth::/)'
//! ```
//!
//! The ground is in `track_col.col`, which the loader once read only for a
//! `.vex` with no collision at all, so every craft fell from the grid into the
//! `.vex`'s reset volume (`docs/formats/2048-collision.md`, "Which circuits read
//! the container").

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_physics::{Ray, Raycaster, Surface};
use oag_raceplay as race;

/// 2048's package, or the Omega extraction, and the title's own entry name
/// for `sol`.
fn source(title: &str) -> Option<(PathBuf, &'static str)> {
    match title {
        "2048" => oag_testdata::exact("data/extracted/vita/PCSF00007")
            .map(|p| (p, r"Data\art\published\environments\sol\track.vex")),
        "omega" => oag_testdata::exact("data/extracted/ps4")
            .map(|p| (p, r"data\environments2048\sol\track.vex")),
        other => panic!("{other} is no title here"),
    }
}

fn start(title: &str) -> Option<race::Race> {
    let (source, track) = source(title)?;
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("{title} sol does not load: {e:#}"));
    Some(race::Race::start(loaded.setup))
}

/// The first surface straight down from `from`, within 200 units.
fn surface_below(race: &race::Race, from: Vec3) -> Option<Surface> {
    let mut out = [None; 8];
    let count = race
        .collision()
        .raycast_all(Ray::new(from, -Vec3::Y, 200.0), None, true, &mut out);
    out[..count]
        .iter()
        .flatten()
        .min_by(|a, b| a.distance.total_cmp(&b.distance))
        .map(|hit| hit.surface)
}

fn every_grid_slot_has_ground_under_it(title: &str) {
    let Some(race) = start(title) else {
        return;
    };
    let floors = race
        .collision()
        .colliders()
        .iter()
        .filter(|c| c.surface() == Surface::Floor && c.triangle_count() > 0)
        .count();
    assert!(floors > 0, "{title} sol loaded no floor collider");
    let world = &race.sim.world;
    for slot in 0..world.ship_count as usize {
        let at = world.ships[slot].physics.body.position;
        let below = surface_below(&race, at);
        assert!(
            matches!(below, Some(Surface::Floor | Surface::MagFloor)),
            "{title} sol: grid slot {slot} at {at:?} has {below:?} under it, not ground"
        );
    }
}

#[test]
#[ignore = "needs the extracted 2048 package"]
fn every_grid_slot_has_ground_under_it_on_2048() {
    every_grid_slot_has_ground_under_it("2048");
}

#[test]
#[ignore = "needs the Omega extraction"]
fn every_grid_slot_has_ground_under_it_on_omega() {
    every_grid_slot_has_ground_under_it("omega");
}

/// The window a field gets to lap in.
const FIELD_TICKS: u32 = 12_000;

/// A field of seven opponents at VENOM with slot 0 sitting out: each craft's
/// respawns and flying laps (a lap begun over the line, not off the grid).
/// Stops once every craft has a flying lap, unless `OAG_SWEEP` asks for the
/// whole window's respawn counts.
fn field(title: &str, level: oag_ai::Difficulty) -> Option<(Vec<u32>, Vec<u32>, u32)> {
    let (source, track) = source(title)?;
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("{title} sol does not load: {e:#}"));
    let mut race = race::Race::start(loaded.setup);
    race.sim.world.ships[0].active = false;
    let count = race.ship_count() as usize;
    let whole_window = std::env::var_os("OAG_SWEEP").is_some();
    let causes = std::env::var_os("OAG_SWEEP_CAUSES").is_some();
    let mut lap: Vec<u32> = (0..count)
        .map(|slot| race.sim.world.ships[slot].standing.lap)
        .collect();
    let mut flying = vec![0u32; count];
    let mut respawns = vec![0u32; count];
    let mut ticks = 0;
    for tick in 0..FIELD_TICKS {
        race.tick(&oag_gameplay::PlayerInputs::none());
        ticks = tick + 1;
        for slot in 1..count {
            let ship = &race.sim.world.ships[slot];
            if causes && race.respawns_of(slot) != respawns[slot] {
                println!(
                    "{title} sol {level:?}: slot {slot} tick {tick} lap {} {:?}@{}",
                    ship.standing.lap,
                    race.last_respawn_cause_of(slot),
                    ship.driver.index
                );
            }
            respawns[slot] = race.respawns_of(slot);
            if ship.standing.lap != lap[slot] {
                flying[slot] += u32::from(lap[slot] > 1);
                lap[slot] = ship.standing.lap;
            }
        }
        if !whole_window && flying[1..].iter().all(|&laps| laps > 0) {
            break;
        }
    }
    Some((respawns[1..].to_vec(), flying[1..].to_vec(), ticks))
}

fn a_field_of_seven_laps(title: &str, level: oag_ai::Difficulty) {
    let Some((respawns, flying, ticks)) = field(title, level) else {
        return;
    };
    println!(
        "{title} sol VENOM {level:?}: respawns per craft {respawns:?}, flying laps per craft \
         {flying:?}, {ticks} ticks"
    );
    assert!(
        flying.iter().all(|&laps| laps > 0),
        "{title} sol {level:?}: a craft has no flying lap in {FIELD_TICKS} ticks: flying laps \
         {flying:?}, respawns {respawns:?}"
    );
}

macro_rules! sol_fields {
    ($($name:ident => $title:literal, $level:ident;)*) => {$(
        #[test]
        #[ignore = "needs the extracted game under data/extracted/"]
        fn $name() {
            a_field_of_seven_laps($title, oag_ai::Difficulty::$level);
        }
    )*};
}

sol_fields! {
    a_field_of_seven_laps_2048_sol_at_novice => "2048", Novice;
    a_field_of_seven_laps_2048_sol_at_skilled => "2048", Skilled;
    a_field_of_seven_laps_2048_sol_at_elite => "2048", Elite;
    a_field_of_seven_laps_2048_sol_at_ace => "2048", Ace;
    a_field_of_seven_laps_omega_sol_at_novice => "omega", Novice;
    a_field_of_seven_laps_omega_sol_at_skilled => "omega", Skilled;
    a_field_of_seven_laps_omega_sol_at_elite => "omega", Elite;
    a_field_of_seven_laps_omega_sol_at_ace => "omega", Ace;
}
