//! Does our craft ride higher than the original's, at rest or at speed?
//!
//! The maintainer's report was that craft fall off the track more easily than
//! the original's, with "flying too high?" as one guess. This is the cheap,
//! exact half of that question: **the height of each hover probe above the
//! surface it found, along the ship's own up axis** (`HoverProbe::height`, the
//! quantity the spring balances against), measured the same way on both sides.
//!
//! - **original**: the recorded poses of a capture, walked through
//!   `hover::evaluate` exactly as `hover_contact_ground_truth.rs` does. No
//!   integration, so nothing of ours enters the number.
//! - **ours**: the same capture replayed through our force law reseeded every
//!   [`RESEED`] ticks (so trajectory drift cannot move the answer), and the
//!   *emitted* poses walked through the same function.
//!
//! Binned by the recorded speed, grounded ticks only. Prints rather than
//! asserts: a measurement pass, not a gate. Run with `--no-capture`.
//!
//! `#[ignore]`d and never run in CI: it needs a disc image under `data/images/`.

use std::num::NonZeroUsize;

use oag_gameplay::{collision_world, handling_for};
use oag_physics::{Environment, Handling, hover};
use oag_pulse as pulse;
use oag_tables::handling;
use oag_trace::Trace;
use oag_trace::replay::{Basis, DeltaSource, Inputs, Options, initial_state, replay};
use oag_trace::trace::AngularReading;
use oag_vex::collision;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TRACK: &str = r"Data\Environments\16_Track\track.vex";
const TEAM: &str = "Assegai";
const CLASS: &str = "VENOM";
const RESEED: usize = 60;

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn load() -> Option<(Handling, oag_physics::CollisionWorld)> {
    let image = workspace(IMAGE);
    if !image.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "{} is missing and OAG_REQUIRE_GAME_DATA is set",
            image.display()
        );
        return None;
    }
    let mut archives = pulse::open(image.to_str().expect("utf-8 path")).ok()?;
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
    Some((handling, collision))
}

fn trace(name: &str) -> Trace {
    let text =
        std::fs::read_to_string(workspace(&format!("data/traces/{name}"))).expect("reading it");
    Trace::parse(&text).expect("parsing it")
}

#[derive(Clone, Copy)]
struct Row {
    tick: u64,
    speed: f32,
    front: f32,
    rear: f32,
    both_contact: bool,
}

fn walk(
    handling: &Handling,
    collision: &oag_physics::CollisionWorld,
    trace: &Trace,
    skip: impl Fn(u64) -> bool,
) -> Vec<Row> {
    let environment = Environment::default();
    trace
        .frames
        .iter()
        .filter(|frame| !skip(frame.tick))
        .map(|frame| {
            let state = initial_state(
                frame,
                handling,
                Basis::LeftUpForward,
                AngularReading::NegatedLocal,
            );
            let reach = hover::target_height(handling, state.mag_lock_blend, state.slowdown_timer);
            let evaluated = hover::evaluate(&state, handling, &environment, collision, reach);
            Row {
                tick: frame.tick,
                speed: frame.speed,
                front: evaluated.probes[0].height,
                rear: evaluated.probes[1].height,
                both_contact: evaluated.probes[0].contact && evaluated.probes[1].contact,
            }
        })
        .collect()
}

struct Stat {
    n: usize,
    front: f32,
    rear: f32,
    front_min: f32,
    front_max: f32,
    rear_min: f32,
    rear_max: f32,
}

fn stat(rows: &[&Row]) -> Option<Stat> {
    if rows.is_empty() {
        return None;
    }
    let n = rows.len();
    let f: Vec<f32> = rows.iter().map(|r| r.front).collect();
    let r: Vec<f32> = rows.iter().map(|r| r.rear).collect();
    Some(Stat {
        n,
        front: f.iter().sum::<f32>() / n as f32,
        rear: r.iter().sum::<f32>() / n as f32,
        front_min: f.iter().copied().fold(f32::INFINITY, f32::min),
        front_max: f.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        rear_min: r.iter().copied().fold(f32::INFINITY, f32::min),
        rear_max: r.iter().copied().fold(f32::NEG_INFINITY, f32::max),
    })
}

const BINS: [(f32, f32, &str); 6] = [
    (0.0, 1.0, "rest    <1"),
    (1.0, 40.0, "1-40"),
    (40.0, 76.0, "40-76"),
    (76.0, 90.0, "76-90"),
    (90.0, 105.0, "90-105"),
    (105.0, 1e9, "105+"),
];

fn table(label: &str, original: &[Row], ours: &[Row]) {
    println!("\n== {label}");
    println!(
        "{:<11} {:>5} {:>7} {:>7} | {:>5} {:>7} {:>7} | front d  rear d",
        "speed bin", "n_o", "o_front", "o_rear", "n_us", "u_front", "u_rear"
    );
    for (lo, hi, name) in BINS {
        let pick = |rows: &[Row]| -> Vec<Row> {
            rows.iter()
                .copied()
                .filter(|r| r.both_contact && r.speed >= lo && r.speed < hi)
                .collect()
        };
        let o = pick(original);
        let u = pick(ours);
        let (so, su) = (
            stat(&o.iter().collect::<Vec<_>>()),
            stat(&u.iter().collect::<Vec<_>>()),
        );
        match (so, su) {
            (Some(a), Some(b)) => println!(
                "{name:<11} {:>5} {:>7.3} {:>7.3} | {:>5} {:>7.3} {:>7.3} | {:>+7.3} {:>+7.3}   \
                 (o front {:.2}..{:.2} rear {:.2}..{:.2}; u front {:.2}..{:.2} rear {:.2}..{:.2})",
                a.n,
                a.front,
                a.rear,
                b.n,
                b.front,
                b.rear,
                b.front - a.front,
                b.rear - a.rear,
                a.front_min,
                a.front_max,
                a.rear_min,
                a.rear_max,
                b.front_min,
                b.front_max,
                b.rear_min,
                b.rear_max,
            ),
            (Some(a), None) => println!(
                "{name:<11} {:>5} {:>7.3} {:>7.3} | ours: no rows",
                a.n, a.front, a.rear
            ),
            _ => {}
        }
    }
}

/// The paired form: our row at tick `t` against the original's at `t`, over
/// ticks where both probes of both are in contact, by how far into the reseed
/// window the tick is. A force imbalance grows with the offset; a seeding
/// artefact is there from the first row.
fn paired(original: &[Row], ours: &[Row], reseed: usize) {
    let by_tick: std::collections::HashMap<u64, &Row> =
        original.iter().map(|r| (r.tick, r)).collect();
    println!("paired (ours - original), speed >= 90, both probes in contact on both sides:");
    println!(
        "{:<14} {:>6} {:>9} {:>9}",
        "offset in win", "n", "d front", "d rear"
    );
    let bands = [(2usize, 10usize), (10, 20), (20, 30), (30, 45), (45, 60)];
    for (lo, hi) in bands {
        let (mut n, mut f, mut r) = (0usize, 0.0f32, 0.0f32);
        for u in ours {
            let off = (u.tick as usize) % reseed;
            if off < lo || off >= hi || u.speed < 90.0 || !u.both_contact {
                continue;
            }
            if let Some(o) = by_tick.get(&u.tick)
                && o.both_contact
            {
                n += 1;
                f += u.front - o.front;
                r += u.rear - o.rear;
            }
        }
        if n > 0 {
            println!(
                "{:<14} {:>6} {:>+9.3} {:>+9.3}",
                format!("{lo}..{hi}"),
                n,
                f / n as f32,
                r / n as f32
            );
        }
    }
}

/// Pose and velocity drift of our reseeded run against the original, on the
/// original's own up axis, by offset into the window. Rows the original holds
/// grounded on the tick and its two neighbours, above 90 units/s.
fn drift(original: &Trace, ours: &Trace, reseed: usize) {
    let by_tick: std::collections::HashMap<u64, &oag_trace::Frame> =
        original.frames.iter().map(|f| (f.tick, f)).collect();
    println!("drift (ours - original) on the original's up axis, speed >= 90, grounded:");
    println!(
        "{:<14} {:>6} {:>9} {:>9} {:>9} {:>9}",
        "offset in win", "n", "d pos up", "d vel up", "d pos fwd", "d vel fwd"
    );
    let bands = [
        (2usize, 4usize),
        (4, 10),
        (10, 20),
        (20, 30),
        (30, 45),
        (45, 60),
    ];
    for (lo, hi) in bands {
        let (mut n, mut dp, mut dv, mut dpf, mut dvf) = (0usize, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for u in &ours.frames {
            let off = (u.tick as usize) % reseed;
            if off < lo || off >= hi {
                continue;
            }
            let Some(o) = by_tick.get(&u.tick) else {
                continue;
            };
            if o.speed < 90.0 || o.grounded < 1.0 {
                continue;
            }
            n += 1;
            dp += (u.position - o.position).dot(o.up);
            dv += (u.velocity - o.velocity).dot(o.up);
            dpf += (u.position - o.position).dot(o.forward);
            dvf += (u.velocity - o.velocity).dot(o.forward);
        }
        if n > 0 {
            let n = n as f32;
            println!(
                "{:<14} {:>6} {:>+9.3} {:>+9.3} {:>+9.3} {:>+9.3}",
                format!("{lo}..{hi}"),
                n as usize,
                dp / n,
                dv / n,
                dpf / n,
                dvf / n
            );
        }
    }
}

fn compare_capture(name: &str, reseed: usize) {
    let Some((handling, collision)) = load() else {
        return;
    };
    let recorded = trace(name);
    let original = walk(&handling, &collision, &recorded, |_| false);
    let options = Options {
        inputs: Inputs::FromTrace,
        dt: DeltaSource::Trace,
        basis: Basis::LeftUpForward,
        angular: AngularReading::NegatedLocal,
        scheme: Default::default(),
        reseed: NonZeroUsize::new(reseed),
        // No speed pads, as before 2026-09-30: this measures the hover, and
        // `pad_crossing_ground_truth.rs` is where the boost is checked.
        pads: Vec::new(),
    };
    let emitted = replay(
        &recorded,
        &handling,
        &Environment::default(),
        &collision,
        &options,
        None,
    );
    // The first two rows after every reseed are the reseed echo, not a step.
    let ours = walk(&handling, &collision, &emitted, |tick| {
        (tick as usize) % reseed < 2
    });
    table(
        &format!(
            "{name} (reseed {reseed}); rows = grounded-both-probes ticks binned by recorded speed"
        ),
        &original,
        &ours,
    );
    paired(&original, &ours, reseed);
    drift(&recorded, &emitted, reseed);
}

#[test]
#[ignore = "needs data/images/"]
fn ride_height_at_rest_and_at_speed() {
    compare_capture("talons-junction-standing-start.csv", 30);
    compare_capture("talons-junction-clean-lap.csv", RESEED);
    compare_capture("talons-junction-autopilot.csv", RESEED);
}
