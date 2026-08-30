//! `.effectSettings`/`.effectsettings`: the per-stage colour-grade table Zone
//! (and Detonator) read to reskin the same environment stage by stage.
//!
//! # Why this matters
//!
//! **2048 has no dedicated Zone circuit at all - it races Zone on whatever
//! circuit the player picked** (`docs/gameplay/race-modes.md#zone`), where
//! HD/Fury's own Zone races one of four ported `/data/environments/zone_N/`
//! circuits. Neither fact explains this file: even on a *dedicated* Zone
//! circuit, the look still has to escalate as the race goes on - the disc's
//! own `MSC_EVENT_ZONE` text says the top speed rises every ten seconds, and
//! this table is the palette that rises with it. One full colour palette per
//! Zone stage (`Start`, `Sub Flash`, ..., `Supersonic`), keyed by the zone
//! number the race is on, layered over whichever circuit is racing rather
//! than replacing its base environment.
//!
//! # The format is `.envsettings`'s tokeniser, with a stage prefix on the key
//!
//! Same syntax [`envsettings`] already parses - one quoted, dotted
//! `"key"=floats` per line, no sections, no escaping:
//!
//! ```text
//! "0 Start.Lighting.Sun colour"=1.000000 1.000000 1.000000 0.000000
//! "Zone 0 Start.Colour 1.Colour"=1.000000 1.000000 1.000000
//! ```
//!
//! so [`EffectSettings::parse`] reads the whole file through
//! [`EnvSettings::parse`] rather than a second tokeniser. What is new is that
//! most keys carry a **stage prefix** before the first `.` - HD writes
//! `"<n> <Name>"`, 2048 writes `"Zone <n> <Name>"` - and a handful of
//! title-wide keys (2048's `"Sky Radius"`, HD's `"Texture U scale"`) carry
//! none at all and read straight off [`EffectSettings::table`].
//!
//! # Confidence
//!
//! **85** on the syntax and the stage-prefix shape: all five files measured -
//! HD's `zonemode.effectsettings`, `zonemodedlc3.effectsettings`,
//! `detonatormode.effectsettings`, `detonatormodedlc3.effectsettings` and
//! 2048's ten identical `ZoneMode2048.effectSettings` copies - parse in full,
//! and every stage name in each recovers correctly (checked in
//! `effectsettings_ground_truth.rs`). Not 90, unlike `.envsettings`: **the
//! stage-index-to-zone-number correspondence is inferred from the names
//! alone**, not checked against `Zone_Update`'s own timer - see
//! `docs/formats/effectsettings.md`'s `## Open`. Nothing here reads either
//! title's executable.
//!
//! [`cross_fade_rgba8`] is the one piece of this module that *does* read
//! the executable - HD/Fury's own runtime cross-fade between adjacent
//! stages, recovered from two independent functions (confidence 80, see
//! its own doc comment). [`StagePalette`] is what it blends, and
//! `oag_game::race::zone_grade` is what applies the result to a race - from
//! an explicit stage index, because **what selects the stage during a race
//! is still unrecovered on both titles that ship one of these tables**.

use std::collections::BTreeMap;

use crate::envsettings::{self, EnvSettings};

/// One stage's own identity, as the file names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    /// The zone number, as the key's own leading integer - `0` for `Start`.
    pub index: u32,
    /// The stage's authored name (`"Start"`, `"Sub Venom"`, `"Supersonic"`).
    pub name: String,
    /// The exact text before the first `.` of every key this stage owns -
    /// `"0 Start"` on HD, `"Zone 0 Start"` on 2048. Kept whole rather than
    /// reconstructed from `index`/`name`, so [`EffectSettings::stage_key`]
    /// never has to guess which title's spelling to rebuild.
    prefix: String,
}

/// The zone index and stage name a key's own leading segment names, or
/// `None` for a key with no stage prefix at all (a title-wide key like
/// `"Sky Radius"`, or a plain group name like `"Lighting"` with no leading
/// number).
fn parse_stage_head(head: &str) -> Option<(u32, &str)> {
    let head = head.strip_prefix("Zone ").unwrap_or(head);
    let (number, name) = head.split_once(' ')?;
    let index: u32 = number.parse().ok()?;
    Some((index, name))
}

/// Everything one `.effectSettings`/`.effectsettings` file says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectSettings {
    /// Every entry, exactly as [`EnvSettings::parse`] read it - stage prefix
    /// and all, still part of the key. Title-wide keys with no stage prefix
    /// read straight off this.
    pub table: EnvSettings,
    /// Every stage this file names, keyed by its zone number.
    ///
    /// A `BTreeMap` for the same determinism reason [`EnvSettings::entries`]
    /// is one - see `docs/architecture/determinism.md`.
    pub stages: BTreeMap<u32, Stage>,
}

impl EffectSettings {
    /// Reads a whole file.
    ///
    /// # Errors
    ///
    /// [`envsettings::Error`] for the first line that is neither blank nor
    /// `key=value` - the same failure [`EnvSettings::parse`] reports, since
    /// this is that parser plus a stage index read off the key.
    pub fn parse(text: &str) -> Result<Self, envsettings::Error> {
        let table = EnvSettings::parse(text)?;
        let mut stages: BTreeMap<u32, Stage> = BTreeMap::new();
        for key in table.entries.keys() {
            let Some((head, _rest)) = key.split_once('.') else {
                continue;
            };
            let Some((index, name)) = parse_stage_head(head) else {
                continue;
            };
            stages.entry(index).or_insert_with(|| Stage {
                index,
                name: name.to_string(),
                prefix: head.to_string(),
            });
        }
        Ok(Self { table, stages })
    }

    /// The full key one stage's own `key` reads under, or `None` for a stage
    /// this file does not name.
    #[must_use]
    pub fn stage_key(&self, stage: u32, key: &str) -> Option<String> {
        let prefix = &self.stages.get(&stage)?.prefix;
        Some(format!("{prefix}.{key}"))
    }

    /// One number, for a per-stage key that carries exactly one.
    #[must_use]
    pub fn stage_scalar(&self, stage: u32, key: &str) -> Option<f32> {
        self.table.scalar(&self.stage_key(stage, key)?)
    }

    /// Three numbers, for a per-stage key that carries exactly three.
    ///
    /// Not clamped or normalised, the same as [`EnvSettings::vec3`] - several
    /// of these exceed `1.0` (2048's `"Edge Colour"` reaches 1.098).
    #[must_use]
    pub fn stage_vec3(&self, stage: u32, key: &str) -> Option<[f32; 3]> {
        self.table.vec3(&self.stage_key(stage, key)?)
    }

    /// Four numbers, for a per-stage key that carries exactly four.
    ///
    /// [`EnvSettings::rgba8`] cannot answer these: HD's four-number colour
    /// keys (`"Lighting.Sun colour"`, `"Lighting.Fog colour"`, ...) are
    /// written with decimal points, not the byte quadruple
    /// [`envsettings::SKY_COLOUR`] is.
    #[must_use]
    pub fn stage_vec4(&self, stage: u32, key: &str) -> Option<[f32; 4]> {
        let key = self.stage_key(stage, key)?;
        match self.table.entries.get(&key)?.numbers.as_slice() {
            [x, y, z, w] => Some([*x, *y, *z, *w]),
            _ => None,
        }
    }

    /// Four bytes, for a per-stage key the file writes as integers.
    ///
    /// [`key::SKY_REFLECTION_COLOUR`] and the two `EQ ... tint` keys are the
    /// only per-stage colours HD writes this way; every other one carries
    /// decimal points and answers [`Self::stage_vec4`] instead.
    #[must_use]
    pub fn stage_rgba8(&self, stage: u32, key: &str) -> Option<[u8; 4]> {
        self.table.rgba8(&self.stage_key(stage, key)?)
    }

    /// A colour key's red, green and blue, whether it is written with three
    /// numbers or four.
    ///
    /// **The fourth lane is dropped rather than carried**: every four-number
    /// colour key in `zonemode.effectsettings` authors `0.000000` there, on
    /// all fifteen stages, and no consumer this project has read takes a
    /// fourth channel from one. 2048's own two fog keys are the exception -
    /// they carry a density there rather than an alpha - and they are read
    /// through [`Self::stage_fourth`] instead of by widening this.
    #[must_use]
    pub fn stage_rgb(&self, stage: u32, key: &str) -> Option<[f32; 3]> {
        let key = self.stage_key(stage, key)?;
        match self.table.entries.get(&key)?.numbers.as_slice() {
            [r, g, b] | [r, g, b, _] => Some([*r, *g, *b]),
            _ => None,
        }
    }

    /// The fourth number of a four-number per-stage key, and `None` for a key
    /// written with any other count.
    ///
    /// Exists for [`key::ENVIRONMENT_FOG_COLOUR_2048`], where that lane is the
    /// fog density this file has no separate key for - see that key's own
    /// evidence. Deliberately narrow: it answers `None` for the three-number
    /// keys rather than substituting anything.
    #[must_use]
    pub fn stage_fourth(&self, stage: u32, key: &str) -> Option<f32> {
        let key = self.stage_key(stage, key)?;
        match self.table.entries.get(&key)?.numbers.as_slice() {
            [_, _, _, fourth] => Some(*fourth),
            _ => None,
        }
    }

    /// One stage's [`StagePalette`], or `None` for a stage this file does not
    /// name at all.
    ///
    /// A named stage that authors none of these keys answers
    /// `Some(StagePalette::default())` - every field `None` - which is a
    /// different statement from an absent stage and is kept distinguishable
    /// for that reason.
    #[must_use]
    pub fn stage_palette(&self, stage: u32) -> Option<StagePalette> {
        self.stages.get(&stage)?;
        Some(StagePalette {
            // HD's spelling first, then 2048's. A file carries one vocabulary
            // or the other, never both, so the order only decides which miss
            // costs a second lookup.
            fog_colour: self
                .stage_rgb(stage, key::FOG_COLOUR)
                .or_else(|| self.stage_rgb(stage, key::ENVIRONMENT_FOG_COLOUR_2048)),
            fog_density: self
                .stage_scalar(stage, key::FOG_DENSITY)
                .or_else(|| self.stage_fourth(stage, key::ENVIRONMENT_FOG_COLOUR_2048)),
            sun_colour: self.stage_rgb(stage, key::SUN_COLOUR),
            ambient_colour: self.stage_rgb(stage, key::AMBIENT_COLOUR),
            prelit_scale: self.stage_rgb(stage, key::PRELIT_SCALE),
            prelit_power: self.stage_rgb(stage, key::PRELIT_POWER),
            sky_reflection_colour: self.stage_rgba8(stage, key::SKY_REFLECTION_COLOUR),
            sky_horizon_colour: self
                .stage_rgb(stage, key::SKY_HORIZON_COLOUR)
                .or_else(|| self.stage_rgb(stage, key::SKY_HORIZON_COLOUR_2048)),
            sky_zenith_colour: self
                .stage_rgb(stage, key::SKY_ZENITH_COLOUR)
                .or_else(|| self.stage_rgb(stage, key::SKY_ZENITH_COLOUR_2048)),
        })
    }

    /// The palette that applies at `stage`, cross-faded against the stage
    /// before it exactly as HD/Fury's own runtime pairs them.
    ///
    /// **`stage - 1`, clamped at zero, is the recovered pairing** - both
    /// functions that compute this blend index the table by the entity's own
    /// stage field and by that value minus one, saturating rather than
    /// wrapping (`ps3-hdfury-eu`, `FUN_003ce2c0` and
    /// `Environment_UpdateStageBlend` at `0x003da540`, confidence 80). At
    /// stage `0` the two indices are the same stage, so the blend is the
    /// identity whatever `weight` reads.
    ///
    /// `None` when the file does not name `stage`. A stage whose predecessor
    /// the file skips blends against itself rather than against a gap.
    #[must_use]
    pub fn blended_palette(&self, stage: u32, weight: f32) -> Option<StagePalette> {
        let current = self.stage_palette(stage)?;
        let previous = self
            .stage_palette(stage.saturating_sub(1))
            .unwrap_or(current);
        Some(current.cross_fade(previous, weight))
    }
}

/// Cross-fades two stages' packed-byte colour channels, byte for byte, the
/// way HD/Fury's own runtime does.
///
/// `weight` is the current stage's own share of the blend (`1.0` at the
/// moment a stage becomes current, falling toward `0.0` as the previous
/// stage's own share, `1.0 - weight`, takes over) - expected in `0.0..=1.0`;
/// values outside that range are not clamped going in, matching the traced
/// code, which never checks the lower bound either.
///
/// # Evidence
///
/// Recovered independently from two functions in HD/Fury's executable that
/// compute this exact arithmetic against the same `0x250`-stride runtime
/// stage table - `FUN_003da540` and `FUN_003ce2c0`, both `ps3-hdfury-eu`,
/// confidence 80 (real, exercised mechanism; corroborated twice rather than
/// found once). Per channel: `(int)(current * weight) + (int)(previous *
/// other_weight)`, each term truncated toward zero before the two are
/// summed, then clamped to `0xff` on the high side only - not a single
/// truncate-after-sum. See
/// [zone-effectsettings-loader.md](../../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-a-fourth-pass-who-writes-0x008b7944--n0x38---one-near-repeat-of-the-opd-trap-caught-before-it-shipped-one-real-correction-one-new-lead).
///
/// # What this does not settle
///
/// **Nothing calls this yet.** Which parsed `.effectSettings` key feeds
/// which channel, which stage counts as "current" at a given moment, and
/// what `weight` should read at runtime are all still unrecovered - see the
/// thread file's `## Open`
/// (`handover/2048-and-hd-ship-an-unread-effectsettings-table.md`). This is
/// the confirmed blend arithmetic alone, ready to reuse once those are
/// found - implementing it ahead of its trigger is not the same as wiring
/// it into a race.
#[must_use]
pub fn cross_fade_rgba8(current: [u8; 4], previous: [u8; 4], weight: f32) -> [u8; 4] {
    let other_weight = 1.0 - weight;
    let mut out = [0u8; 4];
    for i in 0..4 {
        let from_current = (f32::from(current[i]) * weight) as i32;
        let from_previous = (f32::from(previous[i]) * other_weight) as i32;
        out[i] = (from_current + from_previous).clamp(0, 255) as u8;
    }
    out
}

/// The per-stage keys this project reads, spelled the way HD/Fury's own
/// executable spells them.
///
/// **Not a guess at the vocabulary**: every name below appears verbatim in
/// `g_EffectSettingsSchemaKeyNames` (`0x008b79cc`, `ps3-hdfury-eu`), the
/// 73-entry schema table `FwKeyedText_ParseEntry` looks a parsed key up
/// against - see
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`. The
/// British spellings are the schema's own; `.envsettings`' American
/// [`envsettings::SUN_COLOUR`] belongs to a different table's vocabulary and
/// is not a second spelling this one also accepts.
pub mod key {
    /// The stage's own distance fog colour. Four floats, alpha authored `0`.
    pub const FOG_COLOUR: &str = "Lighting.Fog colour";
    /// The coefficient of the same `exp(-(density * depth)^2)` curve
    /// `.envsettings`' `Fog.Fog Density` feeds.
    pub const FOG_DENSITY: &str = "Lighting.Fog density";
    /// Four floats, alpha authored `0`. **No `Sun direction` key exists** in
    /// this schema, which is why a stage can only tint a rig, never aim one.
    pub const SUN_COLOUR: &str = "Lighting.Sun colour";
    /// Four floats, alpha authored `0`.
    pub const AMBIENT_COLOUR: &str = "Lighting.Constant Ambient Colour";
    /// Four floats, alpha authored `0`.
    pub const PRELIT_SCALE: &str = "Lighting.Prelit Colour Scale";
    /// Three floats.
    pub const PRELIT_POWER: &str = "Lighting.Prelit Colour Power";
    /// **The one colour key on this list the file writes as bytes**, which is
    /// what makes it the one [`super::cross_fade_rgba8`] can blend in its own
    /// recovered domain.
    pub const SKY_REFLECTION_COLOUR: &str = "Lighting.Sky reflection colour";
    /// Three floats. Parsed and reported; nothing draws it yet - this engine's
    /// HD sky is the `sky.gtf` cubemap, which has no horizon/zenith term to
    /// feed.
    pub const SKY_HORIZON_COLOUR: &str = "Sky horizon colour";
    /// Three floats, on the same terms as [`SKY_HORIZON_COLOUR`].
    pub const SKY_ZENITH_COLOUR: &str = "Sky zenith colour";

    /// Wipeout 2048's own spelling of the environment's distance fog, and the
    /// one key on this list that is **not** in HD's schema table.
    ///
    /// 2048 reauthored the vocabulary: its per-stage blocks carry
    /// `Fog.Environment Fog Colour` and `Fog.Track Fog Colour` and **no
    /// density key at all** - `grep -ci density` over a shipped
    /// `ZoneMode2048.effectSettings` returns `0`, checked directly. Both fog
    /// keys are written with **four** numbers where every other colour key in
    /// the file is written with three, and the fourth reads `0.0025`/`0.0015`
    /// on the environment block and a flat `0.007` on the track block - the
    /// same magnitude as HD's own [`FOG_DENSITY`] (`0.0021`) and
    /// `.envsettings`' `Fog.Fog Density`, and nothing like an alpha (HD's own
    /// four-number fog colour authors `0.000000` there on all fifteen stages).
    /// So the fourth lane is this file's density. **Confidence 74**: three
    /// converging reads - the missing key, the odd arity, the magnitude - and
    /// no traced consumer in 2048's executable.
    pub const ENVIRONMENT_FOG_COLOUR_2048: &str = "Fog.Environment Fog Colour";
    /// 2048's second fog block, on the same four-number terms. **Read, not
    /// drawn**: what selects between the environment and the track block is
    /// unrecovered, so the primary pair is used, exactly as `.envsettings`'
    /// own reader uses its primary pair and ignores its alternate.
    pub const TRACK_FOG_COLOUR_2048: &str = "Fog.Track Fog Colour";
    /// 2048's spelling of [`SKY_HORIZON_COLOUR`]. Read, not drawn.
    pub const SKY_HORIZON_COLOUR_2048: &str = "Sky.Horizon Colour";
    /// 2048's spelling of [`SKY_ZENITH_COLOUR`]. Read, not drawn.
    pub const SKY_ZENITH_COLOUR_2048: &str = "Sky.Zenith Colour";
}

/// One stage's own palette: the subset of its keys that has somewhere to go.
///
/// **Every field is `Option`** and nothing substitutes for an absent key -
/// `zonemode.effectsettings` is the smallest of HD's four files and exercises
/// only part of the schema, so "this stage does not author that" is an
/// ordinary answer rather than a parse failure.
///
/// Colours are passed through exactly as authored, **unclamped**: the corpus
/// writes values well past `1.0` (`Track.Texture Colour` reaches `9.0`), and
/// clamping here would hide that from a caller that has to decide what to do
/// about it - the same reason [`EnvSettings::vec3`] does not clamp either.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StagePalette {
    /// [`key::FOG_COLOUR`], alpha dropped.
    pub fog_colour: Option<[f32; 3]>,
    /// [`key::FOG_DENSITY`].
    pub fog_density: Option<f32>,
    /// [`key::SUN_COLOUR`], alpha dropped.
    pub sun_colour: Option<[f32; 3]>,
    /// [`key::AMBIENT_COLOUR`], alpha dropped.
    pub ambient_colour: Option<[f32; 3]>,
    /// [`key::PRELIT_SCALE`], alpha dropped.
    pub prelit_scale: Option<[f32; 3]>,
    /// [`key::PRELIT_POWER`].
    pub prelit_power: Option<[f32; 3]>,
    /// [`key::SKY_REFLECTION_COLOUR`], the one byte-written colour here.
    pub sky_reflection_colour: Option<[u8; 4]>,
    /// [`key::SKY_HORIZON_COLOUR`]. Read, not drawn - see the key's own note.
    pub sky_horizon_colour: Option<[f32; 3]>,
    /// [`key::SKY_ZENITH_COLOUR`]. Read, not drawn.
    pub sky_zenith_colour: Option<[f32; 3]>,
}

/// One float channel of the blend, in the float domain the key is authored in.
///
/// **The weighting is the recovered one; the domain is the key's own.**
/// [`cross_fade_rgba8`] is the arithmetic HD/Fury performs on its *packed
/// byte* fields, truncation and `0xff` clamp included. Running a key the file
/// writes as `1.800000` through that would send everything past `1.0` to
/// white - three of `zonemode.effectsettings`' own float colours already do
/// exceed `1.0` - so the quantisation stays where the storage it belongs to
/// is, and a float-authored key is blended by the same weights without it.
/// Which of the two a given runtime field used is not recovered: the three
/// 16-byte fields HD's own cross-fade reads are unidentified, and the schema's
/// destination mapping has not been read at that granularity.
fn fade_scalar(current: f32, previous: f32, weight: f32) -> f32 {
    // Written as two multiplies and an add rather than a `mul_add`, per
    // `docs/architecture/determinism.md`.
    current * weight + previous * (1.0 - weight)
}

/// One field of the blend, in the two-stage `Option` shape the palettes have.
///
/// A field only *blends* where both stages author it. Where the current stage
/// authors one and the previous does not, the current stage's own value
/// stands - there is nothing to fade from, and inventing a zero to fade from
/// would put a value in the picture that no stage wrote. Where the current
/// stage authors none, the result is `None` whatever the previous holds:
/// which stage a key is missing from is the file's own statement.
fn fade_field<T: Copy>(
    current: Option<T>,
    previous: Option<T>,
    blend: impl Fn(T, T) -> T,
) -> Option<T> {
    match (current, previous) {
        (Some(current), Some(previous)) => Some(blend(current, previous)),
        (Some(current), None) => Some(current),
        (None, _) => None,
    }
}

impl StagePalette {
    /// Cross-fades this stage against the one before it.
    ///
    /// `weight` is **this** palette's own share, matching
    /// [`cross_fade_rgba8`]'s own convention - `1.0` is this stage alone,
    /// `0.0` is `previous` alone.
    ///
    /// # What is recovered and what is not
    ///
    /// - **Recovered (confidence 80)**: that HD/Fury cross-fades stage `n`
    ///   against stage `n - 1` by a weight, and the byte arithmetic it does it
    ///   with - see [`cross_fade_rgba8`].
    /// - **Not recovered**: which of the schema's keys the three blended
    ///   runtime fields are, and what drives `weight` during a race. The
    ///   fields blended here are chosen by the file's own key names, not by a
    ///   read of the executable's destination offsets.
    #[must_use]
    pub fn cross_fade(self, previous: Self, weight: f32) -> Self {
        let fade3 =
            |c: [f32; 3], p: [f32; 3]| std::array::from_fn(|i| fade_scalar(c[i], p[i], weight));
        Self {
            fog_colour: fade_field(self.fog_colour, previous.fog_colour, fade3),
            fog_density: fade_field(self.fog_density, previous.fog_density, |c, p| {
                fade_scalar(c, p, weight)
            }),
            sun_colour: fade_field(self.sun_colour, previous.sun_colour, fade3),
            ambient_colour: fade_field(self.ambient_colour, previous.ambient_colour, fade3),
            prelit_scale: fade_field(self.prelit_scale, previous.prelit_scale, fade3),
            prelit_power: fade_field(self.prelit_power, previous.prelit_power, fade3),
            // The one field whose storage is bytes, so the one field the
            // recovered byte arithmetic applies to unaltered.
            sky_reflection_colour: fade_field(
                self.sky_reflection_colour,
                previous.sky_reflection_colour,
                |c, p| cross_fade_rgba8(c, p, weight),
            ),
            sky_horizon_colour: fade_field(
                self.sky_horizon_colour,
                previous.sky_horizon_colour,
                fade3,
            ),
            sky_zenith_colour: fade_field(
                self.sky_zenith_colour,
                previous.sky_zenith_colour,
                fade3,
            ),
        }
    }
}

#[cfg(test)]
mod tests;
