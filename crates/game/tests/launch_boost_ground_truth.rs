//! The launch boost against what Pulse PSP's own craft does off the line.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-usa.chd`.
//!
//! Five launches were read off PPSSPP on 2026-10-01 with
//! `scripts/psp-launch-boost.py` (`RESTART RACE`, a breakpoint on
//! `Weapons_DispatchFire` once a frame, the craft's multiplier, grade, clock and
//! rigid body read each time). Time Trial, Venom, Assegai, `16_Track`. Thrust was
//! held through the countdown, or first pressed 10, 24, 30 and 80 frames after
//! the craft is released; the capture names the frame the thrust first landed
//! (`craft+0x2d5`) as the edge, and every figure below is the craft's speed along
//! its own forward axis that many frames after it. See `docs/physics/launch-boost.md`.

use oag_game::race;
use oag_gameplay::PlayerInputs;
use oag_race::state::COUNTDOWN_TICKS;

const TRACK: &str = "Data\\Environments\\16_Track\\track.vex";

/// One captured launch: the frame, counted from the release frame, that thrust
/// first landed on, and the original's forward speed `offset` frames after it.
struct Launch {
    name: &'static str,
    edge: u64,
    speeds: &'static [(u64, f32)],
}

const LAUNCHES: [Launch; 5] = [
    Launch {
        name: "held through the countdown (stall, then the multiplier is dropped)",
        edge: 1,
        speeds: &[(20, 16.7227), (30, 26.9779), (60, 65.7823), (90, 90.7128)],
    },
    Launch {
        name: "first thrust 11 frames in (the perfect window)",
        edge: 12,
        speeds: &[(20, 25.5863), (30, 43.3182), (60, 88.3981), (120, 115.0564)],
    },
    Launch {
        name: "first thrust 25 frames in (the stall window after it)",
        edge: 26,
        speeds: &[(20, 16.7062), (30, 26.9680), (60, 59.5063), (90, 86.4195)],
    },
    Launch {
        name: "first thrust 31 frames in (after the stall window)",
        edge: 32,
        speeds: &[(20, 20.0983), (30, 32.2298), (60, 64.3404), (90, 89.7790)],
    },
    Launch {
        name: "first thrust 81 frames in (after the whole window)",
        edge: 82,
        speeds: &[(20, 13.1898), (30, 21.0137)],
    },
];

fn held(on: bool) -> oag_gameplay::InputSnapshot {
    let mut buttons = oag_gameplay::input::Input::new();
    if on {
        buttons.begin_frame(oag_gameplay::input::Button::Cross.bit());
        buttons.begin_frame(oag_gameplay::input::Button::Cross.bit());
    }
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

fn start(track: &str) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(track.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

/// Forward speed and position after each tick from the release frame (tick
/// `COUNTDOWN_TICKS - 1`) on, with thrust first held at the release frame plus
/// `edge`.
fn fly(track: &str, edge: u64, frames: u64) -> Vec<(f32, oag_core::math::Vec3)> {
    let mut race = start(track).expect("an image");
    let release = COUNTDOWN_TICKS - 1;
    for _ in 0..release {
        race.tick(&PlayerInputs::single(held(false)));
    }
    (0..frames)
        .map(|frame| {
            race.tick(&PlayerInputs::single(held(frame >= edge)));
            let body = &race.ship().physics.body;
            (
                body.linear_velocity
                    .dot(body.orientation * oag_core::math::Vec3::NEG_Z),
                body.position,
            )
        })
        .collect()
}

/// How far ours may be from the original, as a fraction of its speed.
///
/// The original's step is `1/59.94` s and ours is `1/60`, a `0.1 %` difference in
/// every per-frame increment, and its craft rests `0.1` units off ours in `z`
/// on the grid; the worst figure below is `0.65 %`. Without the boost the same
/// comparison is `21 %` off at 20 frames (held through) and the test fails.
const TOLERANCE: f32 = 0.01;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn each_launch_matches_the_original_off_the_line() {
    if oag_testdata::image("data/images/pulse-psp-usa.chd").is_none() {
        return;
    }
    for launch in &LAUNCHES {
        let last = launch.speeds.iter().map(|&(o, _)| o).max().unwrap_or(0);
        let ours: Vec<f32> = fly(TRACK, launch.edge, launch.edge + last + 2)
            .into_iter()
            .map(|(speed, _)| speed)
            .collect();
        for &(offset, original) in launch.speeds {
            // The capture's row is read after the frame's integration, which is
            // the speed after tick `edge + offset`.
            let at = (launch.edge + offset) as usize;
            let off = (ours[at] - original).abs() / original;
            println!(
                "{}: edge + {offset}: ours {:.3}, original {original}, {:.2} %",
                launch.name,
                ours[at],
                off * 100.0
            );
            assert!(
                off < TOLERANCE,
                "{}: {offset} frames after the first thrust ours is {:.3}, the original {original}",
                launch.name,
                ours[at]
            );
        }
    }
}

/// Thrust held through the countdown on two circuits: the original's speed and
/// the distance it has covered from the first thrust frame, 20, 60 and 120
/// frames on. `(offset, speed, distance)`.
///
/// The distance is the length of the displacement rather than a coordinate, so
/// the same table serves a start that runs along `+X` and one along `+Z`. This is
/// the comparison the previous lane's report called the largest term left: ours
/// was 18 units behind at 120 frames before the boost.
/// `(offset, speed, distance)` rows for one circuit.
type HeldRows = [(u64, f32, f32); 3];

const HELD: [(&str, HeldRows); 2] = [
    (
        "Data\\Environments\\16_Track\\track.vex",
        [
            (20, 16.721, 2.719),
            (60, 65.777, 29.217),
            (120, 106.637, 118.449),
        ],
    ),
    (
        "Data\\Environments\\01_Track\\track.vex",
        [
            (20, 16.749, 2.723),
            (60, 65.835, 29.260),
            (120, 106.450, 118.376),
        ],
    ),
];

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_held_through_launch_covers_the_originals_ground_on_two_circuits() {
    if oag_testdata::image("data/images/pulse-psp-usa.chd").is_none() {
        return;
    }
    for (track, rows) in HELD {
        let ours = fly(track, 1, 125);
        let origin = ours[1].1;
        for (offset, speed, distance) in rows {
            let (our_speed, position) = ours[(1 + offset) as usize];
            let our_distance = (position - origin).length();
            println!(
                "{track}: {offset} frames on: speed {our_speed:.3} against {speed}, \
                 distance {our_distance:.3} against {distance}"
            );
            assert!(
                (our_speed - speed).abs() / speed < TOLERANCE,
                "{track}: speed {offset} frames on is {our_speed}, the original {speed}"
            );
            assert!(
                (our_distance - distance).abs() / distance < 2.0 * TOLERANCE,
                "{track}: {offset} frames on ours has covered {our_distance}, the original {distance}"
            );
        }
    }
}

/// The boost is applied on Pulse PSP, where it was measured, and not by extension
/// to the discs that author the same `<StartBoost>`: Pulse PS2's and Wipeout HD's
/// files carry it too, and neither was watched.
#[test]
#[ignore = "needs disc images in data/images/"]
fn only_pulse_psp_gets_the_launch_boost() {
    for (image, expected) in [
        ("data/images/pulse-psp-usa.chd", true),
        ("data/images/pulse-psp-eu.chd", true),
        ("data/images/pulse-ps2-eu.chd", false),
        ("data/images/hdfury-ps3-eu-dec.iso", false),
    ] {
        let Some(path) = oag_testdata::image(image) else {
            continue;
        };
        let loaded = race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            ..race::Options::default()
        })
        .expect("loading the race");
        assert_eq!(
            loaded.setup.start_boost.is_some(),
            expected,
            "{image}: the launch boost's presence"
        );
    }
}
