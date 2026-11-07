//! A seek reproduces the recorded run on every title, over a recording a pad flew: ADR-0087,
//! stage 3 of decision 10.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content. The fixture's half is
//! `crates/raceplay/src/tests/timeline.rs`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^replay_timeline_ground_truth::/)' --no-capture
//! ```
//!
//! A Single Race of eight craft, the one decision 3 measured, not `replay_ground_truth`'s Time
//! Trial, where one craft leaves the field, the weapons and the wrecks out of what a seek must
//! bring back. The stored hashes cannot judge a seek: they fall on keyframe ticks, where a seek
//! resimulates nothing. So the recording run keeps its own hash of every tick, and the `Debug`
//! text of a few, and the targets sit off the keyframes. The text is also the only oracle for
//! Pilot Assist's level, which no hash reads, so it is taken right after the two changes.
//!
//! The opening pass's cost is a sweep over the same run, `OAG_SWEEP=1` and five minutes of
//! ticks, under `--cargo-profile release` (`docs/architecture/replay-timeline.md`).

use std::path::Path;
use std::time::Instant;

use oag_gameplay::{Controller, PlayerInputs};
use oag_physics::pilot_assist::Level;
use oag_raceplay::photo::rewind::Rewind;
use oag_raceplay::timeline::{KEYFRAME_TICKS, Keeper, Pass, Timeline};
use oag_raceplay::{self as race, Launch, Race};
use oag_replay::{Header, Pose, Replay};

use crate::replay_ground_truth::{Pad, source};

/// The targets a run is checked at.
const TARGETS: usize = 40;
/// A run goes this long at least, then to the finish, then a little past it.
const SHORTEST: u64 = 6_700;
const PAST_THE_FINISH: u64 = 300;
const LONGEST: u64 = 14_500;
/// Pilot Assist changes on a keyframe's own tick, then among the ticks a seek resimulates.
const ASSIST_ON: u64 = 9 * KEYFRAME_TICKS;
const ASSIST_OFF: u64 = ASSIST_ON + 37;
/// Where the run's text is kept: straight after each change, then two ticks far apart.
const TEXT_AT: [u64; 4] = [ASSIST_ON + 1, ASSIST_OFF + 1, 3_617, 6_617];

fn launch() -> Launch {
    Launch {
        scheme: oag_gameplay::ControlScheme::default(),
        difficulty: oag_ai::Difficulty::default(),
        autopilot: false,
        pilot_assist: Level::Off,
    }
}

fn single_race(source: &Path) -> race::Options {
    race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    }
}

fn header_of(title: &str, mode: oag_race::Mode, seed: u64) -> Header {
    let mut header = Header::new(title, "default", mode.name(), "venom", "default", seed);
    header.options = launch().to_table();
    header
}

/// A recorded run, and what it looked like: the hash after every tick, the text at [`TEXT_AT`].
pub(crate) struct Run {
    pub(crate) replay: Replay,
    pub(crate) hashes: Vec<u64>,
    text: Vec<(u64, String)>,
    /// Every craft's drawn pose after tick `k`, at index `k`: the truth a pose track is judged
    /// against.
    pub(crate) truth: Vec<Vec<Pose>>,
}

fn watch(run: &mut Run, race: &Race) {
    let tick = run.hashes.len() as u64;
    run.hashes.push(race.sim.state_hash());
    run.truth.push(
        (0..usize::from(race.ship_count()))
            .map(|slot| race.craft_pose(slot))
            .collect(),
    );
    if TEXT_AT.contains(&tick) {
        run.text.push((tick, format!("{:?}", race.sim)));
    }
}

/// The level this title has a law for, if any: a change to a level with no law moves nothing.
fn assist_level(race: &Race) -> Option<Level> {
    [Level::Extreme, Level::Normal]
        .into_iter()
        .find(|level| race.pilot_assist_available(*level))
}

/// What the run changes outside the tick, through the writers a session calls, so that it is
/// in the recording.
fn write_between_ticks(race: &mut Race, assist: Option<Level>, tick: u64) {
    if let Some(level) = assist {
        if tick == ASSIST_ON {
            race.change_pilot_assist(level);
        }
        if tick == ASSIST_OFF {
            race.change_pilot_assist(Level::Off);
        }
    }
    if tick == 10 {
        assert!(race.give_weapon(0, oag_tables::weapons::Weapon::Rocket));
    }
}

pub(crate) fn record(title: &str, ticks: Option<u64>) -> Option<(std::path::PathBuf, Run)> {
    let source = source(title)?;
    let loaded = race::load(&single_race(&source)).expect("loading the race");
    let seed = loaded.setup.seed;
    let mut race = Race::start(loaded.setup);
    race.apply_launch(&launch());
    race.start_recording(header_of(title, oag_race::Mode::SingleRace, seed));
    let assist = assist_level(&race);
    let mut run = Run {
        replay: race.recorded().expect("recording"),
        hashes: Vec::new(),
        text: Vec::new(),
        truth: Vec::new(),
    };
    watch(&mut run, &race);
    let mut pad = Pad::new(title);
    let mut tick = 0;
    let mut finished_at = None;
    loop {
        let enough = match (ticks, finished_at) {
            (Some(ticks), _) => tick >= ticks,
            (None, Some(at)) => tick >= SHORTEST.max(at + PAST_THE_FINISH),
            (None, None) => tick >= LONGEST,
        };
        if enough {
            break;
        }
        write_between_ticks(&mut race, assist, tick);
        let inputs = pad.inputs(&mut race);
        race.tick(&inputs);
        race.drain_cues();
        tick += 1;
        watch(&mut run, &race);
        if finished_at.is_none() && race.finished() {
            finished_at = Some(tick);
        }
    }
    run.replay =
        Replay::from_bytes(&race.recorded().expect("recording").to_bytes()).expect("reads");
    println!(
        "{title}: {tick} ticks, finished at {finished_at:?}, lap {}, {} events",
        race.player_standing().lap,
        run.replay.events.len()
    );
    Some((source, run))
}

/// [`TARGETS`] ticks off the keyframes, at every distance from one a seek can be, in an order
/// that goes back as often as forward: a probe that kept something of a later tick shows it.
fn targets(ticks: u64) -> Vec<u64> {
    let span = ticks - 2;
    let count = TARGETS as u64;
    let mut targets: Vec<u64> = (0..count)
        .map(|i| {
            let base = (1 + span * i / count) / KEYFRAME_TICKS * KEYFRAME_TICKS;
            base + [1, 59, 17, 30, 43][usize::try_from(i % 5).expect("small")]
        })
        .collect();
    targets.push(ticks);
    targets.sort_by_key(|tick| (tick * 2_654_435_761) % 4_294_967_291);
    targets
}

fn same_text(title: &str, tick: u64, timeline: &Timeline, recorded: &str) {
    let text = format!("{:?}", timeline.probe().sim);
    if let Some(at) = text.bytes().zip(recorded.bytes()).position(|(a, b)| a != b) {
        let window = |s: &str| s[at.saturating_sub(160)..(at + 80).min(s.len())].to_string();
        panic!(
            "{title}: the text at tick {tick} differs:\n{}\n{}",
            window(&text),
            window(recorded)
        );
    }
    assert_eq!(text.len(), recorded.len(), "{title}: tick {tick}");
}

/// The race a file names, loaded from its header alone.
fn file_setup(source: &Path, run: &Run) -> race::Setup {
    race::replay::load_recorded(&run.replay.header, &source.display().to_string())
        .expect("rebuilds")
}

/// Opens the file as a viewer does, seeking to the prefix's end while the pass runs.
fn opened(title: &str, setup: race::Setup, run: &Run) -> Timeline {
    let mut timeline = Timeline::open(setup, run.replay.clone()).expect("agrees at tick zero");
    let mut during = 0;
    while timeline.pass() == Pass::Running {
        if let Some(prefix) = timeline.prefix() {
            assert_eq!(timeline.seek(prefix), Ok(prefix), "{title}");
            assert_eq!(
                timeline.probe().sim.state_hash(),
                run.hashes[prefix as usize],
                "{title}: tick {prefix}, sought while the pass ran"
            );
            during += 1;
        }
    }
    assert_eq!(timeline.pass(), Pass::Complete, "{title}");
    assert_eq!(timeline.prefix(), Some(run.replay.ticks()), "{title}");
    println!("{title}: {during} seeks while the pass ran");
    timeline
}

/// Whether the probe has the speed plans, so that it will not learn them on the caller's
/// thread. Fails on a keyframe 0 from before the first tick.
fn carries_the_plans(title: &str, timeline: &mut Timeline) {
    assert_eq!(timeline.seek(12), Ok(12), "{title}");
    let text = format!("{:?}", timeline.probe().sim);
    assert!(text.contains("plans_built: true"), "{title}");
}

fn seeks_reproduce(title: &str, timeline: &mut Timeline, run: &Run) {
    carries_the_plans(title, timeline);
    let list = targets(run.replay.ticks());
    assert!(list.len() > TARGETS, "{title}");
    let mut ms = Vec::new();
    for &target in &list {
        let clock = Instant::now();
        assert_eq!(timeline.seek(target), Ok(target), "{title}");
        ms.push(clock.elapsed().as_secs_f64() * 1e3);
        assert_eq!(
            timeline.probe().sim.state_hash(),
            run.hashes[target as usize],
            "{title}: the hash at tick {target} after a seek"
        );
    }
    for (tick, recorded) in &run.text {
        assert_eq!(timeline.seek(*tick), Ok(*tick), "{title}");
        same_text(title, *tick, timeline, recorded);
    }
    ms.sort_by(f64::total_cmp);
    println!(
        "{title}: {} seeks by hash, {} by text; median {:.2} ms, worst {:.2} ms, {} keyframes",
        list.len(),
        run.text.len(),
        ms[ms.len() / 2],
        ms[ms.len() - 1],
        timeline.keyframes()
    );
}

/// A second drive of the run with a visit in the middle: the live race's hash stays the
/// recorded one at every tick after it, and its text is the same either side.
fn a_visit_changes_nothing(title: &str, setup: race::Setup, run: &Run) {
    let visit_at = 1_500;
    let seed = setup.seed;
    let mut live = Race::start(setup.clone());
    live.apply_launch(&launch());
    live.start_recording(header_of(title, oag_race::Mode::SingleRace, seed));
    let assist = assist_level(&live);
    let mut timeline = Timeline::live(setup, &live);
    let early = 12;
    let mut early_text = String::new();
    for tick in 0..visit_at + 300 {
        write_between_ticks(&mut live, assist, tick);
        live.tick(&run.replay.inputs_at(tick));
        live.drain_cues();
        timeline.keep(&live);
        if tick + 1 == early {
            early_text = format!("{:?}", live.sim);
        }
        if tick + 1 == visit_at {
            let before = format!("{:?}", live.sim);
            timeline.visit(&live);
            for target in [visit_at, 73, 1_259, visit_at - 1] {
                assert_eq!(timeline.seek(target), Ok(target), "{title}");
                assert_eq!(
                    timeline.probe().sim.state_hash(),
                    run.hashes[target as usize],
                    "{title}: tick {target} on a visit"
                );
            }
            carries_the_plans(title, &mut timeline);
            same_text(title, early, &timeline, &early_text);
            assert!(
                format!("{:?}", live.sim) == before,
                "{title}: the visit wrote"
            );
        }
        assert_eq!(
            live.sim.state_hash(),
            run.hashes[tick as usize + 1],
            "{title}: the live race at tick {}",
            tick + 1
        );
    }
    assert_eq!(timeline.probe().sim.world.tick, 12, "{title}");
}

/// The file's rebuild and pass reproduce; a ceiling a third of the keyframes doubles the
/// interval twice and a seek still arrives, from further away.
fn thinned(title: &str, setup: race::Setup, run: &Run, whole: &Timeline) {
    let one = 32 * 1024;
    let ceiling = whole.keyframes() * one / 3;
    let mut timeline = Timeline::open_within(setup, run.replay.clone(), ceiling).expect("opens");
    assert_eq!(timeline.wait(), Pass::Complete, "{title}");
    assert_eq!(timeline.interval(), 4 * KEYFRAME_TICKS, "{title}");
    let target = 240 * 5 + 239;
    assert_eq!(timeline.seek(target), Ok(target), "{title}");
    assert_eq!(
        timeline.probe().sim.state_hash(),
        run.hashes[target as usize],
        "{title}: tick {target}, 239 ticks from its keyframe"
    );
}

fn seek_reproduces_the_run(title: &str) {
    let Some((source, run)) = record(title, None) else {
        return;
    };
    crate::pose_track_ground_truth::bounds(title, &run);
    let setup = file_setup(&source, &run);
    let mut timeline = opened(title, setup.clone(), &run);
    seeks_reproduce(title, &mut timeline, &run);
    if title == "pulse" {
        thinned(title, setup.clone(), &run, &timeline);
    }
    a_visit_changes_nothing(title, setup, &run);
}

macro_rules! seek_tests {
    ($($name:ident: $title:literal;)*) => {$(
        #[test]
        #[ignore = "needs the title's game data"]
        fn $name() {
            seek_reproduces_the_run($title);
        }
    )*};
}

seek_tests! {
    pulse_seeks_reproduce_a_pad_flown_race: "pulse";
    pure_seeks_reproduce_a_pad_flown_race: "pure";
    hd_seeks_reproduce_a_pad_flown_race: "hd";
    wipeout_2048_seeks_reproduce_a_pad_flown_race: "2048";
    omega_seeks_reproduce_a_pad_flown_race: "omega";
}

/// Photo mode's rewind on the title's own data (ADR-0087, decision 5): the race on screen is moved
/// through its past by the recording its own session wrote, lands on the recorded hash at every
/// tick it is sent to, and is put back to the bit: the text of its simulation is the same either
/// side, the recording holds the ticks it held, and every tick after the visit hashes as the
/// recorded run does. Fails if a seek plays the wrong inputs, if `end` leaves anything of the past
/// in the live race, or if a rewound tick is written into the recording.
fn a_rewind_changes_nothing(title: &str) {
    let Some((source, run)) = record(title, Some(1_900)) else {
        return;
    };
    let setup = file_setup(&source, &run);
    let seed = setup.seed;
    let mut live = Race::start(setup.clone());
    live.apply_launch(&launch());
    live.start_recording(header_of(title, oag_race::Mode::SingleRace, seed));
    let assist = assist_level(&live);
    let mut keeper = Keeper::new(setup, &live);
    let visit_at = 1_500;
    for tick in 0..visit_at + 300 {
        write_between_ticks(&mut live, assist, tick);
        live.tick(&run.replay.inputs_at(tick));
        live.drain_cues();
        keeper.keep(&live);
        if tick + 1 == visit_at {
            let before = format!("{:?}", live.sim);
            let recorded = live.recorded().expect("recording").ticks();
            let mut rewind = Rewind::begin(&mut keeper, &mut live).expect("a running race");
            assert!(rewind.resimulates(), "{title}");
            for target in [visit_at, 73, 1_259, 0, 61, visit_at - 1, 1_200, 540, 541, visit_at] {
                assert_eq!(rewind.seek(&mut keeper, &mut live, target), target, "{title}");
                assert_eq!(
                    live.sim.state_hash(),
                    run.hashes[target as usize],
                    "{title}: tick {target} on a rewind"
                );
            }
            rewind.end(&mut live);
            assert!(
                format!("{:?}", live.sim) == before,
                "{title}: the rewind left the live race changed"
            );
            assert_eq!(
                live.recorded().expect("still recording").ticks(),
                recorded,
                "{title}: the rewound ticks are in the recording"
            );
        }
        assert_eq!(
            live.sim.state_hash(),
            run.hashes[tick as usize + 1],
            "{title}: the live race at tick {}",
            tick + 1
        );
    }
}

macro_rules! rewind_tests {
    ($($name:ident: $title:literal;)*) => {$(
        #[test]
        #[ignore = "needs the title's game data"]
        fn $name() {
            a_rewind_changes_nothing($title);
        }
    )*};
}

rewind_tests! {
    pulse_rewinds_leave_the_live_race_to_the_bit: "pulse";
    pure_rewinds_leave_the_live_race_to_the_bit: "pure";
    hd_rewinds_leave_the_live_race_to_the_bit: "hd";
    wipeout_2048_rewinds_leave_the_live_race_to_the_bit: "2048";
    omega_rewinds_leave_the_live_race_to_the_bit: "omega";
}

/// A split-screen recording, set up as `head_to_head_people_ground_truth` sets one up: two
/// pads, one a seat, and a race whose second seat is a person only because the session said so
/// after [`Race::start`]. So the seats have to come from the header: fails at tick zero if
/// `start_recorded` leaves them out, which the first assertion below shows it would.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_split_screen_recording_reproduces() {
    let title = "pulse";
    let Some(source) = source(title) else {
        return;
    };
    let mode = oag_race::Mode::Head2Head;
    let options = race::Options {
        views: 2,
        mode,
        ..single_race(&source)
    };
    let setup = race::load(&options).expect("loading the race").setup;
    let seed = setup.seed;
    let mut race = Race::start(setup.clone());
    let unseated = race.sim.state_hash();
    race.apply_launch(&launch());
    assert_eq!(race.set_viewport_count(2), 2);
    for slot in 0..2 {
        race.sim.world.controllers[slot] = Controller::Local;
    }
    assert_ne!(
        race.sim.state_hash(),
        unseated,
        "the seats are in the hash, so a rebuild has to restore them"
    );
    race.start_recording(header_of(title, mode, seed));
    let mut run = Run {
        replay: race.recorded().expect("recording"),
        hashes: Vec::new(),
        text: Vec::new(),
        truth: Vec::new(),
    };
    watch(&mut run, &race);
    let mut pads = [Pad::new(title), Pad::new(title)];
    for _ in 0..7_000 {
        let mut inputs = PlayerInputs::none();
        for (slot, pad) in pads.iter_mut().enumerate() {
            let snapshot = pad.snapshot(&mut race, slot);
            inputs.set(slot, snapshot);
        }
        race.tick(&inputs);
        race.drain_cues();
        watch(&mut run, &race);
    }
    let bytes = race.recorded().expect("recording").to_bytes();
    run.replay = Replay::from_bytes(&bytes).expect("reads");
    assert_eq!(run.replay.header.slots, vec![0, 1]);
    assert_eq!(run.replay.inputs.len(), 2, "one stream a seat");
    assert_ne!(run.replay.inputs[0], run.replay.inputs[1], "two pads");
    for slot in 0..2 {
        let lap = race.sim.world.ships[slot].standing.lap;
        assert!(lap >= 2, "seat {slot} flew: lap {lap}");
    }
    let mut timeline = opened(title, file_setup(&source, &run), &run);
    seeks_reproduce(title, &mut timeline, &run);
}

/// What the opening pass costs on each title: `OAG_SWEEP=1`, release, one title a process.
/// Prints and asserts nothing about the clock.
#[test]
#[ignore = "a sweep: needs game data, and OAG_SWEEP to do anything"]
fn the_opening_pass_costs() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let title = std::env::var("OAG_SWEEP_TITLE").unwrap_or_else(|_| "pulse".to_string());
    let ticks = 18_000;
    let Some((source, run)) = record(&title, Some(ticks)) else {
        return;
    };
    let setup = file_setup(&source, &run);
    let clock = Instant::now();
    let mut timeline = Timeline::open(setup, run.replay.clone()).expect("opens");
    let opened = clock.elapsed().as_secs_f64();
    while timeline.prefix().is_none() {
        std::hint::spin_loop();
    }
    let plans = clock.elapsed().as_secs_f64();
    assert_eq!(timeline.wait(), Pass::Complete);
    let pass = clock.elapsed().as_secs_f64();
    println!(
        "{title}: open {opened:.3} s, first keyframe {plans:.3} s (the plans), whole pass of \
         {ticks} ticks {pass:.2} s, {:.0}x real time, {} keyframes; load {}",
        ticks as f64 / 60.0 / pass,
        timeline.keyframes(),
        std::fs::read_to_string("/proc/loadavg")
            .unwrap_or_default()
            .trim()
    );
}
