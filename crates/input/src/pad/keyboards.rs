//! Keyboards as seats: which key goes to which craft when more than one person types.
//!
//! **Chosen, not measured**: the original has no keyboard, let alone two.
//!
//! A keyboard seat is a grid slot a keyboard flies. [`Assignment`] always holds one
//! (the first, which the front end and a single-window race use) and a multi-window
//! race can hold more: **a key press in a window that holds no keyboard seat takes the
//! lowest free seat of that window** ([`Assignment::route_key`]), the way a pad that
//! connects takes the lowest free seat. A seat that is free has no pad and no keyboard on
//! it and is a person's (below the seated head count, not closed). A window whose seats
//! are all taken by pads has nothing to claim, so its keys go nowhere: the keyboard that
//! drives window 1 cannot also fly the pad player's craft in window 2.
//!
//! How a key finds its seat depends on what the platform says about where it came from:
//!
//! - **The window it arrived on** ([`Binding::Shared`]). All that Wayland and macOS
//!   report: one keyboard device id for every keyboard. A window-routed seat takes the keys
//!   of the window it is in. On a compositor with one seat the one focused window gets
//!   them, so only that window's keyboard player moves; on a Wayland compositor with a
//!   `wl_seat` per window (sway's `seat` command) every seat's keyboard focuses its own
//!   window and each window's player flies at once.
//! - **The physical keyboard** ([`Binding::Device`]). X11 and Windows report a keyboard's
//!   id only on the raw key events (`DeviceEvent::Key`), not on the window's. A keyboard
//!   is bound to a seat on its first press and keeps it whichever window has the focus, so
//!   two keyboards fly two craft.

use super::{Assignment, Seating};

/// Which physical keyboard, as the composition root numbers them in the order it first
/// hears of them. Not the platform's own id: this crate has no way to construct one in a
/// test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardId(pub u32);

/// What sends a keyboard seat its keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// Whatever arrives on a window the seat is in, from any keyboard.
    Shared,
    /// One physical keyboard, whichever window has the focus.
    Device(KeyboardId),
}

/// A keyboard seat after the first: the slot it flies and what sends it keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Seat {
    pub(super) binding: Binding,
    pub(super) slot: u8,
}

impl<D: Copy + PartialEq> Assignment<D> {
    /// Every keyboard seat and what routes to it, the first (the one the front end reads)
    /// first.
    #[must_use]
    pub fn keyboard_seats(&self) -> Vec<(Binding, usize)> {
        std::iter::once((self.keyboard_binding, self.keyboard_slot()))
            .chain(
                self.more_keyboards
                    .iter()
                    .map(|seat| (seat.binding, usize::from(seat.slot))),
            )
            .collect()
    }

    /// Whether a person could sit at `slot` and nothing does yet: below the seated head
    /// count, not closed, no pad and no keyboard on it. What a key press may claim.
    #[must_use]
    pub fn is_free(&self, slot: usize) -> bool {
        let Seating::Seated { humans } = self.seating else {
            return false;
        };
        let Ok(slot) = u8::try_from(slot) else {
            return false;
        };
        slot < humans
            && !self.closed.contains(&slot)
            && self.pads.iter().all(|(_, held)| *held != slot)
            && self.keyboard != slot
            && self.more_keyboards.iter().all(|seat| seat.slot != slot)
    }

    /// The slot a key goes to, or `None` when it goes nowhere. `window` is the seats of the
    /// window the key is for (the one in focus, for a raw key event); `device` the physical
    /// keyboard when the platform said which (`None` for a key reported on a window alone);
    /// `race` every seat of the race, which a keyboard with a `device` may claim from when its
    /// window has none free; `claim` whether a press may take a free seat (a release, or a key
    /// bound to nothing, may not).
    ///
    /// 1. A keyboard already bound to a seat goes to it, whatever window has the focus.
    /// 2. Else the window's [shared](Binding::Shared) keyboard seat takes it, and binds
    ///    `device` to itself.
    /// 3. Else, if `claim`, the window's lowest [free](Self::is_free) seat becomes a keyboard
    ///    seat bound to `device` (shared when `None`); a `device` with none free in the
    ///    window takes the lowest free seat of the race.
    /// 4. Else nowhere: a window that holds no keyboard seat and has none to claim ignores
    ///    keys.
    pub fn route_key(
        &mut self,
        device: Option<KeyboardId>,
        window: &[usize],
        race: &[usize],
        claim: bool,
    ) -> Option<usize> {
        if let Some(id) = device
            && let Some((_, slot)) = self
                .keyboard_seats()
                .into_iter()
                .find(|(binding, _)| *binding == Binding::Device(id))
        {
            return Some(slot);
        }
        let shared = self
            .keyboard_seats()
            .into_iter()
            .find(|(binding, slot)| *binding == Binding::Shared && window.contains(slot));
        if let Some((_, slot)) = shared {
            if let Some(id) = device {
                self.bind_keyboard(slot, Binding::Device(id));
            }
            return Some(slot);
        }
        if !claim {
            return None;
        }
        let lowest = |from: &[usize]| {
            from.iter()
                .copied()
                .filter(|slot| self.is_free(*slot))
                .min()
        };
        let slot = lowest(window).or_else(|| device.and_then(|_| lowest(race)))?;
        let binding = device.map_or(Binding::Shared, Binding::Device);
        self.more_keyboards.push(Seat {
            binding,
            slot: u8::try_from(slot).ok()?,
        });
        Some(slot)
    }

    /// Changes what sends keys to the keyboard seat on `slot`, if there is one.
    fn bind_keyboard(&mut self, slot: usize, binding: Binding) {
        if self.keyboard_slot() == slot {
            self.keyboard_binding = binding;
        } else if let Some(seat) = self
            .more_keyboards
            .iter_mut()
            .find(|seat| usize::from(seat.slot) == slot)
        {
            seat.binding = binding;
        }
    }
}

#[cfg(test)]
mod tests;
