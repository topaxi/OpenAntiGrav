//! What the Autopilot pickup does to the player's engine while it flies the craft.
//!
//! Every title's local-player status update, while the pickup's running bit `0x800`
//! is set, lifts the acceleration cap (engine bit `0x8`) and multiplies thrust after
//! the cap by a stored `1.2` (engine bit `0x4`). Read on all five executables and live
//! on two: Pulse (PPSSPP, `craft+0x1c0 = 0xd`) and HD (RPCS3, `craft+0x264 = 0xd`,
//! `craft+0x2f0 = 1.2`). Evidence per title:
//! `docs/ghidra/functions/psp-pulse-usa/autopilot.md` and
//! `docs/ghidra/functions/ps3-hdfury-eu/autopilot.md`. Data, not a comparison on a
//! title's name (ADR-0058).

use crate::pre_race::Sourced;

/// One title's Autopilot engine law.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutopilotEngine {
    /// The thrust multiplier applied after the cap while the pickup runs.
    pub speedup: Sourced<f32>,
    /// Whether the acceleration cap is lifted while the pickup runs.
    pub uncapped: Sourced<bool>,
}

/// The law as every title's executable carries it: thrust x`1.2`, no cap.
pub const MEASURED: AutopilotEngine = AutopilotEngine {
    speedup: Sourced::measured(1.2),
    uncapped: Sourced::measured(true),
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::Origin;

    #[test]
    fn the_measured_law_is_one_point_two_and_uncapped() {
        assert_eq!(MEASURED.speedup, Sourced::measured(1.2));
        assert_eq!(MEASURED.uncapped, Sourced::measured(true));
        assert_eq!(MEASURED.speedup.origin, Origin::Measured);
    }
}
