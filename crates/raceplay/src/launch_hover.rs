//! The grid hover clamp: what `Craft_Update` carries in `entry+0x348`, written to every craft each
//! tick from the countdown clock.
//!
//! Wipeout HD's craft sits at a lower hover than its race value from the first frame of the
//! flyby, through the whole countdown, and for `2.46` s after the green light, because its
//! hover target is `min(base, entry+0x348) * 0.75` and `+0x348` is `3.0x` in the grid state and a
//! seconds timer after it. Read and measured in
//! [`hover-target.md`](../../../docs/ghidra/functions/ps3-hdfury-eu/hover-target.md); the numbers are
//! the title's [`oag_title::launch_hover::LaunchHover`].
//!
//! # What is chosen, not measured
//!
//! - The jitter is dropped: the clamp is the mean of the original's `3.000 .. 3.0765`.
//! - The timer's first step lands on the tick the grid state ends, the original's frame order
//!   being unread to the tick. At `1.0` unit a second that is `0.017` units of clamp.
//! - Every craft gets it, AI included: the original's release loop covers the craft array, and
//!   Time Trial's lone player was the only one measured.

use oag_race::RaceState;
use oag_title::launch_hover::LaunchHover;

use super::Race;

/// The clamp on the pre-scale hover target at `world_tick`, for a race ticking at `dt` seconds.
///
/// On the grid it is the title's `grid_cap`; from the tick the grid state ends it grows by the
/// release rate for every second since, the first step included.
#[must_use]
pub fn cap_at(hover: &LaunchHover, world_tick: u64, dt: f32) -> f32 {
    let grid_cap = hover.grid_cap.value;
    if RaceState::thrust_gated(world_tick + 1) {
        return grid_cap;
    }
    grid_cap + hover.release_rate.value * released_ticks(world_tick) as f32 * dt
}

/// Ticks since the grid state ended, counting the tick it ended on as the first.
fn released_ticks(world_tick: u64) -> u64 {
    (world_tick + 1).saturating_sub(oag_race::COUNTDOWN_TICKS) + 1
}

impl Race {
    /// Writes this tick's grid hover clamp to every craft: `None` for a title with none.
    pub(super) fn set_hover_caps(&mut self) {
        let cap = self
            .sim
            .launch_hover
            .map(|hover| cap_at(hover, self.sim.world.tick, self.sim.dt));
        for ship in &mut self.sim.world.ships {
            ship.physics.hover_cap = cap;
        }
    }
}

#[cfg(test)]
mod tests;
