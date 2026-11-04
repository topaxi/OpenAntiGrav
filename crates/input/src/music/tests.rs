use super::*;

const TABLE: &[(PadControl, MusicAction)] = &[
    (PadControl::Named(gilrs::Button::Mode), MusicAction::MusicUp),
    (PadControl::Raw(0x2c0), MusicAction::NextSong),
];

#[test]
fn a_named_control_holds_while_down_and_a_press_latches() {
    let mut tracker = Tracker::default();
    tracker.note(TABLE, gilrs::Button::Mode, 0, true);
    let seen = tracker.take();
    assert!(seen.pressed(MusicAction::MusicUp) && seen.held(MusicAction::MusicUp));
    assert!(!seen.pressed(MusicAction::NextSong));

    let still = tracker.take();
    assert!(
        !still.pressed(MusicAction::MusicUp),
        "a press is spent once"
    );
    assert!(still.held(MusicAction::MusicUp));

    tracker.note(TABLE, gilrs::Button::Mode, 0, false);
    assert!(tracker.take().is_empty());
}

#[test]
fn a_press_shorter_than_a_frame_still_counts() {
    let mut tracker = Tracker::default();
    tracker.note(TABLE, gilrs::Button::Mode, 0, true);
    tracker.note(TABLE, gilrs::Button::Mode, 0, false);
    let seen = tracker.take();
    assert!(seen.pressed(MusicAction::MusicUp));
    assert!(!seen.held(MusicAction::MusicUp));
}

#[test]
fn a_raw_control_matches_only_its_own_code() {
    let mut tracker = Tracker::default();
    tracker.note(TABLE, gilrs::Button::Unknown, 0x2c1, true);
    assert!(tracker.take().is_empty());
    tracker.note(TABLE, gilrs::Button::Unknown, 0x2c0, true);
    assert!(tracker.take().pressed(MusicAction::NextSong));
    tracker.note(TABLE, gilrs::Button::South, 0x2c0, true);
    assert!(
        !tracker.take().pressed(MusicAction::NextSong),
        "a named button with that code is another control"
    );
}

#[test]
fn a_pad_going_away_releases_what_it_held() {
    let mut tracker = Tracker::default();
    tracker.note(TABLE, gilrs::Button::Mode, 0, true);
    tracker.release_all();
    assert!(!tracker.take().held(MusicAction::MusicUp));
}

#[test]
fn no_music_control_is_also_a_game_button() {
    for &(control, _) in MUSIC_BINDINGS {
        if let PadControl::Named(button) = control {
            assert_eq!(crate::pad::map_button(button), None, "{button:?}");
        }
    }
}

#[test]
fn every_action_has_its_own_bit() {
    let mut seen = 0_u8;
    for action in MusicAction::ALL {
        assert_eq!(seen & action.bit(), 0);
        seen |= action.bit();
    }
}
