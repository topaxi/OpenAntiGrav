//! Whether a driver commits to a barrel roll, and which way.
//!
//! Split out of [`super`] like [`super::ram`]. Its two noise streams stay beside
//! the driver's others: no two decisions may accidentally share one.

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
    /// on a maintainer's directive of the same day: *"our AI shall do barrel
    /// rolls, 'invented', ACE level shall do barrel rolls as long as there's
    /// energy budget, lower AI tiers shall do less barrel rolls"*. A
    /// game-design choice, recorded as one in `docs/gameplay/ai.md`.
    ///
    /// # Two gates here, two more below
    ///
    /// This decides **propensity** only. Being airborne and `cost < shield` are
    /// recovered and enforced in `oag_physics::barrel_roll`, which refuses a
    /// request failing either. The energy budget travels on
    /// [`ShipControls::roll_shield_floor`]: cost and pool capacity are physics'
    /// to know.
    ///
    /// # One decision per flight
    ///
    /// [`Personality::roll_chance`] is a probability, not a rate:
    /// [`Driver::roll_decided`] is set on the first tick past
    /// [`Personality::roll_airtime`] whichever way the coin lands. A per-tick
    /// roll would fire a tenth on any long jump.
    ///
    /// **Provocation is deliberately not an input**: the grudge scales what a
    /// driver does to *other craft* ([`Self::stew`]), and feeding a roll would
    /// make an angry AI faster, the rubber-banding loop this project refuses to
    /// port.
    ///
    /// `state.grounded` is the **last completed tick's** contact count, the value
    /// `oag_physics::forces::evaluate` hands the gesture as its own grounded
    /// gate. `grounded_prev` would be a tick behind and let a second roll into
    /// one flight.
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
        // Which way is arbitrary: nothing about the corner or attitude is read.
        if roll(self.seed, self.phase, ROLL_SIDE_STREAM) < 0.5 {
            Some(TapDirection::Left)
        } else {
            Some(TapDirection::Right)
        }
    }
}
