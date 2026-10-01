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
//! **What is not pinned here, and is open**: the original launches with a
//! multiplier of `1.4` for a frame and then `1.2` for 58 frames on thrust held
//! through the countdown (`craft+0x294`, `Ship_UpdateStartBoost`), which this
//! engine does not implement, so ours is slower off the line: 106.6 units/s at
//! GO + 120 ticks against ours 100.5.

use oag_core::math::Vec3;
use oag_game::race;
use oag_gameplay::PlayerInputs;
use oag_race::state::COUNTDOWN_TICKS;

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
    // 0.15 degrees and 0.25 units: what is left is the per-slot heading
    // (`grid.md`: the original builds each slot's frame from the track's sample,
    // ours takes the node's) and the countdown's slow slide down the slope.
    assert!(
        (yaw - ORIGINAL_YAW_DEGREES).abs() < 0.15,
        "yaw {yaw} against the original's {ORIGINAL_YAW_DEGREES} at GO"
    );
    assert!(
        (position - ORIGINAL_AT_GO).length() < 0.25,
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
