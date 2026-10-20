//! The force-law constants a title changes from Pulse's, carried per craft.
//!
//! Pulse's values are the default. Wipeout Pure's craft update is Pulse's in shape and differs
//! in a handful of constants, read off its `BOOT.BIN` and measured live
//! (`docs/physics/pure-physics.md`). The race passes a title's own set in
//! [`crate::Environment::laws`], from `oag_title::craft_laws::CraftLaws`: a title's constants
//! are not per-craft state, so they live outside the hashed world.

/// One title's force-law constants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Laws {
    /// `Ship_UpdateEngine` doubles its output: `T = T * craft+0x294 * 2.0` on Pulse
    /// ([`crate::engine::ENGINE_OUTPUT_DOUBLE`]). Pure's engine (`0x0892f1f0`) has no such
    /// factor, measured live on 374 frames to `3e-6`.
    pub engine_output_double: bool,
    /// The grounded quadratic drag coefficient: Pulse's [`crate::passive::DRAG_GROUND`],
    /// Pure's [`crate::passive::DRAG_GROUND_PURE`].
    pub drag_ground: f32,
    /// The two-point hover's bank-to-yaw gain: Pulse's [`crate::hover::BANK_TO_YAW_GAIN`]
    /// (`30`), Pure's [`crate::hover::BANK_TO_YAW_GAIN_PURE`] (`50`). The four-corner hover's
    /// `50` is every title's.
    pub bank_to_yaw: f32,
    /// The pitch damping while the craft is in the grid state ([`crate::ShipState::on_grid`]),
    /// in place of the racing `pitch_damping`. Pulse's is the racing value throughout
    /// (`None`). Pure's `Ship_ApplyAngularDamping` (`0x0892cc2c`) damps pitch at `3 / c` with
    /// `c` the roll coefficient, `-5` on the grid and `-2` racing, so `0.6` on the grid
    /// against `1.5`: measured live to `2e-6` (`docs/physics/pure-physics.md`).
    pub pitch_damping_grid: Option<f32>,
}

impl Laws {
    /// Pulse's laws, every title's unless it measured its own.
    pub const PULSE: Self = Self {
        engine_output_double: true,
        drag_ground: crate::passive::DRAG_GROUND,
        bank_to_yaw: crate::hover::BANK_TO_YAW_GAIN,
        pitch_damping_grid: None,
    };
}

impl Default for Laws {
    fn default() -> Self {
        Self::PULSE
    }
}
