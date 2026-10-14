//! The lock-on as a simulation quantity: it takes on the ticks the reticle
//! always took it on, reads no camera, is hashed, and gates every person-flown
//! slot's shot alike. See [`crate::lock`].

use super::*;
use oag_gameplay::PlayerInputs;
use oag_race::sight;
use oag_tables::weapons::Weapon;

/// A race with slot 1 parked `distance` ahead of slot 0 and `side` to its right,
/// slot 0 holding `weapon` under `pulse`'s reticle dialect or Wipeout HD's.
fn parked_race(weapon: Weapon, pulse: bool, distance: f32, side: f32) -> (Race, Vec3) {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(match weapon {
        Weapon::LeachBeam => one_leach_beam_table(),
        _ => one_missile_table(),
    });
    if pulse {
        race.set_sight_dialect(oag_pulse::hud::ART.sights);
    } else {
        race.set_sight_dialect(oag_hd::hud::ART.sights);
    }
    let body = race.sim.world.ships[0].physics.body;
    let at = body.position + body.forward() * distance + body.right() * side;
    race.sim.world.ships[0].pickup.weapon = Some(weapon);
    (race, at)
}

/// One tick with slot 1 held at `at`.
fn tick_parked(race: &mut Race, at: Vec3) {
    race.sim.world.ships[1].physics.body.position = at;
    race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
    race.tick(&PlayerInputs::none());
}

/// The tick the simulation's lock first reads taken, for 200 ticks.
fn first_lock(weapon: Weapon, pulse: bool, distance: f32, side: f32) -> Option<usize> {
    let (mut race, at) = parked_race(weapon, pulse, distance, side);
    (0..200).find(|_| {
        tick_parked(&mut race, at);
        race.lock_of(0).locked()
    })
}

/// The simulation's lock takes on the ticks the camera-fed reticle took it on
/// before the lock moved, measured on the unchanged tree: the Missile's `0.8` s
/// hold (tick 48), the Pulse LeachBeam's closing (tick 20), and Wipeout HD's
/// LeachBeam on the Missile's law (tick 48) - dead ahead and off to a side.
#[test]
fn the_sim_lock_takes_on_the_ticks_the_reticle_always_did() {
    for (weapon, pulse, expected) in [
        (Weapon::Missile, true, 48),
        (Weapon::LeachBeam, true, 20),
        (Weapon::LeachBeam, false, 48),
    ] {
        for (distance, side) in [(60.0, 0.0), (60.0, 15.0)] {
            assert_eq!(
                first_lock(weapon, pulse, distance, side),
                Some(expected),
                "{weapon:?} (pulse dialect {pulse}) at {distance} ahead, {side} aside"
            );
        }
    }
}

/// The picture's reticle follows the simulation's lock on the very tick, in
/// every state it passes through.
#[test]
fn the_reticle_shows_the_lock_the_simulation_took() {
    for (weapon, pulse) in [(Weapon::Missile, true), (Weapon::LeachBeam, true)] {
        let (mut race, at) = parked_race(weapon, pulse, 60.0, 0.0);
        let mut taken = false;
        for tick in 0..120 {
            tick_parked(&mut race, at);
            assert_eq!(
                race.sight().locked(),
                race.lock_of(0).locked(),
                "{weapon:?}: the reticle disagrees with the lock on tick {tick}"
            );
            taken |= race.lock_of(0).locked();
        }
        assert!(taken, "{weapon:?} never locked");
    }
}

/// The lock reads no camera and no reticle: a camera pointed straight away from
/// the target, the widest field of view and a reticle that has never seen it
/// change nothing about when the simulation locks, nor about the state hash.
///
/// The old primary lock was the reticle's own arrival, so a camera looking away
/// put the target behind it and the lock never formed.
#[test]
fn the_lock_does_not_read_the_camera() {
    let (mut plain, at) = parked_race(Weapon::Missile, true, 60.0, 0.0);
    let (mut turned, _) = parked_race(Weapon::Missile, true, 60.0, 0.0);
    let body = turned.sim.world.ships[0].physics.body;
    turned.view.camera_override = Some(crate::options::CameraOverride {
        eye: body.position - body.forward() * 40.0,
        orientation: body.orientation * Quat::from_rotation_y(std::f32::consts::PI),
        fov_deg: Some(120.0),
    });
    turned.set_sight_fov(oag_display::display::Fov::default());
    let mut locked_at = None;
    for tick in 0..100 {
        tick_parked(&mut plain, at);
        tick_parked(&mut turned, at);
        assert_eq!(
            plain.sim.state_hash(),
            turned.sim.state_hash(),
            "the camera reached the state hash on tick {tick}"
        );
        if locked_at.is_none() && turned.lock_of(0).locked() {
            locked_at = Some(tick);
        }
    }
    assert_eq!(locked_at, Some(48), "the lock moved with the camera");
    assert!(
        !turned.sight().locked() || turned.lock_of(0).locked(),
        "the reticle cannot be locked on a lock the simulation does not hold"
    );
}

/// The lock is in the state hash while its slot holds a weapon that locks, so a
/// client and a server that disagree about a lock disagree about the hash - and
/// is left out for a weapon that does not, so no other race's hash moved.
#[test]
fn the_lock_is_hashed_while_a_lock_weapon_is_held() {
    let nudge = |race: &mut Race| {
        let projected = Some(sight::Projected {
            screen: [100.0, 100.0],
            distance: 50.0,
        });
        race.sim.locks[0].update(0.3, projected);
    };
    for (weapon, hashed) in [
        (Weapon::Missile, true),
        (Weapon::LeachBeam, true),
        (Weapon::Rocket, false),
    ] {
        let (mut race, _) = parked_race(weapon, true, 60.0, 0.0);
        race.tick(&PlayerInputs::none());
        let before = race.sim.state_hash();
        nudge(&mut race);
        assert_eq!(
            race.sim.state_hash() != before,
            hashed,
            "{weapon:?}: the lock's presence in the hash is wrong"
        );
    }
}

/// A second person's shot needs the lock as the primary's does: pressed before
/// their own lock has closed the beam fires unlocked, and after it, locked.
#[test]
fn a_second_persons_early_leach_press_fires_unlocked() {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(one_leach_beam_table());
    race.set_sight_dialect(oag_pulse::hud::ART.sights);
    race.sim.world.controllers[1] = oag_gameplay::Controller::Local;
    let forward = race.sim.world.ships[0].physics.body.forward();
    let rival = race.sim.world.ships[0].physics.body.position;
    let shooter = rival - forward * 60.0;
    race.sim.world.ships[1].physics.body.orientation =
        race.sim.world.ships[0].physics.body.orientation;
    race.sim.world.ships[1].pickup.weapon = Some(Weapon::LeachBeam);
    let mut buttons = Buttons::new();
    buttons.tick(0);
    for tick in 0..=5 {
        race.sim.world.ships[0].physics.body.position = rival;
        race.sim.world.ships[0].physics.body.linear_velocity = Vec3::ZERO;
        race.sim.world.ships[1].physics.body.position = shooter;
        race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
        let mut inputs = PlayerInputs::none();
        inputs.set(1, buttons.tick(if tick == 5 { SQUARE } else { 0 }));
        race.tick(&inputs);
    }
    let beam = race.sim.world.leach_beam.expect("slot 1's press must fire");
    assert_eq!(beam.owner, 1);
    assert_eq!(
        beam.kind,
        oag_weapons::projectile::leach_beam::Kind::Unlocked,
        "a second pad's shot took a target before its own lock had closed"
    );
}

/// A quiet replay holds the hash with no exception for the lock: the Missile is
/// handed over at the start of the window, the hold runs entirely under
/// `tick_sim_only` - which advances no camera and no reticle - and the whole
/// simulation, the lock included, matches the normal path on every tick, for
/// both lock weapons.
#[test]
fn a_quiet_replay_locks_exactly_as_a_normal_tick_does() {
    for (weapon, pulse) in [(Weapon::Missile, true), (Weapon::LeachBeam, true)] {
        let (mut normal, at) = parked_race(weapon, pulse, 60.0, 0.0);
        let (mut quiet, _) = parked_race(weapon, pulse, 60.0, 0.0);
        let mut locked_at = None;
        for tick in 0..100 {
            for race in [&mut normal, &mut quiet] {
                race.sim.world.ships[1].physics.body.position = at;
                race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
            }
            normal.tick(&PlayerInputs::none());
            quiet.tick_sim_only(&PlayerInputs::none());
            assert_eq!(
                normal.sim.state_hash(),
                quiet.sim.state_hash(),
                "{weapon:?}: the quiet path left the normal one on tick {tick}"
            );
            if locked_at.is_none() && quiet.lock_of(0).locked() {
                locked_at = Some(tick);
            }
        }
        assert!(
            locked_at.is_some(),
            "{weapon:?}: the lock never formed under the quiet path"
        );
        assert!(
            !quiet.sight().locked() && !quiet.sight().visible(),
            "{weapon:?}: a quiet tick advanced the picture's reticle"
        );
        normal.drain_announcements();
        quiet.drain_announcements();
        normal.drain_class_announcements();
        quiet.drain_class_announcements();
        assert_eq!(
            format!("{:?}", normal.sim),
            format!("{:?}", quiet.sim),
            "{weapon:?}: the sims differ outside the hash"
        );
    }
}
