//! A craft's jump along the circuit ([`crate::transit`]): out of the world for its ticks,
//! frozen, undrawn and untargetable, then put further round at the speed asked for; the
//! refusals; the clearance probe; and, with the `extensions` feature, the same through a
//! guest's `transit` and `probe-clearance` commands and their events.

use super::extensions::grid;
use super::*;
use crate::extensions::Prepared;
use crate::transit::Distance;
use oag_gameplay::PlayerInputs;

/// The fixture grid with slot 1 put ten units down the fixture's 70-unit straight (its
/// grid slot is past the end, where the ring closes on itself), run until it has a fix,
/// then given 30 units a second down the track (it sits through the countdown).
fn moving() -> Race {
    let mut race = grid(Prepared::default());
    let ship = &mut race.sim.world.ships[1];
    ship.physics.body.position.x = 10.0;
    ship.physics.body.position.z = 0.0;
    ship.standing = oag_race::Standing::default();
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }
    assert!(race.sim.world.ships[1].standing.course_index.is_some());
    race.sim.world.ships[1].physics.body.linear_velocity =
        oag_core::math::Vec3::new(30.0, 0.0, 0.0);
    race
}

fn speed(race: &Race, slot: usize) -> f32 {
    race.sim.world.ships[slot]
        .physics
        .body
        .linear_velocity
        .length()
}

#[test]
fn a_craft_in_transit_is_out_of_the_world_then_comes_out_further_round() {
    let mut race = moving();
    let entry = race.sim.world.ships[1].physics.body.position;
    let entry_speed = speed(&race, 1);
    let entry_progress = race.sim.world.ships[1].standing.progress.unwrap();
    let length = race.sim.course.as_ref().unwrap().length();
    assert!(entry_speed > 1.0, "slot 1 is moving: {entry_speed}");
    let drawn = race.ship_model_matrices().len();
    race.begin_transit(1, 30, Distance::Units(20.0), 0.5)
        .unwrap();
    for tick in 1..30 {
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[1];
        assert!(ship.transit.is_some(), "tick {tick}");
        assert_eq!(ship.physics.body.position, entry, "frozen on tick {tick}");
        assert!(!race.ship_active(1), "not drawn");
        assert!(
            !oag_weapons::Craft::active(ship),
            "nothing hits or locks it"
        );
        assert_eq!(race.ship_model_matrices().len(), drawn - 1);
        assert!(ship.active, "the slot still holds its craft");
    }
    race.tick(&PlayerInputs::none());
    let ship = &race.sim.world.ships[1];
    assert_eq!(ship.transit, None, "out on the 30th tick");
    assert!(race.ship_active(1));
    // Twenty units further round, give or take the ring's point spacing and a tick.
    let on = (ship.standing.progress.unwrap() - entry_progress).rem_euclid(length);
    assert!(
        (15.0..25.0).contains(&on),
        "came out {on} units further round"
    );
    assert_ne!(ship.physics.body.position, entry);
    // Out at half its entry speed, then one tick of the force law.
    let out = speed(&race, 1);
    assert!(
        (out - entry_speed * 0.5).abs() < entry_speed * 0.1,
        "{out} against {entry_speed}"
    );
}

#[test]
fn a_transit_the_course_or_the_craft_cannot_take_is_refused_and_changes_nothing() {
    let mut race = moving();
    let half = race.sim.course.as_ref().unwrap().max_jump();
    for (ticks, distance, factor) in [
        (0, Distance::Units(10.0), 1.0),
        (601, Distance::Units(10.0), 1.0),
        (30, Distance::Units(half), 1.0),
        (30, Distance::Lap(0.5), 1.0),
        (30, Distance::Units(-1.0), 1.0),
        (30, Distance::Units(10.0), 2.5),
    ] {
        assert!(
            race.begin_transit(1, ticks, distance, factor).is_err(),
            "{ticks} {distance:?} {factor}"
        );
        assert_eq!(race.sim.world.ships[1].transit, None);
    }
    race.begin_transit(1, 30, Distance::Seconds(0.1), 1.0)
        .unwrap();
    assert!(
        race.begin_transit(1, 30, Distance::Units(10.0), 1.0)
            .is_err(),
        "already in one"
    );
}

/// Its triggers are held: a craft frozen out of the world banks no dwell.
#[test]
fn a_craft_in_transit_has_its_reset_triggers_held() {
    let mut race = moving();
    race.begin_transit(1, 100, Distance::Units(20.0), 1.0)
        .unwrap();
    let respawns = race.sim.respawns[1];
    for _ in 0..90 {
        race.tick(&PlayerInputs::none());
        assert!(race.reset_held(1));
        assert!(!race.armed(1, race.sim.reset_triggers.cage));
    }
    assert_eq!(race.sim.respawns[1], respawns);
    assert_eq!(race.sim.stalled_ticks[1], 0);
}

/// The fixture's straight track is 24 units wide and has no geometry: a half disc of
/// radius 8 fits, one of 13 is wider than the track.
#[test]
fn a_clearance_probe_reads_the_width_and_the_geometry() {
    let race = moving();
    let (fits, ahead) = race.clearance(1, Distance::Units(20.0), 8.0);
    assert!(fits);
    assert_eq!(ahead, 20.0);
    assert!(!race.clearance(1, Distance::Units(20.0), 13.0).0);
    assert_eq!(race.clearance(1, Distance::Lap(0.6), 8.0), (false, 0.0));
}

#[cfg(feature = "extensions")]
mod guest {
    use super::super::extensions::on::{REPLY, STATE, module, prepared};
    use super::*;

    const EVENTS: i32 = 1024 + oag_ext_abi::EVENTS_AT as i32;

    /// On its first call with slot 1 racing (past the countdown) sends it a 20-tick transit of 10 units and a clearance probe;
    /// every call marks each event kind it is sent in the blob (byte `kind`).
    fn porter() -> crate::extensions::Source {
        let mut source = module(
            "test.porter",
            &format!(
                "(local $i i32)
                 (block $done (loop $next
                   (br_if $done (i32.ge_u (local.get $i) (i32.load (i32.const {count}))))
                   (i32.store8
                     (i32.add (i32.const {STATE})
                       (i32.load8_u (i32.add (i32.const {EVENTS}) (i32.mul (local.get $i) (i32.const 16)))))
                     (i32.const 1))
                   (local.set $i (i32.add (local.get $i) (i32.const 1)))
                   (br $next)))
                 (if (i32.and (i32.eqz (i32.load8_u (i32.const {STATE})))
                              (i32.ne (i32.and (i32.load8_u (i32.const {flags})) (i32.const 4)) (i32.const 0)))
                   (then
                     (i32.store8 (i32.const {STATE}) (i32.const 1))
                     (i32.store (i32.const {REPLY}) (i32.const 2))
                     (i32.store8 (i32.const {c0}) (i32.const 27))
                     (i32.store8 (i32.add (i32.const {c0}) (i32.const 1)) (i32.const 1))
                     (i32.store16 (i32.add (i32.const {c0}) (i32.const 2)) (i32.const 20))
                     (f32.store (i32.add (i32.const {c0}) (i32.const 4)) (f32.const 10.0))
                     (i32.store (i32.add (i32.const {c0}) (i32.const 8)) (i32.const {exit}))
                     (i32.store8 (i32.const {c1}) (i32.const 29))
                     (i32.store8 (i32.add (i32.const {c1}) (i32.const 1)) (i32.const 1))
                     (f32.store (i32.add (i32.const {c1}) (i32.const 4)) (f32.const 10.0))
                     (f32.store (i32.add (i32.const {c1}) (i32.const 8)) (f32.const 4.0))))
                 i32.const 0",
                count = 1024 + oag_ext_abi::EVENT_COUNT_AT,
                flags = 1024 + oag_ext_abi::CRAFTS_AT + oag_ext_abi::CRAFT_LEN + oag_ext_abi::CRAFT_FLAGS,
                c0 = REPLY + 4,
                c1 = REPLY + 4 + 12,
                exit = 10_000_u32 << 16,
            ),
        );
        source.state_bytes = 16;
        source
    }

    #[test]
    fn a_guest_transit_takes_the_craft_out_and_says_when_it_is_back() {
        let mut race = grid(prepared(&[porter()]));
        let mut out = 0;
        for _ in 0..600 {
            race.tick(&PlayerInputs::none());
            out += u32::from(race.sim.world.ships[1].transit.is_some());
        }
        let blob = race.extension_state(0).unwrap().to_vec();
        assert_eq!(race.extension_reports()[0].faulted, None);
        assert_eq!(blob[11], 1, "transit-end was sent: {blob:?}");
        assert_eq!(blob[12], 1, "clearance was sent: {blob:?}");
        assert_eq!(race.sim.world.ships[1].transit, None);
        // Landed at tick N's call, the craft is out for the steps of N..N+19 and comes out
        // at N+20's, before its step: twenty ticks end with it out.
        assert_eq!(out, 20, "out of the world for its twenty ticks");
    }

    #[test]
    fn a_guest_transit_is_the_same_twice() {
        let (a, ha) = super::super::extensions::run(grid(prepared(&[porter()])), 600);
        let (b, hb) = super::super::extensions::run(grid(prepared(&[porter()])), 600);
        assert_eq!(ha, hb);
        assert_eq!(a.sim.world.ships, b.sim.world.ships);
    }
}
