//! Pulse's `ready` and `go`, raised on the ticks the original plays them, and
//! rendered out to a WAV so the onsets are a file rather than an assertion.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game countdown_voice_ground_truth:: --run-ignored all
//! ```
//!
//! # What is pinned, and against what
//!
//! `docs/ghidra/functions/psp-pulse-usa/countdown-voice.md` records the live
//! measurement: on Pulse (PSP) the race start plays two cues and nothing else,
//! `ready` and, exactly 180 ticks later, `go`, the latter on the last tick the
//! thrust gate holds. The constants below are those measured numbers **written
//! out as literals**, not read back from `oag_raceplay::countdown`, so
//! moving that module's ticks fails here instead of agreeing with itself.
//!
//! One test per mode, because the mode is what picks the bank: Time Trial and a
//! single race read `speech.bnk`, Eliminator `speech_elim.bnk`, Zone
//! `speech_zone.bnk`. Each renders **speech alone** (music and effects at
//! zero), so the first non-silent frame of a run is a voice onset and not an
//! engine.

use std::path::{Path, PathBuf};

use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;
use oag_raceplay as race;
use oag_sound::Volume;

/// The tick `ready` is raised on, in `World::tick` terms: measured 90.0 entries
/// after the first `InGame` craft update on the docs' axis, one behind the
/// numbering `go` uses because a cue is raised after the step it follows.
const READY_TICK: u64 = 91;
/// The last tick thrust is gated on: `oag_race::COUNTDOWN_TICKS - 1`, with the
/// literal 272 the docs' measured release.
const GO_TICK: u64 = 271;
/// Ticks of audio to render: past `go`'s own 0.8 s.
const RENDER_TICKS: u64 = 340;
/// Omega's start is checked to three-quarters of a second past `go`, which is
/// all its onsets and the window after `go` need.
const OMEGA_RENDER_TICKS: u64 = GO_TICK + 45;
/// Frames of audio per tick at the dump's own sample rate.
const FRAMES_PER_TICK: usize = (oag_sound::DUMP_SAMPLE_RATE / 60) as usize;
/// A run of speech ends after this many silent frames (a fifth of a second).
const RUN_GAP: usize = (oag_sound::DUMP_SAMPLE_RATE / 5) as usize;
/// Below this a frame counts as silence.
const SILENCE: i16 = 8;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// Omega's extracted packages: the race source the start is loaded from.
fn omega_source() -> Option<PathBuf> {
    oag_testdata::exact("data/extracted/ps4")
}

/// One run of non-silent frames in the dump.
#[derive(Debug, Clone, Copy)]
struct Run {
    start: usize,
    end: usize,
}

/// Where speech starts and stops in a 16-bit stereo dump.
fn runs(pcm: &[u8]) -> Vec<Run> {
    let frames = pcm.len() / 4;
    let loud = |frame: usize| {
        let at = frame * 4;
        let l = i16::from_le_bytes([pcm[at], pcm[at + 1]]).unsigned_abs();
        let r = i16::from_le_bytes([pcm[at + 2], pcm[at + 3]]).unsigned_abs();
        l.max(r) > SILENCE as u16
    };
    let mut out: Vec<Run> = Vec::new();
    for frame in 0..frames {
        if !loud(frame) {
            continue;
        }
        match out.last_mut() {
            Some(run) if frame - run.end <= RUN_GAP => run.end = frame,
            _ => out.push(Run {
                start: frame,
                end: frame,
            }),
        }
    }
    out
}

/// One rendered start: the `(tick, cue)` pairs raised for `Ready` and `Go`,
/// the runs of speech in the speech-only dump, and the dump's PCM itself.
struct Start {
    raised: Vec<(u64, Cue)>,
    runs: Vec<Run>,
    pcm: Vec<u8>,
}

/// Renders `ticks` of a race's start from `source`, speech only.
fn countdown(source: &Path, class: &str, mode: oag_race::Mode, name: &str, ticks: u64) -> Start {
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: class.to_string(),
        mode,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in loaded
        .report
        .iter()
        .filter(|l| l.contains("ready") || l.contains("go ->") || l.contains("speech"))
    {
        println!("{line}");
    }
    let mut race = race::Race::start(loaded.setup);

    let wav = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/shots")
        .join(format!("countdown-voice-{name}.wav"));
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings {
            music_volume: Volume::OFFERED[0],
            sfx_volume: Volume::OFFERED[0],
            ..Default::default()
        },
        Some(wav.clone()),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );

    let mut raised = Vec::new();
    for _ in 0..ticks {
        // `Race::tick` raises the cue for the tick it steps; `world.tick` has
        // already moved on by the time it returns.
        race.tick(&PlayerInputs::none());
        let stepped = race.sim.world.tick - 1;
        for event in race.pending_cues() {
            if matches!(event.cue, Cue::Ready | Cue::Go) {
                raised.push((stepped, event.cue));
            }
        }
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();
    }
    audio.finish().expect("writing the dump");
    let pcm = std::fs::read(&wav).expect("the dump")[44..].to_vec();
    let found = runs(&pcm);
    println!("{name}: raised {raised:?}");
    for run in &found {
        println!(
            "{name}: speech from frame {} (tick {:.2}) to {} ({:.2} s)",
            run.start,
            run.start as f32 / FRAMES_PER_TICK as f32,
            run.end,
            (run.end - run.start) as f32 / oag_sound::DUMP_SAMPLE_RATE as f32
        );
    }
    Start {
        raised,
        runs: found,
        pcm,
    }
}

/// The cues, the ticks and the onsets, for one mode. `ready_lead` is how many
/// ticks after the cue's own start its first word is audible - what the mode's
/// bank authors, not something this port adds.
fn check(mode: oag_race::Mode, name: &str, ready_lead: std::ops::RangeInclusive<usize>) {
    let Some(image) = image() else {
        return;
    };
    let Start {
        raised,
        runs: found,
        ..
    } = countdown(&image, "VENOM", mode, name, RENDER_TICKS);
    assert_eq!(
        raised,
        vec![(READY_TICK, Cue::Ready), (GO_TICK, Cue::Go)],
        "{name}: the start must raise ready then go and nothing else"
    );
    assert_eq!(GO_TICK - READY_TICK, 180, "ready to go is 180.0 ticks");

    // The voices themselves, from a speech-only render: two runs of sound and
    // nothing else, `go` on the tick it is raised. `ready` is a timeline, and
    // `speech.bnk`'s six-voice one does not speak at tick 0 of the cue: the
    // original's first `Scream_KeyOnVoice` after that cue's start is 22.5 ticks
    // later (Time Trial, `scripts/psp-countdown-cues.py --bp 0x0899456c`), and
    // here the first audible frame is 23. The Eliminator and Zone banks author
    // a two-voice `ready` with no lead-in; their key-on was not logged.
    assert_eq!(
        found.len(),
        2,
        "{name}: speech other than ready and go: {found:?}"
    );
    let ready_tick = found[0].start / FRAMES_PER_TICK;
    assert!(
        ready_lead.contains(&(ready_tick - READY_TICK as usize)),
        "{name}: ready's first word starts in tick {ready_tick}, not {ready_lead:?} ticks \
         after {READY_TICK}"
    );
    let go_tick = found[1].start / FRAMES_PER_TICK;
    assert!(
        (GO_TICK as usize..=GO_TICK as usize + 1).contains(&go_tick),
        "{name}: go's voice starts in tick {go_tick}, not {GO_TICK}"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn time_trial_counts_in_from_speech_bnk() {
    check(oag_race::Mode::TimeTrial, "time-trial", 21..=25);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_single_race_counts_in_from_speech_bnk() {
    check(oag_race::Mode::SingleRace, "single-race", 21..=25);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn eliminator_counts_in_from_speech_elim_bnk() {
    check(oag_race::Mode::Eliminator, "eliminator", 0..=1);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn zone_counts_in_from_speech_zone_bnk() {
    check(oag_race::Mode::Zone, "zone", 0..=1);
}

/// Below this RMS, one of Omega's authored onsets is not sounding.
const LOUD_RMS: f64 = 100.0;

/// Root-mean-square level of `frames` stereo frames from `from`, on the dump's
/// 16-bit samples.
fn rms(pcm: &[u8], from: usize, frames: usize) -> f64 {
    let end = (from + frames).min(pcm.len() / 4);
    let mut sum = 0.0;
    for frame in from..end {
        for channel in 0..2 {
            let at = frame * 4 + channel * 2;
            let sample = f64::from(i16::from_le_bytes([pcm[at], pcm[at + 1]]));
            sum += sample * sample;
        }
    }
    (sum / (2 * (end - from).max(1)) as f64).sqrt()
}

/// Omega's start, rendered speech only: its `Ready` and `Go` cues are raised on
/// the same ticks as Pulse's, and the Wwise events they post are the voices.
/// The windows are 0.25 s long and start on each authored onset.
fn check_omega(mode: oag_race::Mode, name: &str) {
    let Some(source) = omega_source() else {
        return;
    };
    let start = countdown(&source, "VENOM", mode, name, OMEGA_RENDER_TICKS);
    assert_eq!(
        start.raised,
        vec![(READY_TICK, Cue::Ready), (GO_TICK, Cue::Go)],
        "{name}: the start must raise ready then go and nothing else"
    );
    let rate = oag_sound::DUMP_SAMPLE_RATE as usize;
    let window = rate / 4;
    let ready = READY_TICK as usize * FRAMES_PER_TICK;
    let go = GO_TICK as usize * FRAMES_PER_TICK;
    let before = rms(&start.pcm, 0, ready);
    assert!(
        before < f64::from(SILENCE),
        "{name}: the start is audible before ready: rms {before:.1}"
    );
    let onsets = [400, 1300, 2150]
        .map(|ms| (format!("ready +{ms} ms"), ready + ms * rate / 1000))
        .into_iter()
        .chain([("go".to_string(), go)]);
    for (what, at) in onsets {
        let level = rms(&start.pcm, at, window);
        assert!(
            level > LOUD_RMS,
            "{name}: silent at {what} (frame {at}): rms {level:.1}, floor {LOUD_RMS}"
        );
    }
}

#[test]
#[ignore = "needs the decrypted Omega packages in data/extracted/ps4/"]
fn omega_time_trial_counts_in_from_wwise_events() {
    check_omega(oag_race::Mode::TimeTrial, "omega-time-trial");
}
