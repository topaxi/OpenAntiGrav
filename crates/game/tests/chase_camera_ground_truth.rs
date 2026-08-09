//! Drives `oag_render::camera::chase` with the original's own per-tick craft
//! pose and checks the eye it produces against the original's own camera.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image (for the ship's
//! `<ExternalCameraClose>` block) and a capture under `data/traces/`, neither of
//! which this project ships. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(chase_camera_ground_truth)'
//! ```
//!
//! # Why this file exists rather than a number in a comment
//!
//! `data/traces/pad0-boost.csv` is the only capture that carries the original's
//! craft pose **and** its camera pose on the same tick, for 150 consecutive
//! ticks of a speed-pad crossing at 40-154 units/s. That makes it the one
//! instrument that can tell this project's chase camera from the original's
//! *while the spring is working*, and the distinction matters: at rest the two
//! models this file compares agree exactly, so every settled screenshot ever
//! taken of this game is blind to the difference.
//!
//! Two behaviours of `Ship_UpdateCameraRigs` (`0x08845ed0`) were read at
//! instruction level and left unported for a while - the eye keeps a **rigid
//! distance** from its look-at point, and `pos_height` is applied **after** the
//! spring. Both are now in `chase.rs`, and this is the measurement that says so:
//!
//! | Model | RMS error | Max |
//! | --- | ---: | ---: |
//! | free spring, `pos_height` before | 4.938 | 6.824 |
//! | `pos_height` after the spring only | 4.954 | 6.831 |
//! | rigid radius only | 0.415 | 0.556 |
//! | both, craft scale pre-multiplied into the offsets | 0.121 | 0.332 |
//! | both, craft scale on the way out - what ships | **0.008** | **0.060** |
//!
//! Only the last row is asserted here; the other four are what makes it worth
//! asserting, and they are why [`TOLERANCE`] is set where it is rather than
//! anywhere a wrong model could still slip under.
//!
//! # What this pins beyond the two behaviours it was written for
//!
//! Four digits of agreement over a rolling, accelerating craft is not something
//! one right decision buys. Passing also requires, all at once: the
//! `pos_length`/`lookat_length` sign conventions (`race::chase_params`), the
//! first-order lag `error * rate * dt` with no transcendental, the spring split
//! taken in the **craft's** frame along its own up, and
//! `oag_physics::hover::TARGET_GLOBAL_SCALE` applied about the craft *after* the
//! spring rather than folded into the offsets before it. Break any one of them
//! and this test moves by tenths or by units, not by thousandths - the fourth
//! row of that table is what a wrong-but-close answer looks like.
//!
//! # The one thing it does not pin
//!
//! The capture is the **close** external view of one ship on one track. The far
//! block goes through the same code with different numbers, and nothing here
//! says the original selects the same rig for it - `camera.md` reads that off
//! `Ship_UpdateCameraRigs` running the identical algorithm twice, which is a
//! different kind of evidence from this one.

use std::path::{Path, PathBuf};

use oag_game::race;
use oag_render::camera::chase::{Chase, Target};

/// The largest per-tick eye error, in world units, this comparison may show.
///
/// The measured maximum is `0.060` and the RMS is `0.008`, against a camera the
/// capture measures at `11.6` units from a craft moving at up to `154` units per
/// second - so this is a bound of about half a percent of the rig's own size.
/// Set from the measurement with a factor of two of headroom, and **not** loose
/// enough to admit any of the models it replaces: the nearest of them misses by
/// `0.121` RMS and `0.332` at worst, which is three times this bound.
const TOLERANCE: f32 = 0.12;

/// The team whose `handlingstats.xml` the capture was taken with, and the track
/// it was taken on, for the record. Only the ship matters here.
const TEAM: &str = "Assegai";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Reads a path, or `None` when it is not in the tree - the shape every
/// ground-truth test in this workspace uses, so `just test` skips and
/// `just test-data` does not.
fn optional(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(_) => {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but {} is missing",
                path.display()
            );
            println!("skipping: {} not present", path.display());
            None
        }
    }
}

/// The ship's `<ExternalCameraClose>` block, off the user's own disc and through
/// the same `race::chase_params` the game uses - the craft scale included.
fn close_params() -> Option<oag_render::camera::chase::ChaseParams> {
    let image = repo().join("data/images/pulse-psp-usa.chd");
    if !image.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            image.display()
        );
        println!("skipping: {} not present", image.display());
        return None;
    }
    let mut archives = oag_assets::pulse::Archives::open(&image.display().to_string())
        .expect("opening the PSP archives");
    let name = oag_formats::handling::entry_name(TEAM);
    let blob = archives
        .read_name(&name)
        .unwrap_or_else(|e| panic!("reading {name}: {e}"));
    let stats =
        oag_formats::handling::from_blob(&blob).unwrap_or_else(|e| panic!("parsing {name}: {e}"));
    Some(race::chase_params(stats.external_camera_close))
}

#[test]
#[ignore = "needs a disc image in data/images/ and a capture in data/traces/"]
fn the_chase_camera_reproduces_the_originals_eye_over_a_whole_capture() {
    let path = repo().join("data/traces/pad0-boost.csv");
    let Some(text) = optional(&path) else {
        return;
    };
    let Some(params) = close_params() else {
        return;
    };
    let trace = oag_trace::Trace::parse(&text).expect("pad0-boost.csv is a trace");

    println!(
        "{TEAM} <ExternalCameraClose> as authored: \
         lookat {} up / {} ahead, pos {} up / {} behind, springs {} / {}, \
         craft scale {}",
        params.lookat_height,
        params.lookat_length,
        params.pos_height,
        params.pos_length,
        params.spring_horiz,
        params.spring_vert,
        params.craft_scale
    );

    // The recorded rows go in as they were captured. `right_*` is the ship's
    // left (see `oag_trace::Frame::row0`) and that is deliberately **not**
    // reconciled here: the spring decomposes the error onto the side axis
    // through `dot(error, side) * side`, which is even in the sign of `side`,
    // so the one row whose sign is uncertain is the one row that cannot matter.
    let target_of = |frame: &oag_trace::Frame| Target {
        position: frame.position,
        forward: frame.forward,
        up: frame.up,
    };

    let first = trace.frames.first().expect("a non-empty capture");
    let recorded = |frame: &oag_trace::Frame| {
        frame
            .camera_position
            .expect("pad0-boost.csv was captured with --camera")
    };

    // Seeded from the capture's own tick 0, so tick 0 is an initial condition
    // and every later tick is this module's own answer - the same contract
    // `oag_trace::replay` runs the physics under. Undoing the craft scale and
    // then the vertical offset, in that order, is what turns a published eye
    // back into the state the original keeps at `craft+0x7e0`.
    let start = target_of(first);
    let seed = start.position + (recorded(first) - start.position) / params.craft_scale
        - start.up * params.pos_height;
    let mut camera = Chase::at(seed);

    let mut worst = 0.0f32;
    let mut worst_tick = 0;
    let mut sum_squares = 0.0f64;

    for frame in &trace.frames {
        let target = target_of(frame);
        camera.advance(target, &params, frame.dt);

        let ours = camera.eye(target, &params);
        let error = (ours - recorded(frame)).length();

        sum_squares += f64::from(error) * f64::from(error);
        if error > worst {
            worst = error;
            worst_tick = frame.tick;
        }
    }

    let count = trace.frames.len();
    let rms = (sum_squares / count as f64).sqrt();
    println!("{count} tick(s): rms {rms:.4}, worst {worst:.4} at tick {worst_tick}");

    assert!(
        worst < TOLERANCE,
        "the chase eye is {worst:.4} units off the original's at tick {worst_tick}, \
         over {TOLERANCE}; rms {rms:.4} over {count} tick(s)"
    );
}

/// The property the capture is really measuring, taken off the original's own
/// recorded camera rather than off a synthetic fixture the way `chase.rs`'s unit
/// test does.
#[test]
#[ignore = "needs a capture in data/traces/"]
fn the_originals_own_camera_holds_a_constant_distance_from_the_craft() {
    let path = repo().join("data/traces/pad0-boost.csv");
    let Some(text) = optional(&path) else {
        return;
    };
    let trace = oag_trace::Trace::parse(&text).expect("pad0-boost.csv is a trace");

    let mut min = f32::MAX;
    let mut max: f32 = 0.0;
    let (mut slowest, mut fastest) = (f32::MAX, 0.0f32);
    for frame in &trace.frames {
        let eye = frame
            .camera_position
            .expect("pad0-boost.csv was captured with --camera");
        let distance = (eye - frame.position).length();
        min = min.min(distance);
        max = max.max(distance);
        slowest = slowest.min(frame.speed);
        fastest = fastest.max(frame.speed);
    }
    println!("camera-to-craft {min:.4}..{max:.4} over speeds {slowest:.1}..{fastest:.1} units/s");

    // Flatness, and deliberately **only** flatness. What this test exists to
    // retire is "the chase distance grows with speed", a reading this project
    // has had to retire twice, and a spread this small across a 3.9x speed range
    // retires it without reference to any authored number.
    //
    // The absolute check - that the distance is the one the ship's own block
    // predicts - belongs to
    // [`the_captures_first_tick_agrees_with_the_discs_own_close_block`], where
    // the number comes off the player's disc at runtime. Writing it out here as
    // a literal would put shipped design data in a tracked file, which
    // `docs/architecture/adr/0006-no-copyrighted-content.md` forbids and which
    // this file's own header cites.
    assert!(
        max - min < 0.15,
        "the distance varies by {:.4} units over {slowest:.1}..{fastest:.1} \
         units/s, which is not a rigid rig",
        max - min
    );
    assert!(
        fastest > slowest * 2.0,
        "the capture must cover a speed range"
    );
}

/// The seed of the replay above is not itself an assumption: the pose columns
/// and the camera columns of tick 0 agree with the disc's own block to a
/// fraction of a unit, which is what says the capture is this ship in this view.
#[test]
#[ignore = "needs a disc image in data/images/ and a capture in data/traces/"]
fn the_captures_first_tick_agrees_with_the_discs_own_close_block() {
    let path = repo().join("data/traces/pad0-boost.csv");
    let Some(text) = optional(&path) else {
        return;
    };
    let Some(params) = close_params() else {
        return;
    };
    let trace = oag_trace::Trace::parse(&text).expect("pad0-boost.csv is a trace");
    let frame = trace.frames.first().expect("a non-empty capture");
    let eye = frame
        .camera_position
        .expect("pad0-boost.csv was captured with --camera");
    let offset = eye - frame.position;

    // Behind by `pos_length` and above by `pos_height`, both **scaled**: the
    // block's numbers are authored craft-space lengths and the rig shrinks them
    // toward the craft on the way out.
    let behind = -offset.dot(frame.forward);
    let above = offset.dot(frame.up);
    let sideways = offset.dot(frame.row0);
    let wanted_behind = params.pos_length * params.craft_scale;
    let wanted_above = params.pos_height * params.craft_scale;
    println!(
        "tick 0: {behind:.4} behind (block says {wanted_behind:.4}), {above:.4} above \
         (block says {wanted_above:.4}), {sideways:.4} sideways"
    );

    assert!(
        (behind - wanted_behind).abs() < 0.3,
        "{behind:.4} behind against the block's {wanted_behind:.4}"
    );
    // Looser than the forward axis on purpose: the spring is still working on
    // this tick and the vertical component is the one it has furthest to go on.
    assert!(
        (above - wanted_above).abs() < 0.4,
        "{above:.4} above against the block's {wanted_above:.4}"
    );
    assert!(
        sideways.abs() < 0.3,
        "{sideways:.4} off the craft's centre line"
    );
    assert!(offset.length() > 1.0, "a degenerate offset: {offset:?}");
}
