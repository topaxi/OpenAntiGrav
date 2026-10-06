//! 2048's circuit emitters: which authored `sound` nodes resolve to a cue, and
//! that what resolves is heard.
//!
//! **`#[ignore]`d and never run in CI.** They need the decrypted Vita package
//! under `data/extracted/vita/`. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! Evidence: `docs/formats/2048-audio.md`. The bank location is
//! `oag_title::TrackBanks::circuit_directory`, the shared labels its `shared`
//! list, and the lookup is `sblk::Bank::cue_named_or_hashed`.

use std::path::Path;

use oag_game::sound::listener_of;
use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_sound::sfx::TrackEmitters;

const IMAGE: &str = "data/extracted/vita/PCSF00007";

fn track(circuit: &str) -> String {
    format!(r"Data\art\published\{circuit}\track.vex")
}

fn load(circuit: &str) -> Option<TrackEmitters> {
    let path = oag_testdata::exact(IMAGE)?;
    let mut archives = oag_2048::open(&path.display().to_string()).expect("opening 2048");
    let track = track(circuit);
    let blob = archives.read_name(&track).expect("the circuit");
    let loaded = TrackEmitters::load(&mut archives, oag_2048::race::SOUND_BANKS, &track, &blob);
    for line in &loaded.report {
        println!("{circuit}: {line}");
    }
    Some(loaded)
}

/// `(circuit, emitters, resolved, unresolved references)`, every circuit with a
/// `track.vex` in the base package.
///
/// The unresolved ones are authored against a cue the named bank does not
/// hold: the circuit's bank spells every name it owns (the hashed name pool,
/// `Bank::hashed_names`) and none is the missing one, so these are the disc's
/// own dangling references, as Pulse's five are. `mall` is the odd one: its
/// manifest loads `env_tower.bnk` (label `env_tow`) while its two `startline`
/// nodes spell `env_mal`, a bank that ships nowhere.
const CIRCUITS: [(&str, usize, usize, &[&str]); 10] = [
    (r"environments\altima", 41, 40, &["env_alt~boat"]),
    (r"environments\arena", 3, 3, &[]),
    (r"environments\bridge", 4, 4, &[]),
    (r"environments\cathedral", 12, 12, &[]),
    (r"environments\mall", 5, 3, &["env_mal~startline"]),
    (r"environments\park", 2, 2, &[]),
    (r"environments\sol", 57, 57, &[]),
    (
        r"environments\square",
        45,
        0,
        &["crowd~crowdf", "env_squ~neoon_small"],
    ),
    (r"environments\subway", 9, 9, &[]),
    (
        r"environments\tower",
        48,
        44,
        &[
            "env_tow~NGP_Tannoy_1",
            "env_tow~NGP_Tannoy_3",
            "env_tow~NGP_Tannoy_4",
        ],
    ),
];

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn altima_resolves_forty_of_its_forty_one_emitters_and_the_rest_match_the_table() {
    for (circuit, total, resolved, dangling) in CIRCUITS {
        let Some(loaded) = load(circuit) else {
            return;
        };
        let got = loaded.omni.len() + loaded.directional.len();
        let playing = loaded
            .omni
            .iter()
            .chain(&loaded.directional)
            .filter(|n| n.sound.is_some())
            .count();
        assert_eq!((got, playing), (total, resolved), "{circuit}");
        let unplayed: Vec<&String> = loaded
            .report
            .iter()
            .filter(|l| {
                l.contains("node(s) dangle")
                    || l.contains("node(s) play nothing")
                    || l.contains("control only")
            })
            .collect();
        assert_eq!(unplayed.len(), dangling.len(), "{circuit}: {unplayed:#?}");
        for reference in dangling {
            assert!(
                unplayed
                    .iter()
                    .any(|l| l.starts_with(&format!("track audio {reference}"))),
                "{circuit}: {reference} was not reported"
            );
        }
        for node in loaded.omni.iter().chain(&loaded.directional) {
            if let Some(sound) = &node.sound {
                assert!(
                    sound.waveforms.iter().any(|(w, _)| w.seconds() > 0.0),
                    "{circuit}: {} plays an empty waveform",
                    node.emitter.cue
                );
            }
        }
        assert!(
            !loaded
                .report
                .iter()
                .any(|l| l.contains("not read") || l.contains("not a sound bank")),
            "{circuit}: a bank was not read"
        );
    }
}

struct Render {
    pcm: Vec<i16>,
    peak_voices: usize,
    clipped: u64,
    in_range_ticks: usize,
    in_range: Vec<bool>,
    wav: std::path::PathBuf,
}

/// One 40 s Altima lap rendered through the real mixer to `name`, with the
/// circuit's emitters kept or dropped, as the PCM and the ticks one was in range.
fn render(image: &Path, name: &str, keep: bool) -> Render {
    let mut loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(track(r"environments\altima")),
        ..race::Options::default()
    })
    .expect("loading the race");
    let emitters = loaded.setup.track_emitters.clone();
    if !keep {
        loaded.setup.track_emitters = TrackEmitters::default();
    }
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    let wav = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/shots")
        .join(name);
    std::fs::create_dir_all(wav.parent().expect("a parent")).expect("the shots directory");
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(wav.clone()),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    let mut peak_voices = 0;
    let mut in_range_ticks = 0;
    let mut in_range = Vec::new();
    for tick in 0..60 * 40 {
        race.tick(&PlayerInputs::none());
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();
        peak_voices = peak_voices.max(audio.ambient_voices());
        in_range.push(emitters.in_range(&listener_of(&race)) > 0);
        in_range_ticks += usize::from(in_range[tick]);
    }
    let clipped = audio.output().with_mixer(|mixer| mixer.clipped());
    audio.finish().expect("writing the dump");
    let file = std::fs::read(&wav).expect("the dump");
    let pcm = file[44..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| i16::from_le_bytes(*c))
        .collect();
    Render {
        pcm,
        peak_voices,
        clipped,
        in_range_ticks,
        in_range,
        wav,
    }
}

/// The same lap with and without the circuit's emitters: the ambience is on the
/// same bus as the engine, so it cannot be separated at the output, but the
/// difference of two renders of one deterministic lap is exactly what the
/// emitters add. Where none is in range the two are identical.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_headless_lap_of_altima_hears_its_ambience_where_an_emitter_is_in_range() {
    let Some(image) = oag_testdata::exact(IMAGE) else {
        return;
    };
    let with = render(&image, "altima-ambience.wav", true);
    let without = render(&image, "altima-no-emitters.wav", false);
    assert_eq!(
        without.peak_voices, 0,
        "the control lap opened an ambient voice"
    );
    assert!(with.in_range_ticks > 0, "no emitter was ever in range");
    assert!(
        with.peak_voices > 0,
        "an emitter was in range and no voice opened"
    );
    assert_eq!(with.pcm.len(), without.pcm.len());
    // 48 kHz stereo at 60 ticks a second is 1,600 samples a tick. A tick with no
    // emitter in range now or in the ten before it renders the same bytes in
    // both laps, which is what makes the other ticks' difference the emitters'.
    let tick = |r: &Render, t: usize| r.pcm[t * 1600..(t + 1) * 1600].to_vec();
    let (mut quiet, mut heard, mut differing) = (0, 0, 0);
    for t in 0..with.in_range.len() {
        let near = with.in_range[t.saturating_sub(10)..=t].iter().any(|&r| r);
        if near {
            heard += 1;
            differing += usize::from(tick(&with, t) != tick(&without, t));
        } else {
            quiet += 1;
            assert_eq!(
                tick(&with, t),
                tick(&without, t),
                "tick {t}: no emitter in range and the laps differ"
            );
        }
    }
    let loudest = with
        .pcm
        .iter()
        .zip(&without.pcm)
        .map(|(a, b)| i32::from(*a).abs_diff(i32::from(*b)))
        .max()
        .unwrap_or(0);
    println!(
        "{} - {} samples, {} ambient voice(s) at most, {quiet} tick(s) with none in range \
         (identical in both laps), {heard} near one, of which {differing} differ, by up to {loudest}; \
         clipped {} sample(s) with the emitters, {} without",
        with.wav.display(),
        with.pcm.len(),
        with.peak_voices,
        with.clipped,
        without.clipped,
    );
    assert!(
        differing > 0 && loudest > 0,
        "the emitters added nothing audible"
    );
    assert!(
        quiet > 0,
        "an emitter was in range for the whole lap, so nothing was compared"
    );
}

/// `(circuit, emitters, resolved)` for the twelve downloadable circuits, whose
/// bank sits **beside the track** and is read there, ahead of
/// `Data\audio\sound\`. Only `amphiseum`'s is a hashed v5 bank; the other
/// eleven parse as v3 banks with no name table, which is why they resolve few.
/// The same circuits' banks under `Data\audio\DLC1\` are hashed with 20 to 63
/// spelled cues, and which copy the original loads is unmeasured, so this
/// records what the beside-the-track copy gives and no more.
const DLC: [(&str, usize, usize); 12] = [
    ("Metropia", 13, 1),
    ("Sebenco_Climb", 37, 5),
    ("Sol_2", 20, 0),
    ("Ubermall", 131, 3),
    ("amphiseum", 25, 25),
    ("modesto_heights", 23, 5),
    ("talons_junction", 57, 0),
    ("tech_de_ra", 32, 2),
    ("Vineta_K", 48, 0),
    ("Anulpha_Pass", 52, 2),
    ("Chenghou_Project", 13, 5),
    ("Moa_Therma", 17, 1),
];

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_downloadable_circuits_still_read_the_bank_beside_their_track() {
    for (circuit, total, resolved) in DLC {
        let Some(loaded) = load(&format!(r"DLC1\environments\{circuit}")) else {
            return;
        };
        let playing = loaded
            .omni
            .iter()
            .chain(&loaded.directional)
            .filter(|n| n.sound.is_some())
            .count();
        assert_eq!(
            (loaded.omni.len() + loaded.directional.len(), playing),
            (total, resolved),
            "{circuit}"
        );
        assert!(
            loaded.report.iter().any(|l| l.contains(&format!(
                r"circuit Data\art\published\DLC1\environments\{circuit}\env"
            ))),
            "{circuit}: its own bank was not read beside it"
        );
    }
}
