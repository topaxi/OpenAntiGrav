use super::*;

fn id(n: u64) -> WindowId {
    WindowId::from(n)
}

#[test]
fn an_event_is_routed_to_its_own_window() {
    let seats = [id(2), id(3)];
    assert_eq!(which(id(1), id(1), &seats), Some(Which::Primary));
    assert_eq!(which(id(2), id(1), &seats), Some(Which::Screen(0)));
    assert_eq!(which(id(3), id(1), &seats), Some(Which::Screen(1)));
    assert_eq!(which(id(9), id(1), &seats), None, "a window already closed");
}

/// Closing window 1 quits as it always did; closing a seat window turns that seat to the
/// AI (the caller unseats its slot); a window already let go of is nothing. Fails if a
/// seat's close is treated as a quit, or window 1's as a seat.
#[test]
fn closing_window_one_quits_and_closing_a_seat_hands_it_to_the_ai() {
    let seats = [id(2), id(3)];
    assert_eq!(closed(id(1), id(1), &seats), Closed::Quit);
    assert_eq!(closed(id(3), id(1), &seats), Closed::Screen(1));
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

/// A window is named for the seats it holds: one number for one seat, a range for several, and
/// just the word for a window nobody sits at yet.
#[test]
fn a_window_is_named_for_its_seats() {
    assert_eq!(label("PLAYER", 1, 1), "PLAYER 2");
    assert_eq!(label("PLAYER", 4, 2), "PLAYER 5 - 6");
    assert_eq!(label("PLAYER", 0, 4), "PLAYER 1 - 4");
    assert_eq!(label("PLAYER", 6, 0), "PLAYER");
}

/// Only windows that hold a seat have a first seat to presume a person at.
#[test]
fn every_window_with_a_seat_is_reachable() {
    let windows: [&[usize]; 3] = [&[0, 1], &[], &[2]];
    assert_eq!(first_seats(windows), vec![0, 2]);
}

/// The focus a raw key event is read against is the latest gain still held, and none once everything lost it.
#[test]
fn the_current_window_is_the_latest_gain_still_held() {
    let start = Instant::now();
    let mut focus = Focus::default();
    assert_eq!(focus.current(), None);
    focus.changed(id(1), true, start);
    assert_eq!(focus.current(), Some(id(1)));
    // The gain for window 2 arrives before the loss for window 1.
    focus.changed(id(2), true, start);
    assert_eq!(focus.current(), Some(id(2)));
    focus.changed(id(1), false, start);
    assert_eq!(focus.current(), Some(id(2)));
    focus.changed(id(2), false, start);
    assert_eq!(focus.current(), None);
}

/// A click on a window is that window's first seat's Start, under the pause's owner rule - so a
/// click on window 2 is slot 2's here and never slot 0's, and a window with no seat is nobody's.
/// Fails if a click on a further window answered window 1's seat.
#[test]
fn a_click_is_its_windows_first_seats_start() {
    let windows: [&[usize]; 3] = [&[0, 1], &[2, 3], &[]];
    assert_eq!(click_slots(0b001, &windows), 0b0001, "window 1");
    assert_eq!(click_slots(0b010, &windows), 0b0100, "window 2: slot 2, not 0 or 3");
    assert_eq!(click_slots(0b100, &windows), 0, "a window with no seat");
    assert_eq!(click_slots(0b011, &windows), 0b0101, "both at once");
    assert_eq!(click_slots(0b1000, &windows), 0, "a window the race does not have");
}
