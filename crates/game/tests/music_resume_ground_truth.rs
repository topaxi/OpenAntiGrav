//! A race's music resumes after a restart, on every title's own disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship (`just test-data`).
//!
//! Per title: start the race playlist on a null audio output, let it play 20 s,
//! save its place the way a session does (`music_state::Saver`), open a second
//! `Audio` as a new process would, resume from the file and start the race
//! playlist again. It must be on the same track within a second of the saved
//! place. The null output renders on the tick clock, so no sound leaves the
//! process.
//!
//! A title whose race music cannot be listed or decoded here reports that and
//! passes nothing: the test names which of the five reach the playlist.
//!
//! The same walk checks that **a race's first tick opens no disc on the
//! calling thread**. It used to re-read the booted soundtrack's listing there
//! (`Audio::race_soundtrack_len`), which on HD decompresses every track whole:
//! a 248 to 264 ms frame at race start on the Steam Deck (2026-10-19,
//! `docs/rendering/hd-gpu-cost.md`). `oag_sound::MusicDiscs` now keeps a
//! listing once read.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::time::Duration;

use oag_game::music_state::{self, Saver};
use oag_game::sound::GameLibrary;
use oag_sound::library::{Library, Opened};
use oag_sound::{self as audio, Audio, MusicSource};

thread_local! {
    /// Sources opened on this thread. A fetch worker's opens land on its own
    /// thread's counter, so this counts only what would stall a frame.
    static OPENED_HERE: Cell<usize> = const { Cell::new(0) };
}

/// [`GameLibrary`], counting each open on the thread that asked.
#[derive(Debug)]
struct Counting;

impl Library for Counting {
    fn open(&self, source: &str) -> Option<Opened> {
        OPENED_HERE.with(|n| n.set(n.get() + 1));
        GameLibrary.open(source)
    }

    fn containers(&self) -> Vec<PathBuf> {
        GameLibrary.containers()
    }
}

/// Plays one tick, the race's first, and fails if it opened a source here.
fn first_tick_opens_nothing(audio: &mut Audio, label: &str) {
    let before = OPENED_HERE.with(Cell::get);
    audio.tick();
    let opened = OPENED_HERE.with(Cell::get) - before;
    assert_eq!(
        opened, 0,
        "{label}: the race's first tick opened {opened} source(s) on the calling thread"
    );
}

const SECONDS: u32 = 20;

/// A per-process cache directory, removed on drop so a failed assert does not
/// leave its decoded music (about 120 MB) in the temp directory.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(label: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!("oag-music-resume-{}-{label}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

/// A null output that renders on the tick clock: `--dump-audio`'s, whose WAV is
/// never finished here, so nothing is written and nothing is played.
fn open_audio(cache: &Path) -> Audio {
    Audio::open(
        &audio::settings::Settings::default(),
        Some(cache.join("dump.wav")),
        None,
        Duration::from_millis(50),
        true,
    )
}

fn play(audio: &mut Audio, seconds: u32) {
    for _ in 0..(seconds * 60) {
        audio.tick();
    }
}

fn resumes(label: &str, image: &str) {
    resumes_at(label, oag_testdata::image(image));
}

fn resumes_at(label: &str, image: Option<PathBuf>) {
    let Some(image) = image else {
        return;
    };
    let discs = audio::MusicDiscs::survey(&image.display().to_string(), &Counting);
    let scratch = scratch(label);
    let cache = scratch.0.clone();
    let file = cache.join("music.toml");
    let title = format!("title-{label}");

    let mut first = open_audio(&cache);
    first.start_race_music(&discs, MusicSource::Auto, &cache);
    assert!(
        first.playhead_race().is_some(),
        "{label}: no race music started"
    );
    // The playlist needs its track count on the first tick of a race, and has
    // it without reading the disc again.
    first_tick_opens_nothing(&mut first, label);
    play(&mut first, SECONDS);
    let saved = first
        .race_music_state()
        .unwrap_or_else(|| panic!("{label}: a playing race has a place"));
    assert!(
        (i64::from(saved.second) - i64::from(SECONDS)).abs() <= 1,
        "{label}: saved {} s after {SECONDS} s of racing",
        saved.second
    );
    Saver::default().save_to(&file, &first, &title);
    drop(first);

    let mut second = open_audio(&cache);
    music_state::resume_from(&file, &mut second, &discs, &title);
    second.start_race_music(&discs, MusicSource::Auto, &cache);
    first_tick_opens_nothing(&mut second, label);
    let again = second
        .race_music_state()
        .expect("the resumed race has a place");
    assert_eq!(again.index, saved.index, "{label}: same track");
    let at = second.playhead_race().expect("the resumed race is playing");
    assert!(
        (at - f64::from(saved.second)).abs() <= 1.0,
        "{label}: resumed at {at} s, saved {} s",
        saved.second
    );

    // Every disc's first track is the playlist's normal start, so a resume that
    // dropped the index would still pass above: do it again on the second track.
    // Lengths come from `booted_track_seconds`, which reads the PS2 release's
    // `PS2MUSIC.WAD` directory as well as the archived listings
    // (`library::listing` is `None` for the PS2, which is why this was once
    // skipped there).
    let listed = Audio::booted_track_seconds(&discs);
    assert!(listed.len() > 1, "{label}: a soundtrack of several tracks");
    let wanted = audio::RaceMusicState {
        index: 1,
        second: 7,
        tracks: saved.tracks,
        track_seconds: listed[1],
    };
    let mut third = open_audio(&cache);
    third.restore_race_music(&discs, &wanted).unwrap();
    third.start_race_music(&discs, MusicSource::Auto, &cache);
    assert_eq!(
        third.race_music_state().map(|s| s.index),
        Some(1),
        "{label}"
    );
    let at = third.playhead_race().expect("the second track is playing");
    assert!(
        (at - 7.0).abs() <= 1.0,
        "{label}: second track resumed at {at} s"
    );
    println!(
        "{label}: track {} resumed at {at:.1} s of {:.0} s",
        saved.index, saved.track_seconds
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_psp_resumes() {
    resumes("pulse-psp", "data/images/pulse-psp-eu.chd");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_ps2_resumes() {
    resumes("pulse-ps2", "data/images/pulse-ps2-eu.chd");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_resumes() {
    resumes("pure", "data/images/pure-psp-eu.chd");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_resumes() {
    resumes("hd", "data/images/hdfury-ps3-eu-dec.iso");
}

#[test]
#[ignore = "needs game content in data/extracted/vita/PCSF00007"]
fn wipeout_2048_resumes() {
    resumes_at("2048", oag_testdata::exact("data/extracted/vita/PCSF00007"));
}

#[test]
#[ignore = "needs game content in data/extracted/ps4"]
fn omega_resumes() {
    resumes_at("omega", oag_testdata::exact("data/extracted/ps4"));
}

/// A changed playlist falls back to the normal start, on the real disc.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_saved_place_from_another_soundtrack_is_refused() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let discs = audio::MusicDiscs::survey(&image.display().to_string(), &GameLibrary);
    let scratch = scratch("stale");
    let mut audio = open_audio(&scratch.0);
    let stale = audio::RaceMusicState {
        index: 2,
        second: 5,
        tracks: 3,
        track_seconds: 100.0,
    };
    let why = audio.restore_race_music(&discs, &stale).unwrap_err();
    assert!(
        matches!(why, audio::Unusable::CountChanged { saved: 3, .. }),
        "{why}"
    );
    assert!(audio.race_music_state().is_none(), "nothing was applied");
}

/// The session's own sequence (`music_state::Run`, which `Session::launch_race`,
/// the fixed-timestep loop and `ApplicationHandler::exiting` and the `--race`
/// route all call): restore at launch, a periodic save while racing, a save at
/// quit, and a second run that resumes from it. Dropping any one of the three
/// calls fails an assertion below.
fn a_run_restores_saves_and_quits(label: &str, image: &str) {
    let Some(image) = oag_testdata::image(image) else {
        return;
    };
    let discs = audio::MusicDiscs::survey(&image.display().to_string(), &GameLibrary);
    let scratch = scratch(&format!("run-{label}"));
    let cache = scratch.0.clone();
    let file = cache.join("music.toml");
    let title = format!("title-run-{label}");

    let mut first = open_audio(&cache);
    let mut run = music_state::Run::at(file.clone());
    run.resume(&mut first, &discs, &title);
    run.resume(&mut first, &discs, &title);
    first.start_race_music(&discs, MusicSource::Auto, &cache);
    for _ in 0..(25 * 60) {
        first.tick();
        run.racing_tick(&first, &title);
    }
    let periodic =
        music_state::load_from(&file, &title).expect("a periodic save in 25 s of racing");
    assert_eq!(periodic.second, 20, "{label}: the last whole 10 s save");
    run.save(&first, &title);
    let quit = music_state::load_from(&file, &title).unwrap();
    assert!(
        (i64::from(quit.second) - 25).abs() <= 1,
        "{label}: quit saved {} s",
        quit.second
    );
    drop(first);

    let mut second = open_audio(&cache);
    music_state::Run::at(file).resume(&mut second, &discs, &title);
    second.start_race_music(&discs, MusicSource::Auto, &cache);
    let at = second.playhead_race().expect("the resumed race is playing");
    assert!(
        (at - f64::from(quit.second)).abs() <= 1.0,
        "{label}: {at} s"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_psp_run_restores_saves_and_quits() {
    a_run_restores_saves_and_quits("pulse-psp", "data/images/pulse-psp-eu.chd");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_ps2_run_restores_saves_and_quits() {
    a_run_restores_saves_and_quits("pulse-ps2", "data/images/pulse-ps2-eu.chd");
}
