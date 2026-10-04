//! What the Repulser's timeline, wave walk and hit law in [`super`] are
//! asserted to do. Each test fails if the recovered rule it names is dropped.

use super::*;
use oag_physics::DamageRules;
use oag_vex::track::{AiTrack, Junction, Path, SplinePoint};

/// A circle of `count` control points, radius 400, laid counter-clockwise seen
/// from above, with a 10-unit half width each side and a 30-unit AI corridor.
fn circle_course(count: usize) -> oag_race::Course {
    let radius = 400.0f32;
    let points = (0..count)
        .map(|i| {
            let angle = i as f32 / count as f32 * std::f32::consts::TAU;
            let (sin, cos) = (angle.sin(), angle.cos());
            SplinePoint {
                pos: [radius * cos, 0.0, radius * sin],
                tangent: [-sin, 0.0, cos],
                down: [0.0, -1.0, 0.0],
                // Outward, so `centre` is the midpoint of the two edges.
                lateral: [cos, 0.0, sin],
                progress: 0.0,
                half_width_left: 10.0,
                half_width_right: 10.0,
                ai_bound_left: -15.0,
                ai_bound_right: 15.0,
                racing_line: 0.0,
                section_id: 0,
                flags: 0,
                light_scale: [0xff; 4],
            }
        })
        .collect();
    let track = AiTrack {
        version: 1,
        paths: vec![Path {
            points,
            max_spacing: 20.0,
            entry: Some(0),
            exit: Some(0),
        }],
        junctions: vec![Junction {
            prev: [Some(0), None],
            next: [Some(0), None],
        }],
    };
    oag_race::Course::from_track(&track, None).expect("the circle closes")
}

fn stats() -> RepulserStats {
    RepulserStats {
        absorb: 15.0,
        damage: 30.0,
        blastforce: 40.0,
        slowdown_time: 0.8,
        blast_time: 0.8,
        wave_time: 0.8,
    }
}

const DT: f32 = 1.0 / 60.0;

fn ship_at(course: &oag_race::Course, index: usize) -> Ship {
    let mut ship = Ship {
        active: true,
        ..Ship::default()
    };
    ship.physics.body.position = course.centre(index).unwrap();
    ship.standing.course_index = Some(index as u32);
    ship.handling.dimensions.shield = 100.0;
    ship.physics.shield = 100.0;
    ship
}

/// Ticks a fresh Repulser until its waves have started.
fn started(course: &oag_race::Course, owner_index: usize) -> (Repulser, u32) {
    let mut repulser = Repulser::launch(0, &stats());
    let mut ticks = 0;
    while !repulser.waves_running() {
        assert!(repulser.advance(course, Some(owner_index), DT));
        ticks += 1;
        assert!(ticks < 1000);
    }
    (repulser, ticks)
}

#[test]
fn a_wave_sweeps_a_point_between_its_two_centres_and_within_the_corridor() {
    let previous = Vec3::new(0.0, 0.0, 0.0);
    let point = Vec3::new(20.0, 0.0, 0.0);
    assert!(sweeps(point, previous, Vec3::new(10.0, 0.0, 0.0), 15.0));
    assert!(sweeps(point, previous, Vec3::new(10.0, 0.0, 14.0), 15.0));
    // Off to the side, past the corridor width.
    assert!(!sweeps(point, previous, Vec3::new(10.0, 0.0, 16.0), 15.0));
    // Ahead of the wave by more than the unit of slack, and behind it.
    assert!(sweeps(point, previous, Vec3::new(20.5, 0.0, 0.0), 15.0));
    assert!(!sweeps(point, previous, Vec3::new(21.5, 0.0, 0.0), 15.0));
    assert!(sweeps(point, previous, Vec3::new(-0.5, 0.0, 0.0), 15.0));
    assert!(!sweeps(point, previous, Vec3::new(-1.5, 0.0, 0.0), 15.0));
}

#[test]
fn a_wave_that_has_not_moved_sweeps_nothing() {
    let here = Vec3::new(5.0, 0.0, 5.0);
    assert!(!sweeps(here, here, here, 1000.0));
    assert!(!sweeps(here, here, Vec3::new(6.0, 0.0, 5.0), 1000.0));
}

#[test]
fn the_waves_start_at_the_firer_once_blast_time_has_passed() {
    let course = circle_course(120);
    let (repulser, ticks) = started(&course, 40);
    // 0.8 s at 1/60: the 49th tick is the first whose age is past 0.8.
    assert_eq!(ticks, 49);
    let fronts = repulser.fronts.unwrap();
    for front in fronts {
        assert_eq!(front.index, 40);
        assert_eq!(front.point, course.centre(40).unwrap());
        assert_eq!(front.previous, front.point, "the spawn tick does not move");
    }
}

#[test]
fn the_waves_start_where_the_firer_is_then_not_where_it_fired() {
    let course = circle_course(120);
    let mut repulser = Repulser::launch(0, &stats());
    repulser.advance(&course, Some(10), DT);
    while !repulser.waves_running() {
        repulser.advance(&course, Some(77), DT);
    }
    assert_eq!(repulser.fronts.unwrap()[0].index, 77);
}

#[test]
fn the_forward_wave_walks_five_control_points_a_tick_and_the_backward_two() {
    let course = circle_course(120);
    let (mut repulser, _) = started(&course, 40);
    let count = course.len();
    repulser.advance(&course, Some(40), DT);
    let [forward, backward] = repulser.fronts.unwrap();
    let per = oag_race::Course::STEPS_PER_SEGMENT;
    assert_eq!(forward.index as usize, 40 + 5 * per);
    assert_eq!(backward.index as usize, (40 + count - 2 * per) % count);
    assert_eq!(forward.previous, course.centre(40).unwrap());
}

#[test]
fn it_lives_blast_time_plus_wave_time() {
    let course = circle_course(120);
    let mut repulser = Repulser::launch(0, &stats());
    let mut ticks = 1;
    while repulser.advance(&course, Some(0), DT) {
        ticks += 1;
    }
    // 1.6 s at 1/60 s a tick, give or take the f32 accumulation.
    assert!((95..=97).contains(&ticks), "{ticks} ticks");
}

#[test]
fn nothing_is_hit_during_the_blast_phase() {
    let course = circle_course(120);
    let mut ships = [Ship::default(); MAX_SHIPS];
    ships[0] = ship_at(&course, 40);
    ships[1] = ship_at(&course, 41);
    let mut repulser = Repulser::launch(0, &stats());
    let mut hits = [super::super::WeaponHit::default(); MAX_SHIPS];
    for _ in 0..40 {
        repulser.advance(&course, Some(40), DT);
        let struck = repulser.apply_hits(&mut ships, 2, &course, DamageRules::default(), &mut hits);
        assert!(!struck.iter().any(|&s| s));
    }
}

/// Runs the whole life of a Repulser fired from ring index `owner` against one
/// target, returning the target after and how many ticks hit it.
fn run_against(owner: usize, target: usize) -> (Ship, usize) {
    let course = circle_course(240);
    let mut ships = [Ship::default(); MAX_SHIPS];
    ships[0] = ship_at(&course, owner);
    ships[1] = ship_at(&course, target);
    let mut repulser = Repulser::launch(0, &stats());
    let mut hits = [super::super::WeaponHit::default(); MAX_SHIPS];
    let mut struck_ticks = 0;
    loop {
        let alive = repulser.advance(&course, Some(owner), DT);
        let struck = repulser.apply_hits(&mut ships, 2, &course, DamageRules::default(), &mut hits);
        assert!(!struck[0], "the firer is never hit");
        struck_ticks += usize::from(struck[1]);
        if !alive {
            break;
        }
    }
    (ships[1], struck_ticks)
}

#[test]
fn a_craft_ahead_is_hit_once_and_shoved_forward_with_the_full_force() {
    let course = circle_course(240);
    let (target, struck) = run_against(100, 150);
    assert_eq!(struck, 1, "once per Repulser");
    // Full damage, no falloff.
    assert_eq!(target.physics.shield, 70.0);
    assert_eq!(target.pending_slowdown, 0.8);
    // The shove is the whole blastforce, along the forward wave's travel.
    let velocity = target.physics.body.linear_velocity * target.physics.body.mass;
    assert!((velocity.length() - 40.0).abs() < 1e-3, "{velocity:?}");
    let travel = course.tangent(150).unwrap();
    assert!(
        velocity.normalize().dot(travel) > 0.9,
        "{velocity:?} against {travel:?}"
    );
}

#[test]
fn a_craft_behind_is_shoved_backward_by_the_slower_wave() {
    let course = circle_course(240);
    let (target, struck) = run_against(100, 62);
    assert_eq!(struck, 1);
    let velocity = target.physics.body.linear_velocity;
    let travel = course.tangent(62).unwrap();
    assert!(velocity.normalize().dot(travel) < -0.9, "{velocity:?}");
}

#[test]
fn a_craft_past_either_wave_s_reach_is_not_hit() {
    // 2,400 ring points. The forward wave walks 20 a tick and the backward 8,
    // for the 47 walking ticks of a 0.8 s wave phase: 940 ahead and 376 behind.
    // A craft 1,200 points away either way is out of both.
    let course = circle_course(600);
    assert_eq!(course.len(), 2400);
    let mut ships = [Ship::default(); MAX_SHIPS];
    ships[0] = ship_at(&course, 0);
    ships[1] = ship_at(&course, 1200);
    let mut repulser = Repulser::launch(0, &stats());
    let mut hits = [super::super::WeaponHit::default(); MAX_SHIPS];
    while repulser.advance(&course, Some(0), DT) {
        let struck = repulser.apply_hits(&mut ships, 2, &course, DamageRules::default(), &mut hits);
        assert!(!struck[1]);
    }
    assert_eq!(ships[1].physics.shield, 100.0);
}
