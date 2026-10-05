//! The `MUSIC SOURCE` row's async switch: it must never block the caller,
//! and a stale fetch must not outlive the request that superseded it.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`, the same seam [`super::volumes`] was split
//! on.

use super::*;

/// Ticks `audio` until [`Audio::source_switch`] resolves, real wall-clock
/// sleeps between ticks so the worker's actual OS thread gets scheduled -
/// the tick count alone does not pace it, unlike everything else this
/// module's own docs say ticks pace. Panics past five real seconds, which
/// every case below resolves in well under: the fixture paths do not exist,
/// so the worker fails on the first `open`.
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

/// **The freeze this fixes**, proven rather than assumed. Before
/// this, a source read for the first time this session decoded
/// synchronously on whatever thread called [`Audio::set_music_source`],
/// which is reachable from a settings row a player can press mid-race.
///
/// A source that will *fail* to decode - a fake path, the same fixture
/// convention this file already uses elsewhere - is what proves the call
/// returned before the fetch was even attempted: a synchronous version
/// would have opened `"psp.chd"`, failed, and reported the failure before
/// this method ever returned. This one returns first, with the old voice
/// and `music_from` both untouched, and only fails later, off
/// [`Audio::tick`].
#[test]
fn a_menu_source_switch_does_not_block_the_caller() {
    let discs = MusicDiscs {
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

/// A press that lands back on the release already playing, while a switch
/// *away* from it is still in flight, means "stay here" - the in-flight
/// fetch is dropped rather than left to land later and move the voice a
/// second press already said not to.
///
/// This is new race-shaped territory the old synchronous row could not
/// reach at all: every press used to block until it finished, so a second
/// press physically could not land before the first one's fetch did.
#[test]
fn a_press_back_to_the_current_release_drops_a_switch_still_in_flight() {
    let discs = MusicDiscs {
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
