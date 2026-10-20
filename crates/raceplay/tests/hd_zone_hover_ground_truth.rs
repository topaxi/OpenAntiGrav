//! Wipeout HD's Zone craft rides about three units above the floor, from the grid on.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay hd_zone_hover_ground_truth:: --run-ignored all
//! ```
//!
//! Two Zone-only laws set the height (`docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md`
//! and `hover-target.md`): `Craft_HoverFourPoint`'s trailing ray pulls the centre half the way to
//! `3.0` above the floor every frame, and `Craft_Update` skips the grid clamp on modes 6, 13 and
//! 14, so the target is the race's `4.125` from the flyby on. Measured on RPCS3, a cold boot to
//! mode 6 on Vineta K: the mean probe height reads `2.08` through flyby and countdown and a
//! median `2.12` (tenth to ninetieth percentile `2.02`-`2.21`) over 50 s in flight, so with the
//! probes `1.125` below the centre, the centre is `3.20` above the floor on the grid and `3.24`
//! in flight. Without the ray the racing rig rests at `4.0`; with the grid clamp, at about `2.2`.

use oag_gameplay::PlayerInputs;
use oag_physics::collide::{Ray, Raycaster};
use oag_raceplay as race;
use oag_raceplay::Race;

const GO: u64 = oag_race::COUNTDOWN_TICKS;

fn zone() -> Option<Race> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Zone,
        track: Some(oag_hd::psn::DEFAULT_TRACK.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(Race::start(loaded.setup))
}

/// The craft's centre above the surface straight below it, along that surface's normal.
fn height(race: &Race) -> f32 {
    let body = &race.ship().physics.body;
    let down = -body.up();
    let hit = race
        .collision()
        .raycast(Ray::new(body.position, down, 20.0), None, false)
        .expect("a floor under the craft");
    (body.position - hit.point).dot(hit.normal)
}

fn run_to(race: &mut Race, tick: u64) {
    while race.sim.world.tick < tick {
        race.tick(&PlayerInputs::none());
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_zone_craft_waits_on_the_grid_at_the_rays_height_not_the_lowered_hover() {
    let Some(mut race) = zone() else { return };
    run_to(&mut race, GO - 2);
    let grid = height(&race);
    assert!((3.1..3.3).contains(&grid), "end of the countdown {grid}");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_zone_craft_flies_at_the_rays_height() {
    let Some(mut race) = zone() else { return };
    race.set_autopilot(true);
    run_to(&mut race, GO + 120);
    let mut heights = Vec::new();
    for _ in 0..600 {
        race.tick(&PlayerInputs::none());
        heights.push(height(&race));
    }
    heights.sort_by(f32::total_cmp);
    let median = heights[heights.len() / 2];
    assert!(
        (3.1..3.4).contains(&median),
        "median {median} of {heights:?}"
    );
}
