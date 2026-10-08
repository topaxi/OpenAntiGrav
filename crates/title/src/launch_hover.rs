//! A title's grid hover: the craft's hover target is held low until a measured time after the
//! green light, and then released along a ramp.
//!
//! Wipeout HD's craft update clamps the hover target to a number a timer carries (`docs/ghidra/
//! functions/ps3-hdfury-eu/hover-target.md`): in the craft's grid state the clamp is `3.0` plus a
//! small random jitter, and from the green light it is that value plus the seconds since, so the
//! target `min(base, clamp) * scale` climbs at `scale` units a second until the race's own value
//! takes over. What differs per title is data, so it is here and not a comparison on a title's
//! name (ADR-0058).

use crate::pre_race::Sourced;

/// One title's grid hover.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaunchHover {
    /// The clamp on the pre-scale hover target while the craft is on the grid, in the handling's
    /// own units.
    ///
    /// The original draws `3.0 + rand8 * 0.0003` (`3.000` to `3.0765`) and re-rolls it every
    /// `rand8 * 0.002` seconds; this is the mean of that range, `3.03825`. The mean is **chosen,
    /// not measured**: the jitter is `0.03` world units on the drawn craft and drawing it would
    /// need the shared random stream.
    pub grid_cap: Sourced<f32>,
    /// How fast the clamp rises once the grid state ends, per second of race time.
    ///
    /// Measured: `entry+0x348` advances `1.00` per game second on three boots.
    pub release_rate: Sourced<f32>,
}
