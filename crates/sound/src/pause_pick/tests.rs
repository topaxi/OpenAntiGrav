use super::*;

fn audio() -> Audio {
    Audio::open(
        &crate::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        false,
    )
}

/// A PSP boot whose soundtrack lists three songs, the third unnamed.
fn discs() -> MusicDiscs {
    let discs = MusicDiscs {
        booted: Some(Platform::Psp),
        psp: Some("psp.chd".into()),
        ..MusicDiscs::default()
    };
    let named = |artist: &str, title: &str| {
        Some(SongName {
            artist: Some(artist.to_string()),
            title: title.to_string(),
        })
    };
    discs.listings.seed(
        "psp.chd",
        Platform::Psp,
        Soundtrack {
            tracks: [200.0, 210.0, 220.0]
                .iter()
                .enumerate()
                .map(|(at, seconds)| Track {
                    at: at as u32,
                    seconds: *seconds,
                })
                .collect(),
            names: vec![
                named("Aphex Twin", "Fenix Funk 5 (Wipeout Edit)"),
                named("Booka Shade", "Steady Rush"),
                None,
            ],
        },
    );
    discs
}

/// The list shows each song's title alone, in playlist order, from the song that is playing.
#[test]
fn the_playlist_is_the_soundtrack_by_title_from_the_playing_song() {
    let mut audio = audio();
    let discs = discs();
    assert_eq!(audio.race_playlist(&discs), None, "no race song yet");
    audio.race_index = Some(1);
    assert_eq!(
        audio.race_playlist(&discs),
        Some(RacePlaylist {
            titles: vec![
                "Fenix Funk 5 (Wipeout Edit)".to_string(),
                "Steady Rush".to_string(),
                "Track 3".to_string(),
            ],
            playing: 1,
        })
    );
}

/// Another song picked while paused starts from its beginning when the race resumes, as a new song.
#[test]
fn a_pick_of_another_song_starts_it_from_the_beginning() {
    let mut audio = audio();
    audio.race_index = Some(0);
    audio.race_position = 42.0;
    assert!(audio.pick_race_song(1));
    assert_eq!(audio.race_index, Some(1));
    assert_eq!(audio.race_music_pick(), RacePick::exact(1));
}

/// The playing song picked again resumes where it was cut off.
#[test]
fn picking_the_playing_song_keeps_its_place() {
    let mut audio = audio();
    audio.race_index = Some(0);
    audio.race_position = 42.0;
    assert!(!audio.pick_race_song(0));
    assert_ne!(
        audio.race_music_pick(),
        RacePick::exact(0),
        "the pick carries the place it was cut off at"
    );
    assert!((audio.race_position - 42.0).abs() < f64::EPSILON);
}

/// A pick made while a fetch is in flight (a pause opened while the next track was being read)
/// drops that fetch: landing, it would move the playlist off the pick.
#[test]
fn a_pick_drops_a_fetch_in_flight() {
    let mut audio = audio();
    audio.race_index = Some(0);
    audio.race_job = Some(MusicFetchWorker::spawn(
        MusicDiscs::default(),
        MusicSource::Auto,
        std::path::PathBuf::from("unused"),
        RacePick::exact(0),
        "test-fetch",
    ));
    assert!(audio.pick_race_song(1));
    assert!(audio.race_job.is_none(), "the old fetch is gone");
    assert_eq!(audio.race_music_pick(), RacePick::exact(1));
}

/// A boot surveyed in the background (every live boot) names a PS2 song through the PSP
/// pressing the survey found, not through the value's own unsurveyed fields.
#[test]
fn a_background_surveyed_ps2_boot_names_its_songs_through_the_psp_pressing() {
    let listings = crate::Listings::default();
    let seconds = [200.0, 210.0];
    let soundtrack = |names: Vec<Option<SongName>>| crate::Soundtrack {
        tracks: seconds
            .iter()
            .enumerate()
            .map(|(at, seconds)| Track {
                at: at as u32,
                seconds: *seconds,
            })
            .collect(),
        names,
    };
    listings.seed("ps2.chd", Platform::Ps2, soundtrack(Vec::new()));
    listings.seed(
        "psp.chd",
        Platform::Psp,
        soundtrack(vec![
            Some(SongName {
                artist: None,
                title: "Fenix Funk 5 (Wipeout Edit)".to_string(),
            }),
            Some(SongName {
                artist: None,
                title: "Steady Rush".to_string(),
            }),
        ]),
    );
    let surveyed = MusicDiscs {
        booted: Some(Platform::Ps2),
        ps2: Some("ps2.chd".into()),
        psp: Some("psp.chd".into()),
        listings: listings.clone(),
        ..MusicDiscs::default()
    };
    let discs = MusicDiscs {
        listings,
        later: crate::survey::Later::answered(surveyed),
        ..MusicDiscs::default()
    };
    let mut audio = audio();
    audio.race_index = Some(0);
    assert_eq!(
        audio.race_playlist(&discs).map(|list| list.titles),
        Some(vec![
            "Fenix Funk 5 (Wipeout Edit)".to_string(),
            "Steady Rush".to_string()
        ])
    );
}
