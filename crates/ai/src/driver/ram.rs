//! The shove: whether a driver throws its craft sideways at a rival, and which
//! way.
//!
//! Its own file rather than another block in [`super`], which is over
//! `scripts/check-file-size.py`'s thousand-line rule and may only shrink. The
//! constants it reads - `RAM_STREAM`, `RAM_RATE`, `RAM_CLEARANCE` - stay beside
//! the driver's others, because they are read there and tested there.

use super::{Context, Driver, Personality, RAM_CLEARANCE, RAM_RATE, RAM_STREAM, roll};
use oag_physics::{ShipState, Sideshift};

/// The grid slot a ram is allowed to target: **the player's, and only the
/// player's**.
///
/// The same slot-zero convention [`Driver::for_slot`] already runs on, used
/// here as a rule rather than as an optimisation.
///
/// **Reported from play 2026-08-24: AI-on-AI ramming spirals in a clump.** The
/// gesture is a shove that lands and provokes, and provocation raises the
/// appetite for the next one - so three or four craft running together feed
/// each other until the whole group is off the racing line. That is a positive
/// feedback loop between `Self::stew` and this function with nothing damping
/// it, and it does not read as racing from the outside; it reads as a pile-up
/// that will not stop.
///
/// Two other ways out were available and both are worse. A cooldown longer than
/// the physics' own `SIDESHIFT_LOCKOUT` would be a second source of truth
/// beside a timer already in the snapshot - the thing this function's own doc
/// refuses. Suppressing a shove while several rivals are near would need a
/// count `Field` does not carry, and would let the loop run right up to the
/// threshold. **Aggression toward the player is the part that was wanted**, and
/// it costs nothing to keep it: `Field::alongside` already names the slot.
///
/// The cost, recorded rather than hidden: the field no longer shoves *itself*,
/// so an opponent battle mid-pack is quieter than it was. Wire it back the day
/// something damps the loop - the victim's provocation being spent by the
/// contact rather than only added to would be the honest damper, and
/// `Race::resolve_craft_pairs` already computes the contact it would need.
const PLAYER_SLOT: u8 = 0;

impl Driver {
    /// Whether to throw the craft sideways at a rival this tick, and which way.
    ///
    /// **Gated on the physics' own `ShipState::shift_lockout`** rather than on a
    /// cooldown of the driver's own: the timer deciding whether a shift can fire
    /// is already in the snapshot and already hashed, and a second one beside it
    /// would be a second source of truth that drifts out of step with the first.
    ///
    /// Five conditions, and two are easy to forget: there has to be **corridor
    /// room on the side being shifted toward, measured from where the craft
    /// is** and not from the line (see [`RAM_CLEARANCE`]), and the rival has to
    /// be **the player** (see [`PLAYER_SLOT`]).
    ///
    /// Worth saying plainly: `Race::resolve_craft_pairs` discards the contact it
    /// computes and nothing arms `stun_timer`, so **a ram shoves and nothing
    /// else** - no stun, no damage, no score. Its payoff is positional.
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
        // **From the craft, not from the line**, which a craft hardly ever sits
        // on. Before this, one shove on `16_Track` fired with 3.13 units of
        // corridor to its left while the craft was 11.67 past that edge.
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
