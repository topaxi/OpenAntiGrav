//! Race-voice/menu-voice transitions: starting a race stops the menu voice and
//! starts a race one, ending a race resumes the menu voice from where it was, and
//! both survive `MUSIC SOURCE` changing mid-race. Split out of [`super`] under the
//! 1,000-line rule, the seam of [`super::source_switch`] and [`super::volumes`].

use super::*;
/// Starting a race stops the actual sounding menu voice, not just the field, and
/// starts a race voice on the playlist.
#[test]
fn starting_a_race_stops_the_menu_voice_and_starts_a_race_voice() {
    let (mut audio, _menu_sound, _race_sound) = psp_boot_fixture();
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Psp),
    };
    let menu_voice = audio.music.expect("the fixture starts the menu playing");

    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));

    assert!(
        !audio
            .output
            .with_mixer(|mixer| mixer.is_playing(menu_voice)),
        "the menu voice must actually stop, not just the field clear"
    );
    assert!(audio.music.is_none());
    assert!(audio.race_voice.is_some(), "a race track should be playing");
    assert_eq!(audio.race_from, Some(Platform::Psp));
    assert_eq!(
        audio.playhead_race(),
        Some(0.0),
        "a fresh start, not a seek"
    );
}

/// Ending a race saves the exact cut-off position, and the menu voice sounds
/// again as a new voice, silent until now.
#[test]
fn ending_a_race_saves_the_position_and_the_menu_voice_sounds_again() {
    let (mut audio, _menu_sound, _race_sound) = psp_boot_fixture();
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Psp),
    };
    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));

    for _ in 0..(60 * 45) {
        audio.tick();
    }
    let at = audio.playhead_race().expect("the race track is playing");
    assert!((at - 45.0).abs() < 0.01, "expected 45 s in, got {at}");

    audio.pause_race_music();

    assert!(audio.race_voice.is_none());
    assert!(
        (audio.race_position - 45.0).abs() < 0.01,
        "expected the cut-off saved at 45 s, got {}",
        audio.race_position
    );
    assert_eq!(
        audio.playhead(),
        Some(0.0),
        "the menu voice is a fresh start, not the paused race's position"
    );
}

/// The race analogue of [`changing_the_music_source_seeks_rather_than_restarting`]:
/// leaving and re-entering a race resumes the same track near where it was cut off.
#[test]
fn a_second_race_resumes_within_a_sixtieth_of_a_second_of_the_saved_position() {
    let (mut audio, _menu_sound, _race_sound) = psp_boot_fixture();
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Psp),
    };
    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));
    for _ in 0..(60 * 90) {
        audio.tick();
    }
    audio.pause_race_music();
    let saved = audio.race_position;

    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));
    let resumed = audio.playhead_race().expect("the race track is playing");

    assert!(
        (resumed - saved).abs() < 1.0 / 60.0,
        "expected to resume at {saved} s, landed at {resumed} s"
    );
}

/// `MUSIC SOURCE` moved mid-race must move the race voice, seek-preserving: not
/// no-op (both guard fields would name the menu) and not desync `race_from` from
/// what sounds.
#[test]
fn music_source_changed_while_a_race_is_live_moves_the_race_voice() {
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Ps2),
    };
    let psp_track = silence(180.0, 2, 44_100);
    let ps2_track = silence(180.0, 2, 48_000);

    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: Some(Dump {
            path: PathBuf::from("unused"),
            samples: Vec::new(),
        }),
        music: None,
        music_attempted: true,
        music_from: None,
        held: Vec::new(),
        movie: None,
        race_voice: None,
        race_from: Some(Platform::Ps2),
        race_index: Some(0),
        race_position: 0.0,
        // The target platform is already decoded (as `held` for the menu row),
        // avoiding a disc read with no fixture. See `psp_boot_fixture`.
        race_cache: Some((Platform::Psp, 0, Arc::clone(&psp_track))),
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    audio.race_voice = audio
        .output
        .with_mixer(|mixer| mixer.play(Play::once(Arc::clone(&ps2_track), Bus::Music)));

    for _ in 0..(60 * 20) {
        audio.tick();
    }
    let before = audio.playhead_race().expect("the race track is playing");
    assert!((before - 20.0).abs() < 0.01);

    audio.set_music_source(&discs, MusicSource::Psp, Path::new("unused"));

    assert_eq!(audio.race_from, Some(Platform::Psp), "the row moved it");
    let after = audio.playhead_race().expect("still playing");
    assert!(
        (after - before).abs() < 1.0 / 60.0,
        "the row must seek the race voice, not restart it: {before} s became {after} s"
    );
}

/// A source with no decodable race music still resumes the menu cleanly on
/// pause, with no dangling voice or `None` panic where the race never started.
#[test]
fn a_source_with_no_decodable_race_music_still_resumes_menu_music_cleanly() {
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: None,
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Psp),
    };
    let menu_sound = silence(28.0, 2, 44_100);
    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: Some(Dump {
            path: PathBuf::from("unused"),
            samples: Vec::new(),
        }),
        music: None,
        music_attempted: true,
        music_from: None,
        held: Vec::new(),
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        // Nothing cached and the fake PSP path has no soundtrack, so
        // `start_race_music` decodes nothing: only the menu's loop remains.
        race_cache: None,
        menu_sound: Some(Arc::clone(&menu_sound)),
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };

    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));
    assert!(audio.race_voice.is_none(), "nothing to decode");

    // Never started, so nothing to pause, but safe to call, and the menu returns.
    audio.pause_race_music();
    assert!(audio.music.is_some(), "the menu voice resumed");
    assert_eq!(audio.playhead(), Some(0.0));
}
