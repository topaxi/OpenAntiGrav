//! How long a driver takes to notice something.
//!
//! The fourth axis of the skill vector `docs/gameplay/ai.md` sets out, and the
//! last of its six that decides how a driver treats the craft around it. Every
//! other axis degrades what a driver *does* with what it can see; this one
//! degrades **when it can see it at all**.
//!
//! # A stimulus is invisible until the driver has had time to notice it
//!
//! [`Field`] is a snapshot the caller measured this tick, and until this
//! existed a driver acted on it the tick it arrived: a rival that came
//! alongside was covered on the frame it got there, and one that appeared in
//! the weapon cone was shot at on the same frame. That is not a quick driver,
//! it is one with no perception step at all, and it is the thing that makes a
//! slow opponent still feel machine-like.
//!
//! So each of the three channels [`Field`] carries - ahead, behind,
//! alongside - is held back for [`super::Tuning::reaction_ticks`] ticks after
//! its occupant *changes*, and reads as empty until then. The driver is not
//! told a stale answer; it is told nothing, which is the honest model of
//! not having noticed yet. Acting on a rival that is no longer there is the
//! worse artefact of the two, and that is why a channel **emptying** is
//! acknowledged at once: a driver defending against a ghost reads as a bug,
//! while one that takes a moment to spot a new arrival reads as a driver.
//!
//! # What it deliberately does not delay
//!
//! [`Field::place`], and therefore the grudge in [`super::Driver::stew`]. A
//! place is the standings rather than something seen out of a cockpit, and
//! being annoyed about an overtake is a reaction to having *been* passed, not
//! a reaction time. Delaying it would make a driver notice it was overtaken
//! late, which is not what the axis means.
//!
//! # Determinism
//!
//! Integers only, so [`super::Driver`] stays `Copy` and `Eq` and rides in the
//! world snapshot like the rest of it - see that type for why a driver whose
//! state lived outside the snapshot would not replay. No clock is read: the
//! countdown is in ticks, driven by the caller's own tick.

use crate::field::{Field, Rival};

/// The occupant of an empty channel.
///
/// The grid is eight, so no real slot can collide with this - and it is
/// deliberately **not** zero, which is the player's slot. A [`Reflex`] that
/// zero-initialised would claim to have already noticed the player in all
/// three channels, which is exactly backwards.
const NOBODY: u8 = u8::MAX;

/// The nearest craft in front.
const AHEAD: usize = 0;
/// The nearest craft behind.
const BEHIND: usize = 1;
/// One close enough alongside to touch.
const ALONGSIDE: usize = 2;

/// What a driver has noticed, and what it is still in the middle of noticing.
///
/// One entry per channel of [`Field`], in the order [`AHEAD`], [`BEHIND`],
/// [`ALONGSIDE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reflex {
    /// The rival this driver is acting on in each channel, or [`NOBODY`].
    pub seen: [u8; 3],
    /// The rival whose clock is running in each channel, or [`NOBODY`].
    ///
    /// **Its own array rather than an implied "whoever is there now"**, so a
    /// second rival taking the channel over mid-wait starts its own clock
    /// instead of inheriting what was left of the first one's. A driver part
    /// way through noticing one craft has not part-noticed a different one.
    pub pending: [u8; 3],
    /// Ticks left before [`Self::pending`] is acted on.
    pub wait: [u16; 3],
}

impl Reflex {
    /// A driver that has noticed nothing, which is every driver before its
    /// first tick.
    pub const IDLE: Self = Self {
        seen: [NOBODY; 3],
        pending: [NOBODY; 3],
        wait: [0; 3],
    };

    /// Advances every channel's clock against what the caller measured.
    ///
    /// Call it once a tick, before [`Self::filter`], so a channel whose wait
    /// runs out this tick is acted on this tick rather than next.
    pub fn advance(&mut self, field: &Field, latency: u16) {
        for (channel, rival) in [field.ahead, field.behind, field.alongside]
            .into_iter()
            .enumerate()
        {
            self.notice(channel, rival.map_or(NOBODY, |rival| rival.slot), latency);
        }
    }

    /// What this driver may act on, of what the caller measured.
    ///
    /// Every channel it has not noticed yet reads empty. [`Field::place`] is
    /// passed through - see this module's header for why.
    ///
    /// **[`Field::hazard`] is passed through too, and for a different reason.**
    /// The clock below is keyed on *which craft* is in a channel, so it can tell
    /// "a new rival arrived" from "the same one is still there"; a laid charge
    /// has no identity to track, and giving it one would mean a fourth channel
    /// keyed on something that does not exist. The consequence is stated rather
    /// than hidden: a driver reacts to a charge with no latency at all. It is
    /// the smaller wrong answer, because [`Driver::avoidance`] is already a ramp
    /// that starts gently at the far edge of its lookahead - so the reaction
    /// *looks* gradual even though the noticing is instant - and because the
    /// alternative leaves an opponent driving through a mine a player would have
    /// steered around.
    #[must_use]
    pub fn filter(&self, field: &Field) -> Field {
        Field {
            ahead: self.noticed(AHEAD, field.ahead),
            behind: self.noticed(BEHIND, field.behind),
            alongside: self.noticed(ALONGSIDE, field.alongside),
            hazard: field.hazard,
            place: field.place,
        }
    }

    /// One channel's clock.
    ///
    /// **A latency of zero is acknowledged on the tick the rival arrives**,
    /// which is what keeps [`super::Tuning::default`]'s competent driver
    /// exactly the driver it was before this existed - the clock is not a
    /// rounding of one tick.
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

    /// One channel of [`Self::filter`].
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
