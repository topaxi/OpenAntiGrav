//! What a Pulse-style `Medal_x_y` hex is filled with: the title's flat colour swatch, or a
//! frame of a medal atlas a mod names (`[campaign-medals]`, `docs/tools/mods.md`).

use oag_tables::race_campaign::{Difficulty, Medal, MedalTiers};

use super::hd::medal::{HD_FRAME, HD_ROW_PITCH, atlas_frame};

/// The art a medal hex is drawn with.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum MedalArt {
    /// The title's own `hex_filled` sprite, tinted gold, silver or bronze.
    #[default]
    Swatch,
    /// A frame of an atlas the sprite sheet holds under `image`.
    Atlas(Atlas),
}

/// A medal atlas: a column of frames, `rows` of them to a block, one block per difficulty when
/// `per_difficulty` (a title's atlas: 60 px frames at a 61 px pitch, [`Self::of_title`]; a
/// mod's: [`Self::of_mod`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Atlas {
    /// The sprite sheet key of the atlas.
    pub image: String,
    /// The atlas holds a block of tiers per difficulty.
    pub per_difficulty: bool,
    /// The tier each row of a block holds, top to bottom, by the names a campaign's tiers
    /// carry. A campaign tier no row names draws its colour as a swatch.
    pub rows: Vec<String>,
    /// A frame's size in the atlas's own pixels.
    pub frame: [f32; 2],
    /// Pixels from one frame's top to the next.
    pub row_pitch: f32,
}

impl Atlas {
    /// A title's atlas: gold, silver and bronze in the stored rows `tier_rows` gives
    /// (`oag_title::Campaign::medal_rows`), three to a block.
    #[must_use]
    pub fn of_title(image: String, per_difficulty: bool, tier_rows: [u8; 3]) -> Self {
        let mut rows = vec![String::new(); 3];
        for (name, row) in ["gold", "silver", "bronze"].into_iter().zip(tier_rows) {
            if let Some(slot) = rows.get_mut(usize::from(row)) {
                *slot = name.to_string();
            }
        }
        Self {
            image,
            per_difficulty,
            rows,
            frame: [HD_FRAME, HD_FRAME],
            row_pitch: HD_ROW_PITCH,
        }
    }

    /// A mod's atlas of `width` x `height` pixels: `rows` names the tier of each row of a
    /// block, top to bottom, and the image is cut into equal frames, `rows.len()` to a block
    /// and three blocks (easy, medium, hard from the top) when `per_difficulty`.
    ///
    /// # Errors
    ///
    /// Why the image cannot be cut: no rows, or a height that is not a whole number of frames.
    pub fn of_mod(
        image: String,
        per_difficulty: bool,
        rows: Vec<String>,
        (width, height): (u32, u32),
    ) -> Result<Self, String> {
        let frames = rows.len() as u32 * if per_difficulty { 3 } else { 1 };
        if frames == 0 || height == 0 || height % frames != 0 {
            return Err(format!(
                "{width}x{height} px is not {frames} equal rows ({} tiers{})",
                rows.len(),
                if per_difficulty {
                    " in each of three difficulty blocks"
                } else {
                    ""
                }
            ));
        }
        let frame_height = (height / frames) as f32;
        Ok(Self {
            image,
            per_difficulty,
            rows,
            frame: [width as f32, frame_height],
            row_pitch: frame_height,
        })
    }
}

impl MedalArt {
    /// The atlas frame `[u, v, width, height]` for a medal earned on `difficulty`, or `None`
    /// for the swatch: the art is off, or the atlas has no row for the tier's name, so the
    /// tier draws its own colour.
    #[must_use]
    pub fn frame(
        &self,
        tiers: &MedalTiers,
        medal: Medal,
        difficulty: Difficulty,
    ) -> Option<[f32; 4]> {
        let Self::Atlas(atlas) = self else {
            return None;
        };
        let name = &tiers.get(medal)?.name;
        let row = atlas.rows.iter().position(|row| row == name)?;
        let difficulty = if atlas.per_difficulty {
            difficulty
        } else {
            Difficulty::Easy
        };
        Some(atlas_frame(
            difficulty,
            atlas.rows.len(),
            row,
            atlas.frame,
            atlas.row_pitch,
        ))
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
