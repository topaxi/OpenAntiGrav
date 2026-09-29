//! The steady-state vertical balance at speed: does our force law hold the craft
//! where the original holds it?
//!
//! `ride_height_ground_truth.rs` found that at speed above ~90 units/s our craft,
//! reseeded onto the original's pose, sinks to about 0.25 units lower at the
//! front probe and 0.14 at the rear within 20 ticks and stays there, while at
//! rest the two agree to three decimals. The sag is speed dependent, so it is a
//! force the law lacks (or carries too much of) at speed.
//!
//! This walks the whole `talons-junction-clean-lap.csv` capture the same way
//! `force_term_pose_walk_ground_truth.rs` walks the crest window - every tick
//! evaluated fresh at the recorded pose, no integration - but over the *steady*
//! ticks: fast, grounded on both sides of the tick, and grounded on the ticks
//! before it, so a crest, a landing, or a wall hit is not in the sample.
//! The residual is projected on the ship's own up axis, and the per-term up
//! components are printed beside it so the missing (or surplus) force can be
//! read off. Prints; asserts nothing about the verdict.
//!
//! `#[ignore]`d and never run in CI: it needs a disc image under `data/images/`.

use oag_core::math::Vec3;
use oag_gameplay::{collision_world, handling_for};
use oag_physics::controls::CONTROL_RANGE;
use oag_physics::{Environment, Handling, ShipControls, clamp_dt, forces};
use oag_pulse as pulse;
use oag_tables::handling;
use oag_trace::Trace;
use oag_trace::replay::{Basis, initial_state};
use oag_trace::trace::{AngularReading, Frame};
use oag_vex::collision;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TRACK: &str = r"Data\Environments\16_Track\track.vex";
const CAPTURE: &str = "data/traces/talons-junction-clean-lap.csv";
const TEAM: &str = "Assegai";
const CLASS: &str = "VENOM";

/// Ticks of unbroken full grounding required before a tick counts as steady.
const SETTLED: usize = 6;

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// A capture under `data/traces/`, tracked in git per
/// [ADR-0046](../../../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md).
fn require_capture() -> std::path::PathBuf {
    let path = workspace(CAPTURE);
    assert!(
        path.exists(),
        "{} is missing, but it is tracked in git per ADR-0046 - the checkout \
         is broken, not merely missing optional data.",
        path.display()
    );
    path
}

fn capture() -> Trace {
    let text = std::fs::read_to_string(require_capture()).expect("reading the capture");
    Trace::parse(&text).expect("parsing the capture")
}

fn load() -> Option<(Handling, oag_physics::CollisionWorld, Trace)> {
    let trace = capture();
    let image = workspace(IMAGE);
    if !image.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "{} is missing and OAG_REQUIRE_GAME_DATA is set",
            image.display()
        );
        return None;
    }

    let mut archives =
        pulse::open(image.to_str().expect("the image path is not valid UTF-8")).ok()?;
    let track_blob = archives.read_name(TRACK).expect("reading the track");
    let nodes = collision::from_vex(&track_blob).expect("decoding the collision nodes");
    let collision = collision_world(&nodes);

    let stats_blob = archives
        .read_name(&handling::entry_name(TEAM))
        .expect("reading the handling stats");
    let stats = handling::from_blob(&stats_blob).expect("parsing the handling stats");
    let handling = handling_for(
        &stats,
        CLASS,
        handling::SpeedupPads::default(),
        handling::Special::default(),
    )
    .expect("this team's file authors the class under test");

    Some((handling, collision, trace))
}

/// Replays `craft+0x284`/`craft+0x288`'s update rule -
/// [`oag_physics::forces::evaluate`]'s airborne/grounded bookkeeping - over the
/// capture's own `grounded` and `dt` columns, from tick zero, and returns the
/// `(time_since_landing, time_airborne)` pair a pose at each index would find
/// **entering** its own `evaluate` call.
///
/// This is derivation, not invention: the update rule reads nothing but the
/// previous tick's `grounded` and this tick's `dt`, both of which the capture
/// already carries, so replaying it is exactly the same move
/// `hover_contact_ground_truth.rs` makes seeding `grounded_prev` from the
/// recording - a value the capture does not store directly but is a pure
/// function of ones it does.
///
/// The walk starts both timers at zero at tick zero, an unmeasured initial
/// condition - but [`WINDOW`] opens at tick 1560, and the capture's own
/// airborne event at 1571-1579 sits inside it: any error a wrong tick-zero
/// guess introduced is overwritten the first time a real landing resets
/// `time_since_landing` to zero, which on a circuit with any jump or kerb
/// before tick 1560 has already happened many times over. The 1571-1579 event
/// itself is *inside* the window, so this walk's own accuracy for the ticks
/// that matter here does not even lean on that argument.
fn ground_timers(frames: &[Frame], handling: &Handling) -> Vec<(f32, f32)> {
    let mut out = Vec::with_capacity(frames.len());
    let mut time_since_landing = 0.0f32;
    let mut time_airborne = 0.0f32;
    out.push((time_since_landing, time_airborne));
    for pair in frames.windows(2) {
        let dt = pair[0].dt;
        let grounded_next = pair[1].grounded;
        if grounded_next > 0.0 {
            time_airborne = 0.0;
            time_since_landing += dt;
        } else {
            if handling.antigrav.rebound_jump_time < time_airborne {
                time_since_landing = 0.0;
            }
            time_airborne += dt;
        }
        out.push((time_since_landing, time_airborne));
    }
    out
}

fn main_walk(
    handling: &Handling,
    collision: &oag_physics::CollisionWorld,
    trace: &Trace,
    min_speed: f32,
    max_speed: f32,
) {
    let timers = ground_timers(&trace.frames, handling);
    let environment = Environment::default();
    let names = [
        "engine",
        "lateral_grip",
        "brakes",
        "airbrake",
        "gravity",
        "drag",
        "hover_downforce",
        "hover_probes",
        "rolling_resistance",
        "vertical_damping",
        "speedup_pad",
    ];
    let mut n = 0usize;
    let mut residual_up: Vec<f32> = Vec::new();
    let mut term_up = vec![0.0f32; names.len()];
    let mut pitch_rate = 0.0f32;
    let mut height_front = 0.0f32;
    let mut height_rear = 0.0f32;
    #[allow(clippy::needless_range_loop)]
    for index in SETTLED..trace.frames.len() - 1 {
        let frame = &trace.frames[index];
        let next = &trace.frames[index + 1];
        if frame.speed < min_speed || frame.speed >= max_speed {
            continue;
        }
        if trace.frames[index - SETTLED..=index + 1]
            .iter()
            .any(|f| f.grounded < 1.0)
        {
            continue;
        }
        let (time_since_landing, time_airborne) = timers[index];
        let mut state = initial_state(
            frame,
            handling,
            Basis::LeftUpForward,
            AngularReading::NegatedLocal,
        );
        state.stun_timer = frame.stun_timer.unwrap_or(0.0);
        state.slowdown_timer = frame.timer_2e0.unwrap_or(0.0);
        state.time_since_landing = time_since_landing;
        state.time_airborne = time_airborne;
        let dt = clamp_dt(frame.dt);
        let controls = ShipControls {
            steer_x: frame.steer / CONTROL_RANGE,
            thrust: frame.throttle / CONTROL_RANGE,
            airbrake_left: frame.airbrake_left / CONTROL_RANGE,
            airbrake_right: frame.airbrake_right / CONTROL_RANGE,
            ..ShipControls::default()
        };
        let mass = state.body.mass;
        let orientation = state.body.orientation;
        let up = state.body.up();
        let evaluated =
            forces::evaluate(&mut state, &controls, handling, &environment, collision, dt);
        let hover_probe_force: Vec3 = evaluated
            .hover
            .probes
            .iter()
            .filter(|probe| probe.contact)
            .map(|probe| probe.force)
            .fold(Vec3::ZERO, |a, b| a + b);
        let accels = [
            orientation * evaluated.engine.as_local_force() / mass,
            orientation * evaluated.lateral_grip / mass,
            evaluated.brakes / mass,
            evaluated.airbrake.world_force / mass,
            evaluated.gravity / mass,
            evaluated.drag / mass,
            evaluated.hover.downforce / mass,
            hover_probe_force / mass,
            evaluated.rolling_resistance / mass,
            evaluated.vertical_damping / mass,
            evaluated.speedup_pad / mass,
        ];
        let predicted = state.body.force / mass;
        let measured = (next.velocity - frame.velocity) / dt;
        // Skip a discrete event (wall redirect, escape teleport): a residual an
        // order of magnitude past anything a force explains.
        if (measured - predicted).length() > 30.0 || evaluated.hover.escape != Vec3::ZERO {
            continue;
        }
        n += 1;
        residual_up.push((measured - predicted).dot(up));
        for (slot, a) in accels.iter().enumerate() {
            term_up[slot] += a.dot(up);
        }
        height_front += evaluated.hover.probes[0].height;
        height_rear += evaluated.hover.probes[1].height;
        pitch_rate += frame.angular_rate.map_or(0.0, |w| w.length());
    }
    if n == 0 {
        println!("speed {min_speed}..{max_speed}: no steady ticks");
        return;
    }
    residual_up.sort_by(f32::total_cmp);
    let mean: f32 = residual_up.iter().sum::<f32>() / n as f32;
    println!(
        "\nspeed {min_speed}..{max_speed}: {n} steady ticks; recorded probe heights front {:.3} rear {:.3}",
        height_front / n as f32,
        height_rear / n as f32
    );
    println!(
        "  residual along up (measured - predicted), units/s^2: mean {:+.4}  median {:+.4}  p10 {:+.4}  p90 {:+.4}",
        mean,
        residual_up[n / 2],
        residual_up[n / 10],
        residual_up[n * 9 / 10]
    );
    println!("  mean up-component of each predicted term (units/s^2):");
    for (name, total) in names.iter().zip(&term_up) {
        println!("    {name:>20} {:+9.4}", total / n as f32);
    }
    let _ = pitch_rate;
}

#[test]
#[ignore = "needs data/images/"]
fn steady_state_vertical_balance_by_speed() {
    let Some((handling, collision, trace)) = load() else {
        return;
    };
    for (lo, hi) in [
        (0.0, 30.0),
        (30.0, 60.0),
        (60.0, 90.0),
        (90.0, 105.0),
        (105.0, 1e9),
    ] {
        main_walk(&handling, &collision, &trace, lo, hi);
    }
}
