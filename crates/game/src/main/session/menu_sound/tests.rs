//! The name voices as `Session::speak_name` resolves and starts them, off a
//! real Pulse disc and without a window: the register the catalogue authors
//! for a confirmed id, and the voice that register starts.

use oag_ui_screens::picker::Kind;

use super::{name_register, play_name};

fn pulse() -> Option<oag_source::title::Opened> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    Some(
        oag_source::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source"),
    )
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd, see data/README.md"]
fn a_confirmed_id_resolves_to_the_register_its_plugin_entry_authors() {
    let Some(mut opened) = pulse() else { return };
    let blob = opened
        .archives
        .read_name(opened.title.plugin_definition)
        .expect("the definition reads");
    let documents = vec![oag_tables::fexml::text(&blob).expect("the definition decodes")];
    let tracks = oag_raceplay::catalogue::all_tracks(&documents);
    let teams = oag_raceplay::catalogue::all_teams(&documents);
    let register = |kind, id| name_register(kind, tracks.iter(), &teams, id);

    assert_eq!(register(Kind::Track, "16_Track"), Some(1), "Talon's Junction");
    assert_eq!(register(Kind::Track, "03_Track"), Some(3), "Moa Therma");
    assert_eq!(register(Kind::Ship, "Assegai"), Some(4));
    assert_eq!(register(Kind::Ship, "Feisar"), Some(1));
    assert_eq!(register(Kind::Ship, "no such team"), None);
    assert_eq!(register(Kind::Track, "Assegai"), None, "a team is not a circuit");
    assert_eq!(register(Kind::Ship, "16_Track"), None, "a circuit is not a team");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd, see data/README.md"]
fn the_register_starts_a_voice_that_sounds_after_its_call_and_not_before() {
    let Some(mut opened) = pulse() else { return };
    let tick = opened
        .title
        .race
        .zone_announcer
        .map_or(oag_title::SequenceTick::Unknown, |z| z.tick);
    let banks = *opened.title.race.sounds;
    let mut sfx = oag_sound::sfx::MenuSfx::load(&mut opened.archives, &banks, tick, false);
    for (kind, register) in [(Kind::Track, 1), (Kind::Ship, 4)] {
        let wav = std::env::temp_dir().join(format!("oag-name-voice-{kind:?}.wav"));
        let mut audio = oag_sound::Audio::open(
            &oag_sound::settings::Settings::default(),
            Some(wav.clone()),
            None,
            oag_audio::MIN_BUFFER,
            false,
        );
        for _ in 0..30 {
            audio.tick();
        }
        let before = audio.dump_frames().expect("a dump");
        assert!(play_name(&mut sfx, &audio, kind, register), "{kind:?}");
        for _ in 0..30 {
            audio.tick();
        }
        audio.finish().expect("writing the dump");
        let file = std::fs::read(&wav).expect("the dump");
        let pcm: Vec<i16> = file[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_le_bytes(*c))
            .collect();
        let peak = |from: usize, to: usize| {
            pcm[from * 2..to * 2]
                .iter()
                .map(|s| s.unsigned_abs())
                .max()
                .unwrap_or(0)
        };
        assert_eq!(peak(0, before), 0, "{kind:?}: sound before the call");
        assert!(
            peak(before, before + 30 * 800) > 100,
            "{kind:?}: the voice is silent"
        );
        std::fs::remove_file(&wav).ok();
    }
}
