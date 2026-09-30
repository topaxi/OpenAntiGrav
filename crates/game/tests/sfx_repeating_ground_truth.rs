//! `~BLOWUP` and `~ROCKLOCK` played as the repeating lists Pulse's `hud.bnk`
//! authors, against the discs.
//!
//! **`#[ignore]`d and never run in CI**: they need game content. Run with
//! `just test-data`.
//!
//! `~ROCKLOCK` is `[0x15, guard(param0 == 0), key-on +30, guard(param0 == 1),
//! key-on +15, 0x16]`: a beep every 30 master ticks while the reticle is
//! seeking and every 15 once it is locked. `~BLOWUP` is `[key-on loop, 0x15,
//! key-on, 0x1a, 0x16]`: the second waveform re-keyed every 43 ticks. Both at
//! the measured 258.4 Hz. Set `OAG_RENDER_DIR` to also write each render as a
//! WAV to look at.

use std::path::PathBuf;

use oag_audio::{Bus, Mixer};
use oag_core::Rng;
use oag_game::audio::sfx::{Banks, Cue, Playing, VoicePlace};

const RATE: u32 = 44_100;
const TICK: f64 = 1.0 / 60.0;
const MASTER: f64 = 44_100.0 * 3.0 / 512.0;
const PULSE: [&str; 3] = ["pulse-psp-usa.chd", "pulse-psp-eu.chd", "pulse-ps2-eu.chd"];
/// Titles whose lists are read on no binary this runner was read from.
const NOT_PSP_TICK: [&str; 2] = ["pure-psp-usa.chd", "hdfury-ps3-eu-dec.iso"];

fn banks(path: &std::path::Path) -> Banks {
    let opened = oag_game::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
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

/// Renders `ticks` simulation ticks of `playing`, `before` each one.
fn render(
    playing: &mut Playing,
    mixer: &mut Mixer,
    rng: &mut Rng,
    bus: Bus,
    ticks: usize,
    mut before: impl FnMut(&mut Playing, usize),
) -> Vec<f32> {
    let mut out = Vec::new();
    for tick in 0..ticks {
        before(playing, tick);
        playing.advance(TICK, mixer, bus, VoicePlace::DRY, rng);
        let frames = ((tick + 1) as f64 * TICK * f64::from(RATE)).round() as usize
            - (tick as f64 * TICK * f64::from(RATE)).round() as usize;
        let mut chunk = vec![0.0f32; frames * 2];
        mixer.render(&mut chunk);
        out.extend(chunk.chunks(2).map(|f| f[0] + f[1]));
    }
    out
}

/// Runs of near-silence longer than `min` seconds.
fn gaps(samples: &[f32], min: f64) -> usize {
    let floor = 1e-4;
    let (mut run, mut found) = (0usize, 0usize);
    for &s in samples {
        if s.abs() < floor {
            run += 1;
        } else {
            if run as f64 / f64::from(RATE) >= min {
                found += 1;
            }
            run = 0;
        }
    }
    found
}

fn dump(name: &str, samples: &[f32]) {
    if let Some(dir) = std::env::var_os("OAG_RENDER_DIR") {
        let path = PathBuf::from(dir).join(format!("{name}.wav"));
        let stereo: Vec<f32> = samples.iter().flat_map(|&s| [s, s]).collect();
        std::fs::write(path, oag_audio::wav::from_samples(&stereo, RATE)).expect("write wav");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_lock_tone_beeps_at_the_tempo_its_parameter_picks() {
    let mut ran = 0;
    for file in PULSE {
        let Some(path) = oag_testdata::image(file) else {
            continue;
        };
        ran += 1;
        let banks = banks(&path);
        let program = banks.program(Cue::LockOn).expect("~ROCKLOCK is a program");
        let bus = Cue::LockOn.bus();
        let mut mixer = Mixer::new(RATE);
        let mut rng = Rng::new(1);
        let mut playing = Playing::open(program, &mut mixer, bus, VoicePlace::DRY, &mut rng);

        // One second seeking, then one second locked.
        let mut at_flip = 0;
        let audio = render(&mut playing, &mut mixer, &mut rng, bus, 120, |p, tick| {
            if tick == 60 {
                at_flip = p.keyed();
            }
            p.set_parameter(0, i8::from(tick >= 60));
        });
        let seeking = at_flip;
        let locked = playing.keyed() - at_flip;
        // A beep at ticks 30, 60, 90, ... in 258.4 ticks a second: eight before
        // the second is out (the eighth at 240 is 0.929 s).
        assert_eq!(seeking, (MASTER / 30.0).floor() as usize, "{file}: seeking");
        // The pass that was already waiting when the parameter flipped keys the
        // seeking beep once more, and every pass after it every 15 ticks.
        let expected = (MASTER / 15.0).floor() as usize;
        assert!(
            (expected - 2..=expected + 1).contains(&locked),
            "{file}: {locked} locked beeps against {expected}"
        );
        // Seeking has audible gaps between 52 ms beeps 116 ms apart; locked
        // beeps run 58 ms apart and leave none that long.
        assert!(gaps(&audio[..RATE as usize], 0.03) >= 6, "{file}: seeking");
        assert_eq!(gaps(&audio[RATE as usize + 4_000..], 0.03), 0, "{file}");
        dump(&format!("rocklock-{file}"), &audio);

        playing.stop(&mut mixer);
        assert_eq!(mixer.active_voices(), 0, "{file}: stopped");
    }
    assert!(ran > 0, "no Pulse disc image was present");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_explosion_re_keys_its_second_waveform_every_43_ticks_over_a_held_loop() {
    let mut ran = 0;
    for file in PULSE {
        let Some(path) = oag_testdata::image(file) else {
            continue;
        };
        ran += 1;
        let banks = banks(&path);
        let program = banks.program(Cue::Blowup).expect("~BLOWUP is a program");
        let bus = Cue::Blowup.bus();
        let mut mixer = Mixer::new(RATE);
        let mut rng = Rng::new(1);
        let mut playing = Playing::open(program, &mut mixer, bus, VoicePlace::DRY, &mut rng);
        // The loop and the first re-key start together.
        assert_eq!(playing.keyed(), 2, "{file}");
        let audio = render(&mut playing, &mut mixer, &mut rng, bus, 60, |_, _| {});
        // Re-keys at 43, 86, ... 258 ticks: six more within the first second.
        assert_eq!(
            playing.keyed(),
            2 + (MASTER / 43.0).floor() as usize,
            "{file}"
        );
        assert!(mixer.active_voices() > 0, "{file}: the loop is held");
        dump(&format!("blowup-{file}"), &audio);
        playing.stop(&mut mixer);
        assert_eq!(mixer.active_voices(), 0, "{file}: stopped");
    }
    assert!(ran > 0, "no Pulse disc image was present");
}

/// The runner's semantics (the per-tick clear of `0x16`'s flag, the guard's
/// parameter bytes, `HudSight` writing parameter 0) were read from Pulse's PSP
/// executable, so a title on another tick keeps the one-shot path.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_title_off_the_psp_tick_builds_no_repeating_program() {
    let mut ran = 0;
    for file in NOT_PSP_TICK {
        let Some(path) = oag_testdata::image(file) else {
            continue;
        };
        ran += 1;
        let banks = banks(&path);
        for cue in [Cue::Blowup, Cue::LockOn] {
            assert!(banks.program(cue).is_none(), "{file}: {}", cue.name());
        }
    }
    assert!(ran > 0, "no Pure or HD disc image was present");
}
