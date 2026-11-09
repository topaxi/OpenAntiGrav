//! The changes a race takes outside `Race::tick`, as `EVNT` entries: one method per writer that
//! both makes the change and records it, and the applier a re-drive calls between the same two
//! ticks (ADR-0087, decision 7).
//!
//! **A writer that bypasses these is invisible to a replay.** ADR-0055's rule, now with a place
//! to be recorded: a new out-of-tick writer gets a [`Kind`] and a method here.

use oag_physics::pilot_assist::Level;
use oag_replay::{Event, Kind};
use oag_tables::weapons::Weapon;

use crate::Race;

/// Why an event could not be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ApplyError {
    /// A kind this build does not implement.
    #[error("replay event of kind {0} is not one this build applies")]
    UnknownKind(u8),
    /// A value outside the kind's range.
    #[error("replay event of kind {kind} carries the value {value}, which names nothing")]
    BadValue {
        /// The kind.
        kind: u8,
        /// The value.
        value: u32,
    },
}

/// The weapons a `give` event can name, by the index it stores: [`Weapon::ALL`] in order, then
/// the pickup a simulation mod owns, which `--give extension` can hand out (inert without
/// such a mod).
const STORED_WEAPONS: [Weapon; 15] = {
    let mut stored = [Weapon::Extension; 15];
    let mut at = 0;
    while at < Weapon::ALL.len() {
        stored[at] = Weapon::ALL[at];
        at += 1;
    }
    stored
};

impl Race {
    #[cfg(feature = "view")]
    /// Switches Pilot Assist to `level` for every seat, as the pause menu does, and records it
    /// for each person-flown seat so the recording reproduces: the level changes the player's
    /// physics, so a recording that did not hear of the change would desync at the next hash.
    pub fn change_pilot_assist(&mut self, level: Level) {
        self.set_pilot_assist(level);
        let seats: Vec<usize> = self.sim.world.human_slots().collect();
        for seat in seats {
            self.record_event(seat, Kind::PilotAssist, ordinal(&Level::ALL, level));
        }
    }

    #[cfg(feature = "view")]
    /// Hands `slot` the `weapon` when its pickup slot is empty (the capture flag `--give`),
    /// recording the top-up. Returns whether it wrote.
    pub fn give_weapon(&mut self, slot: usize, weapon: Weapon) -> bool {
        if !self.write_weapon(slot, weapon) {
            return false;
        }
        self.record_event(slot, Kind::Give, ordinal(&STORED_WEAPONS, weapon));
        true
    }

    #[cfg(feature = "view")]
    /// Hands `slot` to the opponent driver or to a person at this machine, from outside the tick
    /// (a pad leaving or returning to a split-screen seat), recording the change: the
    /// controllers are in the state hash, so a recording that did not hear of it would desync.
    /// Returns whether the slot changed.
    pub fn set_controller(&mut self, slot: usize, controller: oag_gameplay::Controller) -> bool {
        let value = match controller {
            oag_gameplay::Controller::Ai => 0,
            oag_gameplay::Controller::Local => 1,
            oag_gameplay::Controller::Remote => return false,
        };
        let Some(have) = self.sim.world.controllers.get_mut(slot) else {
            return false;
        };
        if *have == controller {
            return false;
        }
        *have = controller;
        self.record_event(slot, Kind::Controller, value);
        true
    }

    #[cfg(feature = "view")]
    /// How many events the recording holds so far, or `None` when this race records nothing.
    #[must_use]
    pub fn recorded_event_count(&self) -> Option<usize> {
        self.replay
            .recording
            .as_ref()
            .map(|recording| recording.recorder.events().len())
    }

    /// Applies a recorded `event`, the way its writer made the change live. A re-drive calls
    /// this for each [`oag_replay::Verifier::events_due`] before it runs the tick.
    ///
    /// # Errors
    ///
    /// [`ApplyError`] for a kind this build does not apply or a value that names nothing: a
    /// replay that cannot be applied is refused by name, never played with the change missing.
    pub fn apply_event(&mut self, event: &Event) -> Result<(), ApplyError> {
        let seat = usize::from(event.seat);
        let bad = ApplyError::BadValue {
            kind: event.kind,
            value: event.value,
        };
        match event.kind() {
            Some(Kind::PilotAssist) => {
                let level = usize::try_from(event.value)
                    .ok()
                    .and_then(|at| Level::ALL.get(at))
                    .ok_or(bad)?;
                // Every seat, as the writer does: no hash reads a seat nobody flies, and only
                // this keeps the `RaceSim` of a seek equal to the recorded one as text.
                self.set_pilot_assist(*level);
            }
            Some(Kind::Give) => {
                let weapon = usize::try_from(event.value)
                    .ok()
                    .and_then(|at| STORED_WEAPONS.get(at))
                    .ok_or(bad)?;
                self.write_weapon(seat, *weapon);
            }
            Some(Kind::Controller) => {
                let controller = match event.value {
                    0 => oag_gameplay::Controller::Ai,
                    1 => oag_gameplay::Controller::Local,
                    _ => return Err(bad),
                };
                // Idempotent, as the others: an event a seek applies twice leaves the same seat.
                if let Some(have) = self.sim.world.controllers.get_mut(seat) {
                    *have = controller;
                }
            }
            None => return Err(ApplyError::UnknownKind(event.kind)),
        }
        Ok(())
    }

    /// Fills `slot`'s empty pickup slot with `weapon`.
    fn write_weapon(&mut self, slot: usize, weapon: Weapon) -> bool {
        let Some(ship) = self.sim.world.ships.get_mut(slot) else {
            return false;
        };
        if ship.pickup.weapon.is_some() {
            return false;
        }
        ship.pickup.weapon = Some(weapon);
        true
    }

    #[cfg(feature = "view")]
    fn record_event(&mut self, seat: usize, kind: Kind, value: u32) {
        let (Some(recording), Ok(seat)) = (self.replay.recording.as_mut(), u8::try_from(seat))
        else {
            return;
        };
        recording.recorder.event(seat, kind, value);
    }
}

#[cfg(feature = "view")]
/// A value's index in the list the file stores it by.
fn ordinal<T: PartialEq>(all: &[T], value: T) -> u32 {
    all.iter()
        .position(|candidate| *candidate == value)
        .and_then(|at| u32::try_from(at).ok())
        .expect("every value is in its own list")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_weapon_a_session_can_give_has_an_index_and_the_pool_keeps_its_place() {
        assert_eq!(STORED_WEAPONS[..Weapon::ALL.len()], Weapon::ALL);
        assert_eq!(STORED_WEAPONS[Weapon::ALL.len()], Weapon::Extension);
        assert_eq!(ordinal(&STORED_WEAPONS, Weapon::Extension), 14);
        assert_eq!(ordinal(&STORED_WEAPONS, Weapon::Rocket), 0);
    }
}
