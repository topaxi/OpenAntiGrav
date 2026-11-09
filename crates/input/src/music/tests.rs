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

fn bindings(pairs: &[(&str, &str)]) -> (MusicBindings, Vec<String>) {
    MusicBindings::from_pairs(pairs.iter().copied())
}

#[test]
fn named_and_raw_values_parse() {
    let (table, problems) = bindings(&[
        ("next_song", "rightthumb"),
        ("music_up", "raw:300"),
        ("music_down", "raw:0x2c0"),
    ]);
    assert!(problems.is_empty(), "{problems:?}");
    let mut rows = table.rows().to_vec();
    rows.sort_by_key(|&(_, action)| action as u8);
    assert_eq!(
        rows,
        [
            (
                PadControl::Named(gilrs::Button::RightThumb),
                MusicAction::NextSong
            ),
            (PadControl::Raw(300), MusicAction::MusicUp),
            (PadControl::Raw(0x2c0), MusicAction::MusicDown),
        ]
    );
}

#[test]
fn nothing_is_bound_by_default() {
    assert!(MusicBindings::default().rows().is_empty());
    assert!(bindings(&[]).0.rows().is_empty());
}

#[test]
fn a_bad_value_names_its_key_and_leaves_that_action_unbound() {
    let (table, problems) = bindings(&[
        ("next_song", "rightthumbb"),
        ("music_up", "raw:abc"),
        ("sing_along", "Mode"),
        ("show_song", "Mode"),
    ]);
    assert_eq!(
        table.rows(),
        [(
            PadControl::Named(gilrs::Button::Mode),
            MusicAction::ShowSong
        )]
    );
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert!(problems.iter().any(|p| p.starts_with("next_song")));
    assert!(problems.iter().any(|p| p.starts_with("music_up")));
    assert!(problems.iter().any(|p| p.contains("sing_along")));
}

#[test]
fn a_game_button_is_refused() {
    let (table, problems) = bindings(&[("next_song", "South"), ("restart_song", "DPadUp")]);
    assert!(table.rows().is_empty());
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems.iter().all(|p| p.contains("game button")));
}

#[test]
fn every_name_in_the_table_parses_to_itself_and_describes_back() {
    for &(name, button) in NAMED {
        assert_eq!(PadControl::parse(name), Ok(PadControl::Named(button)));
        assert_eq!(PadControl::describe(button, 0), name);
    }
    assert_eq!(PadControl::describe(gilrs::Button::Unknown, 300), "raw:300");
    assert_eq!(PadControl::parse("raw:300"), Ok(PadControl::Raw(300)));
}

#[test]
fn every_action_has_its_own_bit() {
    let mut seen = 0_u8;
    for action in MusicAction::ALL {
        assert_eq!(seen & action.bit(), 0);
        seen |= action.bit();
    }
}
