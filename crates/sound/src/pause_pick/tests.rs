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
