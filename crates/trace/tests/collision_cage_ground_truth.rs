//! The hit our hull misses is the collision cage's.
//!
//! On `talons-junction-clean-lap.csv` the original's velocity turns from
//! `(-65.59, -8.45, -86.15)` (tick 1583) to `(-83.44, 3.96, -53.43)` (tick 1584),
//! and our hull makes no contact there: the craft is two and a half units above
//! the top of the only wall with that normal, and already past its plane. Live
//! (PPSSPP, the craft put into tick 1583's state), the original's narrowphase
//! makes no contact either; the velocity is redirected after `Body_StepWorld`
//! returns, by `Body_ResolveContact` called from `FUN_08831aa4`, the collision
//! cage. See `docs/ghidra/functions/psp-pulse-usa/collision-cage.md`.
//!
//! [`the_cage_reproduces_the_clean_lap_redirect_our_hull_misses`] steps our
//! physics once from the recorded tick 1583, runs the cage
//! (`oag_gameplay::cage::pass`) and compares with tick 1584. Without the cage
//! the velocity is `38.4` units/s off the record; with it `0.18`. The recorded
//! `dv` is purely along the cage normal (its length and its normal component are
//! both `39.29`), which is the cage's friction of `0`.
//!
//! The two sweeps are scratch (`OAG_SWEEP`): the same step-then-cage walk over
//! any capture (`OAG_CAGE_CSV`, `OAG_SWEEP_TRACK`, `OAG_SWEEP_CLASS`,
//! `OAG_CAGE_TICKS`), which is how Fort Gale's airborne redirect was matched,
//! and a contact-by-contact comparison of our hull against the original's own
//! narrowphase contacts as `scripts/psp-autopilot-contacts.py` captures them.
//!
//! `#[ignore]`d: needs a disc image under `data/images/` (`just test-data`).

use oag_core::math::Vec3;
use oag_gameplay::{collision_world, handling_for};
use oag_physics::controls::CONTROL_RANGE;
use oag_physics::{CollisionWorld, Environment, Handling, ShipControls, step};
use oag_pulse as pulse;
use oag_tables::handling;
use oag_trace::Trace;
use oag_trace::replay::{Basis, initial_state};
use oag_trace::trace::AngularReading;
use oag_vex::{collision, track, vex};

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TRACK: &str = r"Data\Environments\16_Track\track.vex";
const TEAM: &str = "Assegai";
const CLASS: &str = "VENOM";
const CLEAN_LAP: &str = "data/traces/talons-junction-clean-lap.csv";
/// The three tracked Talon's Junction captures (ADR-0046).
const CAPTURES: [&str; 3] = [
    "data/traces/talons-junction-standing-start.csv",
    "data/traces/talons-junction-time-trial-lap.csv",
    CLEAN_LAP,
];

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

struct Circuit {
    world: CollisionWorld,
    handling: Handling,
    cage: oag_gameplay::cage::CageTrack,
}

/// `16_Track` and Assegai VENOM, or the sweep's own track and class.
fn load() -> Option<Circuit> {
    let image = workspace(IMAGE);
    if !image.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "{} is missing",
            image.display()
        );
        return None;
    }
    let mut archives = pulse::open(image.to_str().expect("utf-8")).expect("opening the disc image");
    let name = std::env::var("OAG_SWEEP_TRACK").unwrap_or_else(|_| TRACK.into());
    let blob = archives.read_name(&name).expect("reading the track");
    let nodes = collision::from_vex(&blob).expect("decoding the collision nodes");
    let vex_nodes = vex::nodes(&blob).expect("nodes");
    let node = track::find_node(&blob, &vex_nodes).expect("a WO Track node");
    let ai = track::parse(&blob[node.payload()]).expect("the spline");
    let cage = oag_gameplay::cage::CageTrack::new(&ai, track::cage_flag_in_vex(&blob))
        .expect("every Pulse circuit runs the cage");
    let stats_blob = archives
        .read_name(&handling::entry_name(TEAM))
        .expect("handling stats");
    let stats = handling::from_blob(&stats_blob).expect("parsing");
    let class = std::env::var("OAG_SWEEP_CLASS").unwrap_or_else(|_| CLASS.into());
    let handling = handling_for(
        &stats,
        &class,
        handling::SpeedupPads::default(),
        handling::Special::default(),
    )
    .expect("class");
    Some(Circuit {
        world: collision_world(&nodes),
        handling,
        cage,
    })
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

/// One tick stepped from the record, before and after the cage.
struct Stepped {
    hull_contacts: u32,
    cage: oag_physics::cage::CageResponse,
    without: oag_physics::Body,
    with: oag_physics::Body,
}

fn step_and_cage(c: &Circuit, frame: &oag_trace::Frame) -> Stepped {
    let mut state = initial_state(
        frame,
        &c.handling,
        Basis::LeftUpForward,
        AngularReading::NegatedLocal,
    );
    let evaluated = step(
        &mut state,
        &controls_of(frame),
        &c.handling,
        &Environment::default(),
        &c.world,
        frame.dt,
    );
    let without = state.body;
    let cage = oag_gameplay::cage::pass(Some(&c.cage), &mut state, &c.handling, &evaluated.wall);
    Stepped {
        hull_contacts: evaluated.wall.contacts,
        cage,
        without,
        with: state.body,
    }
}

fn rate(frame: &oag_trace::Frame) -> Vec3 {
    AngularReading::NegatedLocal.to_world(frame.angular_rate.expect("a rate column"), frame.rows())
}

fn trace(relative: &str) -> Trace {
    let text = std::fs::read_to_string(workspace(relative)).expect("reading the capture");
    Trace::parse(&text).expect("parsing the capture")
}

/// Fails if the cage is dropped (the velocity misses by `38` units/s), if its
/// friction is not `0` (a `0.035` scrape costs about `3.8` units/s along the
/// track), or if our hull starts finding a contact here instead.
#[test]
#[ignore = "needs data/images/"]
fn the_cage_reproduces_the_clean_lap_redirect_our_hull_misses() {
    let Some(c) = load() else { return };
    let trace = trace(CLEAN_LAP);
    let frame = trace
        .frames
        .iter()
        .find(|f| f.tick == 1583)
        .expect("tick 1583");
    let next = trace
        .frames
        .iter()
        .find(|f| f.tick == 1584)
        .expect("tick 1584");
    let s = step_and_cage(&c, frame);
    let without = (next.velocity - s.without.linear_velocity).length();
    let with = (next.velocity - s.with.linear_velocity).length();
    let spin = (rate(next) - s.with.angular_velocity).length();
    println!(
        "hull contacts {}, cage contacts {}, |v - recorded| {without:.3} without, {with:.3} with; \
         angular residual {spin:.3}",
        s.hull_contacts, s.cage.contacts
    );
    assert_eq!(s.hull_contacts, 0, "our hull finds a contact at tick 1583");
    assert!(
        s.cage.contacts > 0,
        "the cage makes no contact at tick 1583"
    );
    assert!(!s.cage.reset);
    assert!(
        without > 30.0,
        "without the cage the step is already {without:.2} off"
    );
    assert!(
        with < 0.5,
        "with the cage the step is {with:.3} off the record"
    );
    assert!(spin < 0.15, "the angular rate is {spin:.3} off the record");
}

/// Across the three tracked captures the cage never moves a tick away from the
/// record, and fires where it should: at least once.
#[test]
#[ignore = "needs data/images/"]
fn the_cage_never_moves_a_tick_away_from_the_record() {
    let Some(c) = load() else { return };
    let mut fired = 0;
    for capture in CAPTURES {
        let trace = trace(capture);
        for pair in trace.frames.windows(2) {
            let s = step_and_cage(&c, &pair[0]);
            if s.cage.contacts == 0 && !s.cage.reset {
                continue;
            }
            fired += 1;
            let without = (pair[1].velocity - s.without.linear_velocity).length();
            let with = (pair[1].velocity - s.with.linear_velocity).length();
            assert!(
                with <= without + 0.05,
                "{capture} tick {}: the cage moves the step {without:.3} -> {with:.3} off",
                pair[0].tick
            );
        }
    }
    assert!(fired >= 1, "the cage never fired on the captures");
}

fn fmt(v: Vec3) -> String {
    format!("({:+.3},{:+.3},{:+.3})", v.x, v.y, v.z)
}

/// Scratch sweep: step then cage over any capture, against the next tick.
#[test]
#[ignore = "scratch sweep; set OAG_SWEEP (and OAG_CAGE_CSV, OAG_SWEEP_TRACK, OAG_SWEEP_CLASS, OAG_CAGE_TICKS)"]
fn cage_after_step_against_the_next_tick() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(c) = load() else { return };
    let path = std::env::var("OAG_CAGE_CSV")
        .unwrap_or_else(|_| workspace(CLEAN_LAP).display().to_string());
    let text = std::fs::read_to_string(path).expect("reading the capture");
    let trace = Trace::parse(&text).expect("parsing the capture");
    let only: Option<Vec<u64>> = std::env::var("OAG_CAGE_TICKS")
        .ok()
        .map(|t| t.split(',').map(|t| t.parse().expect("a tick")).collect());
    for pair in trace.frames.windows(2) {
        if only.as_ref().is_some_and(|o| !o.contains(&pair[0].tick)) {
            continue;
        }
        let s = step_and_cage(&c, &pair[0]);
        if s.cage.contacts == 0 && !s.cage.reset {
            continue;
        }
        let without = (pair[1].velocity - s.without.linear_velocity).length();
        let with = (pair[1].velocity - s.with.linear_velocity).length();
        println!(
            "tick {} cage {} reset {} hull {} |v - recorded| {without:.2} -> {with:.2}; \
             cage dw {} w {} recorded {}",
            pair[0].tick,
            s.cage.contacts,
            s.cage.reset,
            s.hull_contacts,
            fmt(s.cage.angular_velocity_delta),
            fmt(s.with.angular_velocity),
            pair[1]
                .angular_rate
                .map(|_| fmt(rate(&pair[1])))
                .unwrap_or_default()
        );
    }
}

/// Scratch sweep: the original's own narrowphase contacts against the documented
/// law on our mesh, from the collider pose the original cached.
///
/// `OAG_CONTACTS_FLAT` is one line per tick: tick, collider centre (3), basis
/// rows (9), the ten sample points (30), a contact count and per contact its
/// point (3), normal (3) and depth, flattened from
/// `scripts/psp-autopilot-contacts.py`'s `.contacts.jsonl`.
#[test]
#[ignore = "scratch sweep; set OAG_SWEEP and OAG_CONTACTS_FLAT"]
fn the_originals_contacts_against_our_mesh() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(c) = load() else { return };
    let flat = std::fs::read_to_string(std::env::var("OAG_CONTACTS_FLAT").expect("the flat file"))
        .expect("reading it");
    let (mut original, mut matched, mut extra) = (0, 0, 0);
    for line in flat.lines() {
        let v: Vec<f32> = line
            .split_whitespace()
            .map(|t| t.parse().expect("a number"))
            .collect();
        let vec = |i: usize| Vec3::new(v[i], v[i + 1], v[i + 2]);
        let tick = v[0];
        let centre = vec(1);
        let samples: Vec<Vec3> = (0..10).map(|k| vec(13 + 3 * k)).collect();
        let n = v[43] as usize;
        let contacts: Vec<(Vec3, Vec3, f32)> = (0..n)
            .map(|k| (vec(44 + 7 * k), vec(47 + 7 * k), v[50 + 7 * k]))
            .collect();
        // `Collision_BoxAgainstMesh` and `Collision_AddContact`, brute force: the
        // centre-to-sample segment crosses, the sample is behind the face, its
        // depth is in (0, 2) and its projection lands in the same triangle.
        let mut law: Vec<(usize, Vec3, f32)> = Vec::new();
        for (index, sample) in samples.iter().enumerate() {
            for soup in c.world.colliders() {
                if soup.surface() == oag_physics::Surface::Reset {
                    continue;
                }
                for triangle in 0..soup.triangle_count() {
                    let Some([a, b, cc]) = soup.triangle(triangle) else {
                        continue;
                    };
                    if oag_physics::collide::segment_triangle(centre, *sample, a, b, cc).is_none() {
                        continue;
                    }
                    let normal = (b - a).cross(cc - a).normalize_or_zero();
                    if (centre - *sample).dot(normal) <= 0.0 {
                        continue;
                    }
                    let d = normal.dot(*sample - a);
                    if !(d < 0.0 && d > -2.0) {
                        continue;
                    }
                    let projected = *sample - normal * (d - 0.01);
                    if oag_physics::collide::segment_triangle(*sample, projected, a, b, cc)
                        .is_none()
                    {
                        continue;
                    }
                    law.push((index, normal, -d));
                }
            }
        }
        for (point, normal, depth) in &contacts {
            original += 1;
            let index = (0..10)
                .min_by(|&i, &j| {
                    (samples[i] - *point)
                        .length()
                        .total_cmp(&(samples[j] - *point).length())
                })
                .expect("ten samples");
            if law
                .iter()
                .any(|(i, n, d)| *i == index && n.dot(*normal) > 0.999 && (d - depth).abs() < 0.01)
            {
                matched += 1;
            } else {
                println!(
                    "tick {tick}: the original's sample {index} n={} depth {depth:.3} is not ours",
                    fmt(*normal)
                );
            }
        }
        for (i, n, d) in &law {
            if !contacts
                .iter()
                .any(|(_, normal, depth)| n.dot(*normal) > 0.999 && (d - depth).abs() < 0.01)
            {
                extra += 1;
                println!(
                    "tick {tick}: ours, not the original's: sample {i} n={} depth {d:.3}",
                    fmt(*n)
                );
            }
        }
    }
    println!("the original's contacts {original}, ours matched {matched}, ours extra {extra}");
}
