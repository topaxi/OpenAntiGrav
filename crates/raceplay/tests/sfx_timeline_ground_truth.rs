//! Pulse's sound cues played as the timelines they author, against the discs.
//!
//! **`#[ignore]`d and never run in CI**: they need game content. Run with
//! `just test-data`.
//!
//! What is asserted is structure read off the mixer's own output, not a golden
//! hash: each voice of a layered cue is silent until its authored delay and
//! audible right after it, and the whole cue ends where its last voice ends.
//! The expected delays are the bank's own (`PLASMA`'s tail keyed 215 master
//! ticks in, `ROCKEXPLSHIP`'s third grain at 50), converted at the measured
//! 258.4 Hz.

use std::path::PathBuf;

use oag_audio::{Bus, Mixer};
use oag_core::Rng;
use oag_gameplay::cue::Cue;
use oag_sound::sfx::{Banks, CueTables, CueVoice, VoicePlace, start_voices};

const RATE: u32 = 44_100;
const PULSE: [&str; 3] = ["pulse-psp-usa.chd", "pulse-psp-eu.chd", "pulse-ps2-eu.chd"];
const NOT_PULSE: [&str; 2] = ["pure-psp-usa.chd", "pure-psp-eu.chd"];

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

fn banks(path: &std::path::Path) -> Banks {
    let opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let sounds = opened.title.race.sounds;
    let tick = opened
        .title
        .race
        .zone_announcer
        .map_or(oag_title::SequenceTick::Unknown, |z| z.tick);
    let mut archives = opened.archives;
    Banks::load(&mut archives, sounds, false, tick)
}

/// One voice alone through a mixer: (first audible frame, last audible frame)
/// in seconds.
fn extent(voice: &CueVoice) -> (f64, f64) {
    let mut mixer = Mixer::new(RATE);
    let _ = start_voices(
        &mut mixer,
        std::slice::from_ref(voice),
        Bus::Sfx,
        VoicePlace::DRY,
    );
    frames(&mut mixer)
}

fn frames(mixer: &mut Mixer) -> (f64, f64) {
    let mut out = vec![0.0f32; 2 * 4096];
    let (mut first, mut last, mut at) = (None, 0usize, 0usize);
    for _ in 0..(6 * RATE as usize / 4096) {
        mixer.render(&mut out);
        for (i, f) in out.chunks(2).enumerate() {
            if f[0].abs() > 1e-4 || f[1].abs() > 1e-4 {
                first.get_or_insert(at + i);
                last = at + i;
            }
        }
        at += 4096;
        if mixer.active_voices() == 0 {
            break;
        }
    }
    (
        first.map_or(f64::NAN, |i| i as f64 / f64::from(RATE)),
        (last + 1) as f64 / f64::from(RATE),
    )
}

fn voices(banks: &Banks, cue: Cue) -> Vec<CueVoice> {
    banks.voices(cue, &mut Rng::new(3)).expect("cue loads")
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_layered_cue_keys_each_voice_on_at_its_authored_delay() {
    // (cue, the delays of its voices in master ticks)
    let cases: [(Cue, &[u32]); 4] = [
        (Cue::Plasma, &[0, 15, 215]),
        (Cue::RocketHitShip, &[0, 0, 50]),
        (Cue::PlasmaHitShip, &[0, 0, 0, 200]),
        (Cue::Missile, &[0, 15, 15]),
    ];
    let mut ran = 0;
    for file in PULSE {
        let Some(path) = image(file) else { continue };
        ran += 1;
        let banks = banks(&path);
        for (cue, ticks) in cases {
            let started = voices(&banks, cue);
            assert_eq!(started.len(), ticks.len(), "{file}: {}", cue.name());
            let mut end = 0.0f64;
            for (voice, &t) in started.iter().zip(ticks) {
                let want = f64::from(t) / (44_100.0 * 3.0 / 512.0);
                assert!(
                    (voice.delay - want).abs() < 1e-9,
                    "{file}: {} voice at {} not {want}",
                    cue.name(),
                    voice.delay
                );
                let (first, last) = extent(voice);
                assert!(
                    first >= want - 0.001 && first < want + 0.02,
                    "{file}: {} first audible at {first}, authored {want}",
                    cue.name()
                );
                let authored = want + f64::from(voice.sound.seconds()) / f64::from(voice.pitch);
                assert!(
                    (last - authored).abs() < 0.03,
                    "{file}: {} ends at {last}, authored {authored}",
                    cue.name()
                );
                end = end.max(authored);
            }
            // Together: the cue ends where its last voice ends.
            let mut mixer = Mixer::new(RATE);
            let _ = start_voices(&mut mixer, &started, cue.bus(), VoicePlace::DRY);
            let (first, last) = frames(&mut mixer);
            assert!(
                first < 0.02,
                "{file}: {} starts late at {first}",
                cue.name()
            );
            assert!(
                (last - end).abs() < 0.03,
                "{file}: {} ends at {last}, authored {end}",
                cue.name()
            );
        }
    }
    assert!(ran > 0, "no Pulse disc image was present");
}

/// Wipeout HD's SCREAM runs a 240 Hz master tick (`SequenceTick::Ps3`), not the
/// PSP's 258.4: the same authored tick counts land later in seconds.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_s_layered_cues_key_on_at_the_ps3_tick() {
    let Some(path) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let banks = banks(&path);
    // (cue, the delays of its voices in master ticks), read off the bank.
    let cases: [(Cue, &[u32]); 2] = [
        (Cue::Plasma, &[0, 15, 215, 230]),
        (Cue::PlasmaHitWall, &[0, 0, 15, 15, 40]),
    ];
    for (cue, ticks) in cases {
        let started = voices(&banks, cue);
        assert_eq!(started.len(), ticks.len(), "{}", cue.name());
        let mut end = 0.0f64;
        for (voice, &t) in started.iter().zip(ticks) {
            let want = f64::from(t) / 240.0;
            assert!(
                (voice.delay - want).abs() < 1e-9,
                "{}: voice at {} not {want}",
                cue.name(),
                voice.delay
            );
            let (first, last) = extent(voice);
            assert!(
                first >= want - 0.001 && first < want + 0.02,
                "{}: first audible at {first}, authored {want}",
                cue.name()
            );
            let authored = want + f64::from(voice.sound.seconds()) / f64::from(voice.pitch);
            assert!(
                (last - authored).abs() < 0.03,
                "{}: ends at {last}, authored {authored}",
                cue.name()
            );
            end = end.max(authored);
        }
        let mut mixer = Mixer::new(RATE);
        let _ = start_voices(&mut mixer, &started, cue.bus(), VoicePlace::DRY);
        let (first, last) = frames(&mut mixer);
        assert!(first < 0.02, "{}: starts late at {first}", cue.name());
        assert!(
            (last - end).abs() < 0.03,
            "{}: ends at {last}, authored {end}",
            cue.name()
        );
    }
    assert!(
        banks
            .report
            .iter()
            .any(|l| l.contains("plays its timeline")),
        "{:?}",
        banks.report
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_shield_holds_both_of_its_loops_at_once() {
    let mut ran = 0;
    for file in PULSE {
        let Some(path) = image(file) else { continue };
        ran += 1;
        let started = voices(&banks(&path), Cue::Shield);
        assert_eq!(started.len(), 2, "{file}");
        assert!(
            started.iter().all(|v| v.looping && v.delay == 0.0),
            "{file}"
        );
        assert_ne!(
            started[0].sound.seconds(),
            started[1].sound.seconds(),
            "{file}: two different loops"
        );
    }
    assert!(ran > 0, "no Pulse disc image was present");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn alternates_stay_a_no_repeat_pick_of_one_voice() {
    let mut ran = 0;
    for file in PULSE {
        let Some(path) = image(file) else { continue };
        ran += 1;
        let banks = banks(&path);
        let mut rng = Rng::new(11);
        let mut previous = None;
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..200 {
            let started = banks.voices(Cue::Collision, &mut rng).expect("loads");
            assert_eq!(started.len(), 1, "{file}");
            let key = started[0].sound.frames();
            assert_ne!(previous, Some(key), "{file}: the same take twice running");
            previous = Some(key);
            seen.insert(key);
        }
        assert!(seen.len() > 5, "{file}: only {} takes heard", seen.len());
    }
    assert!(ran > 0, "no Pulse disc image was present");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn titles_without_a_measured_tick_keep_the_flat_pick() {
    for file in NOT_PULSE {
        let Some(path) = image(file) else { continue };
        let banks = banks(&path);
        assert!(
            !banks
                .report
                .iter()
                .any(|l| l.contains("plays its timeline")),
            "{file}: {:?}",
            banks.report
        );
        let started = voices(&banks, Cue::Shield);
        assert_eq!(started.len(), 1, "{file}");
    }
}

const HD_TICK: f64 = 240.0;

/// The summed absolute level on each output channel of one voice played alone.
fn channel_levels(voice: &CueVoice) -> (f32, f32) {
    let mut mixer = Mixer::new(RATE);
    let _ = start_voices(
        &mut mixer,
        std::slice::from_ref(&CueVoice {
            delay: 0.0,
            ..voice.clone()
        }),
        Bus::Sfx,
        VoicePlace::DRY,
    );
    let mut out = vec![0.0f32; 2 * 4096];
    let (mut left, mut right) = (0.0, 0.0);
    for _ in 0..8 {
        mixer.render(&mut out);
        for frame in out.chunks(2) {
            left += frame[0].abs();
            right += frame[1].abs();
        }
    }
    (left, right)
}

/// `SPEEDUPPAD` on Fury, walked with HD's operand layout and cue variables
/// (`docs/ghidra/functions/ps3-hdfury-eu/cue-variables.md`, "`SPEEDUPPAD`"):
/// the player's own pad (perspective 0) keys a body, then a whoosh behind-left
/// at tick 0 and one behind-right ten ticks on, under one bend draw of up to two
/// semitones; another craft's pad (perspective 2) is the body and one whoosh.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_s_speedup_pad_is_a_body_and_two_rear_whooshes_for_the_player_and_one_for_others() {
    let Some(path) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let banks = banks(&path);

    let mut rng = Rng::new(3);
    let mut pitches = Vec::new();
    for _ in 0..200 {
        let mine = banks
            .voices_for(Cue::SpeedupPad, true, &mut rng)
            .expect("the pad loads");
        assert_eq!(mine.len(), 3, "body and two whooshes");
        let ticks: Vec<f64> = mine.iter().map(|v| v.delay * HD_TICK).collect();
        for (got, want) in ticks.iter().zip([0.0, 0.0, 10.0]) {
            assert!((got - want).abs() < 1e-6, "{ticks:?}");
        }
        let angles: Vec<i32> = mine.iter().map(|v| v.angle).collect();
        assert_eq!(angles, vec![0, 220, 140]);
        assert_eq!(mine[1].pitch, mine[2].pitch, "one bend draw for the pair");
        assert_eq!(mine[0].pitch, 1.0, "the body has no bend range");
        pitches.push(mine[1].pitch);
    }
    let low = pitches.iter().copied().fold(f32::MAX, f32::min);
    let high = pitches.iter().copied().fold(f32::MIN, f32::max);
    let two_semitones = 2f32.powf(2.0 / 12.0);
    assert!(
        low >= 1.0 / two_semitones - 1e-4 && high <= two_semitones + 1e-4,
        "{low} {high}"
    );
    assert!(
        low < 0.95 && high > 1.05,
        "the bend is drawn, not fixed: {low} {high}"
    );

    // Behind-left and behind-right, through the mixer: the 220 degree whoosh
    // is louder on the left, the 140 degree one on the right.
    let mine = banks
        .voices_for(Cue::SpeedupPad, true, &mut Rng::new(5))
        .unwrap();
    let (l, r) = channel_levels(&mine[1]);
    assert!(l > 1.3 * r, "220 degrees: left {l} right {r}");
    let (l, r) = channel_levels(&mine[2]);
    assert!(r > 1.3 * l, "140 degrees: left {l} right {r}");

    // Another craft's pad: the body and one of the two whooshes, centred.
    let mut late = std::collections::BTreeSet::new();
    for _ in 0..200 {
        let theirs = banks
            .voices_for(Cue::SpeedupPad, false, &mut rng)
            .expect("the pad loads");
        assert_eq!(theirs.len(), 2, "body and one whoosh");
        assert_eq!(theirs[0].delay, 0.0);
        assert!(theirs.iter().all(|v| v.angle == 0));
        late.insert((theirs[1].delay * HD_TICK).round() as u32);
    }
    assert_eq!(
        late.into_iter().collect::<Vec<_>>(),
        vec![0, 5],
        "the whoosh is keyed at tick 0 or tick 5, by the draw"
    );
}
