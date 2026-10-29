//! The reply: one typed method per command, refusing what the host would fault on.

use oag_ext_abi::{
    Command, Cue, Effect, MAX_COMMANDS, MAX_THRUST_FACTOR, Pickup, Rgba, Sections, Weapon,
};

use crate::Craft;

/// Why a [`Reply`] method sent nothing. The host would have faulted the module for the
/// race on the same command; here the author decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The reply already holds `MAX_COMMANDS` commands.
    Full,
    /// A second `scale_thrust` or `shield_delta` for one craft this tick.
    Repeated,
    /// A value out of its range: a factor outside `[0, MAX_THRUST_FACTOR]`, a shield
    /// change past the craft's maximum, a non-finite number, a roll period of 0.
    Range,
    /// `give_pickup` to a craft holding something, or `clear_pickup` from one not
    /// holding the mod's own pickup.
    Pickup,
}

/// This tick's commands. The count is written when the tick returns.
#[derive(Debug)]
pub struct Reply<'a> {
    reply: &'a mut [u8],
    count: usize,
    thrust: u32,
    shield: u32,
}

impl<'a> Reply<'a> {
    pub(crate) fn new(reply: &'a mut [u8]) -> Self {
        Self {
            reply,
            count: 0,
            thrust: 0,
            shield: 0,
        }
    }

    pub(crate) fn finish(self) {
        Sections::put_count(self.reply, self.count);
    }

    /// Commands sent so far this tick.
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether none has been sent.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn push(&mut self, command: &Command) -> Result<(), Refused> {
        if self.count >= MAX_COMMANDS {
            return Err(Refused::Full);
        }
        Sections::put_command(self.reply, self.count, command);
        self.count += 1;
        Ok(())
    }

    /// `scale-thrust`: this tick, the craft's engine output times `factor`, in
    /// `[0, MAX_THRUST_FACTOR]`. Once per craft per tick; re-sent each tick it holds.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`], [`Refused::Range`].
    pub fn scale_thrust(&mut self, craft: &Craft<'_>, factor: f32) -> Result<(), Refused> {
        if !(0.0..=MAX_THRUST_FACTOR).contains(&factor) {
            return Err(Refused::Range);
        }
        let bit = craft.slot().bit();
        if self.thrust & bit != 0 {
            return Err(Refused::Repeated);
        }
        self.push(&Command::ScaleThrust {
            slot: craft.slot(),
            factor,
        })?;
        self.thrust |= bit;
        Ok(())
    }

    /// `shield-delta`: add `amount` to the craft's energy (negative takes it, through
    /// the damage path, and can wreck). `|amount|` at most [`Craft::shield_max`]. Once
    /// per craft per tick.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`], [`Refused::Range`].
    pub fn shield_delta(&mut self, craft: &Craft<'_>, amount: f32) -> Result<(), Refused> {
        if !amount.is_finite() || amount.abs() > craft.shield_max() {
            return Err(Refused::Range);
        }
        let bit = craft.slot().bit();
        if self.shield & bit != 0 {
            return Err(Refused::Repeated);
        }
        self.push(&Command::ShieldDelta {
            slot: craft.slot(),
            amount,
        })?;
        self.shield |= bit;
        Ok(())
    }

    /// `give-pickup`: the mod's own pickup to a craft holding nothing.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Pickup`].
    pub fn give_pickup(&mut self, craft: &Craft<'_>) -> Result<(), Refused> {
        if craft.pickup() != Pickup::None {
            return Err(Refused::Pickup);
        }
        self.push(&Command::GivePickup { slot: craft.slot() })
    }

    /// `clear-pickup`: take the mod's own pickup from the craft holding it.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Pickup`].
    pub fn clear_pickup(&mut self, craft: &Craft<'_>) -> Result<(), Refused> {
        if craft.pickup() != Pickup::Ours {
            return Err(Refused::Pickup);
        }
        self.push(&Command::ClearPickup { slot: craft.slot() })
    }

    /// `pickup-roll`: race-wide, every `every`-th time a weapon pad hands out `weapon`,
    /// it hands out the mod's own pickup instead. Held until replaced; sending the same
    /// rule again keeps its count.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Range`] for `every` 0.
    pub fn pickup_roll(&mut self, weapon: Weapon, every: u8) -> Result<(), Refused> {
        if every == 0 {
            return Err(Refused::Range);
        }
        self.push(&Command::PickupRoll { weapon, every })
    }

    /// `pad-mode`: this tick, a speed pad gives the craft no boost and raises
    /// [`Event::PadEnergy`](crate::Event::PadEnergy) instead.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`].
    pub fn pad_mode(&mut self, craft: &Craft<'_>) -> Result<(), Refused> {
        self.push(&Command::PadMode { slot: craft.slot() })
    }

    /// `play-cue`: one of the title's own sounds from the craft. Heard, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`].
    pub fn play_cue(&mut self, craft: &Craft<'_>, cue: Cue) -> Result<(), Refused> {
        self.push(&Command::PlayCue {
            slot: craft.slot(),
            cue,
        })
    }

    /// `fire-effect`: one of the title's own particle effects at the craft's nozzle.
    /// Drawn, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`].
    pub fn fire_effect(&mut self, craft: &Craft<'_>, effect: Effect) -> Result<(), Refused> {
        self.push(&Command::FireEffect {
            slot: craft.slot(),
            effect,
        })
    }

    /// `pickup-tint`: this tick, the craft's held-pickup backdrop in `rgba`. Drawn,
    /// never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`].
    pub fn pickup_tint(&mut self, craft: &Craft<'_>, rgba: Rgba) -> Result<(), Refused> {
        self.push(&Command::PickupTint {
            slot: craft.slot(),
            rgba,
        })
    }
}
