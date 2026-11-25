//! The abstract button layer, reproduced from `Input_BuildState` (it lives in
//! `oag_core::buttons` and is re-exported here), and the per-tick snapshot the
//! simulation consumes.
//!
//! The original never uses PSP button masks downstream. `Input_BuildState`
//! (`0x0894eec4`) translates hardware bits into its own layout and everything
//! after that refers to **bit indices**, not masks, with `Input_ParseButtonName`
//! (`0x0894f2a0`) mapping the names the front-end XML uses. See
//! `docs/ghidra/functions/psp-pulse-usa/input.md`.
//!
//! The indirection is worth keeping rather than flattening: `activate` is cross
//! and `cancel` is circle *by convention*, so this layer is where a region
//! difference would live, and it is what the original's own XML expects.
//!
//! This module lives in `oag-gameplay` rather than in the input system because
//! of rule 1 of `docs/architecture/workspace-layout.md`: the simulation owns the
//! snapshot *type*, and whatever produces it - a keyboard, a pad, a replay, a
//! test - depends on this, not the reverse.

pub use oag_core::buttons::{Button, Input, button_from_name};

/// One tick of pilot intent: the button edges plus the analog axes.
///
/// The whole of what the simulation is allowed to know about input. A replay
/// records these and nothing else, which is only sound because there is nothing
/// else: no key codes, no device identity, no timestamps.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct InputSnapshot {
    /// Held, pressed and released masks over the abstract button indices.
    pub buttons: Input,
    /// Analog stick X, `-1.0..=1.0`, positive right.
    pub stick_x: f32,
    /// Analog stick Y, `-1.0..=1.0`, positive up.
    pub stick_y: f32,
    /// Left airbrake, `0.0..=1.0`.
    ///
    /// An axis rather than the [`Button::L`] bit because the recovered force law
    /// ramps each side toward "its analog input"; on a PSP pad that input can
    /// only ever be 0 or 1, and on anything else it need not be.
    pub airbrake_left: f32,
    /// Right airbrake, `0.0..=1.0`.
    pub airbrake_right: f32,
}

impl InputSnapshot {
    /// Nothing held and sticks centred, as a constant.
    ///
    /// [`Input::EMPTY`]'s argument, one level up.
    pub const EMPTY: Self = Self {
        buttons: Input::EMPTY,
        stick_x: 0.0,
        stick_y: 0.0,
        airbrake_left: 0.0,
        airbrake_right: 0.0,
    };

    /// Nothing held, sticks centred.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clamps the axes into range and drops NaN.
    ///
    /// Called on every snapshot before it reaches the simulation. An out-of-range
    /// axis from a miscalibrated pad would be a divergence between two machines
    /// running the same replay, and a NaN would poison the state hash for the
    /// rest of the race, so this is a determinism guard rather than politeness.
    #[must_use]
    pub fn sanitised(mut self) -> Self {
        self.stick_x = clamp_axis(self.stick_x, -1.0);
        self.stick_y = clamp_axis(self.stick_y, -1.0);
        self.airbrake_left = clamp_axis(self.airbrake_left, 0.0);
        self.airbrake_right = clamp_axis(self.airbrake_right, 0.0);
        self
    }
}

/// Clamps to `min..=1.0`, mapping NaN to `0.0`.
fn clamp_axis(value: f32, min: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(min, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_are_clamped_and_nan_becomes_zero() {
        let snapshot = InputSnapshot {
            stick_x: 4.0,
            stick_y: -9.0,
            airbrake_left: -1.0,
            airbrake_right: f32::NAN,
            ..InputSnapshot::new()
        }
        .sanitised();
        assert_eq!(snapshot.stick_x, 1.0);
        assert_eq!(snapshot.stick_y, -1.0);
        assert_eq!(
            snapshot.airbrake_left, 0.0,
            "a trigger has no negative half"
        );
        assert_eq!(snapshot.airbrake_right, 0.0, "NaN must not reach the sim");
    }
}

/// One tick of pilot intent per grid slot.
///
/// **What [`crate::World::race`]'s widening needs on the other side.** The world
/// now carries a clock per slot and a [`crate::Controller`] per slot; this is
/// the input that reaches them. Split screen, multiple windows and network play
/// are all "N people feeding N snapshots into one simulation per tick", and
/// they differ only in where the snapshots come from - two pads on one machine,
/// two windows on one machine, or a socket. None of that is visible here, which
/// is the point: the simulation's contract is still "a snapshot in", it just
/// has eight of them.
///
/// **A fixed array, indexed by grid slot**, for [`crate::World`]'s own reason -
/// a `Vec` of inputs would make the thing a replay records vary in size with
/// the session that recorded it. Slot `i` here is `ships[i]`, `race[i]` and
/// `controllers[i]`: one index for the whole racer, so there is no mapping to
/// keep in step.
///
/// **A slot whose [`crate::Controller`] is not human is not consulted.** Its
/// entry is [`InputSnapshot::default`] and stays that way - an AI slot is flown
/// by [`oag_ai`], and writing a snapshot into its entry would not steer it. The
/// array is dense rather than sparse because a sparse one would have to be a
/// map, and a map's iteration order must never reach simulation state.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerInputs {
    slots: [InputSnapshot; crate::world::MAX_PLAYERS],
}

impl PlayerInputs {
    /// Nothing held in any slot.
    ///
    /// What a headless run, a capture and every ground-truth test that only
    /// needs the clock to advance hands the tick.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// One person in slot 0, nothing in the rest.
    ///
    /// The shape of every session this engine currently runs, and the reason
    /// widening `oag_raceplay::Race::tick` changed no behaviour: with
    /// [`crate::World::SINGLE_PLAYER`] on the other side, exactly slot 0 is
    /// consulted and it is handed exactly what the single-snapshot signature
    /// used to pass.
    #[must_use]
    pub fn single(snapshot: InputSnapshot) -> Self {
        let mut inputs = Self::none();
        inputs.slots[0] = snapshot;
        inputs
    }

    /// What slot `i` is holding.
    ///
    /// Out of range reads as nothing held rather than panicking: the tick asks
    /// this per slot over a fixed range, and a bounds panic in the middle of a
    /// simulation step is a worse failure than a craft that coasts.
    #[must_use]
    pub fn get(&self, slot: usize) -> &InputSnapshot {
        self.slots.get(slot).unwrap_or(&NOTHING_HELD)
    }

    /// Puts a snapshot in slot `i`, ignoring a slot past the grid.
    pub fn set(&mut self, slot: usize, snapshot: InputSnapshot) {
        if let Some(entry) = self.slots.get_mut(slot) {
            *entry = snapshot;
        }
    }
}

impl From<InputSnapshot> for PlayerInputs {
    fn from(snapshot: InputSnapshot) -> Self {
        Self::single(snapshot)
    }
}

impl From<&InputSnapshot> for PlayerInputs {
    fn from(snapshot: &InputSnapshot) -> Self {
        Self::single(*snapshot)
    }
}

/// What [`PlayerInputs::get`] hands back for a slot past the grid.
static NOTHING_HELD: InputSnapshot = InputSnapshot::EMPTY;
