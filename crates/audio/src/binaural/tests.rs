use std::sync::Arc;

use super::*;
use crate::mixer::{Bus, Mixer, Play, Sound};
use crate::spatial::SpatialMode;

const RATE: u32 = 48_000;

/// A noisy mono source, so every frequency band is exercised.
fn noise(frames: usize) -> Arc<Sound> {
    let mut state = 0x1234_5678_u32;
    let samples = (0..frames)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((state >> 16) as i16) / 2
        })
        .collect();
    Arc::new(Sound::new(samples, 1, RATE).expect("sound"))
}

fn headphone_mixer() -> Mixer {
    let mut mixer = Mixer::new(RATE);
    mixer.set_binaural(Some(
        Binaural::new(RATE).expect("the shipped sphere parses"),
    ));
    mixer.set_spatial_mode(SpatialMode::Headphones);
    mixer
}

fn render_chunked(pan: f32, front: f32, once: bool, chunks: &[usize]) -> Vec<f32> {
    let mut mixer = headphone_mixer();
    let sound = noise(6_000);
    let play = if once {
        Play::once(sound, Bus::Sfx)
    } else {
        Play::looping(sound, Bus::Sfx)
    };
    mixer.play(Play {
        pan: Some(pan),
        front,
        ..play
    });
    let mut all = Vec::new();
    let mut at = 0;
    for &n in chunks.iter().cycle() {
        if at >= 9_000 {
            break;
        }
        let mut out = vec![0.0; 2 * n];
        mixer.render(&mut out);
        all.extend(out);
        at += n;
    }
    all
}

fn ear_energy(out: &[f32]) -> (f32, f32) {
    out.as_chunks::<2>()
        .0
        .iter()
        .skip(LATENCY + 64)
        .fold((0.0, 0.0), |(l, r), f| (l + f[0] * f[0], r + f[1] * f[1]))
}

#[test]
fn the_shipped_sphere_parses_at_both_common_rates() {
    assert!(Binaural::new(48_000).is_ok());
    assert!(Binaural::new(44_100).is_ok());
}

#[test]
fn a_source_to_the_right_is_louder_in_the_right_ear() {
    let (l, r) = ear_energy(&render_chunked(1.0, 0.0, false, &[512]));
    assert!(r > l * 2.0, "right {r} vs left {l}");
    let (l, r) = ear_energy(&render_chunked(-1.0, 0.0, false, &[512]));
    assert!(l > r * 2.0, "left {l} vs right {r}");
}

#[test]
fn headphones_tell_a_source_ahead_from_the_same_source_behind() {
    let ahead = render_chunked(0.0, 1.0, false, &[512]);
    let behind = render_chunked(0.0, -1.0, false, &[512]);
    let spectral = |out: &[f32]| {
        // Energy in the top of the band: the pinna's front/back cue.
        let mono: Vec<f32> = out.as_chunks::<2>().0.iter().map(|f| f[0] + f[1]).collect();
        mono.windows(2)
            .skip(LATENCY + 64)
            .map(|w| (w[1] - w[0]).powi(2))
            .sum::<f32>()
    };
    let (a, b) = (spectral(&ahead), spectral(&behind));
    assert!(
        (a - b).abs() > 0.05 * a.max(b),
        "front and back must differ: {a} vs {b}"
    );
}

#[test]
fn the_output_does_not_depend_on_how_render_is_chunked() {
    let one = render_chunked(0.4, -0.3, false, &[9_000]);
    for chunks in [vec![735], vec![512], vec![1, 7, 300, 1024]] {
        let other = render_chunked(0.4, -0.3, false, &chunks);
        let n = one.len().min(other.len());
        assert!(
            one[..n]
                .iter()
                .zip(&other[..n])
                .all(|(a, b)| a.to_bits() == b.to_bits()),
            "{chunks:?}"
        );
    }
}

#[test]
fn a_one_shot_plays_out_its_last_samples_then_frees_its_voice() {
    let mut mixer = headphone_mixer();
    mixer
        .play(Play {
            pan: Some(0.5),
            front: 0.5,
            ..Play::once(noise(1_000), Bus::Sfx)
        })
        .expect("voice");
    let mut energy = Vec::new();
    for _ in 0..10 {
        let mut out = vec![0.0; 2 * 256];
        mixer.render(&mut out);
        energy.push(out.iter().map(|s| s * s).sum::<f32>());
    }
    assert_eq!(
        mixer.active_voices(),
        0,
        "the voice is freed after its tail"
    );
    // 1,000 frames of source delayed by 512: sound is still coming out in the
    // block that holds frame 1,000 + 512 - 1 (the sixth, frames 1,280..1,536).
    assert!(energy[5] > 0.0, "the delayed tail is played, not cut");
    assert_eq!(energy[9], 0.0);
}

#[test]
fn asked_for_before_the_sphere_loads_headphones_sound_as_the_rear_cue() {
    let render = |mode: SpatialMode| {
        let mut mixer = Mixer::new(RATE);
        mixer.set_spatial_mode(mode);
        mixer.play(Play {
            pan: Some(0.2),
            front: -1.0,
            ..Play::looping(noise(6_000), Bus::Sfx)
        });
        let mut out = vec![0.0; 2 * 512];
        mixer.render(&mut out);
        out
    };
    assert_eq!(
        render(SpatialMode::Headphones),
        render(SpatialMode::RearCue)
    );
}

/// A bright, steady engine-like source: a sawtooth with noise on top.
fn engine_like() -> Arc<Sound> {
    let mut state = 7_u32;
    let samples = (0..RATE as usize)
        .map(|i| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let saw = ((i * 140 % RATE as usize) as f32 / RATE as f32) * 2.0 - 1.0;
            let hiss = ((state >> 16) as f32 / 32_768.0 - 1.0) * 0.25;
            ((saw * 0.5 + hiss) * 16_000.0) as i16
        })
        .collect();
    Arc::new(Sound::new(samples, 1, RATE).expect("sound"))
}

/// Writes a front/behind WAV for every mode, and a source circling the
/// listener, then prints what a headphone voice costs per 512 frames. Scratch
/// output for a person with headphones; asserts nothing.
///
/// `OAG_SWEEP=<dir> cargo nextest run -p oag-audio --run-ignored only spatial_sweep`
#[test]
#[ignore = "writes WAV files and times the convolver; needs OAG_SWEEP=<output dir>"]
fn spatial_sweep() {
    let Ok(dir) = std::env::var("OAG_SWEEP") else {
        return;
    };
    let modes = [
        SpatialMode::Original,
        SpatialMode::RearCue,
        SpatialMode::Headphones,
    ];
    for mode in modes {
        for (name, pan, front) in [("front", 0.0, 1.0), ("behind", 0.0, -1.0)] {
            let mut mixer = headphone_mixer();
            mixer.set_spatial_mode(mode);
            mixer.play(Play {
                pan: Some(pan),
                front,
                gain: 0.5,
                ..Play::looping(engine_like(), Bus::Sfx)
            });
            let mut out = vec![0.0; 2 * RATE as usize * 2];
            mixer.render(&mut out);
            let wav = crate::wav::from_samples(&out, RATE);
            std::fs::write(format!("{dir}/{mode}-{name}.wav"), wav).expect("write");
        }
        let mut mixer = headphone_mixer();
        mixer.set_spatial_mode(mode);
        let id = mixer
            .play(Play {
                pan: Some(0.0),
                gain: 0.5,
                ..Play::looping(engine_like(), Bus::Sfx)
            })
            .expect("voice");
        let mut all = Vec::new();
        for tick in 0..8 * 60 {
            let azimuth = tick as f32 / (8.0 * 60.0) * std::f32::consts::TAU;
            mixer.set_pan(id, Some(azimuth.sin()));
            mixer.set_front(id, azimuth.cos());
            let mut out = vec![0.0; 2 * 800];
            mixer.render(&mut out);
            all.extend(out);
        }
        let wav = crate::wav::from_samples(&all, RATE);
        std::fs::write(format!("{dir}/{mode}-circling.wav"), wav).expect("write");
    }

    for voices in [1_usize, 8, 32] {
        let mut mixer = headphone_mixer();
        for i in 0..voices {
            mixer.play(Play {
                pan: Some((i as f32 * 0.37).sin()),
                front: (i as f32 * 0.91).cos(),
                ..Play::looping(engine_like(), Bus::Sfx)
            });
        }
        let mut out = vec![0.0; 2 * 512];
        for _ in 0..8 {
            mixer.render(&mut out);
        }
        let blocks = 400;
        let start = std::time::Instant::now();
        for _ in 0..blocks {
            mixer.render(&mut out);
        }
        let per_block = start.elapsed() / blocks;
        println!(
            "headphones, {voices} voices: {per_block:?} per 512 frames, {:.2} % of one core, {:?} per voice",
            per_block.as_secs_f64() / (512.0 / f64::from(RATE)) * 100.0,
            per_block / voices as u32
        );
    }
}

fn placed_loop(mixer: &mut Mixer, sound: &Arc<Sound>, gain: f32, pan: f32) {
    mixer
        .play(Play {
            pan: Some(pan),
            front: 0.6,
            gain,
            ..Play::looping(sound.clone(), Bus::Sfx)
        })
        .expect("a free voice");
}

#[test]
fn only_the_cap_loudest_placed_voices_run_the_hrtf() {
    let sound = noise(RATE as usize);
    for (cap, placed) in [(8, 20), (8, 8), (8, 3), (32, 40)] {
        let mut mixer = headphone_mixer();
        mixer.grow_pool(64);
        mixer.set_hrtf_voice_cap(cap);
        for i in 0..placed {
            placed_loop(&mut mixer, &sound, 0.2 + 0.01 * i as f32, 0.3);
        }
        let mut out = vec![0.0; 512 * 2];
        mixer.render(&mut out);
        mixer.render(&mut out);
        assert_eq!(
            mixer.hrtf_voices(),
            cap.min(placed),
            "cap {cap}, placed {placed}"
        );
    }
}

#[test]
fn the_loudest_voices_keep_the_hrtf_when_a_louder_one_arrives() {
    let sound = noise(RATE as usize);
    let mut mixer = headphone_mixer();
    mixer.set_hrtf_voice_cap(2);
    placed_loop(&mut mixer, &sound, 0.5, 0.3);
    placed_loop(&mut mixer, &sound, 0.4, -0.3);
    placed_loop(&mut mixer, &sound, 0.1, 0.0);
    let mut out = vec![0.0; 512 * 2];
    mixer.render(&mut out);
    assert_eq!(mixer.hrtf_voices(), 2);
    placed_loop(&mut mixer, &sound, 0.9, 0.0);
    for _ in 0..4 {
        mixer.render(&mut out);
    }
    assert_eq!(
        mixer.hrtf_voices(),
        2,
        "still exactly the cap after the swap"
    );
}

fn largest_step(out: &[f32]) -> f32 {
    let frames = out.as_chunks::<2>().0;
    frames
        .iter()
        .zip(frames.iter().skip(1))
        .flat_map(|(a, b)| [(b[0] - a[0]).abs(), (b[1] - a[1]).abs()])
        .fold(0.0, f32::max)
}

/// One tone voice rendered through the HRTF, then pushed out of it by a cap of
/// zero; returns the largest sample-to-sample step after that and the HRTF
/// voices left.
fn largest_step_after_losing_the_hrtf(lose: bool) -> (f32, usize) {
    // A constant source, so the mix is flat and any step in it is the hand-over.
    let tone: Vec<i16> = vec![12_000; RATE as usize];
    let sound = Arc::new(Sound::new(tone, 1, RATE).expect("sound"));
    let mut mixer = headphone_mixer();
    placed_loop(&mut mixer, &sound, 0.5, 0.4);
    let mut out = vec![0.0; 512 * 2];
    for _ in 0..6 {
        mixer.render(&mut out);
    }
    if lose {
        mixer.set_hrtf_voice_cap(0);
    }
    // Keep the last settled frame, so a step at the switch itself is seen.
    let mut after = out[out.len() - 2..].to_vec();
    for _ in 0..8 {
        mixer.render(&mut out);
        after.extend_from_slice(&out);
    }
    (largest_step(&after), mixer.hrtf_voices())
}

#[test]
fn handing_over_between_the_hrtf_and_the_rear_cue_has_no_step() {
    let (steady, kept) = largest_step_after_losing_the_hrtf(false);
    assert_eq!(kept, 1);
    let (handed, left) = largest_step_after_losing_the_hrtf(true);
    assert_eq!(left, 0, "the faded voice releases its convolver");
    assert!(
        handed < steady + 0.002,
        "hand-over step {handed} against {steady} with no hand-over"
    );
}

/// A constant tone voice settled in `from`, switched to `to`; the largest
/// sample-to-sample step from the last settled frame on, and the HRTF voices
/// still alive 128 frames after the switch.
fn switch_modes(from: SpatialMode, to: SpatialMode) -> (f32, usize) {
    // A 100 Hz sine, whole cycles to the loop: a source with no steps of its
    // own, so a step in the mix is the switch (a constant would ring through
    // the convolver's own start-up).
    let tone: Vec<i16> = (0..RATE as usize)
        .map(|i| ((i as f32 * std::f32::consts::TAU * 100.0 / RATE as f32).sin() * 12_000.0) as i16)
        .collect();
    let sound = Arc::new(Sound::new(tone, 1, RATE).expect("sound"));
    let mut mixer = headphone_mixer();
    mixer.set_spatial_mode(from);
    placed_loop(&mut mixer, &sound, 0.5, 0.4);
    let mut out = vec![0.0; 512 * 2];
    for _ in 0..6 {
        mixer.render(&mut out);
    }
    mixer.set_spatial_mode(to);
    let mut after = out[out.len() - 2..].to_vec();
    // Half the 256-frame fade: a cut would already have dropped the convolver.
    let mut half = vec![0.0; 128 * 2];
    mixer.render(&mut half);
    after.extend_from_slice(&half);
    let alive = mixer.hrtf_voices();
    for _ in 0..8 {
        mixer.render(&mut out);
        after.extend_from_slice(&out);
    }
    (largest_step(&after), alive)
}

#[test]
fn a_mode_switch_mid_race_crossfades_in_both_directions() {
    let (steady, _) = switch_modes(SpatialMode::RearCue, SpatialMode::RearCue);
    let (leaving, alive) = switch_modes(SpatialMode::Headphones, SpatialMode::RearCue);
    assert_eq!(alive, 1, "the convolver outlives the switch by its fade");
    assert!(
        leaving < steady * 3.0,
        "leaving: step {leaving} against {steady}"
    );
    let (entering, _) = switch_modes(SpatialMode::RearCue, SpatialMode::Headphones);
    assert!(
        entering < steady * 3.0,
        "entering: step {entering} against {steady}"
    );
}

/// Energy gain of the HRTF path over the rear-cue path for the same noise
/// source, over a grid of directions. Ahead and to the side it is within 1.1 dB;
/// behind it is 4 to 9 dB louder because the rear cue darkens and trims what is
/// behind and the HRTF does not on white noise. Asserts the front hemisphere.
#[test]
fn the_hrtf_is_energy_neutral_ahead_and_beside() {
    let energy = |mode: SpatialMode, pan: f32, front: f32| {
        let mut mixer = headphone_mixer();
        mixer.set_limiting(false);
        mixer.set_spatial_mode(mode);
        mixer.play(Play {
            pan: Some(pan),
            front,
            ..Play::looping(noise(6_000), Bus::Sfx)
        });
        let mut out = vec![0.0; 2 * 9_000];
        mixer.render(&mut out);
        let (l, r) = ear_energy(&out[2 * 2_000..]);
        l + r
    };
    let mut total = 0.0;
    let mut count = 0.0;
    for pan in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        for front in [1.0, 0.0, -1.0] {
            let db = 10.0
                * (energy(SpatialMode::Headphones, pan, front)
                    / energy(SpatialMode::RearCue, pan, front))
                .log10();
            println!("pan {pan:+.1} front {front:+.1}: {db:+.2} dB");
            if front >= 0.0 {
                assert!(db.abs() < 2.0, "{db} dB at pan {pan} front {front}");
            }
            total += db;
            count += 1.0;
        }
    }
    let mean = total / count;
    println!("mean {mean:+.2} dB");
    let _ = mean;
}
