//! Who is playing a split-screen race: devices claiming seats before it starts.
//!
//! A [`Lobby`] is the model behind the "press a button to join" screen. It holds
//! the devices that have joined, in the order they did, and the seat a device
//! holds *is* its place in that order: seat 0 flies slot 0, seat 1 slot 1. A
//! leaver closes the gap, so the seated slots are always `0..count` - the shape
//! `oag_game::split_screen::apply` makes `Controller::Local`.
//!
//! **Plain data, generic over the pad's id**, like [`crate::pad::Assignment`]
//! which it ends by filling in: `gilrs` hands out `GamepadId`s no test can
//! construct, and nothing here needs a device, only the fact of one.
//!
//! **The rules are chosen, not measured.** The original's split screen takes its
//! players from the PS2's pads in port order and has no join step to read.
//! Here, on the lobby screen: Cross from a device that has no seat claims the
//! next one; Circle from a device that has one gives it up, and from a device
//! that has none leaves the screen; Start from a seated device begins the race.
//! A pad that disconnects loses its seat.

use crate::pad::{Assignment, KeyboardId};
use oag_gameplay::input::Button;

/// An input device, as far as who is playing is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device<D = gilrs::GamepadId> {
    /// The keyboard, and with it the mouse and the touch controls: a click
    /// reaches the game through the keyboard's latch, so it is this device
    /// speaking.
    Keyboard,
    /// One physical keyboard the platform told apart from the others, by the raw key
    /// events X11 and Windows send (`Controls::lobby_key`). What lets a second keyboard
    /// take a seat of its own; a platform that reports no such id (Wayland, macOS) never
    /// produces one, and its keyboards are all [`Self::Keyboard`].
    Keys(KeyboardId),
    /// One gamepad.
    Pad(D),
}

/// One device's newly pressed buttons, as [`Button::bit`] masks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Press<D = gilrs::GamepadId> {
    /// Who pressed.
    pub device: Device<D>,
    /// What went down since the last tick.
    pub buttons: u32,
}

/// What one press did to the lobby.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The device took the seat (slot) numbered here.
    Joined(usize),
    /// The device gave up the seat numbered here; the ones after it moved down.
    Left(usize),
    /// A seated device asked to begin.
    Start,
    /// A device with no seat asked to leave the screen.
    Back,
    /// Every seat is taken.
    Full,
    /// Nothing the lobby acts on.
    Nothing,
}

/// The devices that have joined, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lobby<D = gilrs::GamepadId> {
    seats: Vec<Device<D>>,
    /// How many seats there are: the grid's, or fewer where a race box authored a head count.
    capacity: usize,
}

impl<D> Default for Lobby<D> {
    fn default() -> Self {
        Self {
            seats: Vec::new(),
            capacity: oag_gameplay::MAX_PLAYERS,
        }
    }
}

impl<D: Copy + PartialEq> Lobby<D> {
    /// An empty lobby.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty lobby of `capacity` seats, at most the grid's.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            seats: Vec::new(),
            capacity: capacity.clamp(1, oag_gameplay::MAX_PLAYERS),
        }
    }

    /// How many seats there are.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Whoever is seated, seat 0 first.
    #[must_use]
    pub fn seats(&self) -> &[Device<D>] {
        &self.seats
    }

    /// How many are seated.
    #[must_use]
    pub fn count(&self) -> usize {
        self.seats.len()
    }

    /// The seat `device` holds.
    #[must_use]
    pub fn seat_of(&self, device: Device<D>) -> Option<usize> {
        self.seats.iter().position(|seat| *seat == device)
    }

    /// Seats `device` at the next free seat, or says why not.
    pub fn join(&mut self, device: Device<D>) -> Outcome {
        if self.seat_of(device).is_some() {
            return Outcome::Nothing;
        }
        if self.seats.len() >= self.capacity {
            return Outcome::Full;
        }
        self.seats.push(device);
        Outcome::Joined(self.seats.len() - 1)
    }

    /// Empties seat `seat`; later seats move down. `Nothing` past the last.
    pub fn leave(&mut self, seat: usize) -> Outcome {
        if seat >= self.seats.len() {
            return Outcome::Nothing;
        }
        self.seats.remove(seat);
        Outcome::Left(seat)
    }

    /// What `press` does. Several buttons at once are taken in the order Start,
    /// Cross, Circle, so one press cannot join and leave in the same tick. **Start from a
    /// device with no seat joins**, as it does on HD and Omega (`FE_GUEST_JOIN`, "PLAYER 2
    /// PRESS START TO JOIN"); from a seated device it begins.
    pub fn press(&mut self, press: Press<D>) -> Outcome {
        let down = |button: Button| press.buttons & button.bit() != 0;
        let seat = self.seat_of(press.device);
        match seat {
            Some(_) if down(Button::Start) => Outcome::Start,
            None if down(Button::Cross) || down(Button::Start) => self.join(press.device),
            Some(seat) if down(Button::Circle) => self.leave(seat),
            None if down(Button::Circle) => Outcome::Back,
            _ => Outcome::Nothing,
        }
    }

    /// Drops every pad not in `connected`: a pad that went away takes its seat
    /// with it. The seats' former places in the order close up.
    pub fn retain_connected(&mut self, connected: &[D]) {
        self.seats.retain(|seat| match seat {
            Device::Keyboard | Device::Keys(_) => true,
            Device::Pad(id) => connected.contains(id),
        });
    }

    /// Makes `assignment` what this lobby says: [`Assignment::seated`] for the
    /// seated count, each pad on its seat, the keyboard on its own or on slot 0
    /// when it did not join (it still has to reach the pause key and the menus).
    ///
    /// `connected` is every pad plugged in now: the ones that did not join are
    /// benched ([`Assignment::bench`]), so a pad on the desk does not take a seat
    /// the moment the race starts. Only a pad that connects after this, or one that
    /// disconnects and comes back, is seated by [`Assignment::reconcile`].
    pub fn seat(&self, assignment: &mut Assignment<D>, connected: &[D]) {
        *assignment = Assignment::seated(self.seats.len().max(1));
        for (slot, seat) in self.seats.iter().enumerate() {
            match seat {
                Device::Keyboard => assignment.claim_keyboard(slot),
                Device::Keys(id) => assignment.claim_keyboard_device(slot, *id),
                Device::Pad(id) => assignment.assign_pad(*id, slot),
            }
        }
        assignment.bench(connected);
    }
}

#[cfg(test)]
mod tests;
