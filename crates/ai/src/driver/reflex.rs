//! How long a driver takes to notice something.
//!
//! The fourth axis of the skill vector in `docs/gameplay/ai.md`. The others
//! degrade what a driver *does* with what it sees; this one degrades **when it
//! can see it at all**.
//!
//! # A stimulus is invisible until the driver has had time to notice it
//!
//! [`Field`] is measured this tick, and a driver used to act on it the tick it
//! arrived: that is no perception step at all, and makes a slow opponent still
//! feel machine-like. So each channel ([`Field`]'s ahead, behind, alongside) is
//! held back for [`super::Tuning::reaction_ticks`] ticks after its occupant
//! *changes* and reads as empty until then, rather than as a stale answer. A
//! channel **emptying** is acknowledged at once: defending against a ghost
//! reads as a bug, a moment's delay on a new arrival reads as a driver.
//!
//! # What it does not delay
//!
//! [`Field::place`], so the grudge in [`super::Driver::stew`]: a place is the
//! standings, not something seen from a cockpit, and annoyance at an overtake
//! is a reaction to having *been* passed.
//!
//! # Determinism
//!
//! Integers only, so [`super::Driver`] stays `Copy` and `Eq` and rides in the
//! world snapshot. No clock: the countdown is in ticks.

use crate::field::{Field, Rival};

/// The occupant of an empty channel. The grid is eight, so no slot collides,
/// and it is deliberately **not** zero, the player's slot: a zero-initialised
/// [`Reflex`] would claim to have already noticed the player in every channel.
const NOBODY: u8 = u8::MAX;

const AHEAD: usize = 0;
const BEHIND: usize = 1;
const ALONGSIDE: usize = 2;

/// What a driver has noticed, and what it is still noticing: one entry per
/// [`Field`] channel, in the order [`AHEAD`], [`BEHIND`], [`ALONGSIDE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reflex {
    /// The rival this driver is acting on in each channel, or [`NOBODY`].
    pub seen: [u8; 3],
    /// The rival whose clock is running in each channel, or [`NOBODY`].
    ///
    /// **Its own array, not an implied "whoever is there now"**: a second rival
    /// taking the channel mid-wait starts its own clock instead of inheriting
    /// the first one's remainder.
    pub pending: [u8; 3],
    pub wait: [u16; 3],
}

impl Reflex {
    /// A driver that has noticed nothing: every driver before its first tick.
    pub const IDLE: Self = Self {
        seen: [NOBODY; 3],
        pending: [NOBODY; 3],
        wait: [0; 3],
    };

    /// Advances every channel's clock against what the caller measured. Call it
    /// once a tick before [`Self::filter`], so a wait that runs out this tick is
    /// acted on this tick.
    pub fn advance(&mut self, field: &Field, latency: u16) {
        for (channel, rival) in [field.ahead, field.behind, field.alongside]
            .into_iter()
            .enumerate()
        {
            self.notice(channel, rival.map_or(NOBODY, |rival| rival.slot), latency);
        }
    }

    /// What this driver may act on, of what the caller measured: every channel
    /// it has not noticed reads empty. [`Field::place`] passes through (see the
    /// module header).
    ///
    /// **[`Field::hazard`] passes through too, for a different reason.** The
    /// clock is keyed on *which craft* is in a channel; a laid charge has no
    /// identity to track. So a driver reacts to a charge with no latency, the
    /// smaller wrong answer: [`Driver::avoidance`] already ramps gently from the
    /// far edge of its lookahead so the reaction *looks* gradual, and the
    /// alternative drives an opponent through a mine a player would have dodged.
    #[must_use]
    pub fn filter(&self, field: &Field) -> Field {
        Field {
            ahead: self.noticed(AHEAD, field.ahead),
            behind: self.noticed(BEHIND, field.behind),
            alongside: self.noticed(ALONGSIDE, field.alongside),
            hazard: field.hazard,
            // A pad has no identity to notice: no latency, like a hazard.
            pad: field.pad,
            place: field.place,
        }
    }

    /// One channel's clock.
    ///
    /// **A latency of zero is acknowledged on the tick the rival arrives**,
    /// which keeps [`super::Tuning::default`]'s competent driver exactly what it
    /// was before this existed.
    fn notice(&mut self, channel: usize, slot: u8, latency: u16) {
        if self.seen[channel] == slot {
            self.pending[channel] = NOBODY;
            self.wait[channel] = 0;
            return;
        }
        if slot == NOBODY {
            self.seen[channel] = NOBODY;
            self.pending[channel] = NOBODY;
            self.wait[channel] = 0;
            return;
        }
        if self.pending[channel] == slot {
            self.wait[channel] = self.wait[channel].saturating_sub(1);
        } else {
            self.pending[channel] = slot;
            self.wait[channel] = latency;
        }
        if self.wait[channel] == 0 {
            self.seen[channel] = slot;
            self.pending[channel] = NOBODY;
        }
    }

    fn noticed(&self, channel: usize, rival: Option<Rival>) -> Option<Rival> {
        rival.filter(|rival| self.seen[channel] == rival.slot)
    }
}

impl Default for Reflex {
    fn default() -> Self {
        Self::IDLE
    }
}

#[cfg(test)]
mod tests;
