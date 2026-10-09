//! HD/Fury's boost and damage zoom-streak ring, on the race's side of the seam.
//!
//! The pass itself is [`oag_post::hd_zoom`] and its numbers are
//! `oag_title::ZoomRing`, HD's own title data. What lives here is the race's part:
//! the pulses [`oag_post::hd_zoom::Pulse`] steps once a tick, **fired by the
//! player's speed pad and Turbo** - the two sources measured on the original (a
//! pad's pulse on Talon's Junction at the start line's pad, a Turbo pickup) - and
//! read by the scene as a [`oag_post::hd_zoom::Frame`].
//!
//! **Not wired, and why:**
//!
//! - **The start boost.** The original did not fire the pulse at `GO` in three
//!   runs, but none of them was shown to have had a start boost at all, so it
//!   stays unfired rather than guessed either way.
//! - **A barrel roll.** Untested on the original.
//! - **Damage `P`.** The pass draws it and the tint is the live one, but the unit
//!   `Ship_ApplyDamage` hands `Hud_RaiseDamagePulse` (`1.8 * damage`) is not
//!   related to a hit of ours anywhere a page establishes, so nothing raises it.
//! - **An opponent's boost.** The original's pulse is the viewing craft's.

use oag_post::hd_zoom::{Frame, Tuning};

use crate::Race;

/// `oag_title::ZoomRing` as the pass reads it.
#[must_use]
pub(crate) fn tuning(ring: &oag_title::ZoomRing) -> Tuning {
    Tuning {
        history_weight: ring.history_weight,
        history_weight_cap: ring.history_weight_cap,
        history_crop: ring.history_crop,
        inner_radius: ring.inner_radius,
        outer_radius: ring.outer_radius,
        segments: ring.segments,
        tap_pull: ring.tap_pull,
        boost_start: ring.boost.start,
        boost_fall_rate: ring.boost.fall_rate,
        boost_slope: ring.boost.slope,
        boost_offset: ring.boost.offset,
        boost_gain: ring.boost.gain,
        boost_decay_per_update: ring.boost.decay_per_update,
        damage_tint: ring.damage.tint,
        damage_decay_per_second: ring.damage.decay_per_second,
        jitter_retain: ring.jitter.retain,
        jitter_step: ring.jitter.step,
        jitter_range: ring.jitter.range,
    }
}

impl Race {
    /// The pulses for the tick last stepped, or `None` on a title with no ring.
    ///
    /// The one read the scene takes; the frame carries the tick, so the history
    /// draw runs once per tick however fast frames render.
    #[must_use]
    pub fn hd_zoom_frame(&self) -> Option<Frame> {
        self.view.zoom.as_ref().map(oag_post::hd_zoom::Pulse::frame)
    }

    /// The boost pulse, for the player's own speed pad or Turbo only.
    pub(super) fn fire_zoom_boost(&mut self, slot: usize) {
        if slot == self.sim.world.primary_slot()
            && let Some(pulse) = &mut self.view.zoom
        {
            pulse.fire_boost();
        }
    }

    /// One tick of the pulses.
    pub(super) fn advance_zoom(&mut self) {
        if let Some(pulse) = &mut self.view.zoom {
            pulse.advance(self.sim.dt);
        }
    }
}
