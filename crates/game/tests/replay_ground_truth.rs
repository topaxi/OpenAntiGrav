//! A replay reproduces a real race bit for bit, a changed input is caught,
//! and a ghost changes nothing the simulation hashes - on every title that
//! races.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all replay_ground_truth --no-capture
//! ```
//!
//! # The pilot is a person, not the autopilot
//!
//! The operator's `--autopilot` flies the player's craft *instead of* reading
//! the snapshot, so a run it drives records an input stream of nothing held,
//! and a tampered input in it changes nothing at all: a desync test over it
//! would pass without testing anything. So the craft here is flown through
//! [`InputSnapshot`]s, the way a pad flies it: an [`oag_ai::Driver`] kept
//! *outside* the world decides where to steer, and its controls are turned
//! into a stick and a held cross before they reach [`race::Race::tick`]. The
//! stick moves nearly every tick, which also makes the recorded size the
//! analog worst case rather than a keyboard's best one.
//!
//! One test per title per property, so the matrix is the test axis rather
//! than a loop inside one test - `CLAUDE.md`'s rule for the data suite.

use std::path::{Path, PathBuf};

use oag_game::race;
use oag_gameplay::input::{Button, Input};
use oag_gameplay::{InputSnapshot, PlayerInputs};
use oag_replay::{Header, Replay, Verifier};

/// How long a run is driven for: long enough for two laps on every circuit
/// here, short enough to keep the suite's budget.
const TICKS: u64 = 7_200;

/// Where each title's source is, or `None` without it.
fn source(title: &str) -> Option<PathBuf> {
    let (path, marker) = match title {
        "pulse" => ("data/images/pulse-psp-usa.chd", ""),
        "pure" => ("data/images/pure-psp-eu.chd", ""),
        "hd" => ("data/images/hdfury-ps3-eu-dec.iso", ""),
        "2048" => ("data/extracted/vita/PCSF00007", "base/PSP2/data.psarc"),
        _ => unreachable!("{title}"),
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join(path);
    let present = if marker.is_empty() {
        path.exists()
    } else {
        path.join(marker).exists()
    };
    if present {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn setup(title: &str) -> Option<race::Setup> {
    let source = source(title)?;
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading {title}: {e:#}"));
    Some(loaded.setup)
}

/// A pad, flown by a driver that is not in the world.
///
/// The driver reads the player's craft and decides; its controls become a
/// stick deflection and a held cross, which is all a pad can say. Nothing it
/// keeps is simulation state - it is the person holding the pad.
struct Pad {
    driver: oag_ai::Driver,
    tuning: oag_ai::Tuning,
    input: Input,
}

impl Pad {
    fn new() -> Self {
        Self {
            driver: oag_ai::Driver::default(),
            tuning: oag_ai::Tuning::default(),
            input: Input::new(),
        }
    }

    fn inputs(&mut self, race: &race::Race) -> PlayerInputs {
        let ctx = oag_ai::Context::new(race.racing_line(), &self.tuning);
        let controls = self.driver.drive(&race.ship().physics, &ctx);
        let mut held = 0;
        if controls.thrust > 0.5 {
            held |= Button::Cross.bit();
        }
        self.input.begin_frame(held);
        PlayerInputs::single(
            InputSnapshot {
                buttons: self.input,
                stick_x: controls.steer_x,
                stick_y: 0.0,
                airbrake_left: controls.airbrake_left,
                airbrake_right: controls.airbrake_right,
            }
            .sanitised(),
        )
    }
}

fn header(title: &str) -> Header {
    Header::new(title, "default", "time_trial", "venom", "default", 0)
}

/// Drives a recorded run of [`TICKS`] and hands back the race and its
/// recording.
fn record(title: &str) -> Option<(race::Race, Replay)> {
    let mut race = race::Race::start(setup(title)?);
    race.start_recording(header(title));
    let mut pad = Pad::new();
    for _ in 0..TICKS {
        let inputs = pad.inputs(&race);
        race.tick(&inputs);
        race.drain_cues();
    }
    let replay = race.recorded().expect("recording");
    Some((race, replay))
}

/// Re-drives `replay` on a fresh race, checking every stored hash.
fn verify(title: &str, replay: &Replay) -> Result<u64, oag_replay::Desync> {
    let mut race = race::Race::start(setup(title).expect("source present"));
    Verifier::run(replay, race.sim.state_hash(), |inputs| {
        race.tick(inputs);
        race.drain_cues();
        race.sim.state_hash()
    })
}

fn round_trip(title: &str) {
    let Some((mut race, replay)) = record(title) else {
        return;
    };
    // The file a session saves as a ghost: cut at the end of the best lap,
    // and still a replay that re-drives to the last tick it kept.
    let ghost = race.take_new_best_ghost().expect("a lap was completed");
    let info = ghost
        .header
        .ghost
        .expect("the ghost's lap is in the header");
    assert_eq!(
        ghost.ticks(),
        info.start_tick + u64::from(info.lap_ticks),
        "{title}: cut at the end of its lap"
    );
    assert_eq!(
        verify(
            title,
            &Replay::from_bytes(&ghost.to_bytes()).expect("reads")
        ),
        Ok(ghost.ticks()),
        "{title}: a saved ghost file's inputs reproduce up to its lap's end"
    );
    let bytes = replay.to_bytes();
    let read = Replay::from_bytes(&bytes).expect("reads back");
    assert_eq!(read, replay, "{title}: the file round-trips");
    let inputs = oag_replay::codec::encode_inputs(&replay.inputs[0]).len();
    let ghost = replay.ghost.as_ref().map_or(0, |lap| lap.poses.len());
    println!(
        "{title}: {} ticks, lap {} reached, best lap {:?} ticks; file {} bytes \
         (inputs {inputs} bytes = {:.2} bytes/tick, {} hashes, ghost {ghost} poses)",
        replay.ticks(),
        race.player_standing().lap,
        race.player_standing().best_lap_ticks,
        bytes.len(),
        inputs as f64 / replay.ticks() as f64,
        replay.hashes.len(),
    );
    assert_eq!(
        verify(title, &read),
        Ok(TICKS),
        "{title}: a replay of an untouched run reproduces every stored hash"
    );
}

/// The desync test on a real race: one stick deflection flipped a little
/// past the countdown, where steering is consumed, must be caught at the next
/// stored hash.
fn tamper(title: &str) {
    let Some((_, mut replay)) = record(title) else {
        return;
    };
    let tick = oag_race::state::COUNTDOWN_TICKS + 100;
    let index = usize::try_from(tick).expect("fits");
    let snapshot = &mut replay.inputs[0][index];
    snapshot.stick_x = if snapshot.stick_x > 0.0 { -1.0 } else { 1.0 };
    let desync = verify(title, &replay).expect_err("a changed stick must desync");
    println!(
        "{title}: tampered tick {tick}, desync reported at {}",
        desync.tick
    );
    assert!(
        desync.tick > tick && desync.tick <= tick + 60,
        "{title}: caught at the first stored hash after the change, got {}",
        desync.tick
    );
}

/// A ghost is outside the simulation: the same inputs with and without one
/// hash identically, tick for tick - and the ghost is really there.
fn ghost_changes_nothing(title: &str) {
    let Some((_, replay)) = record(title) else {
        return;
    };
    let lap = replay
        .ghost
        .clone()
        .unwrap_or_else(|| panic!("{title}: the pad completed no lap in {TICKS} ticks"));
    let mut plain = race::Race::start(setup(title).expect("present"));
    let mut haunted = race::Race::start(setup(title).expect("present"));
    haunted.set_ghost(race::Ghost {
        lap,
        team: "default".to_string(),
    });
    // The haunted race records too, so the recorder is covered by the same
    // equality.
    haunted.start_recording(header(title));
    let mut seen = 0u64;
    for tick in 0..replay.ticks() {
        let inputs = replay.inputs_at(tick);
        plain.tick(&inputs);
        plain.drain_cues();
        haunted.tick(&inputs);
        haunted.drain_cues();
        assert_eq!(
            plain.sim.state_hash(),
            haunted.sim.state_hash(),
            "{title}: tick {tick} hashes differently with a ghost"
        );
        if let Some((pose, _)) = haunted.ghost_pose() {
            assert!(pose.position.is_finite(), "{title}: tick {tick}");
            seen += 1;
        }
    }
    println!(
        "{title}: ghost on the circuit for {seen} of {} ticks",
        replay.ticks()
    );
    assert!(seen > 600, "{title}: the ghost was raced for {seen} ticks");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_a_replay_reproduces_every_stored_hash() {
    round_trip("pulse");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_a_tampered_input_is_caught() {
    tamper("pulse");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_a_ghost_changes_no_state_hash() {
    ghost_changes_nothing("pulse");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_a_replay_reproduces_every_stored_hash() {
    round_trip("pure");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_a_tampered_input_is_caught() {
    tamper("pure");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_a_ghost_changes_no_state_hash() {
    ghost_changes_nothing("pure");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_a_replay_reproduces_every_stored_hash() {
    round_trip("hd");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_a_ghost_changes_no_state_hash() {
    ghost_changes_nothing("hd");
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn wipeout_2048_a_replay_reproduces_every_stored_hash() {
    round_trip("2048");
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn wipeout_2048_a_ghost_changes_no_state_hash() {
    ghost_changes_nothing("2048");
}
