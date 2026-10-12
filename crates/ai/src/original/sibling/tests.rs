use super::*;
use crate::original::{Controller, Laws, Pose};
use oag_tables::pure_ai_stats::{Fatigue, ZoneCurve};

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// One live `Ai_PlanLane` call on Vineta K (`scripts/psp-pure-ai-lane.py`):
/// the lane accelerating toward `0.25`, with the frame's own `dt * 60` of
/// `1.0112` (Pure's timestep is variable). The bang-bang and the speed scale
/// land on the logged bits.
#[test]
fn a_live_planner_call_reproduces_to_the_bit() {
    let mut lane = SingleLane {
        located: true,
        lane: f(0x3dc0_b262),
        target: 0.0,
        velocity: f(0x3c04_146c),
        speed_scale: f(0x3f7f_fffc),
        preferred: 0.25,
        scores: [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
    };
    let velocity = Vec3::new(f(0xc2ee_1cd4), f(0x402c_f298), f(0xc19e_cccd));
    let forward = Vec3::new(f(0xbf7e_9ac4), f(0x3ccd_f96e), f(0xbdcf_3e9d));
    let out = lane.plan(0.2, velocity, forward, f(0x3f81_6e58), &mut Rng::new(1));
    assert_eq!(lane.target, 0.25);
    assert_eq!(out.to_bits(), 0x3dd2_3a9e, "lane {out}");
    assert_eq!(lane.velocity.to_bits(), 0x3c11_5561);
    assert_eq!(lane.speed_scale.to_bits(), 0x3f7f_fffc);
}

/// The bang-bang reaches its target and stops there, overshooting by no more
/// than the step it brakes with.
#[test]
fn the_lane_settles_on_its_target() {
    let mut lane = SingleLane::default();
    let mut rng = Rng::new(3);
    let fast = Vec3::new(0.0, 0.0, -150.0);
    for _ in 0..600 {
        lane.plan(0.9, fast, Vec3::NEG_Z, 1.0, &mut rng);
    }
    assert!((lane.lane - lane.target).abs() < 0.01, "{lane:?}");
    assert!(lane.velocity.abs() < 0.002, "{lane:?}");
}

/// Below `10` units a second along the nose the step is zero: a stopped
/// craft's lane does not move.
#[test]
fn a_stopped_craft_holds_its_lane() {
    let mut lane = SingleLane::default();
    let mut rng = Rng::new(3);
    let first = lane.plan(0.6, Vec3::ZERO, Vec3::NEG_Z, 1.0, &mut rng);
    for _ in 0..100 {
        assert_eq!(
            lane.plan(0.6, Vec3::ZERO, Vec3::NEG_Z, 1.0, &mut rng),
            first
        );
    }
    assert_eq!(first, 0.6, "starts where the craft is");
}

fn table() -> PureClassRace {
    PureClassRace {
        grid_spread: 500.0,
        cruise_min: [80.0; 8],
        cruise_max: [80.0; 8],
        start_speed: [150.0; 8],
        zone: ZoneCurve {
            position: [-100.0, -40.0, -20.0, 50.0, 100.0, 300.0],
            speed: [0.6, 0.8, 1.0, 1.0, 0.8, 0.6],
        },
        lead_zone: ZoneCurve {
            position: [-300.0, -100.0, -20.0, 50.0, 100.0, 300.0],
            speed: [1.4, 1.0, 1.0, 1.0, 0.8, 0.6],
        },
        fatigue: Fatigue {
            start_time: 10.0,
            decay_time: 10.0,
            multiplier: 1.0,
            restore_rate: 4.0,
            zone_front: 10.0,
            zone_back: -10.0,
        },
    }
}

fn field() -> Field {
    Field::cruising(
        table(),
        (1.0, 1.0),
        Controller {
            steer_mul: 0.01,
            steer_damp: 0.0,
            look_ahead_secs: 0.5,
            xtrack_mul: 0.0,
        },
        Laws {
            every_tick: true,
            rail_turns_velocity: false,
            pitch_fade: Some((1030.0, 1180.0)),
            ..Laws::PULSE
        },
        [8, 1, 2, 3, 4, 5, 6, 7],
        42,
    )
}

fn point(z: f32) -> TrackPoint {
    TrackPoint {
        position: Vec3::new(0.0, 0.0, z),
        tangent: Vec3::NEG_Z,
        down: Vec3::NEG_Y,
        lateral: Vec3::X,
        bound_left: -10.0,
        bound_right: 10.0,
        racing_line: 0.0,
        progress: (-z / 5000.0).rem_euclid(1.0),
    }
}

fn tick(z: f32) -> Tick {
    Tick {
        pose: Pose {
            position: Vec3::new(1.0, 3.0, z),
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
            left: Vec3::NEG_X,
            velocity: Vec3::new(0.0, 0.0, -120.0),
            angular_velocity: Vec3::ZERO,
        },
        own: point(z),
        band: None,
        speed_floor: None,
        grounded: 1.0,
        contact_sides: 0,
        touching: false,
        finished: false,
        race_fraction: 0.1,
        lap_distance: 520.0,
    }
}

fn standings(racers: &[(f32, u8)]) -> Standings<'_> {
    Standings {
        racers,
        player: 0,
        player_rubber_band_off: false,
        race_time: 30.0,
        eliminator: false,
        lap_length: 5000.0,
    }
}

/// From the grid every opponent holds its slot's `StartSpeed`, on every tick
/// and not on a round-robin turn; once the field strings out past
/// `GridSpread * tweak` the blend decays to the cruise over 500 ticks.
#[test]
fn the_launch_holds_start_speed_then_decays_to_the_cruise() {
    let mut field = field();
    let grid = [(0.0, 3), (20.0, 2), (40.0, 1)];
    let drive = |field: &mut Field, racers: &[(f32, u8)]| {
        let out = field.drive(
            1,
            false,
            &tick(-20.0),
            &standings(racers),
            1.0 / 60.0,
            |d| Some(point(-20.0 - d)),
        );
        field.end_tick(&standings(racers));
        out
    };
    drive(&mut field, &grid);
    let second = drive(&mut field, &grid);
    assert_eq!(
        second.controls.thrust, 1.5,
        "StartSpeed 150 on a non-turn tick"
    );
    assert_eq!(field.drivers[1].held_thrust, 150.0);
    let strung = [(0.0, 3), (300.0, 2), (600.0, 1)];
    for _ in 0..600 {
        drive(&mut field, &strung);
    }
    // The cruise at the zone the player's nearest opponent (300 ahead) gives.
    assert_eq!(field.drivers[1].cruise.launch, 0.0);
    assert!((field.drivers[1].held_thrust - 80.0 * 0.6).abs() < 1e-3);
}

/// The rail moves the craft and leaves its velocity, and the lane it steers
/// the craft to is the planner's one lane.
#[test]
fn the_rail_moves_position_only() {
    let mut field = field();
    let racers = [(0.0, 3), (20.0, 2), (40.0, 1)];
    let mut rail = None;
    for _ in 0..200 {
        rail = field
            .drive(
                1,
                false,
                &tick(-20.0),
                &standings(&racers),
                1.0 / 60.0,
                |d| Some(point(-20.0 - d)),
            )
            .rail;
    }
    let rail = rail.expect("fires at speed near the track");
    assert_eq!(rail.velocity, None);
}

/// The pitch fade: full near the track, nothing past `1180` units.
#[test]
fn the_pitch_fades_with_distance_from_the_track() {
    let laws = field().laws;
    assert_eq!(laws.pitch_scale(10.0), 1.0);
    assert_eq!(laws.pitch_scale(1105.0), 0.5);
    assert_eq!(laws.pitch_scale(1200.0), 0.0);
    assert_eq!(Laws::PULSE.pitch_scale(5000.0), 1.0);
}

#[test]
fn a_field_replays_identically_from_its_seed() {
    let run = || {
        let mut field = field();
        let racers = [(0.0, 3), (20.0, 2), (40.0, 1)];
        (0..300)
            .map(|_| {
                field.drive(
                    1,
                    false,
                    &tick(-20.0),
                    &standings(&racers),
                    1.0 / 60.0,
                    |d| Some(point(-20.0 - d)),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}
