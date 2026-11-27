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
/// The football's size is `clamp(2.3 s, 1, 2.3)`.
pub(crate) const FOOTBALL_SCALE_GAIN: f32 = 2.3;
/// The spikes' size is `clamp(4 t, 1, football size + 0.95)`.
pub(crate) const SPIKE_SCALE_GAIN: f32 = 4.0;
/// How far past the football's size the spikes may reach.
pub(crate) const SPIKE_REACH: f32 = 0.95;
/// A dark mine's `MineColour` on the football, `(2, 2, 2)`.
pub(crate) const DARK_FOOTBALL: [f32; 3] = [2.0; 3];
/// A dark mine's `MineColour` on the spikes, `(10, 10, 10)`.
pub(crate) const DARK_SPIKES: [f32; 3] = [10.0; 3];

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

/// One laid mine as the draw sees it: its identity and whether it is live this stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LaidMine {
    pub(crate) entry: u16,
    pub(crate) lane: i8,
    pub(crate) live: bool,
}

/// Every laid mine's envelopes and the race clock it was first laid at, and the race clock they
/// were last stepped at.
///
/// **Once per tick, not once per frame**: a frame between two ticks (or a second view of the
/// same one) leaves them as they are, so the envelope's rate is the update rate whatever the
/// frame rate is - the original steps it once per mine update, a call rate not measured
/// (`detonator-mine-material.md`, "What the port needs").
#[derive(Debug, Default)]
pub(crate) struct MineEnvelopes {
    stepped_at: Option<u32>,
    mines: std::collections::BTreeMap<(u16, i8), (MineEnvelope, f32)>,
}

impl MineEnvelopes {
    /// Steps every mine of `laid` against `levels` if `seconds` is a clock not stepped yet, and
    /// drops the mines no longer laid. Returns each mine's envelope, in `laid`'s order; a mine
    /// seen for the first time is laid at `seconds`.
    pub(crate) fn advance(
        &mut self,
        laid: &[LaidMine],
        levels: &[f32],
        seconds: f32,
    ) -> Vec<MineEnvelope> {
        let step = self.stepped_at != Some(seconds.to_bits());
        self.stepped_at = Some(seconds.to_bits());
        self.mines
            .retain(|key, _| laid.iter().any(|mine| (mine.entry, mine.lane) == *key));
        laid.iter()
            .map(|mine| {
                let (envelope, _) = self
                    .mines
                    .entry((mine.entry, mine.lane))
                    .or_insert((MineEnvelope::default(), seconds));
                if step && mine.live {
                    envelope.step(levels, mine_band(mine.entry, mine.lane));
                }
                *envelope
            })
            .collect()
    }

    /// Each mine of `laid`'s age at the race clock `seconds`, in `laid`'s order: the time since
    /// [`Self::advance`] first saw it (`m+0xc0`, which `DetonatorMine_Place` zeroes and every
    /// update steps). A mine `advance` has not seen is `0`.
    #[must_use]
    pub(crate) fn ages(&self, laid: &[LaidMine], seconds: f32) -> Vec<f32> {
        laid.iter()
            .map(|mine| {
                self.mines
                    .get(&(mine.entry, mine.lane))
                    .map_or(0.0, |&(_, laid_at)| (seconds - laid_at).max(0.0))
            })
            .collect()
    }
}

/// What a laid mine's model matrices read: the football's and spikes' sizes and the mine's age.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MinePose {
    pub(crate) football: f32,
    pub(crate) spikes: f32,
    pub(crate) age: f32,
}

impl Default for MinePose {
    fn default() -> Self {
        Self {
            football: 1.0,
            spikes: 1.0,
            age: 0.0,
        }
    }
}

/// How far `model`'s farthest vertex sits from its origin. The model's own animation and a
/// laid pose only turn it about that origin, so this holds in every frame.
pub(crate) fn model_reach(model: &oag_mesh::mesh::Model) -> f32 {
    model
        .vertices
        .iter()
        .map(|vertex| oag_core::math::Vec3::from_array(vertex.position).length())
        .fold(0.0, f32::max)
}

/// The size of a Detonator mine its layout holds still: the one at which a model reaching
/// `reach` from its origin ends on the Mine's `trigger_radius` sphere, the distance a craft
/// sets it off at. `1` where either is not known (`docs/gameplay/detonator-pulse.md`,
/// "Mine size").
pub(crate) fn held_mine_size(reach: Option<f32>, trigger_radius: Option<f32>) -> f32 {
    match (reach, trigger_radius) {
        (Some(reach), Some(radius)) if reach > 0.0 => radius / reach,
        _ => 1.0,
    }
}

/// The angle a mine `age` seconds old is turned by at `rate` radians a second, wrapped to one
/// turn: `DetonatorMine_AnimateToMusic`'s `phase = wrap(4 age)` (`m+0x150`).
#[must_use]
pub(crate) fn spin_phase(rate: f32, age: f32) -> f32 {
    (rate * age).rem_euclid(std::f32::consts::TAU)
}

/// A mine's basis: the world axes turned about `up` by `phase`, not a frame built on `up`.
///
/// The original writes all three rows from scratch, row 0 `(x^2 (1-c) + c, xy (1-c) - z s,
/// xz (1-c) + y s)` and so on for the axis `(x, y, z)` (`0x006ca538`), and its rows are the
/// model's axes, so each row is a column here: about `+Y`, the model's X axis goes to
/// `(c, 0, s)`, a turn of `-phase` in this crate's right-handed sense
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-draw.md`, "The spin").
#[must_use]
pub(crate) fn spun_basis(up: oag_core::math::Vec3, phase: f32) -> oag_core::math::Mat3 {
    let up = up.normalize_or_zero();
    let up = if up == oag_core::math::Vec3::ZERO {
        oag_core::math::Vec3::Y
    } else {
        up
    };
    oag_core::math::Mat3::from_axis_angle(up, -phase)
}

/// A mine's two envelopes, the football's `s` (`m+0xf4`) and the spikes' `t` (`m+0xf8`), and its
/// visualiser's `PitchOffset` (`m+0x108`) and `(MasterVolume, PitchVolume)` (`m+0x104`, `m+0x10c`),
/// which only a comparison pin sets.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct MineEnvelope {
    pub(crate) football: f32,
    pub(crate) spikes: f32,
    pub(crate) pitch_offset: f32,
    pub(crate) volumes: Option<(f32, f32)>,
}

impl MineEnvelope {
    /// One update against the sixteen band `levels`, for a mine on low band `band`.
    pub(crate) fn step(&mut self, levels: &[f32], band: usize) {
        self.football = follow(self.football, band_level(levels, band));
        self.spikes = follow(self.spikes, band_level(levels, band + SPIKE_BAND_OFFSET));
    }

    /// The football's size, `clamp(2.3 s, 1, 2.3)` (`m+0xd8`).
    #[must_use]
    pub(crate) fn football_scale(&self) -> f32 {
        (self.football * FOOTBALL_SCALE_GAIN).clamp(1.0, FOOTBALL_SCALE_GAIN)
    }

    /// The spikes' size, `clamp(4 t, 1, football size + 0.95)`.
    #[must_use]
    pub(crate) fn spike_scale(&self) -> f32 {
        (self.spikes * SPIKE_SCALE_GAIN).clamp(1.0, self.football_scale() + SPIKE_REACH)
    }

    /// `MineBrightness`, the one value both models read: `2 s` of the football's envelope.
    #[must_use]
    pub(crate) fn brightness(&self) -> f32 {
        MINE_BRIGHTNESS_GAIN * self.football
    }
}

/// A laid mine a comparison holds at the original's read.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MinePin {
    /// The football's `s`, the spikes' `t` and the visualiser's `PitchOffset`
    /// (`--detonator-mine-envelopes`); held on a live mine only.
    pub envelope: Option<(f32, f32, f32)>,
    /// The mine's age, seconds (`m+0xc0`, `--detonator-mine-ages`); held live or dark.
    pub age: Option<f32>,
    /// The visualiser's `MasterVolume` and `PitchVolume` (`m+0x104`, `m+0x10c`,
    /// `--detonator-mine-volumes`); held on a live mine only.
    pub volumes: Option<(f32, f32)>,
}

/// Pinned mines by survey entry and lane.
pub(crate) type MinePins = std::collections::BTreeMap<(u16, i8), MinePin>;

/// Replaces the envelope of every live mine in `pins` (by survey entry and lane) with the pinned
/// football `s`, spikes `t` and `PitchOffset`, and the age of every pinned mine with its pinned
/// age: a comparison holds a mine at the original's read.
pub(crate) fn hold_pinned(
    laid: &[LaidMine],
    envelopes: &mut [MineEnvelope],
    ages: &mut [f32],
    pins: &MinePins,
) {
    for ((mine, envelope), age) in laid.iter().zip(envelopes).zip(ages) {
        let Some(pin) = pins.get(&(mine.entry, mine.lane)) else {
            continue;
        };
        if let Some((football, spikes, pitch_offset)) = pin.envelope.filter(|_| mine.live) {
            *envelope = MineEnvelope {
                football,
                spikes,
                pitch_offset,
                volumes: envelope.volumes,
            };
        }
        if let Some(volumes) = pin.volumes.filter(|_| mine.live) {
            envelope.volumes = Some(volumes);
        }
        if let Some(pinned) = pin.age {
            *age = pinned;
        }
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

/// `ElectricityColour` is the stage's colour times this while a mine is live
/// (`DetonatorMine_AnimateToMusic`, `K[0x14]`).
pub(crate) const ELECTRICITY_GAIN: f32 = 1.5;
/// A dark mine's `ElectricityColour`, `(2, 2, 2)` (`K[0x54]`).
pub(crate) const DARK_ELECTRICITY: [f32; 3] = [2.0; 3];
/// `BassHit = clamp(0.25 s, 0, 0.25)`.
pub(crate) const BASS_HIT_GAIN: f32 = 0.25;
/// `MasterVolume = clamp01(5 L)` (`K[0x10]`).
pub(crate) const MASTER_VOLUME_GAIN: f32 = 5.0;
/// `PitchVolume = clamp(1.0 L, 0, 0.4)` (`K[0x48]`, `K[0x4c]`).
pub(crate) const PITCH_VOLUME_CEILING: f32 = 0.4;

/// The values the visualiser's `electricity` material is drawn with for a frame
/// (`oag_mesh::mesh::rcs::electricity`): the mine's `ElectricityColour`, `MasterVolume`,
/// `PitchVolume`, `PitchOffset`, `BassHit` and `TrebleHit`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct VisualiserConstants {
    pub(crate) colour: [f32; 3],
    pub(crate) master_volume: f32,
    pub(crate) pitch_volume: f32,
    pub(crate) pitch_offset: f32,
    pub(crate) bass_hit: f32,
    pub(crate) treble_hit: f32,
}

/// The visualiser's constants: a live mine takes the stage's electricity colour times `1.5` and
/// the music's levels, a dark one the literals `DetonatorMine_AnimateToMusic` stores
/// (`detonator-mine-material.md`).
///
/// `MasterVolume` and `PitchVolume` are both the one whole-spectrum `level`, `clamp01(5 L)` and
/// `clamp(L, 0, 0.4)`, which `0x00304cc8` returns for both (`detonator-mine-material.md`, "The
/// visualiser's level"); a comparison pin replaces the pair. `PitchOffset` is the envelope's: an
/// accumulator whose step is unread, so only a comparison pin moves it from `0`. `TrebleHit` (a
/// `.bss` word whose writer is unread) stays `0`, the value every live read showed
/// (`detonator-mine-draw.md`, "The envelope at the picture's tick").
#[must_use]
pub(crate) fn visualiser_constants(
    live: bool,
    stage: &DetonatorColours,
    envelope: &MineEnvelope,
    level: f32,
) -> VisualiserConstants {
    if !live {
        return VisualiserConstants {
            colour: DARK_ELECTRICITY,
            master_volume: 0.0,
            pitch_volume: 0.0,
            pitch_offset: 0.0,
            bass_hit: 0.0,
            treble_hit: 0.0,
        };
    }
    let (master_volume, pitch_volume) = envelope
        .volumes
        .unwrap_or_else(|| visualiser_volumes(level));
    VisualiserConstants {
        colour: stage.mine_electricity.map(|c| c * ELECTRICITY_GAIN),
        master_volume,
        pitch_volume,
        pitch_offset: envelope.pitch_offset,
        bass_hit: (envelope.football * BASS_HIT_GAIN).clamp(0.0, BASS_HIT_GAIN),
        treble_hit: 0.0,
    }
}

/// `(MasterVolume, PitchVolume)` for the whole-spectrum `level` `L`: `clamp01(5 L)` and
/// `clamp(L, 0, 0.4)`, both off the one reader (`DetonatorMine_AnimateToMusic`, `0x00139774` and
/// `0x00139ad4`).
#[must_use]
pub(crate) fn visualiser_volumes(level: f32) -> (f32, f32) {
    (
        (MASTER_VOLUME_GAIN * level).clamp(0.0, 1.0),
        level.clamp(0.0, PITCH_VOLUME_CEILING),
    )
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

/// The bomb's constants for `flash`, the value `b+0x154` holds. Its writer is unread, so the draw
/// passes `0`: a bomb at rest, brightness `0.2` and `0.8`.
#[must_use]
pub(crate) fn bomb_constants(stage: &DetonatorColours, flash: f32) -> BombConstants {
    BombConstants {
        inner: stage.bomb_inner,
        outer: stage.bomb_outer,
        body_brightness: 0.2 + 2.0 * flash,
        equaliser_brightness: 0.8 + 2.0 * flash,
    }
}

#[cfg(test)]
mod tests;
