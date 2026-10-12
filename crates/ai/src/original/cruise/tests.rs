use super::*;

/// A synthetic class: no disc value, shapes chosen so each term shows.
fn table() -> PureClassRace {
    PureClassRace {
        grid_spread: 500.0,
        cruise_min: [80.0, 78.0, 76.0, 74.0, 72.0, 70.0, 68.0, 100.0],
        cruise_max: [84.0, 82.0, 80.0, 78.0, 76.0, 74.0, 72.0, 100.0],
        start_speed: [160.0, 150.0, 140.0, 130.0, 120.0, 110.0, 100.0, 100.0],
        zone: ZoneCurve {
            position: [-100.0, -40.0, -20.0, 50.0, 100.0, 300.0],
            speed: [0.6, 0.8, 1.0, 1.0, 0.8, 0.6],
        },
        lead_zone: ZoneCurve {
            position: [-300.0, -100.0, -20.0, 50.0, 100.0, 300.0],
            speed: [1.4, 1.0, 1.0, 1.0, 0.8, 0.6],
        },
        fatigue: Fatigue {
            start_time: 3.0,
            decay_time: 8.0,
            multiplier: 0.5,
            restore_rate: 4.0,
            zone_front: 100.0,
            zone_back: -50.0,
        },
    }
}

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

#[test]
fn a_live_sample_reproduces_to_the_bit() {
    // Venom, Vineta K, grid slot 1, 56 s in: the drawn cruise and tweak, the
    // logged zone multiplier and the logged law output
    // (`scripts/psp-pure-ai-thrust.py`, run `trial`).
    let mut t = table();
    t.start_speed[0] = f(0x4320_0000);
    let draws = CruiseDraws {
        tweak: f(0x3f4b_b6b6),
        cruise: [f(0x42b4_11d9), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    };
    assert_eq!(
        cruise_thrust(&t, &draws, 1, f(0x3f38_49d6), 0.0).to_bits(),
        0x424e_4dff
    );
    // The launch blend half in, from `venom-decay`'s first race frame.
    let draws = CruiseDraws {
        tweak: f(0x3f4c_800a),
        cruise: [f(0x42b1_5c5a), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    };
    assert_eq!(
        cruise_thrust(&t, &draws, 1, 1.0, 0.5).to_bits(),
        0x42e6_d726
    );
}

#[test]
fn the_launch_holds_start_speed_and_the_cruise_scales_by_every_term() {
    let t = table();
    let draws = CruiseDraws {
        tweak: 0.8,
        cruise: [82.0; 8],
    };
    assert_eq!(cruise_thrust(&t, &draws, 3, 0.7, 1.0), 140.0);
    assert_eq!(cruise_thrust(&t, &draws, 3, 0.7, 0.0), 82.0 * 0.8 * 0.7);
    assert_eq!(cruise_thrust(&t, &draws, 3, 1.0, 0.0), 82.0 * 0.8);
}

#[test]
fn the_zone_curve_holds_its_ends_and_interpolates_between() {
    let t = table();
    assert_eq!(zone_speed(&t.zone, -500.0), 0.6);
    assert_eq!(zone_speed(&t.zone, -100.0), 0.6);
    assert_eq!(zone_speed(&t.zone, 400.0), 0.6);
    assert_eq!(zone_speed(&t.zone, 0.0), 1.0);
    assert!((zone_speed(&t.zone, -70.0) - 0.7).abs() < 1e-6);
    assert!((zone_speed(&t.zone, 200.0) - 0.7).abs() < 1e-6);
}

#[test]
fn the_gap_wraps_into_half_a_lap_and_signs_ahead_positive() {
    assert_eq!(gap_along_track(100.0, 150.0, 1000.0), 50.0);
    assert_eq!(gap_along_track(100.0, 50.0, 1000.0), -50.0);
    assert_eq!(gap_along_track(950.0, 50.0, 1000.0), 100.0);
    assert_eq!(gap_along_track(50.0, 950.0, 1000.0), -100.0);
}

#[test]
fn the_nearest_opponent_picks_the_curve_point_and_leading_picks_the_curve() {
    let t = table();
    // Nearest is 70 behind: Zone at -70 is 0.7, LeadZone at -70 is 1.0.
    let others = [30.0, 230.0, 600.0];
    assert!((zone_multiplier(&t, 100.0, false, others, 1000.0) - 0.7).abs() < 1e-6);
    assert_eq!(zone_multiplier(&t, 100.0, true, others, 1000.0), 1.0);
    assert_eq!(zone_multiplier(&t, 100.0, false, [], 1000.0), 1.0);
}

#[test]
fn the_launch_ramps_in_at_a_half_and_out_over_five_hundred_frames() {
    let fatigue = table().fatigue;
    let mut s = CruiseState::default();
    s.update(&fatigue, true, 1000.0, 0.0, 1.0);
    assert_eq!(s.launch, 0.5);
    s.update(&fatigue, true, 1000.0, 0.0, 1.0);
    assert_eq!(s.launch, 1.0);
    for _ in 0..499 {
        s.update(&fatigue, false, 1000.0, 0.0, 1.0);
    }
    assert!(s.launch > 0.0 && s.launch < 0.01, "{}", s.launch);
    for _ in 0..2 {
        s.update(&fatigue, false, 1000.0, 0.0, 1.0);
    }
    assert_eq!(s.launch, 0.0);
}

#[test]
fn fatigue_builds_inside_the_window_after_ten_seconds_and_scales_the_zone() {
    let fatigue = table().fatigue;
    let mut s = CruiseState::default();
    // Inside the window but before 10 s: nothing builds.
    assert_eq!(s.update(&fatigue, false, 20.0, 9.0, 0.9), 0.9);
    assert_eq!(s.fatigue_time, 0.0);
    // 7 s inside (420 frames): past the 3 s start, half way through the 8 s decay.
    let mut out = 0.0;
    for _ in 0..420 {
        out = s.update(&fatigue, false, 20.0, 20.0, 1.0);
    }
    assert!((s.fatigue_time - 7.0).abs() < 1e-3, "{}", s.fatigue_time);
    assert!((out - 0.75).abs() < 1e-3, "{out}");
    // Outside, it sheds RestoreRate a second.
    s.update(&fatigue, false, 500.0, 20.0, 1.0);
    assert!((s.fatigue_time - (7.0 - 4.0 / 60.0)).abs() < 1e-3);
}

#[test]
fn the_tail_slows_floors_and_never_drops_below_thirty_five() {
    assert_eq!(finish_thrust(60.0, true, false), 42.0);
    assert_eq!(finish_thrust(60.0, true, true), 100.0);
    assert_eq!(finish_thrust(40.0, true, false), 35.0);
    assert_eq!(finish_thrust(20.0, false, false), 35.0);
}

#[test]
fn draws_land_in_their_bands_and_follow_the_seed() {
    let t = table();
    let a = CruiseDraws::draw(&t, (0.805, 0.795), &mut Rng::new(7));
    let b = CruiseDraws::draw(&t, (0.805, 0.795), &mut Rng::new(7));
    assert_eq!(a, b);
    assert!((0.795..=0.805).contains(&a.tweak));
    for k in 0..8 {
        assert!((t.cruise_min[k]..=t.cruise_max[k]).contains(&a.cruise[k]));
    }
}
