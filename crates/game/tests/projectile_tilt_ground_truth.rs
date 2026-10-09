//! Every projectile that rides the floor keeps riding it on a tilted track, out
//! of a real disc, on Pulse and on Wipeout HD.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run it with `just test-data`.
//!
//! A maintainer report, after the Cannon was fixed on tilted tracks
//! (`cannon_tilt_ground_truth.rs`): "a similar issue might be going on with
//! rockets". The Cannon's fault was a ridden normal seeded with world up, so its
//! first floor probe went down the *world* and landed on a different deck. The
//! Rocket, Missile, Plasma and Shuriken all carry a normal and probe along it, so
//! each is judged here by what that normal says about the floor under it.
//!
//! The craft is flown by the autopilot and the weapon is fired through the
//! pad every 30 ticks (the Cannon test's staging). Its shield is restored every
//! tick (a fixture choice, 2026-10-09): the race is armed, and whether an
//! opponent's beam destroys the firer depends on every line in the field, which
//! any handling change moves. HD's four-point hover once left the firer drained
//! on the first lap and parked, and the count below measured that, not a floor. On every tick a projectile is
//! in the air the test casts the original's own probe, the weapon's own reach (`6.0` for the Rocket, `12.0` for the
//! rest) along `-surface` from the projectile, and asks two things of the floor it finds:
//!
//! - the carried normal agrees with the floor's (angle, degrees), and
//! - the projectile sits near [`RIDE_HEIGHT`] above it (the Plasma is exempt on
//!   the tick it is released: it leaves the nose at the craft's hover height and
//!   settles a tick later).
//!
//! A projectile born riding world up fails both within a few ticks of a bank
//! (measured 2026-10-08 with the seeds forced back to `Vec3::Y`: worst angle
//! 50.9 / 89.7 / 39.7 degrees for the Rocket / Missile / Plasma, and a Shuriken
//! 8.4 units off its ride height). A tick with no floor within the probe is a
//! fall and is not judged.

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_physics::{Ray, Raycaster};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

const PULSE: (&str, &str) = (
    "data/images/pulse-psp-eu.chd",
    r"Data\Environments\03_Track\track.vex",
);
const HD: (&str, &str) = (
    "data/images/hdfury-ps3-eu-dec.iso",
    "/data/environments/01_vineta_k/track.vex",
);

/// How far above the floor a projectile rides, and how far off it may be.
const RIDE_HEIGHT: f32 = oag_weapons::projectile::RIDE_HEIGHT;
const HEIGHT_TOLERANCE: f32 = 2.0;
const ANGLE_TOLERANCE_DEGREES: f32 = 20.0;

fn snapshot(bits: u32, previous: Option<&Input>) -> oag_gameplay::InputSnapshot {
    let mut buttons = previous.copied().unwrap_or_else(Input::new);
    buttons.begin_frame(bits);
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

#[derive(Default)]
struct Ride {
    shots: u32,
    tilted_shots: u32,
    judged_ticks: u32,
    worst_height_error: f32,
    worst_angle: f32,
}

fn start(image: &str, track: &str) -> Option<race::Race> {
    let image = oag_testdata::image(image)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(track.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

/// Fires `weapon` from the autopilot's craft every 30 ticks for `ticks` ticks,
/// judging each tick it is in the air.
fn ride(title: (&str, &str), weapon: Weapon, ticks: u32) -> Option<Ride> {
    let mut race = start(title.0, title.1)?;
    let mut input: Option<Input> = None;
    let mut next_shot = 200;
    let mut out = Ride::default();
    let shield = race.sim.world.ships[0].physics.shield;
    for tick in 0..ticks {
        race.set_autopilot(true);
        race.sim.world.ships[0].physics.shield = shield;
        if tick < next_shot {
            let snap = snapshot(Button::Cross.bit(), input.as_ref());
            input = Some(snap.buttons);
            race.tick(&PlayerInputs::single(snap));
            continue;
        }
        out.shots += 1;
        if race.sim.world.ships[0].physics.body.up().y < 0.95 {
            out.tilted_shots += 1;
        }
        race.sim.world.projectiles.clear();
        race.sim.world.ships[0].pickup.weapon = Some(weapon);
        // A Plasma bolt winds up for a second on the nose before it flies.
        let span = if weapon == Weapon::Plasma { 75 } else { 14 };
        next_shot = tick + 30;
        let mut released = false;
        for k in 0..span {
            let bits = if k == 0 {
                Button::Cross.bit() | Button::Square.bit()
            } else {
                Button::Cross.bit()
            };
            let snap = snapshot(bits, input.as_ref());
            input = Some(snap.buttons);
            race.tick(&PlayerInputs::single(snap));
            let Some(shot) = race
                .sim
                .world
                .projectiles
                .slots
                .iter()
                .find(|p| p.kind == Some(weapon) && p.owner == 0)
                .copied()
            else {
                break;
            };
            if shot.charge > 0.0 {
                continue;
            }
            let first_flying_tick = !std::mem::replace(&mut released, true);
            let reach = if weapon == Weapon::Rocket {
                oag_weapons::projectile::SURFACE_PROBE_LENGTH
            } else {
                oag_weapons::projectile::missile::SURFACE_PROBE_LENGTH
            };
            let Some(floor) = race.collision().raycast(
                Ray::new(shot.position, -shot.surface, reach),
                None,
                false,
            ) else {
                continue;
            };
            // A wall under the probe is the weapon's own ending or bounce, not a
            // floor to ride.
            if !floor.surface.is_hoverable() {
                continue;
            }
            out.judged_ticks += 1;
            out.worst_angle = out.worst_angle.max(
                floor
                    .normal
                    .dot(shot.surface)
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees(),
            );
            if !(weapon == Weapon::Plasma && first_flying_tick) {
                out.worst_height_error = out
                    .worst_height_error
                    .max((floor.distance - RIDE_HEIGHT).abs());
            }
        }
    }
    Some(out)
}

fn assert_rides(title: (&str, &str), weapon: Weapon, ticks: u32) {
    let Some(ride) = ride(title, weapon, ticks) else {
        return;
    };
    println!(
        "{weapon:?} on {}: {} shots, {} tilted, {} judged ticks, worst height error {:.2}, worst angle {:.1}",
        title.1,
        ride.shots,
        ride.tilted_shots,
        ride.judged_ticks,
        ride.worst_height_error,
        ride.worst_angle
    );
    assert!(
        ride.tilted_shots >= 20,
        "only {} of {} shots left a tilted craft",
        ride.tilted_shots,
        ride.shots
    );
    assert!(
        ride.judged_ticks >= 100,
        "{} ticks judged",
        ride.judged_ticks
    );
    assert!(
        ride.worst_angle < ANGLE_TOLERANCE_DEGREES,
        "a {weapon:?} carried a normal {:.1} degrees off the floor under it",
        ride.worst_angle
    );
    assert!(
        ride.worst_height_error < HEIGHT_TOLERANCE,
        "a {weapon:?} rode {:.2} units off its ride height",
        ride.worst_height_error
    );
}

macro_rules! rides_laid {
    ($($name:ident: $title:expr, $weapon:expr, $pose:expr;)*) => {$(
        #[test]
        #[ignore = "needs a disc image in data/images/"]
        fn $name() {
            assert_laid_on_a_bank($title, $weapon, $pose);
        }
    )*};
}

macro_rules! rides {
    ($($name:ident: $title:expr, $weapon:expr, $ticks:expr;)*) => {$(
        #[test]
        #[ignore = "needs a disc image in data/images/"]
        fn $name() {
            assert_rides($title, $weapon, $ticks);
        }
    )*};
}

rides! {
    pulse_rocket_rides_the_floor_on_a_bank: PULSE, Weapon::Rocket, 9_000;
    pulse_missile_rides_the_floor_on_a_bank: PULSE, Weapon::Missile, 9_000;
    pulse_plasma_rides_the_floor_on_a_bank: PULSE, Weapon::Plasma, 9_000;
    pulse_shuriken_rides_the_floor_on_a_bank: PULSE, Weapon::Shuriken, 9_000;
    hd_rocket_rides_the_floor_on_a_bank: HD, Weapon::Rocket, 9_000;
    hd_missile_rides_the_floor_on_a_bank: HD, Weapon::Missile, 9_000;
    hd_plasma_rides_the_floor_on_a_bank: HD, Weapon::Plasma, 9_000;
    hd_shuriken_rides_the_floor_on_a_bank: HD, Weapon::Shuriken, 9_000;
}

/// A Mine or Bomb is laid where the craft is, wherever it is tilted, and the
/// drawn pose has the craft's up where the title poses it from the craft.
///
/// Pulse's Mine spins on its own axis (`Mine_PoseNode`, measured), so its up
/// is judged on HD, which keeps the frozen craft pose; Pulse's Bomb is posed once
/// from the craft's up (`Bomb_Init`, measured), HD's keeps the frozen craft pose.
fn assert_laid_on_a_bank(title: (&str, &str), weapon: Weapon, judge_pose: bool) {
    let Some(mut race) = start(title.0, title.1) else {
        return;
    };
    let mut input: Option<Input> = None;
    let mut next_drop = 200;
    let (mut tilted, mut worst_offset, mut worst_pose) = (0, 0.0_f32, 1.0_f32);
    for tick in 0..9_000_u32 {
        // Pressed and released on alternate ticks: a held button is one edge.
        let dropping = tick >= next_drop && (tick - next_drop) % 2 == 0;
        race.set_autopilot(true);
        // Anything in the pool after the tick was laid on it.
        race.sim.world.projectiles.clear();
        if tick >= next_drop {
            race.sim.world.ships[0].pickup.weapon = Some(weapon);
        }
        let bits = if dropping {
            Button::Cross.bit() | Button::Square.bit()
        } else {
            Button::Cross.bit()
        };
        let snap = snapshot(bits, input.as_ref());
        input = Some(snap.buttons);
        let before = race.sim.world.ships[0].physics;
        race.tick(&PlayerInputs::single(snap));
        let laid: Vec<_> = race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .filter(|p| p.kind == Some(weapon) && p.owner == 0)
            .copied()
            .collect();
        if laid.is_empty() {
            continue;
        }
        next_drop = tick + 20;
        let up = before.body.up();
        if up.y >= 0.95 {
            continue;
        }
        for laid in laid {
            tilted += 1;
            worst_offset = worst_offset.max((laid.position - before.body.position).length());
        }
        if judge_pose {
            let matrices = match weapon {
                Weapon::Mine => race.mine_model_matrices(),
                _ => race.bomb_model_matrices(),
            };
            let pose = matrices
                .iter()
                .map(|m| m.y_axis.truncate().normalize().dot(up))
                .fold(f32::MAX, f32::min);
            worst_pose = worst_pose.min(pose);
        }
    }
    println!(
        "{weapon:?} on {}: {tilted} laid on a tilt, worst offset {worst_offset}, worst pose.up {worst_pose}",
        title.1
    );
    assert!(tilted >= 20, "only {tilted} laid on a tilt");
    assert!(
        worst_offset < 1e-3,
        "laid {worst_offset} units from the craft"
    );
    assert!(
        worst_pose > 0.99,
        "drawn with up {worst_pose} off the craft's"
    );
}

rides_laid! {
    pulse_mine_is_laid_where_the_banked_craft_is: PULSE, Weapon::Mine, false;
    pulse_bomb_is_laid_where_the_banked_craft_is_and_posed_from_its_up: PULSE, Weapon::Bomb, true;
    hd_mine_is_laid_where_the_banked_craft_is_and_posed_from_its_up: HD, Weapon::Mine, true;
    hd_bomb_is_laid_where_the_banked_craft_is_and_posed_from_its_up: HD, Weapon::Bomb, true;
}
