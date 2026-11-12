//! What a Pulse-style `Medal_x_y` hex is filled with: the title's flat colour swatch, or a
//! frame of a medal atlas a mod names (`[campaign-medals]`, `docs/tools/mods.md`).

use oag_tables::race_campaign::{Difficulty, Medal};

use super::hd::medal::hd_medal_frame;

/// The art a medal hex is drawn with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum MedalArt {
    /// The title's own `hex_filled` sprite, tinted gold, silver or bronze.
    #[default]
    Swatch,
    /// A frame of an atlas the sprite sheet holds under `image`.
    Atlas(Atlas),
}

/// A medal atlas: 60 px frames at a 61 px row pitch, one block of three tiers per
/// difficulty at a 183 px pitch when `per_difficulty` (`hd_medal_frame`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atlas {
    /// The sprite sheet key of the atlas.
    pub image: String,
    /// The atlas holds a block of tiers per difficulty.
    pub per_difficulty: bool,
    /// The stored row of gold, silver and bronze within a block, top to bottom.
    pub tier_rows: [u8; 3],
}

impl MedalArt {
    /// The atlas frame `[u, v, width, height]` for a medal earned on `difficulty`, or `None`
    /// for the swatch.
    #[must_use]
    pub fn frame(&self, medal: Medal, difficulty: Difficulty) -> Option<[f32; 4]> {
        let Self::Atlas(atlas) = self else {
            return None;
        };
        let difficulty = if atlas.per_difficulty {
            difficulty
        } else {
            Difficulty::Easy
        };
        Some(hd_medal_frame(medal, difficulty, atlas.tier_rows))
    }

    /// The atlas's sprite sheet key, `None` for the swatch.
    #[must_use]
    pub fn image(&self) -> Option<&str> {
        match self {
            Self::Swatch => None,
            Self::Atlas(atlas) => Some(&atlas.image),
        }
    }
}
