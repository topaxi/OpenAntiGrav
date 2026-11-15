//! Wipeout HD's Plasma against the original's own bolt, read live: where it charges, when
//! it leaves, how far it has flown by age, and where a bolt aimed at a wall ends.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does
//! not ship.
//!
//! **The original side.** RPCS3 on the EU disc, Racebox Single Race on Talon's Junction,
//! Venom, Feisar, the craft at rest on its grid slot, pickup state 7 fired with the bolt's
//! own object polled through `/proc/<pid>/mem` at every game tick (charge `+0x54`, age
//! `+0x58`, position `+0xa0`): `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, "Measured
//! live". Two shots: one down the grid, one with the craft turned ten degrees so the bolt
//! meets the left wall 130 units out.
//!
//! **What is not asserted.** Past a second of flight, on the banked climb after the grid,
//! this engine's bolt bends with the road where the original's holds its heading and ends
//! on the right-hand wall at age 2.24 s; the same page, "Open", has both tracks.

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

/// The resting craft of the straight shot, and its yaw off the spline (degrees).
const STRAIGHT: ([f32; 3], f32) = ([-132.2678, -49.5985, -175.2943], 1.867);
/// The craft of the shot at the wall.
const YAWED: ([f32; 3], f32) = ([-132.2410, -49.5996, -175.3121], 12.805);

/// Where the original's bolt sat for all sixty charging ticks of each shot.
const ORIGINAL_CHARGE_POINT: [f32; 3] = [-127.6546, -49.8659, -175.5523];
const ORIGINAL_CHARGE_POINT_YAWED: [f32; 3] = [-127.7169, -49.8071, -176.3296];

/// The original's bolt: flight age in seconds and its distance from the charge point.
const ORIGINAL_FLOWN: [(f32, f32); 4] = [
    (0.0997, 6.38),
    (0.2945, 23.84),
    (0.5994, 64.91),
    (0.9924, 142.61),
];

/// Where and at what flight age the original's bolt met the wall.
const ORIGINAL_WALL: ([f32; 3], f32) = ([1.4376, -50.3203, -207.0496], 0.95);

/// The tick after the press on which the bolt is released: sixty charging ticks, the
/// press tick the first.
const RELEASE: usize = 59;

fn snap(input: &mut Input, buttons: u32) -> oag_gameplay::InputSnapshot {
    input.begin_frame(buttons);
    oag_gameplay::InputSnapshot {
        buttons: *input,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// One shot from a craft held at rest on `pose`: the bolt's position after each tick from
/// the press (`None` once it is gone) and its launch speed in km/h.
fn fly(pose: ([f32; 3], f32), ticks: usize) -> Option<(Vec<Option<Vec3>>, f32)> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        team: Some("FEISAR".to_string()),
        mode: oag_race::Mode::SingleRace,
        track: Some(r"Data\Environments\Talons_Junction\track.vex".to_string()),
        pose: Some(race::PoseRequest::SplineAligned {
            position: pose.0.into(),
            yaw: pose.1.to_radians(),
        }),
        ..race::Options::default()
    })
    .expect("loading Talon's Junction");
    let mut race = race::Race::start(loaded.setup);
    let rest = (
        race.sim.world.ships[0].physics.body.position,
        race.sim.world.ships[0].physics.body.orientation,
    );
    let hold = |race: &mut race::Race| {
        race.place_player(rest.0, rest.1);
        let body = &mut race.sim.world.ships[0].physics.body;
        body.linear_velocity = Vec3::ZERO;
        body.angular_velocity = Vec3::ZERO;
    };
    let mut input = Input::new();
    // Past the countdown, the craft kept on the captured pose as the original's rested.
    for _ in 0..600 {
        race.tick(&PlayerInputs::single(snap(&mut input, 0)));
        hold(&mut race);
    }
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Plasma);
    let mut positions = Vec::new();
    let mut launch_kmh = 0.0;
    for k in 0..ticks {
        let buttons = if k == 0 { Button::Square.bit() } else { 0 };
        race.tick(&PlayerInputs::single(snap(&mut input, buttons)));
        if k <= RELEASE {
            hold(&mut race);
        }
        let bolt = race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .find(|p| p.kind == Some(Weapon::Plasma) && p.owner == 0);
        if let Some(bolt) = bolt {
            launch_kmh = bolt.launch_speed_kmh;
        }
        positions.push(bolt.map(|p| p.position));
        if bolt.is_none() && k > RELEASE {
            break;
        }
    }
    Some((positions, launch_kmh))
}

/// The bolt's position `age` seconds after its release, between the two ticks around it.
fn at_age(positions: &[Option<Vec3>], age: f32) -> Vec3 {
    let ticks = age * 60.0;
    let (low, part) = (ticks.floor() as usize, ticks.fract());
    let a = positions[RELEASE + low].expect("the bolt is flying");
    let b = positions[RELEASE + low + 1].expect("the bolt is flying");
    a.lerp(b, part)
}

/// One shot down the grid, every claim: the bolt charges on the hull's `Ship Muzzle` node
/// (4.66 ahead of the body and 0.24 under it on Feisar, where the stand-in point was 2.23
/// ahead), stays there for sixty ticks, leaves at `launchSpeed` from a resting craft, and
/// is where the original's is through its first second.
#[test]
#[ignore = "needs a disc image"]
fn a_bolt_charges_on_the_ship_muzzle_and_flies_the_originals_first_second() {
    let Some((positions, launch_kmh)) = fly(STRAIGHT, 140) else {
        return;
    };
    let charge_point = positions[0].expect("the bolt exists on the press tick");
    let off = (charge_point - Vec3::from(ORIGINAL_CHARGE_POINT)).length();
    assert!(
        off < 0.25,
        "the bolt charges at {charge_point:?}, {off:.2} from the original's \
         {ORIGINAL_CHARGE_POINT:?}"
    );
    for (k, position) in positions.iter().enumerate().take(RELEASE + 1) {
        let position = position.expect("the charging bolt is live");
        assert!(
            (position - charge_point).length() < 1e-3,
            "tick {k}: the charging bolt moved to {position:?}"
        );
    }
    let first_move = positions[RELEASE + 1].expect("the released bolt is live");
    assert!(
        (first_move - charge_point).length() > 0.3,
        "the bolt is still on the nose a tick after its sixtieth charging tick"
    );
    assert!(
        (launch_kmh - 200.0).abs() < 0.5,
        "a resting craft's bolt left at {launch_kmh:.1} km/h, launchSpeed is 200"
    );
    for (age, expected) in ORIGINAL_FLOWN {
        let flown = (at_age(&positions, age) - charge_point).length();
        assert!(
            (flown - expected).abs() < 2.5,
            "at flight age {age:.3} s the bolt has flown {flown:.2}, the original's {expected:.2}"
        );
    }
}

/// The craft turned ten degrees: the bolt charges on the turned muzzle and ends on the
/// left wall near where and when the original's did.
#[test]
#[ignore = "needs a disc image"]
fn a_bolt_aimed_at_a_wall_ends_near_the_originals_wall_point() {
    let Some((positions, _)) = fly(YAWED, 200) else {
        return;
    };
    let charge_point = positions[0].expect("the bolt exists on the press tick");
    let off = (charge_point - Vec3::from(ORIGINAL_CHARGE_POINT_YAWED)).length();
    assert!(
        off < 0.25,
        "the bolt charges at {charge_point:?}, {off:.2} from the original's"
    );
    let last = positions
        .iter()
        .rposition(Option::is_some)
        .expect("the bolt flew");
    assert!(
        last + 1 < positions.len(),
        "the bolt never met the wall in {} ticks",
        positions.len()
    );
    let end = positions[last].expect("just found");
    let (wall, age) = ORIGINAL_WALL;
    // 12.2 as measured: this engine's bolt drifts 1.4 degrees off the craft's heading
    // on the way where the original's drifts 0.6, so it meets the wall two ticks sooner.
    let miss = (end - Vec3::from(wall)).length();
    assert!(
        miss < 15.0,
        "the bolt ended at {end:?}, {miss:.1} from the original's wall point {wall:?}"
    );
    #[expect(clippy::cast_precision_loss, reason = "a tick count under 200")]
    let ended_at = (last - RELEASE) as f32 / 60.0;
    assert!(
        (ended_at - age).abs() < 0.1,
        "the bolt ended {ended_at:.3} s after its release, the original's {age:.2} s"
    );
}
