//! A full grid's pose track costs about what ADR-0087 decision 4 measured and loses no more
//! than it states, on every title, and a respawn in the track is bounded the same way.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^pose_track_ground_truth::/)' --no-capture
//! ```
//!
//! The cost and the error are a named check over `replay_timeline_ground_truth`'s one pad-flown
//! Single Race of eight craft per title (`bounds`), so no race is run for them twice. The
//! respawn is its own, short run per title: it stops at the tick the claim is decided.

use oag_raceplay::{self as race, Race};
use oag_replay::track::{MAX_POSITION_ERROR, MAX_ROTATION_ERROR, PoseRecorder, degrees_between};
use oag_replay::{Pose, PoseTrack};

use crate::replay_ground_truth::{Pad, source};
use crate::replay_timeline_ground_truth::Run;

/// Error percentiles over every craft and tick: p50, p99, p99.9, max.
fn percentiles(mut values: Vec<f32>) -> [f32; 4] {
    values.sort_by(f32::total_cmp);
    let at = |p: f64| values[((values.len() as f64 * p) as usize).min(values.len() - 1)];
    [at(0.5), at(0.99), at(0.999), values[values.len() - 1]]
}

/// Position (units) and rotation (degrees) distance of the track from the truth, at every tick
/// of `ticks` for every craft.
fn errors(
    track: &PoseTrack,
    truth: &[Vec<Pose>],
    ticks: std::ops::RangeInclusive<usize>,
) -> (Vec<f32>, Vec<f32>) {
    let mut position = Vec::new();
    let mut rotation = Vec::new();
    for tick in ticks {
        for (craft, pose) in truth[tick].iter().enumerate() {
            let got = track.pose(craft, tick as f64).expect("a craft of the grid");
            position.push((got.position - pose.position).length());
            rotation.push(degrees_between(got.rotation, pose.rotation));
        }
    }
    (position, rotation)
}

/// Bytes a craft a second, the error percentiles, and the cost of capturing and finishing, for
/// the recording `run` holds.
pub(crate) fn bounds(title: &str, run: &Run) {
    let track = run
        .replay
        .pose_track
        .as_ref()
        .unwrap_or_else(|| panic!("{title}: a recording carries a pose track"));
    let ticks = run.replay.ticks();
    let crafts = track.crafts();
    assert_eq!(crafts, 8, "{title}: a Single Race grid");
    assert_eq!(track.last_tick(), ticks, "{title}");
    let (position, rotation) = errors(track, &run.truth, 0..=ticks as usize);
    let (position, rotation) = (percentiles(position), percentiles(rotation));
    let bytes = track.encoded_len();
    let per_second = bytes as f64 / crafts as f64 / (ticks as f64 / 60.0);
    let keys: usize = (0..crafts).map(|craft| track.keys(craft)).sum();
    println!(
        "{title}: POSE {ticks} ticks, {crafts} craft, {keys} keys, {bytes} bytes = {per_second:.1} B \
         per craft per s; position p50/p99/p99.9/max {:.4}/{:.4}/{:.4}/{:.4} units, rotation \
         {:.3}/{:.3}/{:.3}/{:.3} degrees",
        position[0],
        position[1],
        position[2],
        position[3],
        rotation[0],
        rotation[1],
        rotation[2],
        rotation[3],
    );
    assert!(
        position[3] <= MAX_POSITION_ERROR + 1e-3,
        "{title}: {} units off at worst",
        position[3]
    );
    assert!(
        rotation[3] <= MAX_ROTATION_ERROR + 1e-2,
        "{title}: {} degrees off at worst",
        rotation[3]
    );
    assert!(
        (100.0..=220.0).contains(&per_second),
        "{title}: {per_second:.1} B per craft per s is not near the 149 to 168 ADR-0087 measured"
    );
    capture_cost(title, run);
}

/// What the recording spends on the track: a push a tick, and the copy `Race::recorded` takes
/// and the file bytes it writes at the end.
fn capture_cost(title: &str, run: &Run) {
    let crafts = run.truth[0].len();
    let mut recorder = PoseRecorder::new(crafts);
    let started = std::time::Instant::now();
    for poses in &run.truth {
        assert!(recorder.push(poses));
    }
    let push = started.elapsed();
    let started = std::time::Instant::now();
    let track = recorder.track().expect("pushed");
    let finish = started.elapsed();
    let started = std::time::Instant::now();
    let bytes = track.encoded_len();
    let encode = started.elapsed();
    assert_eq!(Some(&track), run.replay.pose_track.as_ref(), "{title}");
    println!(
        "{title}: POSE capture {:.1} us a tick over {} ticks; at the end: track() {:.2} ms, \
         encode {:.2} ms ({bytes} bytes); {} KiB held as keys",
        push.as_secs_f64() * 1e6 / run.truth.len() as f64,
        run.truth.len(),
        finish.as_secs_f64() * 1e3,
        encode.as_secs_f64() * 1e3,
        (0..crafts).map(|craft| track.keys(craft)).sum::<usize>() * 20 / 1024,
    );
}

/// Ticks flown before the craft is thrown off the circuit: past the countdown.
const THROW_AT: usize = 600;
/// Ticks flown after the respawn: enough to see the track settle.
const AFTER: usize = 120;

/// A race whose player's craft is put back on the circuit by the game, recorded as any race is.
///
/// A pad flies to [`THROW_AT`], then the test lifts the craft 500 units off the circuit (the
/// same hand on `World` the `reset_zone_ground_truth` test uses), and the game's own rescue puts
/// it back on the racing line: the jump under test is the game's. Both jumps are in the track.
fn respawn(title: &str) {
    let Some(source) = source(title) else {
        return;
    };
    let options = race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    };
    let loaded = race::load(&options).expect("loading the race");
    let seed = loaded.setup.seed;
    let mut race = Race::start(loaded.setup);
    race.start_recording(oag_replay::Header::new(
        title,
        "default",
        "single_race",
        "venom",
        "default",
        seed,
    ));
    let mut pad = Pad::new(title);
    let mut truth: Vec<Vec<Pose>> = vec![grid(&race)];
    let mut thrown = None;
    let mut respawned = None;
    let mut tick = 0;
    while respawned.is_none_or(|at: usize| tick < at + AFTER) {
        assert!(tick < 3_000, "{title}: the rescue never put the craft back");
        if tick == THROW_AT {
            race.sim.world.ships[0].physics.body.position += oag_core::math::Vec3::Y * 500.0;
            thrown = Some(tick);
        }
        let inputs = pad.inputs(&mut race);
        race.tick(&inputs);
        race.drain_cues();
        tick += 1;
        truth.push(grid(&race));
        if respawned.is_none() && race.respawns() > 0 {
            respawned = Some(tick);
        }
    }
    let (thrown, respawned) = (thrown.expect("thrown"), respawned.expect("respawned"));
    let replay = race.recorded().expect("recording");
    let replay = oag_replay::Replay::from_bytes(&replay.to_bytes()).expect("reads");
    let track = replay.pose_track.as_ref().expect("a pose track");
    println!(
        "{title}: thrown after tick {thrown}, respawned at tick {respawned} ({:?}), {} ticks, {} keys",
        race.last_respawn_cause_of(0),
        replay.ticks(),
        track.keys(0)
    );
    let (position, rotation) = errors(track, &truth, 0..=replay.ticks() as usize);
    let (position, rotation) = (percentiles(position), percentiles(rotation));
    println!(
        "{title}: whole run, position max {:.4} units, rotation max {:.3} degrees",
        position[3], rotation[3]
    );
    assert!(position[3] <= MAX_POSITION_ERROR + 1e-3, "{title}");
    assert!(rotation[3] <= MAX_ROTATION_ERROR + 1e-2, "{title}");
    let jump = |at: usize| (truth[at][0].position - truth[at - 1][0].position).length();
    println!(
        "{title}: the player's step into tick {respawned} is {:.1} units",
        jump(respawned)
    );
    assert!(jump(respawned) > 16.0, "{title}: the respawn is a teleport");
    let mut window = String::new();
    for at in respawned - 2..=respawned + 30 {
        let got = track.pose(0, at as f64).expect("a craft");
        window.push_str(&format!(
            " {}:{:.4}u/{:.3}d",
            at as i64 - respawned as i64,
            (got.position - truth[at][0].position).length(),
            degrees_between(got.rotation, truth[at][0].rotation)
        ));
    }
    println!("{title}: error at tick (relative to the respawn) in units / degrees:{window}");
    // A frame between the two ticks of the jump shows the craft at one end, never between.
    let before = track.pose(0, (respawned - 1) as f64).expect("a craft");
    let after = track.pose(0, respawned as f64).expect("a craft");
    for fraction in [0.25, 0.5, 0.75] {
        let between = track
            .pose(0, (respawned - 1) as f64 + fraction)
            .expect("a craft");
        assert_eq!(between, before, "{title}: a frame {fraction} into the jump");
    }
    assert!((after.position - before.position).length() > 16.0, "{title}");
}

fn grid(race: &Race) -> Vec<Pose> {
    (0..usize::from(race.ship_count()))
        .map(|slot| race.craft_pose(slot))
        .collect()
}

macro_rules! respawn_tests {
    ($($name:ident: $title:literal;)*) => {$(
        #[test]
        #[ignore = "needs the title's game data"]
        fn $name() {
            respawn($title);
        }
    )*};
}

respawn_tests! {
    pulse_pose_track_bounds_a_respawn: "pulse";
    pure_pose_track_bounds_a_respawn: "pure";
    hd_pose_track_bounds_a_respawn: "hd";
    wipeout_2048_pose_track_bounds_a_respawn: "2048";
    omega_pose_track_bounds_a_respawn: "omega";
}
