//! [`Lod`]: whether an authored `LodGroup`'s children are all drawn.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// Whether [`super::build_with_textures`] draws every child of an authored
/// `LodGroup` (class `0x2ee`), or only the first.
///
/// **[`Self::Single`] is what the original shows, and the default.** PPSSPP
/// frames of the player's hull and of a `16_Track` grandstand `LodGroup`,
/// both well inside the authored switch distance, match `Single` detail for
/// detail; [`Self::Both`] lays the coarse tier over the fine one - a dark
/// ring round the rear hull, flat panels and seams over the grandstand's
/// grass. See `docs/formats/vex.md`, "the running original does not draw
/// tier 1 up close" (confidence 80). The static read above that section -
/// no code in the PSP binary ever selects a tier - is what `Both` was built
/// on, and the pixels contradict it for what reaches the screen.
///
/// Not a quality tier and not a live switch: `Single` is decided when the
/// model is built and keeps the higher-detail first child. Whether the
/// original swaps to the coarse tier beyond the authored switch distance is
/// not read; a real distance switch would need `Model` to carry per-node
/// bounds and the render loop to re-check them every frame.
///
/// There is no settings-file key for this since 2026-09-23 - a player gets
/// `Single`. `Both` stays reachable from `oag-game --lod both` and
/// `oag-view --lod both`, a diagnostic view of the coarse tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lod {
    /// Draw every child of every `LodGroup`, coarse tier included - a
    /// diagnostic view; the original does not draw this up close.
    Both,
    /// Draw only the first child of a two-child `LodGroup` - the
    /// higher-triangle-count tier in all ten measured cases - and skip the
    /// rest. What the original shows within the authored switch distance.
    #[default]
    Single,
}

impl Lod {
    /// The spelling used on the `--lod` command-line flag.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Single => "single",
        }
    }

    /// Every mode, for error messages.
    pub const ALL: [Self; 2] = [Self::Both, Self::Single];
}

impl std::str::FromStr for Lod {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|lod| lod.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a level-of-detail mode; try both or single"))
    }
}

impl std::fmt::Display for Lod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
