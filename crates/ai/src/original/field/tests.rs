use super::*;
use crate::original::thrust::tests_table;

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

fn field() -> Field {
    let controller = Controller {
        steer_mul: 0.01,
        steer_damp: 0.0,
        look_ahead_secs: 0.5,
        xtrack_mul: 0.0,
    };
    Field::new(
        tests_table(),
        controller,
        Laws::PULSE,
        1.0,
        1,
        [8, 1, 2, 3, 4, 5, 6, 7],
        42,
    )
}

const RACERS: [(f32, u8); 3] = [(500.0, 3), (520.0, 2), (540.0, 1)];

fn standings() -> Standings<'static> {
    Standings {
        racers: &RACERS,
        player: 0,
        player_rubber_band_off: false,
        race_time: 30.0,
        eliminator: false,
        lap_length: 5000.0,
    }
}

fn run(field: &mut Field, ticks: usize) -> Vec<Output> {
    (0..ticks)
        .map(|_| {
            let lane = field.begin_tick(&[1, 2]);
            field.drive(1, lane == 1, &tick(-520.0), &standings(), 1.0 / 60.0, |d| {
                Some(point(-520.0 - d))
            })
        })
        .collect()
}

#[test]
fn the_round_robin_runs_one_slot_a_tick_with_period_eight() {
    let mut field = field();
    let slots: Vec<usize> = (0..17).map(|_| field.begin_tick(&[1, 2])).collect();
    assert_eq!(slots, [0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0]);
}

#[test]
fn the_shuffle_runs_once_and_arms_the_wander() {
    let mut field = field();
    field.begin_tick(&[1, 2]);
    assert_eq!(field.drivers[1].wander.segments, 2.0);
    assert_eq!(
        field.drivers[0].wander,
        Wander::constructed(8),
        "the player's is untouched"
    );
    let after = field.drivers[1].wander;
    field.begin_tick(&[1, 2]);
    assert_eq!(field.drivers[1].wander, after);
}

#[test]
fn thrust_is_set_on_the_drivers_own_tick_and_held_between() {
    let outputs = run(&mut field(), 20);
    assert_eq!(
        outputs[0].controls.thrust, 0.0,
        "nothing before its first turn"
    );
    let set = outputs[1].controls.thrust;
    assert!(set > 0.0);
    assert!(outputs[1..9].iter().all(|o| o.controls.thrust == set));
}

#[test]
fn the_steering_authority_ramps_in_over_forty_ticks() {
    let mut field = field();
    run(&mut field, 39);
    assert!(field.drivers[1].authority < 1.0);
    run(&mut field, 2);
    assert_eq!(field.drivers[1].authority, 1.0);
}

#[test]
fn a_speed_floor_forces_full_throttle_below_its_speed() {
    let mut field = field();
    field.begin_tick(&[1, 2]);
    field.begin_tick(&[1, 2]);
    let mut slow = Tick {
        speed_floor: Some(0),
        ..tick(-520.0)
    };
    slow.pose.velocity.z = -60.0;
    let out = field.drive(1, true, &slow, &standings(), 1.0 / 60.0, |d| {
        Some(point(-520.0 - d))
    });
    assert_eq!(out.controls.thrust, 1.0, "60 u/s is under the floor's 95");
    let out = field.drive(1, true, &tick(-520.0), &standings(), 1.0 / 60.0, |d| {
        Some(point(-520.0 - d))
    });
    assert!(
        out.controls.thrust < 1.0,
        "120 u/s is over it, the law's own throttle"
    );
}

#[test]
fn a_field_replays_identically_from_its_seed() {
    assert_eq!(run(&mut field(), 300), run(&mut field(), 300));
}

fn hd_field(halved_lookahead: Option<(f32, f32)>) -> Field {
    let mut field = field();
    field.laws = Laws {
        finished_spread: 75.0,
        finished_wander_to_place: true,
        eliminator_drops_when_behind: true,
        round_robin_pre_increment: true,
        halved_lookahead,
        ..Laws::PULSE
    };
    field.counter = 1;
    field
}

#[test]
fn hds_round_robin_flags_racer_one_first() {
    let laws = Laws {
        round_robin_pre_increment: true,
        ..Laws::PULSE
    };
    let mut field = Field::new(
        tests_table(),
        field().controller,
        laws,
        1.0,
        1,
        [8, 1, 2, 3, 4, 5, 6, 7],
        42,
    );
    let slots: Vec<usize> = (0..9).map(|_| field.begin_tick(&[1, 2])).collect();
    assert_eq!(slots, [1, 2, 3, 4, 5, 6, 7, 0, 1]);
}

#[test]
fn a_finished_racer_on_hd_takes_its_place_as_its_slot() {
    let finished = Tick {
        finished: true,
        ..tick(-520.0)
    };
    let drive = |field: &mut Field| {
        field.begin_tick(&[1, 2]);
        field.drive(1, true, &finished, &standings(), 1.0 / 60.0, |d| {
            Some(point(-520.0 - d))
        });
        field.drivers[1]
    };
    // `RACERS[1]` is in second place: `8 - 2`.
    let hd = drive(&mut hd_field(None));
    assert_eq!((hd.spread, hd.wander.start, hd.wander.end), (6.0, 6.0, 6.0));
    let pulse = drive(&mut field());
    assert_ne!(pulse.spread, 6.0, "Pulse's wander has no finished branch");
}

#[test]
fn the_lookahead_halves_outside_the_circuits_window() {
    let asked = |field: &mut Field| {
        let mut distance = 0.0;
        field.drive(1, false, &tick(-520.0), &standings(), 1.0 / 60.0, |d| {
            distance = d;
            Some(point(-520.0 - d))
        });
        distance
    };
    // Smoothed speed `6` after one tick at 120 u/s: `max(6 * t, 15)` is the
    // 15 floor either way, so drive it up first.
    let warm = |field: &mut Field| {
        for _ in 0..200 {
            asked(field);
        }
        asked(field)
    };
    let full = warm(&mut field());
    let inside = warm(&mut hd_field(Some((250.0, 4600.0))));
    let outside = warm(&mut hd_field(Some((600.0, 4600.0))));
    assert_eq!(inside, full, "520 is inside [250, 4600]");
    assert!(
        (outside - full * 0.5).abs() < 1e-3,
        "520 is under 600: half the lookahead time, {outside} against {full}"
    );
}

#[test]
fn laws_halve_only_outside_the_window() {
    let laws = Laws {
        halved_lookahead: Some((400.0, 5400.0)),
        ..Laws::PULSE
    };
    assert!(laws.halves_lookahead(399.0));
    assert!(!laws.halves_lookahead(400.0));
    assert!(!laws.halves_lookahead(5400.0));
    assert!(laws.halves_lookahead(5400.5));
    assert!(!Laws::PULSE.halves_lookahead(0.0));
}

#[test]
fn every_tick_runs_the_thrust_law_off_the_racers_turn() {
    let held = |every_tick: bool| {
        let mut field = field();
        field.laws = Laws {
            every_tick,
            ..Laws::PULSE
        };
        field.drive(1, false, &tick(-520.0), &standings(), 1.0 / 60.0, |d| {
            Some(point(-520.0 - d))
        });
        field.drivers[1].held_thrust
    };
    let off_turn = held(false);
    let every = held(true);
    assert_ne!(every, off_turn, "the law ran: {every} against {off_turn}");
}

#[test]
fn an_events_ai_skills_set_the_ring_rate_only_under_its_law() {
    let rate = |event_ring_rate: bool| {
        let mut field = Field::new(
            tests_table(),
            field().controller,
            Laws {
                event_ring_rate,
                ..Laws::PULSE
            },
            1.0,
            1,
            [8, 1, 2, 3, 4, 5, 6, 7],
            42,
        );
        field.set_event_ai_skills(1.0, 1.0);
        field.drivers[3].planner.rate
    };
    assert_eq!(rate(false), crate::original::RING_RATE);
    assert_eq!(rate(true), crate::original::event_ring_rate(1.0, 1.0));
}

#[test]
fn the_2048_ring_rate_is_what_vita3k_read() {
    // `driver+0xc4` read `0.0016499999910593033` on all eight drivers of
    // "2048 - Event 4", whose two AI skills are the constructor's `1.0`.
    assert_eq!(
        f64::from(crate::original::event_ring_rate(1.0, 1.0)),
        0.001_649_999_991_059_303_3
    );
    assert_eq!(crate::original::event_ring_rate(0.0, 0.0), 0.001);
    // The larger skill decides, clamped to one.
    assert_eq!(
        crate::original::event_ring_rate(0.0, 4.0),
        crate::original::event_ring_rate(1.0, 0.5)
    );
}

#[test]
fn a_2048_field_draws_each_drivers_coin_and_the_planner_rule() {
    let drawn = Field::new(
        tests_table(),
        field().controller,
        Laws {
            coin_ordered_search: true,
            preferred_follows_choice: true,
            ..Laws::PULSE
        },
        1.0,
        1,
        [8, 1, 2, 3, 4, 5, 6, 7],
        42,
    );
    assert!(drawn.drivers.iter().all(|d| d.planner.coin.is_some()));
    assert!(
        drawn
            .drivers
            .iter()
            .all(|d| d.planner.preferred_follows_choice)
    );
    assert!(field().drivers.iter().all(|d| d.planner.coin.is_none()));
}

/// Omega's HD-lineage driver: the coin search on its own, with 2048's three
/// lineage-gated laws off. The coin is drawn from `coin_ordered_search`
/// alone, the planner keeps its preferred lane, and the lane choice stays on
/// the round robin.
#[test]
fn the_coin_search_stands_without_the_2048_lineage_laws() {
    let omega = Field::new(
        tests_table(),
        field().controller,
        Laws {
            coin_ordered_search: true,
            ..Laws::PULSE
        },
        1.0,
        1,
        [8, 1, 2, 3, 4, 5, 6, 7],
        42,
    );
    assert!(omega.drivers.iter().all(|d| d.planner.coin.is_some()));
    assert!(
        omega
            .drivers
            .iter()
            .all(|d| !d.planner.preferred_follows_choice)
    );
    assert!(
        omega
            .drivers
            .iter()
            .all(|d| d.planner.rate == crate::original::RING_RATE)
    );
}
