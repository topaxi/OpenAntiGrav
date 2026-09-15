//! What [`super`] is asserted to do. Its own file for the 200-line rule.

use super::*;
use oag_tables::weapons::{DisruptorEffect, DisruptorEffectKind as Kind};

/// A table with every rolled effect authored at an invented `time`.
fn stats() -> DisruptorStats {
    let mut effects = [None; 10];
    for kind in Kind::ROLLED {
        effects[kind as usize] = Some(DisruptorEffect {
            time: 3.0,
            amount: None,
            speed_percent: None,
        });
    }
    DisruptorStats {
        absorb: 1.0,
        speed: 300.0,
        effects,
    }
}

fn full_controls() -> ShipControls {
    ShipControls {
        steer_x: 0.5,
        steer_y: 0.0,
        thrust: 1.0,
        airbrake_left: 0.7,
        airbrake_right: 0.4,
        ..ShipControls::default()
    }
}

fn disrupted(kind: Kind, timer: f32) -> Disruption {
    Disruption {
        kind: Some(kind),
        timer,
    }
}

#[test]
fn a_stall_zeroes_thrust_for_a_human_and_not_for_an_ai() {
    let d = disrupted(Kind::Stall, 2.0);
    assert_eq!(d.filter(full_controls(), false).thrust, 0.0);
    assert_eq!(d.filter(full_controls(), true).thrust, 1.0);
    // Nothing else moves.
    assert_eq!(d.filter(full_controls(), false).steer_x, 0.5);
}

#[test]
fn no_airbrakes_zeroes_both_and_leaves_the_rest() {
    let out = disrupted(Kind::NoAirbrakes, 2.0).filter(full_controls(), false);
    assert_eq!((out.airbrake_left, out.airbrake_right), (0.0, 0.0));
    assert_eq!((out.thrust, out.steer_x), (1.0, 0.5));
}

/// `-1` above one second, `1 - 2t` inside it: through zero at half a second,
/// `+1` at the end.
#[test]
fn mirror_negates_then_blends_back_over_the_last_second() {
    assert_eq!(mirror_scale(5.0), -1.0);
    assert_eq!(mirror_scale(1.5), -1.0);
    assert_eq!(mirror_scale(1.0), -1.0);
    assert_eq!(mirror_scale(0.5), 0.0);
    assert_eq!(mirror_scale(0.25), 0.5);
    assert_eq!(mirror_scale(0.0), 1.0);
    let out = disrupted(Kind::MirrorLeftRight, 4.0).filter(full_controls(), false);
    assert_eq!(out.steer_x, -0.5);
    let out = disrupted(Kind::MirrorLeftRight, 0.25).filter(full_controls(), true);
    assert_eq!(out.steer_x, 0.25);
}

#[test]
fn the_two_autopilots_scale_thrust_and_filter_nothing() {
    let slow = disrupted(Kind::AutopilotSlow, 2.0);
    let fast = disrupted(Kind::AutopilotFast, 2.0);
    assert_eq!(slow.autopilot_thrust_scale(), Some(AUTOPILOT_SLOW_THRUST));
    assert_eq!(fast.autopilot_thrust_scale(), Some(AUTOPILOT_FAST_THRUST));
    assert_eq!(slow.filter(full_controls(), true), full_controls());
    assert_eq!(disrupted(Kind::Stall, 2.0).autopilot_thrust_scale(), None);
    assert_eq!(Disruption::default().autopilot_thrust_scale(), None);
}

/// The three that land and do nothing: state set, controls untouched.
#[test]
fn drunk_rubber_and_drunk_camera_land_without_filtering() {
    for kind in [Kind::Drunk, Kind::RubberShip, Kind::DrunkCamera] {
        let d = disrupted(kind, 2.0);
        assert!(d.active());
        assert_eq!(
            d.filter(full_controls(), false),
            full_controls(),
            "{kind:?}"
        );
        assert_eq!(d.autopilot_thrust_scale(), None, "{kind:?}");
    }
}

/// Applied, then counted: a `time` of exactly one tick is one tick of effect.
#[test]
fn the_countdown_tears_down_at_or_below_zero() {
    let dt = 1.0 / 60.0;
    let mut d = disrupted(Kind::Stall, dt);
    assert_eq!(d.filter(full_controls(), false).thrust, 0.0);
    d.advance(dt);
    assert_eq!(d, Disruption::default());
    // An idle one stays idle and its timer stays zero.
    d.advance(dt);
    assert_eq!(d, Disruption::default());
}

#[test]
fn landing_refuses_a_disrupted_or_shielded_craft_and_an_unauthored_effect() {
    let stats = stats();
    let mut ship = Ship::default();
    assert!(land(&mut ship, Kind::Stall, &stats));
    assert_eq!(ship.disruption, disrupted(Kind::Stall, 3.0));
    // Already disrupted: the second bolt is wasted, the first keeps its time.
    ship.disruption.timer = 1.0;
    assert!(!land(&mut ship, Kind::Drunk, &stats));
    assert_eq!(ship.disruption, disrupted(Kind::Stall, 1.0));

    let mut shielded = Ship::default();
    shielded.physics.shield_pickup_timer = 2.0;
    assert!(!land(&mut shielded, Kind::Stall, &stats));
    assert!(!shielded.disruption.active());

    let mut bare = Ship::default();
    assert!(!land(&mut bare, Kind::Trippy, &stats));
    assert!(!bare.disruption.active());
}

#[test]
fn the_field_pass_counts_every_racing_slot_and_no_other() {
    let mut world = crate::World::new(1);
    world.ship_count = 2;
    for slot in 0..3 {
        world.ships[slot].disruption = disrupted(Kind::Stall, 1.0);
    }
    advance(&mut world, 0.5);
    assert_eq!(world.ships[0].disruption.timer, 0.5);
    assert_eq!(world.ships[1].disruption.timer, 0.5);
    assert_eq!(world.ships[2].disruption.timer, 1.0, "past ship_count");
}
