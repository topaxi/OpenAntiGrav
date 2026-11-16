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
