use super::*;

fn state() -> RaceMusicState {
    RaceMusicState {
        index: 2,
        second: 45,
        tracks: 4,
        track_seconds: 180.0,
    }
}

const LISTING: [f64; 4] = [200.0, 190.0, 180.0, 170.0];

#[test]
fn a_matching_listing_accepts_the_state() {
    assert_eq!(state().check(&LISTING), Ok(()));
}

#[test]
fn decoder_padding_is_the_same_track() {
    let mut listed = LISTING;
    listed[2] = 179.917;
    assert_eq!(state().check(&listed), Ok(()));
}

#[test]
fn a_changed_playlist_is_refused_with_its_reason() {
    assert_eq!(state().check(&[]), Err(Unusable::Unlisted));
    assert_eq!(
        state().check(&LISTING[..3]),
        Err(Unusable::CountChanged { saved: 4, now: 3 })
    );
    let mut swapped = LISTING;
    swapped[2] = 183.0;
    assert!(matches!(
        state().check(&swapped),
        Err(Unusable::TrackChanged { index: 2, .. })
    ));
    let late = RaceMusicState {
        second: 180,
        ..state()
    };
    assert!(matches!(
        late.check(&LISTING),
        Err(Unusable::PastEnd { .. })
    ));
    let out = RaceMusicState {
        index: 9,
        ..state()
    };
    assert_eq!(
        out.check(&LISTING),
        Err(Unusable::OutOfRange { index: 9, now: 4 })
    );
}

#[test]
fn a_refused_state_changes_nothing() {
    let (mut audio, ..) = psp_boot_fixture();
    assert!(audio.apply_restored(&state(), &LISTING[..3]).is_err());
    assert_eq!(audio.race_index, None);
    assert_eq!(audio.race_position, 0.0);
}

/// 45 s into a race, the state is read out, a fresh `Audio` takes it back, and the
/// next race starts on the same track at the same second.
#[test]
fn a_race_resumes_on_the_same_track_at_the_same_second_in_a_new_audio() {
    let (mut first, ..) = psp_boot_fixture();
    let discs = discs();
    first.race_tracks = 4;
    first.race_index = Some(MUSIC_TRACK);
    first.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));
    for _ in 0..(60 * 45) {
        first.tick();
    }
    let saved = first.race_music_state().expect("a race is playing");
    assert_eq!(saved.index, MUSIC_TRACK);
    assert_eq!(saved.second, 45);
    assert_eq!(saved.track_seconds, 180.0);

    let (mut second, ..) = psp_boot_fixture();
    let mut listing = vec![100.0; 4];
    listing[MUSIC_TRACK] = 180.0;
    second.apply_restored(&saved, &listing).unwrap();
    second.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));

    assert_eq!(second.race_index, Some(MUSIC_TRACK));
    let at = second.playhead_race().expect("the race track is playing");
    assert!((at - 45.0).abs() <= 1.0, "resumed at {at}, saved 45");
}

#[test]
fn leaving_the_race_keeps_the_state_readable() {
    let (mut audio, ..) = psp_boot_fixture();
    audio.race_tracks = 4;
    audio.start_race_music(&discs(), MusicSource::Auto, Path::new("unused"));
    for _ in 0..(60 * 12) {
        audio.tick();
    }
    audio.pause_race_music();
    assert_eq!(audio.race_music_state().map(|s| s.second), Some(12));
}

#[test]
fn nothing_is_saved_before_a_race_has_started_the_playlist() {
    let (audio, ..) = psp_boot_fixture();
    assert_eq!(audio.race_music_state(), None);
}

fn discs() -> MusicDiscs {
    MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Psp),
    }
}
