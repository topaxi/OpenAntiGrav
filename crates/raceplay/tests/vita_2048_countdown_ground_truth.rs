//! Wipeout 2048's race start: a named mode plays `ready` and `go` from the speech
//! bank, a campaign event plays `Go_NGP` from `Weapons_NGP.bnk` and no `ready`.
//!
//! The original picks on the mode id (`docs/ghidra/functions/vita-2048-eu-v104/
//! countdown-voice.md`): below `0x17`, every named mode, `ready` and `go`; from
//! `0x17`, the CRC ids of its own events, `321_NGP` and `Go_NGP`. `321_NGP` is
//! authored and not played (two looping voices keyed off 3.75 s later under a
//! rotating pan, none of which the mixer models), so an event race starts with
//! `go` alone.
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
fn a_named_mode_starts_with_ready_and_go_from_the_speech_bank() {
    let Some(source) = source() else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        class: "VENOM".to_string(),
        ..options(&source)
    })
    .expect("the named mode loads");
    let banks = &loaded.setup.sounds;
    assert!(report_has(
        &loaded,
        r"sfx: ready -> 2 waveform(s) from Data\audio\sound\speech.bnk; plays its timeline"
    ));
    assert!(report_has(
        &loaded,
        r"sfx: go -> 4 waveform(s) from Data\audio\sound\speech.bnk; plays its timeline"
    ));
    assert_eq!(lengths(banks, Cue::Ready).len(), 1, "one timeline");
    assert_eq!(lengths(banks, Cue::Go).len(), 1, "one timeline");
    assert_eq!(
        lengths(banks, Cue::Ready)[0].len(),
        2,
        "ready keys a stereo pair"
    );
    assert_eq!(lengths(banks, Cue::Go)[0].len(), 2, "go keys a pair");
    // `cont_elim` resolves nowhere on this title: its trigger is unread.
    assert!(!banks.voices_available(Cue::ContElim));
}

#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita/"]
fn a_campaign_event_starts_with_go_ngp_alone() {
    let Some(source) = source() else {
        return;
    };
    let loaded =
        oag_raceplay::load_event(&options(&source), "2048 - Event 4").expect("the event loads");
    let banks = &loaded.setup.sounds;
    assert!(report_has(
        &loaded,
        "sfx: 321_NGP not played: two looping voices"
    ));
    assert!(report_has(
        &loaded,
        r"sfx: Go_NGP -> 4 waveform(s) from Data\audio\sound\Weapons_NGP.bnk"
    ));
    assert!(report_has(&loaded, "its start plays the event pair"));
    assert!(
        !banks.voices_available(Cue::Ready),
        "the named `ready` must not play on an event"
    );
    let go = lengths(banks, Cue::Go);
    assert_eq!(go.len(), 4, "Go_NGP's four arms, one waveform each: {go:?}");
    assert!(go.iter().all(|shape| shape.len() == 1));
    let named = oag_raceplay::load(&oag_raceplay::Options {
        class: "VENOM".to_string(),
        ..options(&source)
    })
    .expect("the named mode loads");
    assert_ne!(
        go,
        lengths(&named.setup.sounds, Cue::Go),
        "an event's go is not the named mode's"
    );
}
