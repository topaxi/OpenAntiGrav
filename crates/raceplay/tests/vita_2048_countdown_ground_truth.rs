//! Wipeout 2048's race start: a named mode plays `ready`, `HBEAT` and `go` from the
//! speech bank, a campaign event plays `321_NGP` and `Go_NGP` from `Weapons_NGP.bnk`.
//!
//! The names resolve to the cue the name table's `+0xc` field gives, as the
//! original's `Scream_FindCueByName` returns it, **not** the hash bucket at `+8`
//! (`docs/formats/2048-audio.md`): on the old reading `ready` played another
//! word's cue (`rockets`), `HBEAT` the Eliminator call, `go` a different cue.
//!
//! The original picks on the mode id (`docs/ghidra/functions/vita-2048-eu-v104/
//! countdown-voice.md`): below `0x17`, every named mode, `ready` and `go`; from
//! `0x17`, the CRC ids of its own events, `321_NGP` and `Go_NGP`. Both pan a pair to 90
//! and 270 degrees; 90 is the rear edge of the pan law (`a' = 180`), played hard right.
//!
//! **`#[ignore]`d and never run in CI**: it needs the extracted Vita package.
//! Run with `just test-data`. Without the event/named split in
//! `oag_sound::sfx::Banks::use_event_start` the event race would carry the named
//! `ready` and `go` and this fails.

use std::path::{Path, PathBuf};

use oag_core::Rng;
use oag_gameplay::cue::Cue;

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists() {
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

fn options(source: &Path) -> oag_raceplay::Options {
    oag_raceplay::Options {
        source: source.display().to_string(),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    }
}

fn report_has(loaded: &oag_raceplay::Loaded, needle: &str) -> bool {
    loaded.report.iter().any(|line| line.contains(needle))
}

/// The lengths of every voice a cue can start, over enough draws to meet each alternate.
fn lengths(banks: &oag_sound::sfx::Banks, cue: Cue) -> Vec<Vec<String>> {
    let mut rng = Rng::new(7);
    let mut seen: Vec<Vec<String>> = Vec::new();
    for _ in 0..64 {
        let voices = banks.voices(cue, &mut rng).expect("the cue loaded");
        let shape = voices
            .iter()
            .map(|v| format!("{:.3}s+{:.3}s", v.sound.seconds(), v.delay))
            .collect::<Vec<_>>();
        if !seen.contains(&shape) {
            seen.push(shape);
        }
    }
    seen.sort();
    seen
}

#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita/"]
fn the_start_names_resolve_to_the_cues_the_original_returns() {
    let Some(source) = source() else {
        return;
    };
    let mut archives = oag_2048::open(&source.display().to_string()).expect("open 2048");
    for (entry, names) in [
        (
            r"Data\audio\sound\speech.bnk",
            // `ready` is a lone `0x08` handing its voice to `321_GO`, as on HD.
            vec![("ready", 0), ("321_GO", 2), ("go", 3), ("HBEAT", 27)],
        ),
        (
            r"Data\audio\sound\Weapons_NGP.bnk",
            vec![("Go_NGP", 68), ("321_NGP", 70)],
        ),
    ] {
        let blob = archives.read_name(entry).expect("the bank reads");
        let bank = oag_formats::sblk::Bank::parse(&blob).expect("the bank parses");
        for (name, cue) in names {
            assert_eq!(
                bank.cue_named_or_hashed(name).map(|c| c.index),
                Some(cue),
                "{entry}: {name}"
            );
        }
    }
}

#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita/"]
fn a_named_mode_starts_with_ready_the_beat_and_go_from_the_speech_bank() {
    let Some(source) = source() else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        class: "VENOM".to_string(),
        ..options(&source)
    })
    .expect("the named mode loads");
    let banks = &loaded.setup.sounds;
    // `ready` follows its child cue `321_GO` to the 3.0 s pair at its marker (14 waveforms in the
    // walk), `go` is the long pair, `HBEAT` three groups of 3, 3 and 4 voices.
    for line in [
        r"sfx: ready -> 14 waveform(s) from Data\audio\sound\speech.bnk; plays its timeline",
        r"sfx: go -> 6 waveform(s) from Data\audio\sound\speech.bnk; plays its timeline",
        r"sfx: HBEAT -> 10 waveform(s) from Data\audio\sound\speech.bnk; plays its timeline: 1 variant(s) of 10 voice(s)",
    ] {
        assert!(report_has(&loaded, line), "{line}");
    }
    let ready = lengths(banks, Cue::Ready);
    assert_eq!(ready.len(), 1, "one timeline: {ready:?}");
    assert_eq!(ready[0].len(), 2, "a stereo pair: {ready:?}");
    assert!(
        ready[0]
            .iter()
            .all(|v| v.starts_with("3.0") && v.ends_with("+0.000s")),
        "3.0 s from the start: {ready:?}"
    );
    let go = lengths(banks, Cue::Go);
    assert_eq!(go.len(), 1, "one timeline: {go:?}");
    assert_eq!(go[0].len(), 2, "a pair: {go:?}");
    let beat = lengths(banks, Cue::StartBeat);
    assert_eq!(beat.len(), 1);
    assert_eq!(beat[0].len(), 10, "three groups of 3, 3 and 4: {beat:?}");
    // `cont_elim` resolves nowhere on this title: its trigger is unread.
    assert!(!banks.voices_available(Cue::ContElim));
}

/// The summed output of one voice alone through a mixer: (left, right) energy.
fn channels(voice: &oag_sound::sfx::CueVoice) -> (f64, f64) {
    let mut mixer = oag_audio::Mixer::new(44_100);
    let _ = oag_sound::sfx::start_voices(
        &mut mixer,
        std::slice::from_ref(voice),
        oag_audio::Bus::Sfx,
        oag_sound::sfx::VoicePlace::DRY,
    );
    let (mut left, mut right) = (0.0_f64, 0.0_f64);
    let mut out = vec![0.0_f32; 2 * 4096];
    for _ in 0..(4 * 44_100 / 4096) {
        mixer.render(&mut out);
        for frame in out.chunks(2) {
            left += f64::from(frame[0]).powi(2);
            right += f64::from(frame[1]).powi(2);
        }
    }
    (left, right)
}

#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita/"]
fn a_campaign_event_plays_its_pair_panned_as_the_original_pans_it() {
    let Some(source) = source() else {
        return;
    };
    let loaded =
        oag_raceplay::load_event(&options(&source), "2048 - Event 4").expect("the event loads");
    for name in ["321_NGP", "Go_NGP"] {
        assert!(
            report_has(&loaded, &format!(r"sfx: {name} -> "))
                && report_has(&loaded, "plays its timeline"),
            "{name} loads as a timeline"
        );
        assert!(!report_has(&loaded, &format!("sfx: {name} not played")));
    }
    assert!(report_has(&loaded, "its start plays the event pair"));
    let banks = &loaded.setup.sounds;
    let mut rng = Rng::new(7);
    // `Go_NGP`: one pair keyed 100 ticks in, at 90 degrees (hard right, the rear edge) and 270
    // (hard left).
    let go = banks.voices(Cue::Go, &mut rng).expect("go plays");
    assert_eq!(go.len(), 2, "a pair: {}", go.len());
    for voice in &go {
        assert!(
            (voice.delay - 100.0 / 240.0).abs() < 1e-9,
            "{}",
            voice.delay
        );
    }
    let mut angles: Vec<i32> = go.iter().map(|v| v.angle.rem_euclid(360)).collect();
    angles.sort_unstable();
    assert_eq!(angles, [90, 270]);
    for voice in &go {
        let (left, right) = channels(voice);
        if voice.angle.rem_euclid(360) == 90 {
            assert!(
                right > 0.0 && left < right * 1e-6,
                "90 is hard right: {left} {right}"
            );
        } else {
            assert!(
                left > 0.0 && right < left * 1e-6,
                "270 is hard left: {left} {right}"
            );
        }
    }
    // `321_NGP`: three groups on delays 125, 205 and 205 ticks, each a centre voice and the pair.
    let ready = banks.voices(Cue::Ready, &mut rng).expect("ready plays");
    assert_eq!(ready.len(), 9, "three groups of three");
    let mut delays: Vec<u32> = ready
        .iter()
        .map(|v| (v.delay * 240.0).round() as u32)
        .collect();
    delays.sort_unstable();
    assert_eq!(delays, [125, 125, 125, 330, 330, 330, 535, 535, 535]);
    assert_eq!(ready.iter().filter(|v| v.angle == 0).count(), 3);
    assert!(
        !banks.voices_available(Cue::StartBeat),
        "an event plays no start beat"
    );
}
