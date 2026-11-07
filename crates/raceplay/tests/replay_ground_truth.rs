//! A replay reproduces a real race bit for bit, a changed input is caught,
//! and a ghost changes nothing the simulation hashes - on every title that
//! races.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all replay_ground_truth --no-capture
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
//! One test per title, each property a named function over that title's one
//! recording. Until 2026-10-13 it was one test per title per property, and
//! each recorded the same 7,200-tick run again; the recording is deterministic
//! (the same setup and the same pad), so every property still sees exactly the
//! run it saw, and the title stays the test axis.

use std::path::{Path, PathBuf};

use oag_gameplay::input::{Button, Input};
use oag_gameplay::{InputSnapshot, PlayerInputs};
use oag_raceplay as race;
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
        "omega" => ("data/images/omega-ps4-eu.pkg", ""),
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

/// The launch request a test races, over `source`.
fn options(source: &Path, mode: oag_race::Mode) -> race::Options {
    race::Options {
        source: source.display().to_string(),
        mode,
        ..race::Options::default()
    }
}

fn setup(title: &str) -> Option<race::Setup> {
    setup_in(title, oag_race::Mode::TimeTrial)
}

fn setup_in(title: &str, mode: oag_race::Mode) -> Option<race::Setup> {
    let source = source(title)?;
    let loaded =
        race::load(&options(&source, mode)).unwrap_or_else(|e| panic!("loading {title}: {e:#}"));
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
    /// `Some` when the pad is the race's own driver for the player's craft
    /// ([`race::Race::autopilot_wants`]): the speed plan, the field and the
    /// hull's yaw ceiling, kept outside the world, as `--net-bot` flies.
    race_driver: Option<Option<oag_ai::Driver>>,
    /// Whether the bare driver has been found on the line yet. Its search is
    /// windowed round its last index, so it starts from a whole-line search,
    /// as `Race::start` seats each opponent: a grid far from line index 0
    /// (Pure's Vineta K, about sample 817) otherwise leaves it aiming at a
    /// sample hundreds of units away, into the first wall.
    seated: bool,
}

/// The titles whose pad is the race's own driver rather than a bare
/// [`oag_ai::Driver`] on the line alone, so the file also records the pad
/// `--net-bot` flies.
const RACE_DRIVER: [&str; 1] = ["pure"];

impl Pad {
    fn new(title: &str) -> Self {
        Self {
            driver: oag_ai::Driver::default(),
            tuning: oag_ai::Tuning::default(),
            input: Input::new(),
            race_driver: RACE_DRIVER.contains(&title).then_some(None),
            seated: false,
        }
    }

    fn inputs(&mut self, race: &mut race::Race) -> PlayerInputs {
        let controls = if let Some(driver) = self.race_driver.as_mut() {
            let slot = race.sim.world.primary_slot();
            race.autopilot_wants(slot, driver)
        } else {
            let line = race.racing_line();
            if !self.seated {
                let position = race.ship().physics.body.position;
                self.driver.index = line.nearest(position, 0, line.len()) as u32;
                self.seated = true;
            }
            let ctx = oag_ai::Context::new(line, &self.tuning);
            self.driver.drive(&race.ship().physics, &ctx)
        };
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

/// The header a session writes: the seed is the race's own, which a rebuild loads it with.
fn header(title: &str, seed: u64) -> Header {
    Header::new(title, "default", "time_trial", "venom", "default", seed)
}

/// Drives a recorded run of [`TICKS`] and hands back the race and its
/// recording.
fn record(title: &str) -> Option<(race::Race, Replay)> {
    let setup = setup(title)?;
    let seed = setup.seed;
    let mut race = race::Race::start(setup);
    race.start_recording(header(title, seed));
    let mut pad = Pad::new(title);
    for _ in 0..TICKS {
        let inputs = pad.inputs(&mut race);
        race.tick(&inputs);
        race.drain_cues();
    }
    let replay = race.recorded().expect("recording");
    Some((race, replay))
}

/// Re-drives `replay` on a race **rebuilt from its header alone** (the title's source is the
/// reader's own), applying the recorded events between the same two ticks, and checks every
/// stored hash.
fn verify(title: &str, replay: &Replay) -> Result<u64, oag_replay::Desync> {
    let source = source(title).expect("source present");
    let mut race = race::rebuild(&replay.header, &source.display().to_string())
        .unwrap_or_else(|e| panic!("{title}: rebuilding from the header: {e:#}"));
    Verifier::run(replay, race.sim.state_hash(), |events, inputs| {
        for event in events {
            race.apply_event(event)
                .unwrap_or_else(|e| panic!("{title}: {e}"));
        }
        race.tick(inputs);
        race.drain_cues();
        race.sim.state_hash()
    })
}

/// Every property, on one recording of `title`.
fn properties(title: &str) {
    let Some((race, replay)) = record(title) else {
        return;
    };
    if let Some(tampered) = TAMPERED.iter().find(|t| **t == title) {
        tamper(tampered, replay.clone());
    }
    ghost_changes_nothing(title, &replay);
    round_trip(title, race, replay);
}

/// The titles the tamper property is asserted on.
const TAMPERED: [&str; 2] = ["pulse", "pure"];

fn round_trip(title: &str, mut race: race::Race, replay: Replay) {
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
fn tamper(title: &str, mut replay: Replay) {
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
fn ghost_changes_nothing(title: &str, replay: &Replay) {
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
    haunted.start_recording(header(title, replay.header.seed));
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
fn pulse_replays_reproduce_catch_a_tampered_input_and_ghosts_change_nothing() {
    properties("pulse");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_replays_reproduce_catch_a_tampered_input_and_ghosts_change_nothing() {
    properties("pure");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_replays_reproduce_and_ghosts_change_nothing() {
    properties("hd");
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn wipeout_2048_replays_reproduce_and_ghosts_change_nothing() {
    properties("2048");
}

/// A bare pad finds its craft on the whole line before its windowed search takes over. Pure's
/// Vineta K grid sits near line sample 817, far outside a search round sample 0: unseated, the
/// pad aimed at a sample about 830 units away and pinned itself on the start straight's first
/// wall. Fails if the seating is dropped.
#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn a_bare_pad_starts_on_the_line_where_the_grid_is_far_from_sample_zero() {
    let Some(setup) = setup("pure") else {
        return;
    };
    let mut race = race::Race::start(setup);
    let mut pad = Pad::new("pure");
    pad.race_driver = None;
    let _ = pad.inputs(&mut race);
    let line = race.racing_line();
    let gap = line
        .point(pad.driver.index as usize)
        .distance(race.ship().physics.body.position);
    assert!(
        gap < 20.0,
        "the pad's line sample is {gap} units from its craft"
    );
}

/// A race started from `title`'s source in `mode`, with non-default launch settings, recorded
/// under a header and not ticked.
fn recorded_at_tick_zero(
    title: &str,
    mode: oag_race::Mode,
) -> Option<(PathBuf, race::Race, Replay, race::Launch)> {
    let source = source(title)?;
    let loaded = race::load(&options(&source, mode))
        .unwrap_or_else(|e| panic!("loading {title} {}: {e:#}", mode.name()));
    let seed = loaded.setup.seed;
    let mut race = race::Race::start(loaded.setup);
    let launch = race::Launch {
        scheme: oag_gameplay::ControlScheme::Novice,
        difficulty: oag_ai::Difficulty::Ace,
        autopilot: false,
        pilot_assist: oag_physics::pilot_assist::Level::Normal,
    };
    race.apply_launch(&launch);
    let mut header = Header::new(title, "default", mode.name(), "venom", "default", seed);
    header.options = launch.to_table();
    race.start_recording(header);
    let replay = race.recorded().expect("recording");
    Some((source, race, replay, launch))
}

/// The rebuild from the file's header alone agrees with the recorded race at tick zero, on the
/// bytes a file is written as and read back from.
fn tick_zero(title: &str, mode: oag_race::Mode) {
    let Some((source, race, replay, launch)) = recorded_at_tick_zero(title, mode) else {
        return;
    };
    assert_eq!(replay.initial_hash, race.sim.state_hash());
    let read = Replay::from_bytes(&replay.to_bytes()).expect("reads back");
    let setup = read
        .header
        .setup
        .clone()
        .expect("the header carries the request");
    assert_eq!(
        setup.mode,
        mode.name(),
        "{title}: the request names its mode"
    );
    assert_eq!(race::Launch::from_table(&read.header.options), Ok(launch));

    let rebuilt = race::rebuild(&read.header, &source.display().to_string()).expect("rebuilds");
    Verifier::new(&read, rebuilt.sim.state_hash()).unwrap_or_else(|d| {
        panic!(
            "{title} {}: the rebuild disagrees at tick zero: {d}",
            mode.name()
        )
    });
    assert_eq!(rebuilt.pilot_assist_level(), launch.pilot_assist);
    assert_eq!(rebuilt.control_scheme(), launch.scheme);

    // The request is what rebuilds it: the same file with another mode in it is another race,
    // refused at tick zero.
    let other = if mode == oag_race::Mode::SingleRace {
        oag_race::Mode::TimeTrial
    } else {
        oag_race::Mode::SingleRace
    };
    let mut swapped = read.clone();
    swapped.header.setup.as_mut().expect("present").mode = other.name().to_string();
    let rebuilt = race::rebuild(&swapped.header, &source.display().to_string()).expect("rebuilds");
    let desync = Verifier::new(&swapped, rebuilt.sim.state_hash())
        .expect_err("a request naming another mode is another race");
    assert_eq!(desync.tick, 0);

    // A header without it is refused by name before any load.
    let mut bare = read.header.clone();
    bare.setup = None;
    let refused = race::rebuild(&bare, &source.display().to_string())
        .err()
        .expect("no request, no race");
    assert!(format!("{refused:#}").contains("[setup]"), "{refused:#}");
}

macro_rules! tick_zero_tests {
    ($($name:ident: $title:literal, $mode:ident;)*) => {$(
        #[test]
        #[ignore = "needs the title's game data"]
        fn $name() {
            tick_zero($title, oag_race::Mode::$mode);
        }
    )*};
}

tick_zero_tests! {
    pulse_time_trial_rebuilds_from_its_header_at_tick_zero: "pulse", TimeTrial;
    pulse_speed_lap_rebuilds_from_its_header_at_tick_zero: "pulse", SpeedLap;
    pulse_single_race_rebuilds_from_its_header_at_tick_zero: "pulse", SingleRace;
    pure_time_trial_rebuilds_from_its_header_at_tick_zero: "pure", TimeTrial;
    pure_speed_lap_rebuilds_from_its_header_at_tick_zero: "pure", SpeedLap;
    pure_single_race_rebuilds_from_its_header_at_tick_zero: "pure", SingleRace;
    hd_time_trial_rebuilds_from_its_header_at_tick_zero: "hd", TimeTrial;
    hd_speed_lap_rebuilds_from_its_header_at_tick_zero: "hd", SpeedLap;
    hd_single_race_rebuilds_from_its_header_at_tick_zero: "hd", SingleRace;
    wipeout_2048_time_trial_rebuilds_from_its_header_at_tick_zero: "2048", TimeTrial;
    wipeout_2048_speed_lap_rebuilds_from_its_header_at_tick_zero: "2048", SpeedLap;
    wipeout_2048_single_race_rebuilds_from_its_header_at_tick_zero: "2048", SingleRace;
    omega_time_trial_rebuilds_from_its_header_at_tick_zero: "omega", TimeTrial;
    omega_speed_lap_rebuilds_from_its_header_at_tick_zero: "omega", SpeedLap;
    omega_single_race_rebuilds_from_its_header_at_tick_zero: "omega", SingleRace;
}

/// A mid-race Pilot Assist change and a handed-over weapon are recorded as events: the file
/// reproduces with them applied between the same two ticks, and each one dropped desyncs.
fn mid_race_events(title: &str) {
    use oag_physics::pilot_assist::Level;
    let Some((_, mut race, _, _)) = recorded_at_tick_zero(title, oag_race::Mode::TimeTrial) else {
        return;
    };
    let assist_at = oag_race::state::COUNTDOWN_TICKS + 60;
    // On the grid, where the pickup slot is empty: a top-up only writes an empty one.
    let give_at = 10;
    let ticks = oag_race::state::COUNTDOWN_TICKS + 420;
    assert!(
        race.pilot_assist_available(Level::Extreme),
        "{title}: no Extreme law, so the change would prove nothing"
    );
    let mut pad = Pad::new(title);
    for tick in 0..ticks {
        if tick == assist_at {
            race.change_pilot_assist(Level::Extreme);
        }
        if tick == give_at {
            assert!(
                race.give_weapon(0, oag_tables::weapons::Weapon::Rocket),
                "{title}: the slot holds {:?}",
                race.sim.world.ships[0].pickup.weapon
            );
        }
        let inputs = pad.inputs(&mut race);
        race.tick(&inputs);
        race.drain_cues();
    }
    let replay =
        Replay::from_bytes(&race.recorded().expect("recording").to_bytes()).expect("reads");
    assert_eq!(
        replay.events,
        vec![
            oag_replay::Event::new(give_at, 0, oag_replay::Kind::Give, 0),
            oag_replay::Event::new(assist_at, 0, oag_replay::Kind::PilotAssist, 2),
        ],
        "{title}: the changes are in the file, between the ticks they were made between"
    );
    assert_eq!(
        verify(title, &replay),
        Ok(ticks),
        "{title}: reproduces with its events"
    );

    for (dropped, keep) in [
        ("both", None),
        ("the Pilot Assist change", Some(oag_replay::Kind::Give)),
        ("the weapon", Some(oag_replay::Kind::PilotAssist)),
    ] {
        let mut stripped = replay.clone();
        stripped
            .events
            .retain(|e| keep.is_some_and(|kind| e.kind() == Some(kind)));
        let desync = verify(title, &stripped)
            .expect_err(&format!("{title}: a replay without {dropped} must desync"));
        println!("{title}: without {dropped}, desync at {}", desync.tick);
    }
}

macro_rules! event_tests {
    ($($name:ident: $title:literal;)*) => {$(
        #[test]
        #[ignore = "needs the title's game data"]
        fn $name() {
            mid_race_events($title);
        }
    )*};
}

event_tests! {
    pulse_records_a_mid_race_pilot_assist_change_and_reproduces: "pulse";
    hd_records_a_mid_race_pilot_assist_change_and_reproduces: "hd";
}
