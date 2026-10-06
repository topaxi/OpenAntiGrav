//! The `MUSIC SOURCE` row's async switch: it must never block the caller, and a
//! stale fetch must not outlive the request that superseded it. Split out of
//! [`super`] under the 1,000-line rule, as [`super::volumes`].

use super::*;

/// Ticks `audio` until [`Audio::source_switch`] resolves, with real sleeps so the
/// worker's OS thread gets scheduled (tick count does not pace it). Panics past
/// five real seconds; every case resolves far sooner as the fixture paths do not
/// exist and the worker fails on the first `open`.
fn wait_for_source_switch(audio: &mut Audio) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while audio.source_switch.is_some() {
        assert!(
            std::time::Instant::now() < deadline,
            "the source-switch worker never landed"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
        audio.tick();
    }
}

/// The freeze this fixes, proven: a source read for the first time used to
/// decode synchronously on the thread calling [`Audio::set_music_source`],
/// reachable from a row pressed mid-race.
///
/// A fake path that fails to decode proves the call returned before the fetch was
/// attempted: a synchronous version would have opened `"psp.chd"`, failed and
/// reported before returning. This one returns first with the old voice and
/// `music_from` untouched, and fails later off [`Audio::tick`].
#[test]
fn a_menu_source_switch_does_not_block_the_caller() {
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()), // does not exist
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Ps2),
    };
    let ps2_sound = silence(180.0, 2, 48_000);
    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: Some(Dump {
            path: PathBuf::from("unused"),
            samples: Vec::new(),
        }),
        music: None,
        music_attempted: true,
        music_from: Some(Platform::Ps2),
        held: vec![(Platform::Ps2, Arc::clone(&ps2_sound))],
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    audio.music = audio
        .output
        .with_mixer(|mixer| mixer.play(Play::looping(Arc::clone(&ps2_sound), Bus::Music)));

    audio.set_music_source(&discs, MusicSource::Psp, Path::new("unused"));
    assert_eq!(
        audio.music_from,
        Some(Platform::Ps2),
        "not swapped before the fetch even started"
    );
    assert!(
        audio.source_switch.is_some(),
        "fetching in the background instead"
    );

    wait_for_source_switch(&mut audio);
    assert_eq!(
        audio.music_from,
        Some(Platform::Ps2),
        "a failed fetch leaves the music where it was, the same as the old synchronous Err arm"
    );
}

/// [`a_menu_source_switch_does_not_block_the_caller`]'s race counterpart,
/// through [`Audio::set_race_music_source`] instead.
#[test]
fn a_race_source_switch_does_not_block_the_caller() {
    let (mut audio, _menu_sound, _race_sound) = psp_boot_fixture();
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()), // does not exist
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Psp),
    };
    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));
    assert!(audio.race_voice.is_some(), "the cached track played");
    assert_eq!(audio.race_from, Some(Platform::Psp));

    audio.set_music_source(&discs, MusicSource::Ps2, Path::new("unused"));
    assert_eq!(
        audio.race_from,
        Some(Platform::Psp),
        "not swapped before the fetch even started"
    );
    assert!(
        audio.source_switch.is_some(),
        "fetching in the background instead"
    );

    wait_for_source_switch(&mut audio);
    assert_eq!(
        audio.race_from,
        Some(Platform::Psp),
        "a failed fetch leaves the race music where it was"
    );
}

/// A press back onto the release already playing, while a switch away is in
/// flight, means "stay here": the fetch is dropped rather than landing later.
/// Race-shaped territory the old synchronous row could not reach, since every
/// press blocked until its fetch finished.
#[test]
fn a_press_back_to_the_current_release_drops_a_switch_still_in_flight() {
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()), // does not exist
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Ps2),
    };
    let ps2_sound = silence(180.0, 2, 48_000);
    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: Some(Dump {
            path: PathBuf::from("unused"),
            samples: Vec::new(),
        }),
        music: None,
        music_attempted: true,
        music_from: Some(Platform::Ps2),
        held: vec![(Platform::Ps2, Arc::clone(&ps2_sound))],
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    audio.music = audio
        .output
        .with_mixer(|mixer| mixer.play(Play::looping(Arc::clone(&ps2_sound), Bus::Music)));

    audio.set_music_source(&discs, MusicSource::Psp, Path::new("unused"));
    assert!(
        audio.source_switch.is_some(),
        "fetching PSP in the background"
    );

    audio.set_music_source(&discs, MusicSource::Ps2, Path::new("unused"));
    assert!(
        audio.source_switch.is_none(),
        "the stale PSP fetch must be dropped on the spot, not left to land later"
    );
    assert_eq!(audio.music_from, Some(Platform::Ps2), "never left PS2");
}
