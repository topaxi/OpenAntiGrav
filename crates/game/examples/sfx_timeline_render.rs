//! Renders Pulse cues as the timeline they author, to WAV, and prints each
//! voice's delay and length beside what the mixer produced.
//!
//! ```sh
//! cargo run -p oag-game --example sfx_timeline_render -- data/images/pulse-psp-eu.chd out_dir
//! ```
//!
//! No audio device is opened: the mixer renders to a buffer.

use oag_audio::{Bus, Mixer};
use oag_sound::sfx::{Banks, Cue, VoicePlace, start_voices};

const RATE: u32 = 44_100;
/// Longest a render runs, so a held loop ends the run.
const CAP_SECONDS: usize = 8;

fn main() {
    let mut args = std::env::args().skip(1);
    let image = args.next().expect("image");
    let out = args.next().expect("out dir");
    std::fs::create_dir_all(&out).unwrap();
    let opened = oag_game::title::open_source(&image, Vec::new(), Vec::new()).expect("opens");
    let tick = opened
        .title
        .race
        .zone_announcer
        .map_or(oag_title::SequenceTick::Unknown, |z| z.tick);
    let sounds = opened.title.race.sounds;
    let mut archives = opened.archives;
    let banks = Banks::load(&mut archives, sounds, false, tick);
    for line in banks.report.iter().filter(|l| l.contains("timeline")) {
        println!("{line}");
    }
    for cue in Cue::ALL {
        if cue == Cue::Engine {
            continue;
        }
        let mut rng = oag_core::Rng::new(7);
        let Some(voices) = banks.voices(cue, &mut rng) else {
            println!("{}: not loaded", cue.name());
            continue;
        };
        let end = voices
            .iter()
            .map(|v| v.delay + f64::from(v.sound.seconds()) / f64::from(v.pitch))
            .fold(0.0_f64, f64::max);
        let looping = voices.iter().any(|v| v.looping);
        println!(
            "{}: {} voice(s), authored end {end:.3}s{}",
            cue.name(),
            voices.len(),
            if looping { " (holds a loop)" } else { "" }
        );
        for v in &voices {
            println!(
                "    at {:.3}s {:.3}s {}Hz pitch {:.3} angle {} loop={}",
                v.delay,
                v.sound.seconds(),
                v.sound.sample_rate(),
                v.pitch,
                v.angle,
                v.looping
            );
        }
        let mut mixer = Mixer::new(RATE);
        let _ = start_voices(&mut mixer, &voices, cue.bus(), VoicePlace::DRY);
        let mut pcm: Vec<i16> = Vec::new();
        let mut buf = vec![0.0f32; 1024];
        while mixer.active_voices() > 0 && pcm.len() < CAP_SECONDS * RATE as usize * 2 {
            mixer.render(&mut buf);
            pcm.extend(buf.iter().map(|&x| (x * 32767.0) as i16));
        }
        let quiet = pcm
            .chunks(2)
            .rposition(|f| f[0].unsigned_abs() > 32 || f[1].unsigned_abs() > 32)
            .map_or(0.0, |i| (i + 1) as f64 / f64::from(RATE));
        println!(
            "    rendered {:.3}s, last audible frame at {quiet:.3}s",
            pcm.len() as f64 / 2.0 / f64::from(RATE)
        );
        let data = (pcm.len() * 2) as u32;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&RATE.to_le_bytes());
        wav.extend_from_slice(&(RATE * 4).to_le_bytes());
        wav.extend_from_slice(&4u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data.to_le_bytes());
        for s in &pcm {
            wav.extend_from_slice(&s.to_le_bytes());
        }
        let file = cue.name().replace(['~', '.'], "_");
        std::fs::write(format!("{out}/{file}.wav"), wav).unwrap();
        let _ = Bus::Sfx;
    }
}
