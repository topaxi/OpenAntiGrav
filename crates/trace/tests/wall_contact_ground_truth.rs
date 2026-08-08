//! Does our hull start scraping the wall where the original's does?
//!
//! The standing start crosses Talon's Junction diagonally under full thrust and
//! meets the outer wall, and the capture dates that moment precisely: the
//! original's `speed` column climbs on every one of ticks 0-186 and then falls by
//! 4.38 units/s on 187.
//!
//! This test takes our simulation out of the question entirely. It walks the
//! **recorded** poses - the original's own position and basis, tick by tick - and
//! asks [`oag_physics::wall::resolve`] whether our hull finds a wall there. No
//! integration, no force law, no accumulated error: just our contact geometry
//! against the original's trajectory.
//!
//! # The box was a third too large, and correcting it overshot
//!
//! **First measured (2026-07-29): 6 ticks early on this capture, 3 on the lap
//! capture below.** `hull_sample_points`/`hull_extent` built the collision box
//! straight from `<Misc width height length>`, unscaled. **Corrected
//! (2026-07-30):** `Ship_InitCraft` (`0x08841360`-`0x0884139c`) scales all three
//! by `hover::TARGET_GLOBAL_SCALE` (`0.75`, the same global the hover target
//! height uses) before building the box - see
//! `docs/ghidra/functions/psp-pulse-usa/collision.md`. Applying that scale is not
//! optional and the evidence for `0.75` itself is not in question - it is read
//! three independent ways (the global's own bit pattern, the multiply
//! instructions in `Ship_InitCraft`, and the argument order reaching
//! `Body_SetBoxDimensions`) - but it **overshoots past zero**: our hull now
//! finds the wall **3 ticks late** here (tick 190 against the original's 187)
//! and **5 ticks late** on the lap capture (176 against 171), both a smaller
//! magnitude of error than before, but on the *other* side. See
//! `our_hull_finds_the_wall_within_a_few_ticks_of_where_the_original_does`.
//!
//! **The two captures moved by different amounts, and that itself is a
//! reading.** A uniform box-size correction would be expected to move both by
//! a similar ratio; instead the standing start's gap shrank (6 early -> 3 late)
//! while the lap's grew (3 early -> 5 late). The two approach the wall at
//! different angles (4.05 degrees on the standing start against 7.33 on the
//! lap - see the module docs on `the_same_gap_shows_on_the_lap_capture...`
//! below), which is consistent with a residual that depends on approach angle
//! rather than being a further uniform scale error. **Do not fit a second
//! scale to close this** - the `0.75` is recovered, not fitted, and chasing a
//! few ticks with another constant is exactly the anti-pattern this project's
//! history warns against. The residual is written down as an open question
//! instead.
//!
//! **It is a hull probe finding it, not the swept query, on both readings, and
//! it is still a lower corner rather than the flank pair.** Both paths run
//! inside [`oag_physics::wall::resolve`], and the swept one is the only thing
//! the `previous_position` argument here could confound, but a corner probe
//! alone reproduces the same tick with no swept contact anywhere near it.
//! Instrumented at the first-contact tick (`firing_probe_index` below): index
//! **2** (`lower - right + forward`) on the standing start, index **1**
//! (`lower + right - forward`) on the lap - both in `hull_sample_points`'s
//! 0..3 range, on opposite sides of the hull as expected for opposite walls,
//! and neither the flank pair (8/9) that the `0.75` scale shrank furthest in
//! absolute terms. So the residual is not a probe-identity change the box
//! shrink caused; it is the same *kind* of probe as before, just no longer at
//! the same corner-to-wall distance.
//!
//! **A geometry change that moves this has a cliff under it, in the other
//! direction now.** [`oag_physics::wall::MAX_CONTACT_DEPTH`] gates contact
//! generation, dropping a sample point that has penetrated more than `2.0`
//! units rather than clamping it - so a fix that makes the hull *larger* again
//! has to stay clear of that gate, and a fix that leaves it late risks nothing
//! there, only tunnelling risk from the swept path.
//!
//! **What a failure means.** This is not a tolerance on a fitted number. It
//! pins a measured gap that is currently an open question, so a change here is
//! a finding either way - which is why the bound below is symmetric (an
//! absolute tick difference) rather than one-sided: the gap has already
//! crossed zero once.
//!
//! `#[ignore]`d and never run in CI: it needs `data/traces/` and a disc image
//! under `data/images/`, both gitignored. Run it with `just test-data`.

use oag_assets::pulse;
use oag_formats::{collision, handling};
use oag_gameplay::{collision_world, handling_for};
use oag_physics::{Environment, Handling, wall};
use oag_trace::Trace;
use oag_trace::replay::{Basis, initial_state};
use oag_trace::trace::AngularReading;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TRACK: &str = r"Data\Environments\16_Track\track.vex";
const CAPTURE: &str = "data/traces/talons-junction-standing-start.csv";
const LAP_CAPTURE: &str = "data/traces/talons-junction-time-trial-lap.csv";

/// Same team and class as every other capture-backed test here; see
/// `yaw_authority_ground_truth.rs` for why this is written down rather than
/// inferred from the filename.
const TEAM: &str = "Assegai";
const CLASS: oag_physics::SpeedClass = oag_physics::SpeedClass::Venom;

/// Paths in `data/` are workspace-relative, but a test's working directory is its
/// own package root.
fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// The tick the **original** first loses speed, and so first meets the wall.
///
/// Read off the capture rather than asserted: `speed` rises on every one of ticks
/// 0-186 and falls by 4.38 on 187.
/// [`the_capture_still_dates_its_own_wall_the_way_this_test_assumes`] re-derives
/// it from the file, so this constant cannot quietly go stale against a
/// recapture.
const RECORDED_WALL_TICK: usize = 187;

/// How many ticks our hull may disagree with the original about when it meets
/// the wall, in **either** direction.
///
/// **Measured, not chosen.** It was 6 ticks early when this test was written;
/// after `hull_sample_points` picked up the `0.75` box-dimension scale (see the
/// module docs), it is 3 ticks *late* here and 5 late on the lap capture below -
/// the gap crossed zero rather than closing, so the bound has to cover both
/// signs. Kept at the largest measurement plus one.
const ALLOWED_TICK_GAP: usize = 6;

fn load() -> (Handling, oag_physics::CollisionWorld, Trace) {
    let mut archives = pulse::Archives::open(
        workspace(IMAGE)
            .to_str()
            .expect("the image path is not valid UTF-8"),
    )
    .expect("opening the disc image");

    let track_blob = archives.read_name(TRACK).expect("reading the track");
    let nodes = collision::from_vex(&track_blob).expect("decoding the collision nodes");
    let collision = collision_world(&nodes);

    let stats_blob = archives
        .read_name(&handling::entry_name(TEAM))
        .expect("reading the handling stats");
    let stats = handling::from_blob(&stats_blob).expect("parsing the handling stats");
    // Zero for the reason `yaw_authority_ground_truth.rs` gives: this capture
    // crosses no pad, so step 15 must contribute nothing here.
    let handling = handling_for(
        &stats,
        CLASS,
        handling::SpeedupPads::default(),
        handling::Special::default(),
    );

    let text = std::fs::read_to_string(workspace(CAPTURE)).expect("reading the capture");
    let trace = Trace::parse(&text).expect("parsing the capture");

    (handling, collision, trace)
}

/// The first recorded tick at which our hull reports a respondable contact.
///
/// `previous_position` is the previous *recorded* position, which is the honest
/// value here: the original really was there one tick earlier, so the swept query
/// sees the segment the original actually travelled.
fn first_contact_on_the_recorded_line(
    handling: &Handling,
    collision: &oag_physics::CollisionWorld,
    trace: &Trace,
) -> Option<usize> {
    let environment = Environment::default();
    trace
        .frames
        .windows(2)
        .position(|pair| {
            let mut state = initial_state(
                &pair[1],
                handling,
                Basis::LeftUpForward,
                AngularReading::NegatedLocal,
            );
            let response = wall::resolve(
                &mut state,
                handling,
                &environment,
                collision,
                pair[0].position,
            );
            assert!(
                !response.hull_degenerate,
                "the hull has no usable extent, so this test measures nothing"
            );
            response.contacts > 0
        })
        // `windows(2).position` counts from the *second* frame, so index 0 is tick 1.
        .map(|index| index + 1)
}

/// Which of [`wall::hull_sample_points`]'s ten indices produced the contact at
/// a given tick, on the recorded pose.
///
/// `WallContact::point` is the sample point itself, not the triangle
/// intersection (`docs/ghidra/functions/psp-pulse-usa/collision.md`), so it should
/// land exactly on one of the ten - matched by nearest distance rather than
/// equality to stay robust to float noise carried through the resolve call.
fn firing_probe_index(
    handling: &Handling,
    collision: &oag_physics::CollisionWorld,
    tick: usize,
    trace: &Trace,
) -> Option<usize> {
    let environment = Environment::default();
    let previous = trace.frames.get(tick.checked_sub(1)?)?;
    let frame = trace.frames.get(tick)?;
    let mut state = initial_state(
        frame,
        handling,
        Basis::LeftUpForward,
        AngularReading::NegatedLocal,
    );
    let points = wall::hull_sample_points(&state.body, handling);
    let response = wall::resolve(
        &mut state,
        handling,
        &environment,
        collision,
        previous.position,
    );
    let contact = response.resolved?;
    points
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (**a - contact.point)
                .length()
                .total_cmp(&(**b - contact.point).length())
        })
        .map(|(index, _)| index)
}

/// Guards [`RECORDED_WALL_TICK`] against a recapture that moves it.
#[test]
#[ignore = "needs data/traces/ and a disc image; run with `just test-data`"]
fn the_capture_still_dates_its_own_wall_the_way_this_test_assumes() {
    let (_, _, trace) = load();
    // A real deceleration, not the sub-tick jitter a craft at rest shows: the
    // capture's `speed` reads 0.021 at tick 0 and 0.015 at tick 1, which is a
    // stationary ship and not a wall. The impact itself is 4.38 units/s in one
    // tick, so any threshold between those two dates the same event.
    let first_loss = trace
        .frames
        .windows(2)
        .position(|pair| pair[1].speed < pair[0].speed - 1.0)
        .map(|index| index + 1)
        .expect("the capture never loses speed, so it never meets the wall");

    assert_eq!(
        first_loss, RECORDED_WALL_TICK,
        "the capture's own wall moved to tick {first_loss}; re-derive the gap \
         below rather than editing the bound to fit"
    );
}

/// The measurement this file exists for.
#[test]
#[ignore = "needs data/traces/ and a disc image; run with `just test-data`"]
fn our_hull_finds_the_wall_within_a_few_ticks_of_where_the_original_does() {
    let (handling, collision, trace) = load();
    let ours = first_contact_on_the_recorded_line(&handling, &collision, &trace)
        .expect("our hull never finds the wall on a line that ends up scraping one");
    let probe = firing_probe_index(&handling, &collision, ours, &trace);
    eprintln!(
        "our hull first finds the wall on the recorded line at tick {ours} \
         (probe index {probe:?}); the original's own response begins at \
         {RECORDED_WALL_TICK}"
    );

    // The sign is the finding, not just the magnitude: pre-fix, our hull was
    // early on both captures; post-fix it is late on both. A magnitude-only
    // bound would also pass the pre-fix (unscaled) box, which is exactly the
    // regression this test exists to catch - so assert lateness explicitly,
    // on top of the magnitude bound.
    assert!(
        ours > RECORDED_WALL_TICK,
        "our hull finds the wall at tick {ours}, at or before the original's own \
         response at {RECORDED_WALL_TICK} - the box-dimension scale in \
         `hull_sample_points` may have regressed to unscaled `<Misc>` values"
    );
    let gap = ours - RECORDED_WALL_TICK;
    assert!(
        gap <= ALLOWED_TICK_GAP,
        "our hull finds the wall at tick {ours}, {gap} tick(s) after the \
         original's own response at {RECORDED_WALL_TICK}; the largest measured \
         gap when this was written was 5 (the lap capture below), 3 here"
    );
    // Pinned so a change that moves the firing probe out of the lower-corner
    // range (0..3) - onto the flank pair (8/9) the scale shrank furthest, or
    // the upper corners (4..7), which nothing here approaches - is a change
    // of *kind* and wants its own investigation rather than silently sliding
    // through a tick-count bound.
    assert!(
        probe.is_some_and(|i| i < 4),
        "the firing probe moved to index {probe:?}, out of the lower-corner \
         range this finding was written about"
    );
}

/// The lap capture, which is where the gap stops being one wall's property.
///
/// It cannot date its wall by `speed` the way the standing start does - the craft
/// is cornering under power and loses speed for reasons that are not walls - so
/// the cleanliness test dates it instead: `speed / |velocity|` reads exactly
/// `1.0000` in free flight, and the first tick it does not is the first tick the
/// craft is touching something. That puts the original's first wall at tick 171.
/// Before the `0.75` box-dimension scale (see the module docs) ours was 168,
/// three ticks early; after it, ours is **176, five ticks late**.
///
/// **Both captures fire the same probe**, the lower *front* corner, on opposite
/// sides of the craft and on different walls - which is what makes this a
/// property of the hull rather than of one triangle. What the pair does **not**
/// do is separate the width term from the length term. The forward-to-wall angle
/// is 4.05 degrees on the standing start against 7.33 here, and the two captures
/// no longer move in the same direction by the same amount now that the box is
/// scaled correctly (6 early -> 3 late here, 3 early -> 5 late on the lap) -
/// consistent with a residual that depends on approach angle rather than a
/// further uniform scale error. Read `Collider_BoxSamplePoints` for both axes if
/// this is picked up again, but do not fit a second scale to close it.
#[test]
#[ignore = "needs data/traces/ and a disc image; run with `just test-data`"]
fn the_same_gap_shows_on_the_lap_capture_and_the_same_probe_finds_it() {
    let (handling, collision, _) = load();
    let text = std::fs::read_to_string(workspace(LAP_CAPTURE)).expect("reading the lap capture");
    let trace = Trace::parse(&text).expect("parsing the lap capture");

    // The original's first contact, by the cleanliness test. Ticks 0-5 are the
    // craft sitting on the grid, where `|velocity|` is small enough for the ratio
    // to be noise rather than a reading.
    let theirs = trace
        .frames
        .iter()
        .enumerate()
        .skip(6)
        .find(|(_, frame)| {
            let magnitude = frame.velocity.length();
            magnitude > 1e-6 && (frame.speed / magnitude - 1.0).abs() >= 1e-4
        })
        .map(|(tick, _)| tick)
        .expect("the lap capture never touches anything");

    let ours = first_contact_on_the_recorded_line(&handling, &collision, &trace)
        .expect("our hull never finds a wall on a line that spends a third of itself in one");
    let probe = firing_probe_index(&handling, &collision, ours, &trace);
    eprintln!("lap: ours {ours} (probe index {probe:?}), the original's {theirs}");

    // See the sign note on `our_hull_finds_the_wall_within_a_few_ticks_of_where_
    // the_original_does`: assert lateness explicitly so a regression to the
    // unscaled box (which was early here too) cannot hide behind a
    // magnitude-only bound.
    assert!(
        ours > theirs,
        "our hull finds the lap's wall at tick {ours}, at or before the \
         original's {theirs} - the box-dimension scale in `hull_sample_points` \
         may have regressed to unscaled `<Misc>` values"
    );
    let gap = ours - theirs;
    assert!(
        gap <= ALLOWED_TICK_GAP,
        "our hull finds the lap's wall at tick {ours}, {gap} tick(s) after \
         the original's {theirs}; the largest measured gap when this was \
         written was 5, here"
    );
    // See the same assertion on `our_hull_finds_the_wall_within_a_few_ticks_of_
    // where_the_original_does`.
    assert!(
        probe.is_some_and(|i| i < 4),
        "the firing probe moved to index {probe:?}, out of the lower-corner \
         range this finding was written about"
    );
}
