use super::*;

#[test]
fn previous_is_the_inverse_of_next_and_wraps() {
    for len in [2_usize, 3, 16] {
        for index in 0..len {
            let next = crate::next_race_index(index, len);
            assert_eq!(race_music::previous_race_index(next, len), index);
        }
    }
    assert_eq!(race_music::previous_race_index(0, 16), 15);
    assert_eq!(race_music::previous_race_index(5, 0), 5);
}

#[test]
fn previous_resolves_one_before_in_the_booted_order() {
    let discs = MusicDiscs {
        booted: Some(Platform::Psp),
        psp: Some("psp.chd".into()),
        ..MusicDiscs::default()
    };
    assert_eq!(
        race_music::RacePick::before(0).resolve_for_test(&discs, 16),
        15
    );
    assert_eq!(
        race_music::RacePick::before(7).resolve_for_test(&discs, 16),
        6
    );
}

#[test]
fn nothing_playing_means_every_action_is_a_no_op() {
    let mut audio = Audio::open(
        &crate::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    assert!(!audio.restart_race_song());
    assert!(!audio.next_race_song());
    assert!(!audio.previous_race_song());
}
