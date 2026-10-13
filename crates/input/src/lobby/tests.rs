use super::*;
use crate::pad::Assignment;

type Lobby = super::Lobby<u32>;

fn press(device: Device<u32>, button: Button) -> Press<u32> {
    Press {
        device,
        buttons: button.bit(),
    }
}

/// Cross claims the next seat, in the order the devices pressed it.
#[test]
fn confirm_from_an_unseated_device_takes_the_next_seat() {
    let mut lobby = Lobby::new();
    assert_eq!(
        lobby.press(press(Device::Pad(7), Button::Cross)),
        Outcome::Joined(0)
    );
    assert_eq!(
        lobby.press(press(Device::Keyboard, Button::Cross)),
        Outcome::Joined(1)
    );
    assert_eq!(
        lobby.press(press(Device::Pad(3), Button::Cross)),
        Outcome::Joined(2)
    );
    assert_eq!(lobby.seat_of(Device::Pad(7)), Some(0));
    assert_eq!(lobby.seat_of(Device::Pad(3)), Some(2));
    assert_eq!(lobby.count(), 3);
}

/// A seated device pressing Cross again does not take a second seat.
#[test]
fn a_seated_device_cannot_join_twice() {
    let mut lobby = Lobby::new();
    lobby.join(Device::Keyboard);
    assert_eq!(
        lobby.press(press(Device::Keyboard, Button::Cross)),
        Outcome::Nothing
    );
    assert_eq!(lobby.count(), 1);
}

/// Circle leaves a seat when the device has one and leaves the screen when it
/// has none; the seats after a leaver move down so slots stay `0..count`.
#[test]
fn back_leaves_a_seat_then_the_screen_and_closes_the_gap() {
    let mut lobby = Lobby::new();
    for pad in [10, 11, 12] {
        lobby.join(Device::Pad(pad));
    }
    assert_eq!(
        lobby.press(press(Device::Pad(10), Button::Circle)),
        Outcome::Left(0)
    );
    assert_eq!(lobby.seats(), &[Device::Pad(11), Device::Pad(12)]);
    assert_eq!(
        lobby.press(press(Device::Pad(10), Button::Circle)),
        Outcome::Back
    );
}

/// Only a seated device starts the race: a bystander's Start is nothing.
#[test]
fn start_needs_a_seated_device() {
    let mut lobby = Lobby::new();
    assert_eq!(
        lobby.press(press(Device::Pad(1), Button::Start)),
        Outcome::Nothing
    );
    lobby.join(Device::Pad(1));
    assert_eq!(
        lobby.press(press(Device::Pad(1), Button::Start)),
        Outcome::Start
    );
}

/// A ninth device is told the grid is full rather than silently dropped.
#[test]
fn the_grid_has_eight_seats() {
    let mut lobby = Lobby::new();
    for pad in 0..8 {
        assert_eq!(lobby.join(Device::Pad(pad)), Outcome::Joined(pad as usize));
    }
    assert_eq!(lobby.join(Device::Pad(8)), Outcome::Full);
    assert_eq!(lobby.count(), 8);
}

/// A pad that is unplugged while the lobby is open takes its seat with it.
#[test]
fn a_disconnected_pad_loses_its_seat() {
    let mut lobby = Lobby::new();
    lobby.join(Device::Pad(1));
    lobby.join(Device::Keyboard);
    lobby.join(Device::Pad(2));
    lobby.retain_connected(&[2]);
    assert_eq!(lobby.seats(), &[Device::Keyboard, Device::Pad(2)]);
}

/// The lobby ends by filling in the assignment: each pad on its seat, the
/// keyboard on its own, every other pad driving nothing.
#[test]
fn the_lobby_seats_the_assignment() {
    let mut lobby = Lobby::new();
    lobby.join(Device::Pad(5));
    lobby.join(Device::Keyboard);
    lobby.join(Device::Pad(9));
    let mut assignment = Assignment::<u32>::default();
    lobby.seat(&mut assignment, &[5, 9, 1]);
    assert_eq!(assignment.humans(), Some(3));
    assert_eq!(assignment.slot_of(5), Some(0));
    assert_eq!(assignment.keyboard_slot(), 1);
    assert_eq!(assignment.slot_of(9), Some(2));
    assert_eq!(assignment.slot_of(1), None, "a pad that never joined");
}

/// The bug a seated race must not have: the keyboard player joined on slot 0, a
/// second pad sits on the desk without having joined, and the first poll of the
/// race must not hand that pad slot 0 (or any slot).
#[test]
fn a_pad_that_did_not_join_does_not_take_a_seat_when_the_race_starts() {
    let mut lobby = Lobby::new();
    lobby.join(Device::Keyboard);
    lobby.join(Device::Pad(5));
    let mut assignment = Assignment::<u32>::default();
    lobby.seat(&mut assignment, &[5, 8]);

    assignment.reconcile(&[5, 8]);
    assert_eq!(assignment.slot_of(8), None, "a bystander took a seat");
    assert_eq!(assignment.slot_of(5), Some(1));
    assert_eq!(assignment.keyboard_slot(), 0);
}

/// A pad plugged in after the race started takes a free seat, never the one the
/// keyboard player holds; a joined pad that drops and returns gets a seat back
/// (the lowest no pad and no seated keyboard holds); and a benched pad that
/// disconnects and comes back is a new pad.
#[test]
fn a_mid_race_pad_never_lands_on_the_keyboards_seat() {
    let mut lobby = Lobby::new();
    lobby.join(Device::Keyboard);
    lobby.join(Device::Pad(5));
    lobby.join(Device::Pad(6));
    let mut assignment = Assignment::<u32>::default();
    lobby.seat(&mut assignment, &[5, 6, 8]);

    assignment.reconcile(&[5, 6, 8]);
    assignment.reconcile(&[6, 8]);
    assert_eq!(assignment.slot_of(5), None, "unplugged");
    assignment.reconcile(&[6, 8, 5]);
    assert_eq!(assignment.slot_of(5), Some(1), "back on the seat it left");
    assert_eq!(assignment.slot_of(6), Some(2));

    assignment.reconcile(&[5, 6]);
    assignment.reconcile(&[5, 6, 8]);
    assert_eq!(assignment.slot_of(8), None, "no seat is free for it");
    assignment.reconcile(&[6]);
    assignment.reconcile(&[6, 8]);
    assert_eq!(
        assignment.slot_of(8),
        Some(1),
        "unplugged and back, it is a new pad and takes the freed seat"
    );
}
