//! Renders a race's first 340 ticks to a WAV and prints the cues raised and the runs of sound.
//!
//! `cargo run -p oag-game --example countdown_census -- <image> <out.wav> [Mode]`

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_sound::Volume;

fn main() {
    let mut args = std::env::args().skip(1);
    let image = args.next().expect("image");
    let wav = std::path::PathBuf::from(args.next().expect("out.wav"));
    let mode = match args.next().as_deref() {
        Some("zone") => oag_race::Mode::Zone,
        Some("elim") => oag_race::Mode::Eliminator,
        _ => oag_race::Mode::SingleRace,
    };
    let loaded = race::load(&race::Options {
        source: image,
        class: "VENOM".to_string(),
        mode,
        ..race::Options::default()
    })
    .expect("load");
    for line in loaded.report.iter().filter(|l| {
        let l = l.to_lowercase();
        l.contains("ready") || l.contains("go") || l.contains("speech") || l.contains("sfx")
    }) {
        println!("report: {line}");
    }
    println!("countdown_voice: {}", loaded.setup.countdown_voice);
    let mut setup = loaded.setup;
    if std::env::var("CENSUS_NOVOICE").is_ok() {
        setup.countdown_voice = false;
    }
    let mut race = race::Race::start(setup);
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings {
            music_volume: Volume::OFFERED[0],
            sfx_volume: if std::env::var("CENSUS_SFX").is_ok() {
                Volume::default()
            } else {
                Volume::OFFERED[0]
            },
            ..Default::default()
        },
        Some(wav.clone()),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    for _ in 0..340 {
        race.tick(&PlayerInputs::none());
        let stepped = race.sim.world.tick - 1;
        for e in race.pending_cues() {
            println!("tick {stepped}: cue {:?}", e.cue);
        }
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();
    }
    audio.finish().expect("dump");
    let pcm = std::fs::read(&wav).expect("wav");
    let pcm = &pcm[44..];
    let fpt = (oag_sound::DUMP_SAMPLE_RATE / 60) as usize;
    let gap = (oag_sound::DUMP_SAMPLE_RATE / 5) as usize;
    let mut runs: Vec<(usize, usize, i32)> = Vec::new();
    for f in 0..pcm.len() / 4 {
        let l = i16::from_le_bytes([pcm[f * 4], pcm[f * 4 + 1]]).unsigned_abs() as i32;
        let r = i16::from_le_bytes([pcm[f * 4 + 2], pcm[f * 4 + 3]]).unsigned_abs() as i32;
        let a = l.max(r);
        if a <= 8 {
            continue;
        }
        match runs.last_mut() {
            Some(run) if f - run.1 <= gap => {
                run.1 = f;
                run.2 = run.2.max(a);
            }
            _ => runs.push((f, f, a)),
        }
    }
    for (s, e, peak) in runs {
        println!(
            "sound: tick {:.1} to {:.1} ({:.2} s) peak {peak}",
            s as f32 / fpt as f32,
            e as f32 / fpt as f32,
            (e - s) as f32 / oag_sound::DUMP_SAMPLE_RATE as f32
        );
    }
}
