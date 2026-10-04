use super::*;

const DT: f32 = 1.0 / 60.0;

/// Invented figures, each distinct and none of them the game's. The four times
/// are whole numbers of 60 Hz ticks (12, 24, 30 and 75), which is the case the
/// count-not-sum rule exists for: a running `f32` sum of 75 steps reads
/// `1.2499995`, a tick short of the limit.
const P: StartBoost = StartBoost {
    window_start: 0.2,
    window_end: 0.4,
    stall_end: 0.5,
    overall_duration: 1.25,
    stall_mul: 1.25,
    normal_mul: 1.5,
    boost_mul: 1.75,
};

/// The last tick whose engine read is boosted: `overall_duration` of 75 ticks,
/// the boost writer's reads running from tick 1.
const WINDOW: usize = 75;

/// Runs the launch one tick at a time the way `forces::evaluate` does: read the
/// multiplier (what the engine uses), then advance. Tick 0 is the release
/// frame, tick 1 the first racing frame; thrust first lands at `press`.
/// Returns the multiplier the engine read on every tick.
fn run(press: Option<usize>, ticks: usize) -> Vec<f32> {
    let mut launch = LaunchState::default();
    (0..ticks)
        .map(|tick| {
            let read = launch.multiplier;
            advance(
                &mut launch,
                Some(&P),
                true,
                press.is_some_and(|p| tick >= p),
                DT,
            );
            read
        })
        .collect()
}

/// How many ticks the engine read exactly `value`.
fn count(reads: &[f32], value: f32) -> usize {
    reads.iter().filter(|&&m| m == value).count()
}

#[test]
fn a_held_through_launch_reads_normal_once_then_stall_for_the_window() {
    // Thrust is first non-zero on tick 1, the first racing frame: the engine
    // reads the resting grade's multiplier on it, the grader runs behind it.
    let reads = run(Some(1), 100);
    assert_eq!(reads[0], 1.0, "the release frame reads the resting 1.0");
    assert_eq!(reads[1], P.normal_mul, "the first thrust frame");
    assert!(
        reads[2..=WINDOW].iter().all(|&m| m == P.stall_mul),
        "{reads:?}"
    );
    assert_eq!(reads[WINDOW + 1], 1.0, "the window is over");
}

#[test]
fn a_thrust_inside_the_perfect_window_earns_the_perfect_multiplier() {
    // Tick 20 is 19 clock ticks into the racing frames: 0.317 s, inside [0.2, 0.4).
    let reads = run(Some(20), 100);
    assert_eq!(
        reads[20], P.normal_mul,
        "the edge frame reads the old grade"
    );
    assert!(
        reads[21..=WINDOW].iter().all(|&m| m == P.boost_mul),
        "{reads:?}"
    );
    assert_eq!(count(&reads, P.stall_mul), 0);
}

#[test]
fn a_thrust_in_the_stall_window_after_it_earns_stall() {
    // Tick 28: 27 clock ticks, 0.45 s, between windowEnd and stallEnd.
    let reads = run(Some(28), 100);
    assert!(
        reads[29..=WINDOW].iter().all(|&m| m == P.stall_mul),
        "{reads:?}"
    );
    assert_eq!(count(&reads, P.boost_mul), 0);
}

#[test]
fn a_thrust_after_the_stall_window_keeps_normal() {
    let reads = run(Some(40), 100);
    assert!(reads[1..=WINDOW].iter().all(|&m| m == P.normal_mul));
    assert_eq!(reads[WINDOW + 1], 1.0);
}

#[test]
fn a_thrust_after_the_window_never_sees_a_boost() {
    let reads = run(Some(90), 140);
    assert!(reads[WINDOW + 1..].iter().all(|&m| m == 1.0), "{reads:?}");
    assert_eq!(
        count(&reads[..=WINDOW], P.normal_mul),
        WINDOW,
        "nothing graded it"
    );
}

#[test]
fn the_boost_window_is_exactly_its_ticks_whatever_the_grade() {
    // The boost writer reads its own timer before adding to it, at 1/60 in f32:
    // seventy-five additions land at `1.2499995`, short of the limit, and would
    // give a seventy-sixth boosted tick if the timer were a running sum.
    for press in [1, 20, 28, 40] {
        let reads = run(Some(press), 120);
        let boosted = reads.iter().filter(|&&m| m != 1.0).count();
        assert_eq!(boosted, WINDOW, "press at tick {press}: {reads:?}");
    }
}

#[test]
fn without_parameters_nothing_runs() {
    let mut launch = LaunchState::default();
    for tick in 0..120 {
        advance(&mut launch, None, true, tick > 0, DT);
    }
    assert!(launch.is_idle());
}

#[test]
fn on_the_grid_the_clock_and_timer_hold_at_zero() {
    let mut launch = LaunchState::default();
    for _ in 0..300 {
        advance(&mut launch, Some(&P), false, true, DT);
    }
    assert_eq!(launch.ticks, 0);
    assert_eq!(launch.multiplier, 1.0);
    assert!(!launch.latched || launch.grade == Grade::Normal);
}

#[test]
fn a_thrust_held_on_the_release_frame_is_a_lost_edge() {
    // The countdown handler owns the release frame: the grader does not run, the
    // one-shot is gone by the next frame, and the grade stays at its resting
    // value - Normal, not Stall.
    let mut launch = LaunchState::default();
    advance(&mut launch, Some(&P), true, true, DT);
    advance(&mut launch, Some(&P), true, true, DT);
    assert!(launch.latched);
    assert_eq!(launch.grade, Grade::Normal);
}

mod wired {
    use super::*;
    use crate::collide::{CollisionWorld, Surface, TriangleSoup};
    use crate::forces::{Environment, evaluate};
    use crate::params::{Antigrav, Engine, Handling, Physical};
    use crate::ship::{Body, ShipControls, ShipState};
    use oag_core::math::Vec3;

    fn flat_floor() -> CollisionWorld {
        let mut world = CollisionWorld::new();
        world.push(TriangleSoup::new(
            vec![
                [-500.0, 0.0, -500.0],
                [-500.0, 0.0, 500.0],
                [500.0, 0.0, 0.0],
            ],
            vec![[0, 1, 2]],
            Vec::new(),
            Surface::Floor,
            0,
        ));
        world
    }

    /// Round numbers; the cap is out of reach so the multiplier is the only thing
    /// that can move the engine's output.
    fn handling() -> Handling {
        Handling {
            antigrav: Antigrav {
                ride_height: 6.0,
                rebound: 1.0,
                ..Antigrav::default()
            },
            physical: Physical {
                mass: 1.0,
                normal_gravity: 10.0,
                flight_gravity: 4.0,
                ..Physical::default()
            },
            engine: Engine {
                amount: 0.5,
                accelcap: 1.0e6,
                ..Engine::default()
            },
            ..Handling::ZERO
        }
    }

    /// The engine's output on the tick after the first thrust, for a craft that is
    /// released and holds thrust from its first racing frame.
    fn thrust_on_tick_two(start_boost: Option<StartBoost>) -> f32 {
        let handling = handling();
        let world = flat_floor();
        let env = Environment {
            start_boost,
            ..Environment::default()
        };
        let mut state = ShipState {
            body: Body {
                position: Vec3::new(0.0, 4.0, 0.0),
                ..Body::default()
            },
            grounded: 1.0,
            on_grid: false,
            ..ShipState::default()
        };
        let mut out = 0.0;
        for tick in 0..3 {
            let controls = ShipControls {
                thrust: if tick >= 1 { 1.0 } else { 0.0 },
                ..ShipControls::default()
            };
            let evaluated = evaluate(&mut state, &controls, &handling, &env, &world, DT);
            out = evaluated.engine.thrust;
        }
        out
    }

    #[test]
    fn the_engine_reads_the_graded_multiplier_through_the_force_law() {
        let plain = thrust_on_tick_two(None);
        let boosted = thrust_on_tick_two(Some(P));
        assert!(plain > 0.0, "no thrust to scale");
        let ratio = boosted / plain;
        assert!(
            (ratio - P.stall_mul).abs() < 1.0e-4,
            "a held-through launch reads the stall multiplier on its second thrust \
             tick: ratio {ratio}"
        );
    }
}

/// A driver that thrusts whenever [`holds_first_thrust`] lets it, from the
/// release on, earns the perfect grade; the same driver ungated (thrusting from
/// the first racing frame, as the countdown gate first lets it) is graded
/// stall. The falsifier is the second half.
#[test]
fn holding_the_first_thrust_to_the_window_earns_the_perfect_grade() {
    let mut timed = LaunchState::default();
    let mut eager = LaunchState::default();
    for tick in 0..120 {
        let thrust = !holds_first_thrust(&timed, Some(&P), true, DT);
        advance(&mut timed, Some(&P), true, thrust, DT);
        advance(&mut eager, Some(&P), true, tick >= 1, DT);
    }
    assert_eq!(timed.grade, Grade::Perfect);
    assert_eq!(eager.grade, Grade::Stall);
    assert!(!holds_first_thrust(&timed, Some(&P), true, DT), "latched");
    assert!(!holds_first_thrust(&LaunchState::default(), None, true, DT));
    assert!(!holds_first_thrust(
        &LaunchState::default(),
        Some(&P),
        false,
        DT
    ));
}
