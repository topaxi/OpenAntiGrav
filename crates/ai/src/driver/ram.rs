//! The shove: whether a driver throws its craft sideways at a rival, and which
//! way.
//!
//! Its own file rather than another block in [`super`], which is over
//! `scripts/check-file-size.py`'s thousand-line rule and may only shrink. The
//! constants it reads - `RAM_STREAM`, `RAM_RATE`, `RAM_CLEARANCE` - stay beside
//! the driver's others, because they are read there and tested there.

use super::{Context, Driver, Personality, RAM_CLEARANCE, RAM_RATE, RAM_STREAM, roll};
use oag_physics::{ShipState, Sideshift};

impl Driver {
    /// Whether to throw the craft sideways at a rival this tick, and which way.
    ///
    /// **Gated on the physics' own `ShipState::shift_lockout`** rather than on a
    /// cooldown of the driver's own: the timer deciding whether a shift can fire
    /// is already in the snapshot and already hashed, and a second one beside it
    /// would be a second source of truth that drifts out of step with the first.
    ///
    /// Four conditions, and the last is the one that is easy to forget: there
    /// has to be **corridor room on the side being shifted toward, measured from
    /// where the craft is** and not from the line. See [`RAM_CLEARANCE`].
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
