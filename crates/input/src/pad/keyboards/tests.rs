use super::*;

type Seated = Assignment<u32>;

const ALL: &[usize] = &[0, 1, 2, 3];

/// The flaw this exists to fix: a pad player in window 2 is not flown by window 1's keyboard
/// when a key arrives on window 2. The window holds no keyboard seat and has nothing to claim.
#[test]
fn a_window_whose_seats_are_pads_ignores_keys() {
    let mut assignment = Seated::seated(2);
    assignment.reconcile(&[10, 11]);
    assert_eq!(assignment.slot_of(11), Some(1), "a pad is in window 2");
    assert_eq!(assignment.route_key(None, &[0], ALL, true), Some(0));
    assert_eq!(assignment.route_key(None, &[1], ALL, true), None);
    assert_eq!(assignment.keyboard_seats(), vec![(Binding::Shared, 0)]);
}

/// A key press in a window with a free seat takes it, the way a pad takes the lowest free one.
#[test]
fn a_press_in_a_window_with_a_free_seat_claims_the_lowest() {
    let mut assignment = Seated::seated(4);
    assert_eq!(
        assignment.route_key(None, &[2, 3], ALL, false),
        None,
        "a release claims nothing"
    );
    assert_eq!(assignment.route_key(None, &[2, 3], ALL, true), Some(2));
    assert_eq!(
        assignment.route_key(None, &[2, 3], ALL, true),
        Some(2),
        "and keeps it"
    );
    assert_eq!(
        assignment.keyboard_seats(),
        vec![(Binding::Shared, 0), (Binding::Shared, 2)]
    );
    assert!(assignment.drives(2));
    assert!(!assignment.is_free(2));
    assert!(assignment.is_free(3));
}

/// A pad that connects afterwards does not take the seat a keyboard claimed.
#[test]
fn a_pad_does_not_take_a_claimed_keyboard_seat() {
    let mut assignment = Seated::seated(2);
    assignment.route_key(None, &[1], ALL, true);
    assignment.reconcile(&[10, 11]);
    assert_eq!(
        assignment.slot_of(10),
        Some(0),
        "the first pad shares the first keyboard's seat"
    );
    assert_eq!(
        assignment.slot_of(11),
        None,
        "no free seat is left for the second"
    );
}

/// No seat is free merged (one person), closed, or past the seated head count.
#[test]
fn nothing_is_free_where_nobody_is_seated() {
    let mut merged = Seated::default();
    assert_eq!(merged.route_key(None, &[1], ALL, true), None);
    let mut seated = Seated::seated(2);
    assert_eq!(seated.route_key(None, &[5], ALL, true), None);
    seated.unseat(1);
    assert_eq!(seated.route_key(None, &[1], ALL, true), None);
}

/// Two physical keyboards: the first press binds a keyboard to the window's keyboard seat or a free
/// one, and from then on its keys go there whichever window is asked.
#[test]
fn a_device_keeps_its_seat_whichever_window_has_the_focus() {
    let mut assignment = Seated::seated(2);
    let (a, b) = (KeyboardId(0), KeyboardId(1));
    assert_eq!(assignment.route_key(Some(a), &[0], ALL, true), Some(0));
    assert_eq!(
        assignment.route_key(Some(b), &[0], ALL, true),
        Some(1),
        "window 1's seat is A's; B takes a free one"
    );
    assert_eq!(
        assignment.route_key(Some(a), &[1], ALL, true),
        Some(0),
        "A in window 2's focus"
    );
    assert_eq!(
        assignment.route_key(Some(b), &[0], ALL, true),
        Some(1),
        "B in window 1's focus"
    );
    assert_eq!(
        assignment.keyboard_seats(),
        vec![(Binding::Device(a), 0), (Binding::Device(b), 1)]
    );
}

/// A device already bound is not overtaken by a window's shared seat, and a window event (no device)
/// is not routed to a seat bound to a device.
#[test]
fn a_seat_bound_to_a_device_takes_no_window_events() {
    let mut assignment = Seated::seated(2);
    assignment.route_key(Some(KeyboardId(0)), &[0], ALL, true);
    assert_eq!(assignment.route_key(None, &[0], ALL, false), None);
    assert_eq!(
        assignment.route_key(None, &[0], ALL, true),
        None,
        "window 1 has no free seat"
    );
}

/// Closing a window drops its keyboard seat with it.
#[test]
fn unseating_removes_the_keyboard_seat() {
    let mut assignment = Seated::seated(2);
    assignment.route_key(None, &[1], ALL, true);
    assert!(assignment.drives(1));
    assignment.unseat(1);
    assert!(!assignment.drives(1));
    assert_eq!(assignment.keyboard_seats(), vec![(Binding::Shared, 0)]);
}

/// The seat assignment can give a window slots that are not a run (window 1 shows slots 0 and
/// 2, window 2 slot 1): a key goes to the keyboard seat whichever window slots it sits among,
/// and a window that holds only pads' slots ignores it.
#[test]
fn a_window_whose_slots_are_not_a_run_still_routes_to_the_keyboard_seat() {
    let mut assignment = Seated::seated(3);
    assignment.claim_keyboard(2);
    assignment.reconcile(&[10, 11]);
    assert_eq!(assignment.slot_of(10), Some(0));
    assert_eq!(assignment.slot_of(11), Some(1));
    assert_eq!(
        assignment.route_key(None, &[0, 2], &[0, 1, 2], true),
        Some(2)
    );
    assert_eq!(assignment.route_key(None, &[1], &[0, 1, 2], true), None);
    let mut first = Seated::seated(3);
    first.claim_keyboard(0);
    first.reconcile(&[10, 11]);
    assert_eq!(
        first.route_key(None, &[1, 2], &[0, 1, 2], true),
        None,
        "slot 0 is in the other window"
    );
    assert_eq!(first.route_key(None, &[0], &[0, 1, 2], true), Some(0));
}
