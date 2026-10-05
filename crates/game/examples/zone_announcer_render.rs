//! Renders Pulse's Zone announcer lines to WAV and prints their loudness envelope.
//!
//! ```sh
//! cargo run -p oag-game --example zone_announcer_render -- data/images/pulse-psp-usa.chd out_dir
//! ```

use oag_sound::sfx::Announcer;

fn main() {
    let mut args = std::env::args().skip(1);
    let image = args.next().expect("image");
    let out = args.next().expect("out dir");
    std::fs::create_dir_all(&out).unwrap();
    let opened = oag_game::title::open_source(&image, Vec::new(), Vec::new()).expect("opens");
    let mut archives = opened.archives;
    let announcer = Announcer::load(&mut archives, opened.title.race.zone_announcer);
    for line in &announcer.report {
        println!("{line}");
    }
    let mut rng = oag_core::Rng::new(1);
    for milestone in [5u16, 10, 25, 50, 100] {
        let Some((sound, _)) = announcer.pick(milestone, &mut rng) else {
            println!("zone_{milestone}: none");
            continue;
        };
        let rate = 44_100u32;
        let mut mixer = oag_audio::Mixer::new(rate);
        let _ = mixer.play(oag_audio::Play::once(sound.clone(), oag_audio::Bus::Speech));
        let mut pcm: Vec<i16> = Vec::new();
        let mut buf = vec![0.0f32; 1024];
        while mixer.active_voices() > 0 {
            mixer.render(&mut buf);
            pcm.extend(buf.iter().map(|&x| (x * 32767.0) as i16));
        }
        let frames = pcm.len() / 2;
        let win = (rate / 50) as usize;
        let mut env = String::new();
        for w in 0..frames.div_ceil(win) {
            let lo = w * win * 2;
            let hi = ((w + 1) * win * 2).min(pcm.len());
            let e: f32 = pcm[lo..hi]
                .iter()
                .map(|&x| (f32::from(x) / 32768.0).powi(2))
                .sum();
            let db = 10.0 * (e / (hi - lo) as f32 + 1e-12).log10();
            env.push(match db {
                d if d > -20.0 => '#',
                d if d > -30.0 => '+',
                d if d > -45.0 => '.',
                _ => ' ',
            });
        }
        let mut wav = Vec::new();
        let data: u32 = (pcm.len() * 2) as u32;
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&rate.to_le_bytes());
        wav.extend_from_slice(&(rate * 4).to_le_bytes());
        wav.extend_from_slice(&4u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data.to_le_bytes());
        for s in &pcm {
            wav.extend_from_slice(&s.to_le_bytes());
        }
        std::fs::write(format!("{out}/composed_zone_{milestone}.wav"), wav).unwrap();
        println!(
            "zone_{milestone} {:.3}s (sound) {:.3}s (mixed) gain {:.3}\n  |{env}|",
            sound.seconds(),
            frames as f32 / rate as f32,
            sound.pan_volume_gain()
        );
    }
}
