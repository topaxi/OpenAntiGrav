//! The shove: whether a driver throws its craft sideways at a rival, and which
//! way.
//!
//! Split out of [`super`] for `scripts/check-file-size.py`. The constants it
//! reads (`RAM_STREAM`, `RAM_RATE`, `RAM_CLEARANCE`) stay beside the driver's
//! others, where they are read and tested.

use super::{Context, Driver, Personality, RAM_CLEARANCE, RAM_RATE, RAM_STREAM, roll};
use oag_physics::{ShipState, Sideshift};

/// The grid slot a ram is allowed to target: **the player's, and only the
/// player's**. The slot-zero convention [`Driver::for_slot`] runs on, used as a
/// rule.
///
/// **Reported from play 2026-08-24: AI-on-AI ramming spirals in a clump.** A
/// shove lands and provokes, and provocation raises the appetite for the next,
/// so three or four craft running together feed each other until the group is
/// off the line: a positive feedback loop between `Self::stew` and this
/// function, reading as a pile-up that will not stop.
///
/// Worse alternatives: a cooldown beyond the physics' `SIDESHIFT_LOCKOUT` is a
/// second source of truth beside a timer already in the snapshot; suppressing
/// shoves while several rivals are near needs a count `Field` lacks and runs the
/// loop up to the threshold. Aggression toward the player is the wanted part,
/// and `Field::alongside` already names the slot.
///
/// The cost: the field no longer shoves *itself*, so a mid-pack battle is
/// quieter. Wire it back once something damps the loop (the victim's provocation
/// being spent by contact; `Race::resolve_craft_pairs` already computes it).
const PLAYER_SLOT: u8 = 0;

impl Driver {
    /// Whether to throw the craft sideways at a rival this tick, and which way.
    ///
    /// **Gated on the physics' own `ShipState::shift_lockout`**, not a cooldown
    /// of the driver's own: that timer is already in the snapshot and hashed, and
    /// a second would drift out of step.
    ///
    /// Five conditions, two easy to forget: **corridor room on the side being
    /// shifted toward, measured from the craft** not the line
    /// ([`RAM_CLEARANCE`]), and the rival must be **the player** ([`PLAYER_SLOT`]).
    ///
    /// `Race::resolve_craft_pairs` discards the contact it computes and nothing
    /// arms `stun_timer`, so **a ram shoves and nothing else**: no stun, damage
    /// or score. Its payoff is positional.
    pub(super) fn ram(
        &self,
        state: &ShipState,
        ctx: &Context<'_>,
        personality: &Personality,
    ) -> Sideshift {
        if personality.ram <= 0.0 || state.shift_lockout > 0.0 {
            return Sideshift::None;
        }
        if state.sideshift_timers.iter().any(|timer| *timer > 0.0) {
            return Sideshift::None;
        }
        let Some(rival) = ctx.field.alongside else {
            return Sideshift::None;
        };
        if rival.slot != PLAYER_SLOT {
            return Sideshift::None;
        }

        // Rammed toward the rival, so the room that matters is on its side.
        let toward = rival.offset;
        if toward.abs() <= f32::EPSILON {
            return Sideshift::None;
        }
        let aim = ctx.line.aim(self.index as usize, 0.0);
        // **From the craft, not the line**, which a craft hardly sits on: one
        // shove on `16_Track` fired with 3.13 units of corridor to its left while
        // the craft was 11.67 past that edge.
        let room = aim.corridor.map_or(f32::INFINITY, |frame| {
            let offset = (state.body.position - aim.point).dot(frame.lateral);
            frame.room(toward) - offset * toward.signum()
        });
        if room < RAM_CLEARANCE {
            return Sideshift::None;
        }

        let appetite = personality.ram * (1.0 + self.provoked());
        if roll(self.seed, self.phase, RAM_STREAM) >= appetite * RAM_RATE {
            return Sideshift::None;
        }
        if toward > 0.0 {
            Sideshift::Right
        } else {
            Sideshift::Left
        }
    }
}
