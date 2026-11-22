//! The colour and brightness a Detonator mine and bomb are drawn in, per object and per frame.
//!
//! View state, not `World` state: the simulation never reads a band level or a stage colour for
//! this. The original binds the constants by pointer to fields of each mine and bomb that
//! `DetonatorMine_AnimateToMusic` and `DetonatorBomb_AnimateToMusic` rewrite every update
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`); the draw takes them as
//! per-instance values of the material's named `float4`s, which here are the entry a drawable's
//! glow table holds ([`oag_mesh::mesh::Emissive`], `PROGRAM_MINE_RIM`).

use oag_tables::effectsettings::DetonatorColours;

/// A band level rising by more than this in one update jumps the envelope to it
/// (`s + 0.3 < band`).
pub(crate) const ATTACK: f32 = 0.3;
/// What the envelope keeps of itself in an update where the band did not jump.
pub(crate) const DECAY: f32 = 0.95;
/// A mine picks one of this many low bands (`Libc_Rand() % 4`) for its football.
pub(crate) const BAND_CHOICES: usize = 4;
/// Its spikes read the matching high band, `a + 11`.
pub(crate) const SPIKE_BAND_OFFSET: usize = 11;
/// `MineBrightness = 2 s`.
pub(crate) const MINE_BRIGHTNESS_GAIN: f32 = 2.0;
/// A dark mine's `MineColour` on the football, `(2, 2, 2)`.
pub(crate) const DARK_FOOTBALL: [f32; 3] = [2.0; 3];
/// A dark mine's `MineColour` on the spikes, `(10, 10, 10)`.
pub(crate) const DARK_SPIKES: [f32; 3] = [10.0; 3];
/// The bomb's flash decays by this each update.
pub(crate) const FLASH_DECAY: f32 = 0.02;

/// One step of the original's envelope: `s = (s + 0.3 < band) ? band : s * 0.95`, with `band`
/// clamped to `0..=1` first.
#[must_use]
pub(crate) fn follow(level: f32, band: f32) -> f32 {
    let band = band.clamp(0.0, 1.0);
    if level + ATTACK < band {
        band
    } else {
        level * DECAY
    }
}

/// The level of `band` in `levels`, `0` beyond them (`SoundSystem_GetBandLevel` answers `0.0`
/// for a band at or past sixteen).
#[must_use]
pub(crate) fn band_level(levels: &[f32], band: usize) -> f32 {
    levels.get(band).copied().unwrap_or(0.0)
}

/// Which low band a mine's football follows: `0..4`.
///
/// **Chosen, not measured**: the original's is `Libc_Rand() % 4` drawn once at construction,
/// which this project has no counterpart for. The survey entry and lane identify a laid mine
/// for as long as it exists, so the pick survives the pool's swap-removes.
#[must_use]
pub(crate) fn mine_band(entry: u16, lane: i8) -> usize {
    (usize::from(entry) + 3 * (lane.unsigned_abs() as usize) + usize::from(lane < 0)) % BAND_CHOICES
}

/// A mine's two envelopes: the football's `s` (`m+0xf4`) and the spikes' `t` (`m+0xf8`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct MineEnvelope {
    pub(crate) football: f32,
    pub(crate) spikes: f32,
}

impl MineEnvelope {
    /// One update against the sixteen band `levels`, for a mine on low band `band`.
    pub(crate) fn step(&mut self, levels: &[f32], band: usize) {
        self.football = follow(self.football, band_level(levels, band));
        self.spikes = follow(self.spikes, band_level(levels, band + SPIKE_BAND_OFFSET));
    }

    /// `MineBrightness`, the one value both models read: `2 s` of the football's envelope.
    #[must_use]
    pub(crate) fn brightness(&self) -> f32 {
        MINE_BRIGHTNESS_GAIN * self.football
    }
}

/// The constants of one mine part's material for a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MineConstants {
    /// `MineColour`.
    pub(crate) colour: [f32; 3],
    /// `MineBrightness`.
    pub(crate) brightness: f32,
}

/// Which model of the mine a [`MineConstants`] is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MinePart {
    Football,
    Spikes,
}

/// What a mine's part is drawn with. A live mine takes the stage's `Detonator Mine Colour` and
/// `2 s`; a dark one takes the literals `DetonatorMine_AnimateToMusic` stores whatever the stage
/// is, with brightness `0`.
#[must_use]
pub(crate) fn mine_constants(
    live: bool,
    part: MinePart,
    stage: &DetonatorColours,
    envelope: &MineEnvelope,
) -> MineConstants {
    if live {
        return MineConstants {
            colour: stage.mine,
            brightness: envelope.brightness(),
        };
    }
    MineConstants {
        colour: match part {
            MinePart::Football => DARK_FOOTBALL,
            MinePart::Spikes => DARK_SPIKES,
        },
        brightness: 0.0,
    }
}

/// What a bomb's three models are drawn with for a flash value `f`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BombConstants {
    /// `BombColour`: the stage's inner colour, on the body.
    pub(crate) inner: [f32; 3],
    /// `BombEQColour`, `Colour` and `RainbowColour`: the stage's outer colour.
    pub(crate) outer: [f32; 3],
    /// `BombBrightness`, `0.2 + 2 f`.
    pub(crate) body_brightness: f32,
    /// `BombEQBrightness`, `0.8 + 2 f`.
    pub(crate) equaliser_brightness: f32,
}

/// The bomb's constants for `flash`.
#[must_use]
pub(crate) fn bomb_constants(stage: &DetonatorColours, flash: f32) -> BombConstants {
    BombConstants {
        inner: stage.bomb_inner,
        outer: stage.bomb_outer,
        body_brightness: 0.2 + 2.0 * flash,
        equaliser_brightness: 0.8 + 2.0 * flash,
    }
}

/// The flash after one update: it falls by `0.02` while positive, held to `0..=1`.
#[must_use]
pub(crate) fn flash_after(flash: f32) -> f32 {
    (if flash > 0.0 { flash - FLASH_DECAY } else { flash }).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests;
