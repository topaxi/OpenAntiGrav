//! A seek in a replay sounds like playing on to it: ADR-0087, decision 6.
//!
//! The mixer is rendered to a WAV twice over one recording of a Time Trial (one craft, flown by
//! the Autopilot): once played continuously through tick `T`, once played to a few seconds in and
//! then sought to `T`, with the voices rebuilt, and played on. The two are compared at `T` **by
//! the WAV, never by ear**: the loudness (the engine's gain) and the spectrum (its pitch) over
//! the same window of ticks after `T`.
//!
//! A third render seeks the same way without `Audio::rebuild_race_sfx`: its engine is still the
//! one of the craft that was nearly at rest, and its spectrum sits more than a semitone below
//! the continuous render's. That is what fails if the rebuild is dropped. One craft, because
//! eight engines and the ambience make a noise whose peaks do not compare; the engine's
//! contribution to a Single Race's mix is the same law per slot.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game replay_audio_ground_truth:: \
//!     --run-ignored all --no-capture
//! ```

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay::viewer::{Moved, Viewer};
use oag_raceplay::{self as race, Race};
use oag_replay::Header;

const RATE: usize = oag_sound::DUMP_SAMPLE_RATE as usize;
const FRAMES_PER_TICK: usize = RATE / 60;
/// The tick compared at, far enough in that the engines have run for a while.
const AT: u64 = 1_200;
/// Where a seek starts from: a few seconds into the race, the craft still near the grid, so a
/// voice carried over a seek without a rebuild sounds the craft it was, not the one at `AT`.
const FROM: u64 = 285;
/// The window compared, in ticks, beginning this many after the target so the first frames
/// after a rebuild are not held against it.
const SKIP: u64 = 12;
const WINDOW: u64 = 64;
const RECORDED: u64 = AT + 300;

fn source() -> Option<PathBuf> {
    oag_testdata::image("pulse-psp-usa.chd")
}

fn options(source: &std::path::Path) -> race::Options {
    race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        seed: Some(7),
        ..race::Options::default()
    }
}

/// A recording of the Autopilot flying `RECORDED` ticks.
fn recording(loaded: &race::Loaded) -> oag_replay::Replay {
    let launch = race::Launch {
        scheme: oag_gameplay::ControlScheme::default(),
        difficulty: oag_ai::Difficulty::default(),
        autopilot: true,
        pilot_assist: oag_physics::pilot_assist::Level::Off,
        original_ai: None,
        campaign_skill: None,
        skill_level: None,
    };
    let mut race = Race::start(loaded.setup.clone());
    race.apply_launch(&launch);
    let mut header = Header::new("pulse", "default", "time_trial", "venom", "default", 7);
    header.options = launch.to_table();
    race.start_recording(header);
    for _ in 0..RECORDED {
        race.tick(&PlayerInputs::none());
        race.drain_cues();
    }
    race.recorded().expect("recording")
}

fn audio(path: &std::path::Path) -> oag_sound::Audio {
    oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(path.to_path_buf()),
        None,
        oag_audio::MIN_BUFFER,
        false,
    )
}

fn pcm(path: &std::path::Path) -> Vec<f32> {
    let file = std::fs::read(path).expect("the dump");
    file[44..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| f32::from(i16::from_le_bytes(*b)) / 32768.0)
        .collect()
}

/// What each render does at the target.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reach {
    /// Played from tick 0.
    Play,
    /// A seek, and the voices rebuilt from the state there.
    Seek,
    /// A seek and nothing else.
    SeekOnly,
    /// Played at 1X to `FROM`, then at 2X (the effects muted) to `AT`, then at 1X again.
    Fast,
}

/// Renders the mixer for the window after `AT`, returning its samples (left and right
/// interleaved), the first of which is tick `AT` itself.
fn render(
    loaded: &race::Loaded,
    replay: &oag_replay::Replay,
    reach: Reach,
    name: &str,
) -> Vec<f32> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/scratch")
        .join(name);
    std::fs::create_dir_all(path.parent().expect("a directory")).expect("a scratch directory");
    let mut audio = audio(&path);
    let mut viewer = Viewer::open(loaded.setup.clone(), replay.clone()).expect("rebuilds");
    let mut race = Box::new(Race::start(loaded.setup.clone()));
    viewer.begin(&mut race).expect("the launch settings");
    while viewer.built().is_none() {
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    // Let the opening pass cover the target before anything is played or sought.
    while viewer
        .built()
        .is_some_and(|built| built < AT + SKIP + WINDOW)
    {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(viewer.update(&mut race), Moved::Seeked);
    let keep_from = match reach {
        Reach::Play => {
            // Continuous: every tick from 0 sounds and ticks the mixer; the part before `AT`
            // is dropped from the samples afterwards.
            while viewer.at() < AT {
                if let Moved::Stepped(_) = viewer.update(&mut race) {
                    oag_game::sound::race_tick(&mut audio, &mut race);
                }
                audio.tick();
            }
            AT as usize * FRAMES_PER_TICK * 2
        }
        Reach::Fast => {
            while viewer.at() < FROM {
                if let Moved::Stepped(_) = viewer.update(&mut race) {
                    oag_game::sound::race_tick(&mut audio, &mut race);
                }
                audio.tick();
            }
            let mut muted = false;
            viewer.faster();
            while viewer.at() < AT - 2 {
                if let Moved::Stepped(ticks) = viewer.update(&mut race) {
                    let at = viewer.at();
                    oag_game::sound::viewer_step(
                        &mut audio,
                        &mut race,
                        ticks == 1 && viewer.speed() == 16,
                        &mut muted,
                        at,
                    );
                }
                audio.tick();
            }
            // 2X to 4X to 0.5X to 1X: the next speed wraps round.
            for _ in 0..3 {
                viewer.faster();
            }
            assert_eq!(viewer.speed(), 16);
            // The first frames at 1X are what is compared, so the rebuild is in them.
            let mut kept = None;
            for _ in 0..(SKIP + WINDOW + 2) {
                if let Moved::Stepped(ticks) = viewer.update(&mut race) {
                    let at = viewer.at();
                    if kept.is_none() {
                        kept = Some(at);
                    }
                    oag_game::sound::viewer_step(&mut audio, &mut race, ticks == 1, &mut muted, at);
                }
                audio.tick();
            }
            audio.finish().expect("writing the dump");
            // Every frame before the first at 1X, which is `AT - 1` or `AT - 2` ticks in.
            let before = pcm(&path).len() - (SKIP + WINDOW + 2) as usize * FRAMES_PER_TICK * 2;
            let _ = kept;
            return pcm(&path).split_off(before);
        }
        Reach::Seek | Reach::SeekOnly => {
            // The viewer has been playing from the start up to `FROM`, as a player's would.
            while viewer.at() < FROM {
                if let Moved::Stepped(_) = viewer.update(&mut race) {
                    oag_game::sound::race_tick(&mut audio, &mut race);
                }
                audio.tick();
            }
            assert_eq!(viewer.seek(&mut race, AT), Moved::Seeked);
            if reach == Reach::Seek {
                oag_game::sound::rebuild(&mut audio, &mut race, AT);
            } else {
                drop(race.drain_cues());
                drop(race.drain_announcements());
                drop(race.drain_class_announcements());
            }
            FROM as usize * FRAMES_PER_TICK * 2
        }
    };
    for _ in 0..(SKIP + WINDOW) {
        if let Moved::Stepped(_) = viewer.update(&mut race) {
            oag_game::sound::race_tick(&mut audio, &mut race);
        }
        audio.tick();
    }
    audio.finish().expect("writing the dump");
    pcm(&path).split_off(keep_from)
}

/// The window: `WINDOW` ticks starting `SKIP` after the target, as a mono signal.
fn window(samples: &[f32]) -> Vec<f32> {
    let from = SKIP as usize * FRAMES_PER_TICK;
    (from..from + WINDOW as usize * FRAMES_PER_TICK)
        .map(|frame| 0.5 * (samples[frame * 2] + samples[frame * 2 + 1]))
        .collect()
}

fn rms(signal: &[f32]) -> f32 {
    (signal.iter().map(|s| s * s).sum::<f32>() / signal.len() as f32).sqrt()
}

/// The magnitude spectrum of the first `n` samples (a power of two) under a Hann window.
fn spectrum(signal: &[f32], n: usize) -> Vec<f32> {
    let mut re: Vec<f32> = (0..n)
        .map(|i| {
            let hann = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos();
            signal[i] * hann
        })
        .collect();
    let mut im = vec![0.0_f32; n];
    // In-place radix-2 FFT.
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            re.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = -std::f32::consts::TAU / len as f32;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (s, c) = (angle * k as f32).sin_cos();
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * c - im[b] * s, re[b] * s + im[b] * c);
                (re[b], im[b]) = (re[a] - tr, im[a] - ti);
                (re[a], im[a]) = (re[a] + tr, im[a] + ti);
            }
        }
        len *= 2;
    }
    (0..n / 2)
        .map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt())
        .collect()
}

/// The energy in each semitone band from 100 Hz to 4 kHz, averaged over the window's frames of
/// 4096 samples at half overlap, so a note's place shows and the phase of a loop does not.
fn semitone_bands(signal: &[f32]) -> Vec<f32> {
    const N: usize = 4096;
    let bands = ((4_000.0_f32 / 100.0).log2() * 12.0).ceil() as usize;
    let mut energy = vec![0.0_f32; bands];
    let mut frames = 0;
    let mut at = 0;
    while at + N <= signal.len() {
        let magnitudes = spectrum(&signal[at..at + N], N);
        for (bin, magnitude) in magnitudes.iter().enumerate().skip(1) {
            let hz = bin as f32 * RATE as f32 / N as f32;
            if (100.0..4_000.0).contains(&hz) {
                let band = ((hz / 100.0).log2() * 12.0) as usize;
                energy[band.min(bands - 1)] += magnitude * magnitude;
            }
        }
        frames += 1;
        at += N / 2;
    }
    assert!(frames > 4, "the window holds frames to average");
    energy.iter().map(|e| e / frames as f32).collect()
}

/// The `count` strongest local peaks of the window's averaged spectrum between 60 Hz and 3 kHz,
/// in Hz, strongest first: the notes the engines hold.
fn peaks(signal: &[f32], count: usize) -> Vec<f32> {
    const N: usize = 8192;
    let mut sum = vec![0.0_f32; N / 2];
    let mut at = 0;
    while at + N <= signal.len() {
        for (acc, m) in sum.iter_mut().zip(spectrum(&signal[at..at + N], N)) {
            *acc += m * m;
        }
        at += N / 2;
    }
    let hz = |bin: usize| bin as f32 * RATE as f32 / N as f32;
    let mut found: Vec<usize> = (2..sum.len() - 1)
        .filter(|&b| {
            (60.0..3_000.0).contains(&hz(b)) && sum[b] > sum[b - 1] && sum[b] >= sum[b + 1]
        })
        .collect();
    found.sort_by(|&a, &b| sum[b].total_cmp(&sum[a]));
    found.into_iter().take(count).map(hz).collect()
}

/// Where the energy sits, in semitones above 100 Hz: the engines' note, as one number.
fn centroid(bands: &[f32]) -> f32 {
    let total: f32 = bands.iter().sum();
    bands
        .iter()
        .enumerate()
        .map(|(i, e)| (i as f32 + 0.5) * e)
        .sum::<f32>()
        / total
}

/// The correlation of the bands' decibels: 1 is the same shape.
fn shape_correlation(a: &[f32], b: &[f32]) -> f32 {
    let db = |v: &[f32]| -> Vec<f32> { v.iter().map(|e| 10.0 * (e + 1e-9).log10()).collect() };
    let (a, b) = (db(a), db(b));
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    let (ma, mb) = (mean(&a), mean(&b));
    let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(&b) {
        ab += (x - ma) * (y - mb);
        aa += (x - ma) * (x - ma);
        bb += (y - mb) * (y - mb);
    }
    ab / (aa.sqrt() * bb.sqrt())
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_seek_sounds_like_playing_on_to_it() {
    let Some(source) = source() else {
        return;
    };
    let loaded = race::load(&options(&source)).expect("loading the race");
    let replay = recording(&loaded);
    let played = window(&render(
        &loaded,
        &replay,
        Reach::Play,
        "replay-audio-played.wav",
    ));
    let sought = window(&render(
        &loaded,
        &replay,
        Reach::Seek,
        "replay-audio-sought.wav",
    ));
    let bare = window(&render(
        &loaded,
        &replay,
        Reach::SeekOnly,
        "replay-audio-bare.wav",
    ));
    let fast = window(&render(
        &loaded,
        &replay,
        Reach::Fast,
        "replay-audio-fast.wav",
    ));

    let (bp, bs, bb) = (
        semitone_bands(&played),
        semitone_bands(&sought),
        semitone_bands(&bare),
    );
    let (rp, rs, rb) = (rms(&played), rms(&sought), rms(&bare));
    let (cp, cs, cb) = (centroid(&bp), centroid(&bs), centroid(&bb));
    let (shape, shape_bare) = (shape_correlation(&bp, &bs), shape_correlation(&bp, &bb));
    let bf = semitone_bands(&fast);
    let (rf, cf, shape_fast) = (rms(&fast), centroid(&bf), shape_correlation(&bp, &bf));
    println!(
        "2X then 1X again:         rms {rf:.5} ({:.3} of played), centroid {cf:.2} ({:+.2} semitones), spectrum correlation {shape_fast:.4}",
        rf / rp,
        cf - cp
    );
    assert!(
        (0.8..=1.25).contains(&(rf / rp)) && (cf - cp).abs() < 0.5 && shape_fast > 0.9,
        "back at 1X after a fast stretch the engines are rebuilt, not started cold: \
         gain {rf} against {rp}, centroid {cf} against {cp}, correlation {shape_fast}"
    );
    println!(
        "at tick {AT}, a window of {WINDOW} ticks from {SKIP} after it, a seek from tick {FROM}\n\
         played (continuous):      rms {rp:.5}, centroid {cp:.2} semitones over 100 Hz\n\
         seek with the rebuild:    rms {rs:.5} ({:.3} of played), centroid {cs:.2} ({:+.2} semitones), spectrum correlation {shape:.4}\n\
         seek without the rebuild: rms {rb:.5} ({:.3} of played), centroid {cb:.2} ({:+.2} semitones), spectrum correlation {shape_bare:.4}",
        rs / rp,
        cs - cp,
        rb / rp,
        cb - cp,
    );
    println!(
        "strongest notes (Hz), played: {:.0?}\n                      sought: {:.0?}\n                      no rebuild: {:.0?}",
        peaks(&played, 8),
        peaks(&sought, 8),
        peaks(&bare, 8)
    );
    assert!(
        rp > 0.005,
        "the continuous render has an engine sound to compare: {rp}"
    );
    assert!(
        (0.8..=1.25).contains(&(rs / rp)),
        "the engines' gain after a seek is the continuous render's: {rs} against {rp}"
    );
    assert!(
        (cs - cp).abs() < 0.5,
        "the engines' pitch after a seek is the continuous render's: {cs} against {cp} semitones"
    );
    assert!(shape > 0.9, "the spectrum's shape after a seek: {shape}");
    // Without the rebuild the voices carried over a seek are the craft's at the tick it left, and
    // the render is measurably not the continuous one: the rebuild is what makes it so.
    assert!(
        (cb - cp).abs() > 2.0 * (cs - cp).abs() + 0.25 || shape_bare < shape - 0.02,
        "a seek with no rebuild sounds like the continuous render, so the rebuild proves nothing: \
         centroid {cb} against {cp}, correlation {shape_bare} against {shape}"
    );
}
