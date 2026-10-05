//! The standing start on Talon's Junction White, from the grid to GO and the
//! first half second after it, against what Pulse PSP's own craft does.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-usa.chd`.
//!
//! The original numbers were read off PPSSPP on 2026-10-01 with
//! `scripts/psp-start-pose.py`: `RESTART RACE`, a breakpoint on
//! `Weapons_DispatchFire` once a frame, the player's rigid body read each time,
//! cross held through the countdown. Time Trial, Venom, Assegai, `16_Track`.
//! Four runs agree to the digits quoted. See `docs/physics/grid-state.md`.
//!
//! What the capture showed, and what each test pins:
//!
//! - **Through the countdown the craft holds its heading.** Yaw reads
//!   `-0.146` degrees on every one of 330 frames, to the third decimal, while
//!   pitch and roll settle on the banked start. A craft that drifts through the
//!   countdown arrives at GO pointing somewhere else: ours, before the grid
//!   state was ported, was `1.3` degrees off at GO and `1.7` a second later.
//! - **From GO it yaws at `0.29` degrees a second**, the bank-to-yaw coupling
//!   acting on a start that is banked `1.1` degrees, `0.102` degrees in the
//!   first 30 ticks. That coupling is skipped in the grid state and is the one
//!   thing that differs before and after.
//!
//! The launch boost (`craft+0x294`, `Ship_UpdateStartBoost`) that the original
//! applies on thrust held through the countdown is its own test,
//! `launch_boost_ground_truth.rs`.

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_race::state::COUNTDOWN_TICKS;
use oag_raceplay as race;

const TRACK: &str = "Data\\Environments\\16_Track\\track.vex";

/// The original's craft at GO (the first frame of thrust), read off its body.
const ORIGINAL_AT_GO: Vec3 = Vec3::new(6.081, -50.064, -195.945);
/// Its yaw there, degrees from `+X` toward `-Z`, constant through the countdown.
const ORIGINAL_YAW_DEGREES: f32 = -0.146;
/// How far its yaw moved in the 30 frames after GO, degrees (`-0.146` to `-0.248`).
const ORIGINAL_YAW_DRIFT_30: f32 = 0.102;

fn throttle_held() -> oag_gameplay::InputSnapshot {
    let mut buttons = oag_gameplay::input::Input::new();
    buttons.begin_frame(oag_gameplay::input::Button::Cross.bit());
    buttons.begin_frame(oag_gameplay::input::Button::Cross.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

fn start() -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(TRACK.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

/// Yaw about world up, in degrees from `+X` toward `-Z`, the way the capture
/// reads it (`atan2(forward.z, forward.x)`).
fn yaw_degrees(race: &race::Race) -> f32 {
    let forward = race.ship().physics.body.orientation * Vec3::NEG_Z;
    forward.z.atan2(forward.x).to_degrees()
}

/// Ticks the race with thrust held, recording yaw and position before each.
fn fly(race: &mut race::Race, ticks: u64) -> Vec<(f32, Vec3)> {
    let held = throttle_held();
    (0..ticks)
        .map(|_| {
            let row = (yaw_degrees(race), race.ship().physics.body.position);
            race.tick(&PlayerInputs::single(held));
            row
        })
        .collect()
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_craft_holds_its_heading_through_the_countdown() {
    let Some(mut race) = start() else {
        return;
    };
    let rows = fly(&mut race, COUNTDOWN_TICKS);
    // From tick 30, once the placement settle's own roll has gone: the
    // original's yaw is constant to 0.001 degrees from its first frame.
    let held = rows[30].0;
    for (tick, (yaw, _)) in rows.iter().enumerate().skip(30) {
        assert!(
            (yaw - held).abs() < 0.01,
            "tick {tick}: yaw {yaw} has left {held} during the countdown"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_craft_arrives_at_go_where_and_pointing_as_the_original_does() {
    let Some(mut race) = start() else {
        return;
    };
    let rows = fly(&mut race, COUNTDOWN_TICKS + 1);
    let (yaw, position) = rows[COUNTDOWN_TICKS as usize];
    println!("at GO: yaw {yaw:.3} position {position:?}");
    // 0.05 degrees and 0.15 units. What is left of the position is 0.1 in z:
    // the original's craft is placed 2 units above the floor and rises to its
    // rest height along its own up axis, which on this banked start moves it
    // 0.04 units in z, and it then slides 0.03 down the slope through the
    // countdown. Ours is placed at its rest height already.
    assert!(
        (yaw - ORIGINAL_YAW_DEGREES).abs() < 0.05,
        "yaw {yaw} against the original's {ORIGINAL_YAW_DEGREES} at GO"
    );
    assert!(
        (position - ORIGINAL_AT_GO).length() < 0.15,
        "position {position:?} against the original's {ORIGINAL_AT_GO:?} at GO"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_craft_starts_to_yaw_at_go_at_the_originals_rate() {
    let Some(mut race) = start() else {
        return;
    };
    let rows = fly(&mut race, COUNTDOWN_TICKS + 31);
    let drift = (rows[COUNTDOWN_TICKS as usize + 30].0 - rows[COUNTDOWN_TICKS as usize].0).abs();
    println!("yaw drift over the first 30 ticks: {drift:.4} degrees");
    assert!(
        (drift - ORIGINAL_YAW_DRIFT_30).abs() < 0.25 * ORIGINAL_YAW_DRIFT_30,
        "yaw moved {drift} degrees in 30 ticks, the original {ORIGINAL_YAW_DRIFT_30}"
    );
}

/// The eight craft of a Single Race at placement, read off the racer table
/// (`scripts/psp-grid-pose.py`): `x` and the heading, degrees from `+X` toward
/// `-Z`, of each slot from slot 1 to slot 8. The heading is `forward.z` read
/// at the third decimal, which is `0.06` degrees a step.
const ORIGINAL_GRID: [(f32, f32); 8] = [
    (6.145, -0.1432),
    (-13.561, -0.1833),
    (-33.427, -0.2235),
    (-53.142, -0.2521),
    (-73.032, -0.2808),
    (-92.745, -0.3037),
    (-112.624, -0.3266),
    (-132.291, -0.3495),
];

/// Every slot's heading against the original's, within 0.08 degrees (**chosen, not measured**;
/// the table's own `forward.z` quantisation is 0.03). Read 0.045 at worst while a slot took
/// the sample tangent's frame and **0.002 now that it takes the edge chords'**
/// (`oag_gameplay::grid_walk`, `grid_walk_ground_truth.rs` pins it at 0.001 against the
/// full-precision axes). The name is the 2026-10-01 reading and is kept so the pages that cite
/// it still resolve.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_grid_slot_points_along_the_tracks_own_frame_as_the_original_does() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(TRACK.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let race = race::Race::start(loaded.setup);
    let mut worst = 0.0f32;
    for (x, yaw_original) in ORIGINAL_GRID {
        // The slot is found by where it stands: ours puts the player on slot 8
        // and the opponents on 1 to 7, which is the original's order too, but
        // matching by position does not depend on that.
        let ship = (0..8)
            .map(|slot| &race.sim.world.ships[slot].physics.body)
            .min_by(|a, b| {
                (a.position.x - x)
                    .abs()
                    .total_cmp(&(b.position.x - x).abs())
            })
            .expect("eight craft");
        assert!(
            (ship.position.x - x).abs() < 3.0,
            "no craft near the original's x {x}"
        );
        let forward = ship.orientation * Vec3::NEG_Z;
        let yaw = forward.z.atan2(forward.x).to_degrees();
        worst = worst.max((yaw - yaw_original).abs());
        assert!(
            (yaw - yaw_original).abs() < 0.08,
            "slot at x {x}: yaw {yaw}, the original's {yaw_original}"
        );
    }
    println!("worst slot heading error: {worst:.3} degrees");
}
