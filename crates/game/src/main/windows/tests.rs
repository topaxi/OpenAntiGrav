use super::*;

fn id(n: u64) -> WindowId {
    WindowId::from(n)
}

#[test]
fn an_event_is_routed_to_its_own_window() {
    let seats = [id(2), id(3)];
    assert_eq!(which(id(1), id(1), &seats), Some(Which::Primary));
    assert_eq!(which(id(2), id(1), &seats), Some(Which::Seat(0)));
    assert_eq!(which(id(3), id(1), &seats), Some(Which::Seat(1)));
    assert_eq!(which(id(9), id(1), &seats), None, "a window already closed");
}

/// Closing window 1 quits as it always did; closing a seat window turns that seat to the
/// AI (the caller unseats its slot); a window already let go of is nothing. Fails if a
/// seat's close is treated as a quit, or window 1's as a seat.
#[test]
fn closing_window_one_quits_and_closing_a_seat_hands_it_to_the_ai() {
    let seats = [id(2), id(3)];
    assert_eq!(closed(id(1), id(1), &seats), Closed::Quit);
    assert_eq!(closed(id(3), id(1), &seats), Closed::Seat(1));
    assert_eq!(closed(id(7), id(1), &seats), Closed::Nothing);
}

#[test]
fn focus_moving_between_windows_is_not_away() {
    let start = Instant::now();
    let mut focus = Focus::default();
    focus.changed(id(1), true, start);
    // Window 1 loses it and window 2 gains it, in either order, inside the grace.
    focus.changed(id(1), false, start);
    focus.changed(id(2), true, start);
    assert!(!focus.away(start + FOCUS_GRACE * 4), "a window has focus");
    focus.changed(id(2), false, start + Duration::from_secs(1));
    focus.changed(id(1), true, start + Duration::from_secs(1));
    assert!(!focus.away(start + Duration::from_secs(5)));
}

#[test]
fn focus_lost_by_every_window_is_away_once_after_the_grace() {
    let start = Instant::now();
    let mut focus = Focus::default();
    focus.changed(id(1), true, start);
    focus.changed(id(2), true, start);
    focus.changed(id(1), false, start);
    assert!(!focus.away(start), "window 2 still has it");
    focus.changed(id(2), false, start);
    assert!(!focus.away(start), "inside the grace");
    assert!(focus.away(start + FOCUS_GRACE), "after it");
    assert!(!focus.away(start + FOCUS_GRACE * 2), "said once");
}

#[test]
fn a_closed_window_cannot_keep_the_focus() {
    let start = Instant::now();
    let mut focus = Focus::default();
    focus.changed(id(2), true, start);
    focus.forget(id(2));
    focus.changed(id(1), true, start);
    focus.changed(id(1), false, start);
    assert!(focus.away(start + FOCUS_GRACE), "window 2 is gone");
}

#[test]
fn the_windows_share_the_largest_allocation_any_of_them_wants() {
    assert_eq!(shared_allocation((1280, 720), []), (1280, 720));
    assert_eq!(
        shared_allocation((1280, 720), [(1920, 1080), (800, 1200)]),
        (1920, 1200)
    );
}

#[test]
fn a_seat_window_is_named_for_its_player() {
    assert_eq!(title("OpenAntiGrav", "PLAYER", 2), "OpenAntiGrav - PLAYER 2");
}

/// A race is shown as windows only when each person past the first has one; a gap (a
/// closed window) or too few windows leaves it cut up in window 1.
#[test]
fn a_race_is_windowed_only_when_every_person_has_a_window() {
    assert!(covers(&[1, 2], 3));
    assert!(covers(&[1, 2, 3], 3), "a spare window is a holding frame");
    assert!(!covers(&[1], 3), "too few windows");
    assert!(!covers(&[2], 2), "slot 1 has none");
    assert!(!covers(&[1, 2], 1), "one person is no multi-window race");
}
