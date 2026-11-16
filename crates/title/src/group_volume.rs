//! How a title's effects and speech volume groups reach the speakers.
//!
//! Pulse's commit helper `FUN_089957c0` multiplies a voice's volume by its group's volume
//! **squared**, so the SFX slider and the flyby's fade, which reach the group as one fraction,
//! act as the square of that fraction (`docs/gameplay/race-intro.md`, "What the original plays
//! under the flyby"). The Speech group (2) takes the same helper and is measured squared against
//! the SFX fraction too (same page, "The Speech group"). A title says which law each runs, and
//! where that came from, as [`crate::Title::effects_volume_law`] and
//! [`crate::Title::speech_volume_law`].

/// The curve from a group's volume fraction (for effects, slider times fade) to the gain on its bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupVolumeLaw {
    /// The gain is the fraction itself.
    Linear,
    /// The gain is the fraction squared.
    Squared,
}

impl GroupVolumeLaw {
    /// The bus gain for `fraction`, `0..=1`.
    #[must_use]
    pub fn gain(self, fraction: f32) -> f32 {
        match self {
            Self::Linear => fraction,
            Self::Squared => fraction * fraction,
        }
    }
}
