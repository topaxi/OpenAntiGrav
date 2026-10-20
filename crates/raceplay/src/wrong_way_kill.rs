//! The Eliminator's wrong-way kill: a craft flying back down the track loses its engine.
//!
//! Pulse PSP's entity update `FUN_088418e0` ends (`0x088428a4`-`0x08842924`) with
//!
//! ```c
//! if (mode == 8 || mode == 18)            // Elimination, Multiplayer Elimination
//!     if (dot(node->forward, entity->tangent) < -0.4f) craft->flags |=  0x2000;
//!     else                                              craft->flags &= ~0x2000;
//! ```
//!
//! and the bit zeroes the engine and holds both airbrakes full
//! ([`oag_physics::ShipState::wrong_way_kill`]). In every other mode the block is skipped and
//! nothing else sets the bit. The tangent is `entity+0xb00`, the located spline record's;
//! **chosen, not measured:** here it is the nearest sample's tangent, normalised, which is
//! what [`crate::Race::wrong_way`] reads too, at its own threshold (`0.0`, the HUD's).
//!
//! The original sets the bit at the end of a craft's update and the force law reads it in the
//! next; written here at the top of the tick from the same pose, which is the same value.

use oag_core::math::Vec3;
use oag_race::Mode;

use super::Race;

/// `dot(forward, tangent)` below this is wrong-way for the kill (`0xbecccccd`, `0x088428dc`).
pub const WRONG_WAY_DOT: f32 = -0.4;

impl Race {
    /// Writes [`oag_physics::ShipState::wrong_way_kill`] for every craft this tick: set only in
    /// an Eliminator race, on a title that has the law, for a craft facing back down the track.
    pub(super) fn set_wrong_way_kill(&mut self) {
        let armed = self.sim.eliminator_wrong_way_kill && self.sim.world.mode() == Mode::Eliminator;
        for ship in &mut self.sim.world.ships {
            ship.physics.wrong_way_kill = armed && {
                let body = &ship.physics.body;
                self.sim
                    .spline
                    .nearest(body.position)
                    .is_some_and(|(_, sample, _)| {
                        let tangent = Vec3::from_array(sample.tangent).normalize_or_zero();
                        body.forward().dot(tangent) < WRONG_WAY_DOT
                    })
            };
        }
    }
}
