//! Does our hover cushion let go of a crest where the original's does?
//!
//! `handover/`'s hover thread measured a **seven-tick phase lag** on one crest of
//! `talons-junction-clean-lap.csv`: reseeded every 60 ticks, the original is
//! airborne on ticks 1595-1608 and we are airborne on 1602-1614 - nearly the same
//! duration, started late. Two candidates, and a whole-lap replay cannot separate
//! them, because a reseed restores position and not the phase of the crest the
//! craft is climbing: either our probe reach is too generous, or the lag is
//! residual crest phase.
//!
//! This test removes the phase question by construction, exactly the way
//! `wall_contact_ground_truth.rs` removes it for the hull: it walks the
//! **recorded** poses - the original's own position and basis, tick by tick - and
//! asks [`oag_physics::hover::evaluate`] whether our probes find ground there. No
//! integration, no accumulated error, no phase.
//!
//! # The answer: the reach is exact, and the lag is phase
//!
//! On the original's own poses our contact test reproduces the recording's whole
//! `grounded` column - **0 disagreements over all 2,976 comparable ticks**,
//! including the crest's `0 / 0.5 x6 / 0 x7` shape and the second event at
//! 1571-1579. See `the_pose_walk_reproduces_the_whole_grounded_column_one_tick_shifted`.
//!
//! The comparison is our verdict at pose `t` against the recording's `grounded`
//! at `t + 1`, which is not a fitted offset: it is the frame ordering our own
//! replay already emits under. `oag_trace::replay::replay` pushes the row
//! *before* stepping, so a replay row at `t` carries the contact evaluated at
//! pose `t - 1` too, and the recording's column is aligned the same way -
//! `speed_cached` is documented as the previous frame's `|dot(velocity,
//! forward)|` at 199/199 for the same reason.
//!
//! Confirmed a second way, with the integrator and the sweep in the loop -
//! neither of which a pose walk exercises, since `hover::sweep` runs from the
//! force path only when both probes miss. `oag-trace run --reseed 2` on the same
//! capture reports `grounded  max error 0.000e0 ... exact`, and its emitted crest
//! column is the original's tick for tick. **Read that as half a column, not a
//! whole one**: at reseed 2 the even rows are the reseed echo, emitted before the
//! step from a state whose `grounded` `initial_state` copied off the recording,
//! so only the odd rows carry a stepped verdict. The odd rows do cover the
//! transitions - 1595, 1601, 1603, 1605, 1607 - which is what the claim needs.
//!
//! **So the lag at `--reseed 60` is crest phase, not reach.** Measured here
//! rather than inherited: at reseed 60 our airborne run on this crest is
//! 1603-1615 against the original's 1595-1608, liftoff eight ticks late, plus a
//! three-tick event at 1590-1592 the original does not have at all. Nothing in
//! the cushion needs shortening; what is left is the force-law drift that moves
//! the craft off the recording's phase between one reseed and the next.
//!
//! # A correction that falls out: `grounded` does read `0.5` at speed
//!
//! All 23 of the capture's `0.5` ticks are above
//! [`oag_physics::hover::FAST_PROBE_SPEED`] - `speed_cached`, which is
//! `craft+0x2ec` and so the branch's own input, reads between 76.4 and 111.6 on
//! every one of them. `docs/gameplay/ai.md` said that at speed "`grounded` cannot
//! read `0.5`" because the fast path copies the front probe's hit flag onto the
//! rear. The guarantee is **one-directional**, and this crate already implements
//! it that way: a front probe *in contact* does force the rear into contact, but
//! a front probe that *misses* falls through to casting the rear for real, and
//! that is exactly the state the original is in on all 23 of those ticks - front
//! miss, rear hit. Corrected on that page; the branch and the `6.0` are
//! untouched.
//!
//! # One thing the walk deliberately does not have to control for
//!
//! `initial_state` seeds `grounded_prev` from the **same** tick's recorded
//! `grounded` rather than the previous tick's, which would matter if the contact
//! verdict depended on it. It does not: `HoverProbe::contact` is set at
//! `crates/physics/src/hover.rs:807` from the raycast hitting plus the surface
//! class being hoverable, and nothing else - `grounded_prev` only scales the
//! spring's load below that point. So the seeding cannot bias what this test
//! measures.
//!
//! `#[ignore]`d and never run in CI: it needs a disc image under `data/images/`.
//! Run it with `just test-data`.

use oag_gameplay::{collision_world, handling_for};
use oag_physics::{Environment, Handling, hover};
use oag_pulse as pulse;
use oag_tables::handling;
use oag_trace::Trace;
use oag_trace::replay::{Basis, initial_state};
use oag_trace::trace::AngularReading;
use oag_vex::collision;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TRACK: &str = r"Data\Environments\16_Track\track.vex";
const CAPTURE: &str = "data/traces/talons-junction-clean-lap.csv";

/// Same team and class as every other capture-backed test here; see
/// `yaw_authority_ground_truth.rs` for why this is written down rather than
/// inferred from the filename.
const TEAM: &str = "Assegai";
const CLASS: &str = "VENOM";

/// The window the pose walk is printed over, wide enough to hold the crest the
/// hover thread measured *and* the smaller event at 1571-1579 that we already
/// match tick for tick - the contrast is the point.
const WINDOW: std::ops::RangeInclusive<u64> = 1560..=1625;

/// The crest itself, isolated from that earlier event: it holds the original's
/// airborne ticks (1595-1608) and ours (1602-1614) and nothing else.
const CREST: std::ops::RangeInclusive<u64> = 1585..=1625;

/// The tick the **original** first leaves the ground on this crest, read off the
/// capture's own `grounded` column rather than asserted -
/// `the_capture_still_dates_its_own_crest_the_way_this_test_assumes` re-derives
/// it so the constant cannot go stale against a recapture.
const RECORDED_LIFTOFF: u64 = 1595;

/// The tick the original lands again, same provenance.
const RECORDED_LANDING: u64 = 1609;

/// How many ticks our probes may disagree with the original about when the
/// cushion lets go, in **either** direction.
///
/// **Measured, not chosen**: the pose walk leaves and rejoins within a tick, and
/// this is that measurement plus one. It is not a tolerance on a fitted number -
/// a change that moves it is a finding either way, which is why it is symmetric.
const ALLOWED_TICK_GAP: u64 = 2;

/// Paths in `data/` are workspace-relative, but a test's working directory is its
/// own package root.
fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// A capture under `data/traces/`, tracked in git per
/// [ADR-0046](../../../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md).
///
/// Its absence means the checkout is broken, not that this test does not apply,
/// so it panics unconditionally rather than skipping - the rule
/// `wall_contact_ground_truth.rs` states at length.
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

/// One tick of the pose walk.
#[derive(Debug, Clone, Copy)]
struct Sample {
    tick: u64,
    /// What the original recorded.
    recorded: f32,
    /// What our probes say at that exact pose.
    ours: f32,
    front_contact: bool,
    rear_contact: bool,
    /// The reach both probes were cast with, so a difference in ray length can
    /// never be mistaken for a difference in geometry.
    reach: f32,
    /// Whether the fast branch was taken - the rear hit manufactured from the
    /// front rather than cast.
    fast_path: bool,
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

/// Walks the recorded poses over [`WINDOW`] and asks our probes about each one.
fn pose_walk(
    handling: &Handling,
    collision: &oag_physics::CollisionWorld,
    trace: &Trace,
) -> Vec<Sample> {
    let environment = Environment::default();
    trace
        .frames
        .iter()
        .filter(|frame| WINDOW.contains(&frame.tick))
        .map(|frame| {
            let state = initial_state(
                frame,
                handling,
                Basis::LeftUpForward,
                AngularReading::NegatedLocal,
            );
            let reach = hover::target_height(handling, state.mag_lock_blend, state.slowdown_timer);
            let evaluated = hover::evaluate(&state, handling, &environment, collision, reach);
            let forward_speed = state.body.linear_velocity.dot(state.body.forward()).abs();
            Sample {
                tick: frame.tick,
                recorded: frame.grounded,
                ours: oag_physics::ShipState::quantise_grounded(evaluated.contacts),
                front_contact: evaluated.probes[0].contact,
                rear_contact: evaluated.probes[1].contact,
                reach,
                fast_path: forward_speed > hover::FAST_PROBE_SPEED,
            }
        })
        .collect()
}

/// The capture still says what the constants above claim it does.
#[test]
fn the_capture_still_dates_its_own_crest_the_way_this_test_assumes() {
    let trace = capture();
    let airborne: Vec<u64> = trace
        .frames
        .iter()
        .filter(|frame| CREST.contains(&frame.tick) && frame.grounded < 1.0)
        .map(|frame| frame.tick)
        .collect();
    assert_eq!(
        airborne.first().copied(),
        Some(RECORDED_LIFTOFF),
        "the capture's own crest moved: airborne ticks in the window are {airborne:?}"
    );
    assert_eq!(
        airborne.last().copied(),
        Some(RECORDED_LANDING - 1),
        "the capture's own crest moved: airborne ticks in the window are {airborne:?}"
    );
}

/// **The discriminating experiment.** At the original's own pose, do our probes
/// let go where its do?
#[test]
#[ignore = "needs data/images/"]
fn the_pose_walk_dates_the_crest_within_a_tick() {
    let Some((handling, collision, trace)) = load() else {
        return;
    };
    let samples = pose_walk(&handling, &collision, &trace);
    assert!(!samples.is_empty(), "the window selected no frames");

    for sample in &samples {
        println!(
            "{tick}  recorded {recorded:>3}  ours {ours:>3}  front {front:>5}  rear {rear:>5}  \
             reach {reach:.4}  {path}",
            tick = sample.tick,
            recorded = sample.recorded,
            ours = sample.ours,
            front = sample.front_contact,
            rear = sample.rear_contact,
            reach = sample.reach,
            path = if sample.fast_path { "fast" } else { "two-ray" },
        );
    }

    // Structural, not a check that can fail: `initial_state` never seeds
    // `mag_lock_blend` or `slowdown_timer`, so `hover::target_height` is the
    // same 4.125 on every tick by construction. Asserted anyway so the day one
    // of those is seeded, this line is what says the walk's ray length stopped
    // being constant. **A pose walk on a magstrip circuit would silently use the
    // wrong reach**, since the recording carries no `mag_lock_blend` column to
    // seed one from; Talon's Junction has no magstrip on this stretch, and what
    // establishes that is the 2,976-of-2,976 agreement below, not this line.
    let reach = samples[0].reach;
    assert!(
        samples.iter().all(|sample| sample.reach == reach),
        "the probe reach varies across the window, so a reach difference could \
         be mistaken for a geometry one"
    );

    let ours_liftoff = samples
        .iter()
        .filter(|sample| CREST.contains(&sample.tick))
        .find(|sample| sample.ours < 1.0)
        .map(|sample| sample.tick)
        .expect("our probes never leave the ground anywhere in the window");
    let ours_landing = samples
        .iter()
        .rev()
        .filter(|sample| CREST.contains(&sample.tick))
        .find(|sample| sample.ours < 1.0)
        .map(|sample| sample.tick + 1)
        .expect("our probes never leave the ground anywhere in the window");

    assert!(
        ours_liftoff.abs_diff(RECORDED_LIFTOFF) <= ALLOWED_TICK_GAP,
        "on the original's own poses our cushion lets go at {ours_liftoff}, \
         against the original's {RECORDED_LIFTOFF}"
    );
    assert!(
        ours_landing.abs_diff(RECORDED_LANDING) <= ALLOWED_TICK_GAP,
        "on the original's own poses our cushion catches again at {ours_landing}, \
         against the original's {RECORDED_LANDING}"
    );
}

/// The pose walk over the **whole** capture, one tick shifted.
///
/// `the_pose_walk_dates_the_crest_within_a_tick` says the two agree on one
/// crest; this asks how far that goes. It is the same walk over all 2,977
/// recorded poses, comparing our verdict at pose `t` against the recording's
/// own `grounded` at `t + 1` - the offset the crest window makes visible and
/// which our own replay already emits under (`frame_of` writes the state
/// *before* the step, so a replay row at `t` also carries the contact evaluated
/// at pose `t - 1`).
#[test]
#[ignore = "needs data/images/"]
fn the_pose_walk_reproduces_the_whole_grounded_column_one_tick_shifted() {
    let Some((handling, collision, trace)) = load() else {
        return;
    };
    let environment = Environment::default();
    let mut compared = 0usize;
    let mut disagreements = Vec::new();
    for pair in trace.frames.windows(2) {
        let state = initial_state(
            &pair[0],
            &handling,
            Basis::LeftUpForward,
            AngularReading::NegatedLocal,
        );
        let reach = hover::target_height(&handling, state.mag_lock_blend, state.slowdown_timer);
        let evaluated = hover::evaluate(&state, &handling, &environment, &collision, reach);
        let ours = oag_physics::ShipState::quantise_grounded(evaluated.contacts);
        compared += 1;
        if ours != pair[1].grounded {
            disagreements.push((pair[1].tick, pair[1].grounded, ours));
        }
    }
    println!(
        "compared {compared} ticks, {} disagreements: {:?}",
        disagreements.len(),
        &disagreements[..disagreements.len().min(40)]
    );
    assert_eq!(
        disagreements.len(),
        0,
        "our contact test no longer reproduces the recorded grounded column"
    );
}

/// The original reads `0.5` above the fast-probe threshold, which the wording
/// corrected in the module docs said it could not.
///
/// A standing record of the measurement behind that correction. It says nothing
/// about our own `0.5` behaviour - `the_pose_walk_...` above is what pins that.
#[test]
fn the_original_reads_half_grounded_above_the_fast_probe_speed() {
    let trace = capture();
    let halves: Vec<(u64, f32)> = trace
        .frames
        .iter()
        .filter(|frame| frame.grounded == 0.5)
        .map(|frame| (frame.tick, frame.speed_cached))
        .collect();
    assert_eq!(halves.len(), 23, "the capture's half-grounded ticks moved");
    for (tick, cached) in &halves {
        assert!(
            *cached > hover::FAST_PROBE_SPEED,
            "tick {tick} reads grounded 0.5 at speed_cached {cached}, below the \
             fast-probe threshold - the refutation this test records no longer holds"
        );
    }
}
