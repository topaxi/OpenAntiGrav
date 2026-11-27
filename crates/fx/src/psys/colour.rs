//! How a particle takes its colour from the emitter's 256-entry table, and the per-title law
//! that says which walk a file's `+0xbc` word selects.
//!
//! The parser reads every title's file the same way (forward walk unless `+0xbc == 2`); a source
//! whose executable reads the word differently says so once, at load, with
//! [`Effect::with_colour_law`] (`docs/formats/pob.md`, "The colour table is RGB-only").

use super::Effect;
use crate::rocket_smoke::UNLIT_RGB;

/// `0x437fffbe`, the index scale all three of the original's walks use.
const INDEX_SCALE: f32 = 255.99997;

/// How a particle takes its colour from the emitter's 256-entry table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourMode {
    /// `+0xbc == 2`: one random entry, drawn once at spawn.
    RandomEntry,
    /// Anything else: `palette[age * 255.999]`, walked over the particle's
    /// life (Pulse and Pure).
    OverLife,
    /// 2048 and Omega, `+0xbc` not 0: the table walked from its end, `palette[(1 - age) * 255.99997]`.
    ReverseLife,
    /// HD, `+0xbc == 3`: the table's red byte weighs entry 0 toward the scene light
    /// ([`light_blend`]).
    LightBlend,
}

impl ColourMode {
    /// What the parser makes of a file's `+0xbc` word before any title's law is applied.
    #[must_use]
    pub fn authored(word: u32) -> Self {
        if word == 2 {
            Self::RandomEntry
        } else {
            Self::OverLife
        }
    }

    /// The particle's palette colour at `age` (`0..=1`); the alpha byte comes from the channel
    /// either way.
    #[must_use]
    pub fn colour(self, palette: &[[f32; 4]; 256], age: f32, random_entry: u8) -> [f32; 4] {
        match self {
            Self::RandomEntry => palette[usize::from(random_entry)],
            // `palette[(int)(age * 255.999)]`, the original's own index.
            Self::OverLife => palette[(age * 255.999) as usize & 0xff],
            Self::ReverseLife => palette[((1.0 - age) * INDEX_SCALE) as usize & 0xff],
            Self::LightBlend => light_blend(palette, age, LIGHT),
        }
    }
}

/// The scene light `L` HD's mode 3 blends toward: `RibbonEffects_SampleLighting`'s sample, which
/// this project has not read, so the colour our lit ribbons already use.
///
/// **Chosen, not measured.** It is [`UNLIT_RGB`], the bright end of what the original's lighting
/// volume returns for the rocket's ribbon; the live word of `Psys_BuildUpdateContext`'s `ctx+0xb04`
/// is the unread value (`docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, "`colour_mode` 3 is a
/// blend toward the scene light", read recipe).
pub const LIGHT: [f32; 3] = UNLIT_RGB;

/// HD's mode 3 colour (`Psys_UpdateParticles`, `0x0028ce10`-`0x0028cf60`): per byte
/// `trunc(E0 + (L - E0) * t)` with `E0` the table's entry 0 and
/// `t = table[trunc(age * 255.99997) & 0xff].R / 255`. The table's alpha column is never read; the
/// result's alpha is entry 0's.
#[must_use]
pub fn light_blend(palette: &[[f32; 4]; 256], age: f32, light: [f32; 3]) -> [f32; 4] {
    let t = palette[(age * INDEX_SCALE) as usize & 0xff][0];
    let e0 = palette[0];
    let mut out = e0;
    for lane in 0..3 {
        let from = (e0[lane] * 255.0).round();
        let to = (light[lane] * 255.0).round();
        out[lane] = (from + (to - from) * t).trunc() / 255.0;
    }
    out
}

/// Which walk a source's `+0xbc` word selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColourLaw {
    /// Pulse and Pure: the parser's reading.
    #[default]
    Forward,
    /// 2048 and Omega: any word but 0 and 2 walks the table backwards.
    Reversed,
    /// HD: word 3 blends toward the scene light; the other words keep the forward walk.
    LightBlend,
}

impl Effect {
    /// Applies a source's colour-table law to every emitter that walks its table over life.
    pub fn with_colour_law(&mut self, law: ColourLaw) {
        for spec in &mut self.emitters {
            if spec.authored_colour_mode == 2 {
                continue;
            }
            spec.colour_mode = match (law, spec.authored_colour_mode) {
                (ColourLaw::Reversed, word) if word != 0 => ColourMode::ReverseLife,
                (ColourLaw::LightBlend, 3) => ColourMode::LightBlend,
                _ => ColourMode::OverLife,
            };
        }
    }

    /// Every emitter's table set to the one entry `rgba` (bytes), the way HD's
    /// `Psys_CopyPaletteToTree` (`0x002834f8`) copies one record's 256 words to every instance
    /// of a spawned tree (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`, "The blast's
    /// colour").
    pub fn with_one_colour(&mut self, rgba: [u8; 4]) {
        for spec in &mut self.emitters {
            let entry = rgba.map(|byte| f32::from(byte) / spec.colour_divisor);
            spec.palette.fill(entry);
        }
    }
}

/// The entry `DetonatorMine_Explode` (`0x00138e58`) writes over a live mine's blast, from the
/// stage's `Detonator Mine Colour` `[r, g, b, a]`: each of `r, g, b` taken `0.8` of the way to
/// `255` and truncated, then packed as `a << 24 | b << 16 | g << 9 | r << 1` into a word whose
/// bytes the table reads as `a, b, g, r` (`0x001390c8`-`0x00139164`). The shifts of `9` and `1`
/// are the executable's: green and red land one bit high, so red loses its top bit, green takes
/// red's, and blue takes green's (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`,
/// "The blast's colour").
#[must_use]
pub fn detonator_mine_blast_entry(mine: [u8; 4]) -> [u8; 4] {
    let toward_white = |byte: u8| -> u32 {
        let c = f32::from(byte);
        ((255.0 - c).mul_add(0.8, c) as u32).min(255)
    };
    let [r, g, b] = [mine[0], mine[1], mine[2]].map(toward_white);
    let word = u32::from(mine[3]) << 24 | b << 16 | g << 9 | r << 1;
    let [a, b, g, r] = word.to_be_bytes();
    [r, g, b, a]
}

#[cfg(test)]
mod tests;
