//! How a title's effects volume group reaches the speakers.
//!
//! Pulse's commit helper `FUN_089957c0` multiplies a voice's volume by its group's volume
//! **squared**, so the SFX slider and the flyby's fade, which reach the group as one fraction,
//! act as the square of that fraction (`docs/gameplay/race-intro.md`, "What the original plays
//! under the flyby"). A title says which law its effects run, and where that came from, as
//! [`crate::Title::effects_volume_law`].

/// The curve from the effects fraction (slider times fade) to the gain on the effects bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectsVolumeLaw {
    /// The gain is the fraction itself.
    Linear,
    /// The gain is the fraction squared.
    Squared,
}

impl EffectsVolumeLaw {
    /// The bus gain for `fraction`, `0..=1`.
    #[must_use]
    pub fn gain(self, fraction: f32) -> f32 {
        match self {
            Self::Linear => fraction,
            Self::Squared => fraction * fraction,
        }
    }
}
