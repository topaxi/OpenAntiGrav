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
}

impl Laws {
    /// Pulse's laws, every title's unless it measured its own.
    pub const PULSE: Self = Self {
        engine_output_double: true,
    };
}

impl Default for Laws {
    fn default() -> Self {
        Self::PULSE
    }
}
