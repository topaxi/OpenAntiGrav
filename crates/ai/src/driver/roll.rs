//! Whether a driver commits to a barrel roll, and which way.
//!
//! Its own file rather than another block in [`super`], which is over
//! `scripts/check-file-size.py`'s thousand-line rule and may only shrink - the
//! same seam [`super::ram`] took. The two noise streams it rolls against stay
//! beside the driver's others, because the whole point of a stream constant is
//! that no two decisions accidentally share one.

use super::{Driver, Personality, ROLL_SIDE_STREAM, ROLL_STREAM, roll};
use oag_physics::ShipState;
use oag_physics::barrel_roll::TapDirection;

impl Driver {
    /// Whether this driver commits to a barrel roll this tick, and which way.
    ///
    /// # This is invented, and the original's opponents never roll
    ///
    /// Recovered 2026-09-06 at confidence 85: the original reads the roll's tap
    /// history out of the human player's pad block and nothing else. Ours roll
    /// on a maintainer's directive of the same day - *"our AI shall do barrel
    /// rolls, 'invented', ACE level shall do barrel rolls as long as there's
    /// energy budget, lower AI tiers shall do less barrel rolls"* - so this
    /// whole method is a game-design choice rather than a port, and
    /// `docs/gameplay/ai.md` records it as one.
    ///
    /// # Two gates here, two more below
    ///
    /// This decides **propensity** only. Being airborne at all and
    /// `cost < shield` are both recovered, both enforced in
    /// `oag_physics::barrel_roll`, and neither is weakened by anything here -
    /// a request this returns is refused there if either fails. The energy
    /// budget travels with it on [`ShipControls::roll_shield_floor`], because
    /// the cost and the pool's capacity are physics' to know.
    ///
    /// # One decision per flight
    ///
    /// [`Personality::roll_chance`] is a probability and not a rate, unlike
    /// [`Personality::trigger`]: [`Driver::roll_decided`] is set on the first
    /// tick past [`Personality::roll_airtime`] whichever way the coin lands, so
    /// a jump gets exactly one decision. A per-tick roll would make a tenth
    /// fire on any long jump and the axis would not mean what it says.
    ///
    /// **Provocation is deliberately not an input.** The grudge scales what a
    /// driver does to *other craft*, never what it asks of its own - see
    /// [`Self::stew`] - and a roll is entirely the latter. Feeding it would
    /// make an angry AI faster, closing the rubber-banding loop this project
    /// refuses to port.
    ///
    /// `state.grounded` is **last completed tick's** contact count, which is
    /// exactly the value `oag_physics::forces::evaluate` will hand the gesture
    /// as its own grounded gate on the tick these controls are stepped with.
    /// Reading `grounded_prev` instead would be a tick behind it, and the
    /// disagreement would let a second roll into one flight.
    pub(super) fn wants_to_roll(
        &mut self,
        state: &ShipState,
        personality: &Personality,
    ) -> Option<TapDirection> {
        if state.is_grounded() {
            self.roll_decided = false;
            return None;
        }
        if self.roll_decided || state.time_airborne < personality.roll_airtime {
            return None;
        }
        self.roll_decided = true;
        if roll(self.seed, self.phase, ROLL_STREAM) >= personality.roll_chance {
            return None;
        }
        // Which way is arbitrary and says so: nothing about the corner ahead or
        // the craft's attitude is read. A roll is aimed at nobody.
        if roll(self.seed, self.phase, ROLL_SIDE_STREAM) < 0.5 {
            Some(TapDirection::Left)
        } else {
            Some(TapDirection::Right)
        }
    }
}
