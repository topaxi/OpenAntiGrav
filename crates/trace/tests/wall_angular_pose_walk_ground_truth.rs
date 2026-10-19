//! Does a wall contact turn our craft by as much as it turns the original's?
//!
//! Steps [`oag_physics::step`] once from every recorded pose of Talon's
//! Junction captures (`16_Track`, Assegai Venom) and compares the angular
//! velocity it produces with the one the original recorded on the next tick.
//! `step` reports the share its wall constraint contributed
//! (`WallResponse::angular_velocity_delta`), so on a contact tick the
//! original's wall share is the recorded rate minus what our step produced
//! without it. That tests `Body_ResolveContact`'s angular half as `wall.rs`
//! carries it - the lever arm, the inertia term in the denominator and
//! [`oag_physics::wall::ANGULAR_IMPULSE_SCALE`] - against the original's own
//! state, at every angle and speed the captures hold
//! (`docs/ghidra/functions/psp-pulse-usa/contact-response.md`).
//!
//! # What it measured, 2026-10-19
//!
//! Over the ticks where our wall share exceeds `0.05` rad/s, the original's
//! share has a median `|measured| / |ours|` of `0.986` (autopilot, 87 ticks),
//! `0.998` (clean lap, 69), `0.990` (time-trial lap, 65) and `1.035` (standing
//! start, 1), at a median cosine of `0.965` to `0.999`. Dropping the share
//! leaves a mean contact-tick residual 4-23 times larger. The few ticks
//! that disagree are one-tick shifts of the contact itself (time-trial lap
//! tick 410: ours fires a tick before the original, and both agree at 411),
//! which is `wall_contact_ground_truth.rs`'s detection residual, not the
//! response law.
//!
//! A separate file from `wall_contact_ground_truth.rs` although it reads the
//! same captures: that one resolves a contact on one recorded pose; this one
//! runs a whole `step` from each pose and compares with the next, about 0.2 s
//! for all three captures.
//!
//! `gates_at_the_clean_lap_redirect` is a scratch sweep for the one wall hit
//! our hull does not find (`docs/physics/README.md`).

use oag_core::math::Vec3;
use oag_gameplay::{collision_world, handling_for};
use oag_physics::controls::CONTROL_RANGE;
use oag_physics::{CollisionWorld, Environment, Handling, ShipControls, step};
use oag_pulse as pulse;
use oag_tables::handling;
use oag_trace::Trace;
use oag_trace::replay::{Basis, initial_state};
use oag_trace::trace::AngularReading;
use oag_vex::collision;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TRACK: &str = r"Data\Environments\16_Track\track.vex";
const TEAM: &str = "Assegai";
const CLASS: &str = "VENOM";
/// The three tracked captures (ADR-0046). `talons-junction-autopilot.csv` is
/// not tracked; it measured `0.986` over 87 ticks and is left out.
const CAPTURES: [&str; 3] = [
    "data/traces/talons-junction-standing-start.csv",
    "data/traces/talons-junction-time-trial-lap.csv",
    "data/traces/talons-junction-clean-lap.csv",
];
const CLEAN_LAP: &str = "data/traces/talons-junction-clean-lap.csv";

/// Below this our wall share is in the noise of the free-flight residual
/// (`0.02`-`0.04` rad/s on the lap captures).
const SIGNIFICANT: f32 = 0.05;

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn load() -> (CollisionWorld, Handling) {
    let image = workspace(IMAGE);
    let mut archives = pulse::open(image.to_str().expect("utf-8")).expect("opening the disc image");
    let track_blob = archives.read_name(TRACK).expect("reading the track");
    let nodes = collision::from_vex(&track_blob).expect("decoding the collision nodes");
    let stats_blob = archives
        .read_name(&handling::entry_name(TEAM))
        .expect("handling stats");
    let stats = handling::from_blob(&stats_blob).expect("parsing");
    let handling = handling_for(
        &stats,
        CLASS,
        handling::SpeedupPads::default(),
        handling::Special::default(),
    )
    .expect("class");
    (collision_world(&nodes), handling)
}

fn controls_of(frame: &oag_trace::Frame) -> ShipControls {
    ShipControls {
        steer_x: frame.steer / CONTROL_RANGE,
        thrust: frame.throttle / CONTROL_RANGE,
        airbrake_left: frame.airbrake_left / CONTROL_RANGE,
        airbrake_right: frame.airbrake_right / CONTROL_RANGE,
        ..ShipControls::default()
    }
}

/// One significant wall tick: our wall share and the original's.
struct Share {
    tick: u64,
    ours: Vec3,
    measured: Vec3,
    residual_with: f32,
    residual_without: f32,
}

fn shares(world: &CollisionWorld, handling: &Handling, capture: &str) -> Vec<Share> {
    let text = std::fs::read_to_string(workspace(capture)).expect("reading the capture");
    let trace = Trace::parse(&text).expect("parsing the capture");
    let environment = Environment::default();
    let mut out = Vec::new();
    for pair in trace.frames.windows(2) {
        let Some(recorded) = pair[1].angular_rate else {
            continue;
        };
        let target = AngularReading::NegatedLocal.to_world(recorded, pair[1].rows());
        let mut state = initial_state(
            &pair[0],
            handling,
            Basis::LeftUpForward,
            AngularReading::NegatedLocal,
        );
        let evaluated = step(
            &mut state,
            &controls_of(&pair[0]),
            handling,
            &environment,
            world,
            pair[0].dt,
        );
        let ours = evaluated.wall.angular_velocity_delta;
        let walled = evaluated.wall.contacts > evaluated.wall.floor_contacts;
        if !walled || ours.length() <= SIGNIFICANT {
            continue;
        }
        let with = state.body.angular_velocity;
        let without = with - ours;
        out.push(Share {
            tick: pair[1].tick,
            ours,
            measured: target - without,
            residual_with: (target - with).length(),
            residual_without: (target - without).length(),
        });
    }
    out
}

fn median(mut values: Vec<f32>) -> f32 {
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

/// The original's angular response to a wall, in size and direction, is ours.
///
/// Fails if `ANGULAR_IMPULSE_SCALE` doubles (`0.2`: the ratio halves) or the
/// angular term is dropped (no significant ticks at all), and if the share
/// points the wrong way (the cosine).
#[test]
#[ignore = "needs data/images/"]
fn a_wall_turns_our_craft_by_what_it_turns_the_originals() {
    let (world, handling) = load();
    let mut total = 0;
    for capture in CAPTURES {
        let shares = shares(&world, &handling, capture);
        total += shares.len();
        if shares.is_empty() {
            continue;
        }
        let ratio = median(
            shares
                .iter()
                .map(|s| s.measured.length() / s.ours.length())
                .collect(),
        );
        let cosine = median(
            shares
                .iter()
                .map(|s| s.ours.dot(s.measured) / (s.ours.length() * s.measured.length()))
                .collect(),
        );
        let with: f32 = shares.iter().map(|s| s.residual_with).sum();
        let without: f32 = shares.iter().map(|s| s.residual_without).sum();
        println!(
            "{capture}: {} ticks, median |measured|/|ours| {ratio:.3}, median cos {cosine:.3}, \
             residual with {with:.3} without {without:.3}",
            shares.len()
        );
        for share in shares
            .iter()
            .filter(|s| s.ours.dot(s.measured) < 0.5 * s.ours.length() * s.measured.length())
        {
            println!("  disagreeing tick {}", share.tick);
        }
        assert!(
            (0.9..=1.1).contains(&ratio),
            "{capture}: the original's wall share is {ratio:.3} of ours"
        );
        assert!(
            cosine > 0.95,
            "{capture}: our wall share points {cosine:.3} away from the original's"
        );
        assert!(
            with < 0.5 * without,
            "{capture}: the wall share does not explain the recorded rate \
             ({with:.3} with it, {without:.3} without)"
        );
    }
    assert!(
        total >= 120,
        "only {total} significant wall ticks across the captures; was 135"
    );
}
fn fmt(v: Vec3) -> String {
    format!("({:+.3},{:+.3},{:+.3})", v.x, v.y, v.z)
}

/// Which of `Collision_AddContact`'s gates turns the clean lap's tick-1584 wall
/// hit away in our hull, at the post-integrate pose stepped from tick 1583.
#[test]
#[ignore = "sweep: needs data/images/, set OAG_SWEEP to run"]
fn gates_at_the_clean_lap_redirect() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let (world, handling) = load();
    let environment = Environment::default();
    let text = std::fs::read_to_string(workspace(CLEAN_LAP)).expect("reading the capture");
    let trace = Trace::parse(&text).expect("parsing the capture");
    let from: u64 = std::env::var("OAG_GATE_TICK")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(1583);
    let frame = trace.frames.iter().find(|f| f.tick == from).expect("tick");
    let mut state = initial_state(
        frame,
        &handling,
        Basis::LeftUpForward,
        AngularReading::NegatedLocal,
    );
    let controls = controls_of(frame);
    let dt = oag_physics::clamp_dt(frame.dt);
    state.body.clear_accumulators();
    let clip =
        oag_physics::wall::pre_integration_clip(&mut state, &handling, &environment, &world, dt);
    println!(
        "pass 1 clip: moved {} surface {:?}",
        fmt(clip.moved),
        clip.surface
    );
    let _ =
        oag_physics::forces::evaluate(&mut state, &controls, &handling, &environment, &world, dt);
    let before = state.body.position;
    oag_physics::integrate(&mut state.body, dt);
    if let Some((correction, _)) = oag_physics::hover::sweep(&state, &environment, &world, before) {
        println!("hover sweep correction {}", fmt(correction));
    }
    let centre = state.body.position;
    let scale: f32 = std::env::var("OAG_GATE_SCALE")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(1.0);
    let mut scaled = handling;
    scaled.dimensions.width *= scale;
    scaled.dimensions.height *= scale;
    scaled.dimensions.length *= scale;
    let samples = oag_physics::wall::hull_sample_points(&state.body, &scaled);
    for (index, sample) in samples.iter().enumerate() {
        for (collider, soup) in world.colliders().iter().enumerate() {
            for triangle in 0..soup.triangle_count() {
                let Some([a, b, c]) = soup.triangle(triangle) else {
                    continue;
                };
                if oag_physics::collide::segment_triangle(centre, *sample, a, b, c).is_none() {
                    continue;
                }
                let normal = (b - a).cross(c - a).normalize_or_zero();
                let facing = (centre - *sample).dot(normal) > 0.0;
                let d = normal.dot(*sample - a);
                let projected = *sample - normal * (d - 0.01);
                let inside =
                    oag_physics::collide::segment_triangle(*sample, projected, a, b, c).is_some();
                println!(
                    "sample {index} collider {collider} {:?} tri {triangle} n={} facing={facing} d={d:+.3} gate={} projects={inside}",
                    soup.surface(),
                    fmt(normal),
                    d > -2.0 && d < 0.0,
                );
            }
        }
    }
}
