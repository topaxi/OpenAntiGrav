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
//! The same walk checks that **the calling thread does no music work**: no
//! listing, disc read or decode, counted by `oag_sound::probe`, across the
//! menu music's start, a race's start, its first tick, a resume from the
//! menus, a track change and (where both releases are here) a `MUSIC SOURCE`
//! switch. That thread is the frame thread in the game. On the Steam Deck the
//! synchronous versions cost 231 ms (HD's menu music), 305 ms (HD's listing,
//! for a saved place) and 416 ms (a race track) (2026-10-19,
//! `docs/rendering/hd-gpu-cost.md`).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use oag_game::music_state::{self, Saver};
use oag_game::sound::GameLibrary;
use oag_sound::{self as audio, Audio, MusicSource, probe};

/// Runs `step` and fails if it did music work on this thread.
fn here_does_no_music_work<T>(label: &str, what: &str, step: impl FnOnce() -> T) -> T {
    let before = probe::on_this_thread();
    let out = step();
    let after = probe::on_this_thread();
    assert_eq!(
        after.calls - before.calls,
        0,
        "{label}: {what} did {} piece(s) of music work on the calling thread, {:?}",
        after.calls - before.calls,
        after.time - before.time
    );
    out
}

/// Ticks until `ready` holds, as a frame loop would, failing after a minute.
fn tick_until(audio: &mut Audio, label: &str, what: &str, ready: impl Fn(&Audio) -> bool) {
    let start = Instant::now();
    while !ready(audio) {
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "{label}: {what} never landed"
        );
        audio.tick();
        std::thread::sleep(Duration::from_millis(2));
    }
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
    let discs = audio::MusicDiscs::survey(&image.display().to_string(), &GameLibrary);
    let scratch = scratch(label);
    let cache = scratch.0.clone();
    let file = cache.join("music.toml");
    let title = format!("title-{label}");

    a_frame_thread_does_no_music_work(label, &discs, &cache);

    let mut first = open_audio(&cache);
    here_does_no_music_work(label, "starting the race music", || {
        first.start_race_music(&discs, MusicSource::Auto, &cache);
    });
    assert!(
        first.playhead_race().is_some(),
        "{label}: no race music started"
    );
    // The playlist needs its track count on the first tick of a race, and has
    // it without reading the disc again.
    here_does_no_music_work(label, "the race's first tick", || first.tick());
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
    // The saved place is checked against a listing HD's survey has not read:
    // the race's fetch reads and checks it, not this thread.
    here_does_no_music_work(label, "resuming a saved place", || {
        music_state::resume_from(&file, &mut second, &discs, &title);
        second.start_race_music(&discs, MusicSource::Auto, &cache);
        second.tick();
    });
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

    // A track change: the second track resumed a second before its end.
    let near_end = audio::RaceMusicState {
        second: listed[1].floor() as u32 - 1,
        ..wanted
    };
    let mut fourth = open_audio(&cache);
    fourth.restore_race_music(&discs, &near_end).unwrap();
    fourth.start_race_music(&discs, MusicSource::Auto, &cache);
    here_does_no_music_work(label, "a track change", || play(&mut fourth, 3));
    assert_eq!(
        fourth.race_music_state().map(|s| s.index),
        Some(2 % listed.len()),
        "{label}: the playlist moved on to the next track"
    );
}

/// The steps a frame loop takes, on a null output with no dump, so every start
/// takes the route a window does: started, then polled by `tick` until it lands.
fn a_frame_thread_does_no_music_work(label: &str, discs: &audio::MusicDiscs, cache: &Path) {
    let mut audio = Audio::open(
        &audio::settings::Settings::default(),
        None,
        None,
        Duration::from_millis(50),
        true,
    );
    here_does_no_music_work(label, "the menu music", || {
        audio.start_music(discs, MusicSource::Auto, cache);
        tick_until(&mut audio, label, "the menu music", |a| {
            !a.music_in_flight()
        });
    });
    let menu = audio.playhead().is_some();
    here_does_no_music_work(label, "a race's music", || {
        audio.start_race_music(discs, MusicSource::Auto, cache);
        tick_until(&mut audio, label, "the race music", |a| {
            !a.music_in_flight()
        });
    });
    assert!(audio.playhead_race().is_some(), "{label}: no race music");
    here_does_no_music_work(label, "a resume from the menus", || {
        audio.tick();
        audio.pause_race_music();
        audio.start_race_music(discs, MusicSource::Auto, cache);
    });
    assert!(
        audio.playhead_race().is_some(),
        "{label}: the decoded track resumes at once"
    );
    if discs.both() {
        let other = match discs.booted() {
            Some(oag_disc::Platform::Ps2) => MusicSource::Psp,
            _ => MusicSource::Ps2,
        };
        here_does_no_music_work(label, "a MUSIC SOURCE switch", || {
            audio.set_music_source(discs, other, cache);
            tick_until(&mut audio, label, "the source switch", |a| {
                !a.music_in_flight()
            });
        });
    }
    println!("{label}: no music work on the calling thread (menu music: {menu})");
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
