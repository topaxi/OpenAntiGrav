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
//! `oag_game::race::zone_grade` is what applies the result to a race. 2048
//! drives it off its own recovered zone-number ladder; **HD/Fury still takes
//! an explicit stage index, because what selects the stage during a race is
//! unrecovered on that title**.

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

    /// The Zone shader's `zoneColourTint.xy`: the two title-wide, prefix-free
    /// keys [`key::TEXTURE_U_SCALE`] and [`key::TEXTURE_V_SCALE`].
    ///
    /// `None` unless the file authors **both** - a half-authored scale would
    /// be an invented lane, and only HD's own two Zone files carry these keys
    /// at all (2048 reauthored the vocabulary and drops them).
    #[must_use]
    pub fn zone_uv_scale(&self) -> Option<[f32; 2]> {
        Some([
            self.table.scalar(key::TEXTURE_U_SCALE)?,
            self.table.scalar(key::TEXTURE_V_SCALE)?,
        ])
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
            scene_texture_colour: self.stage_rgb(stage, key::SCENE_TEXTURE_COLOUR),
            scene_eq_brightness: self.stage_scalar(stage, key::SCENE_EQ_BRIGHTNESS),
            scene_base_colour: self.stage_rgb(stage, key::SCENE_BASE_COLOUR),
            scene_base_colour_highlight: self.stage_rgb(stage, key::SCENE_BASE_COLOUR_HIGHLIGHT),
            scene_aniso_power: self.stage_scalar(stage, key::SCENE_ANISO_POWER),
            track_texture_colour: self.stage_rgb(stage, key::TRACK_TEXTURE_COLOUR),
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
    /// **The one key on this list whose consumer is traced end to end.**
    ///
    /// Three floats. HD/Fury cross-fades it per frame, stores its rgb and the
    /// separately-authored [`SCENE_EQ_BRIGHTNESS`] to two different addresses,
    /// reassembles them into one `float4` in `Scene_PrepareFrame`, and
    /// publishes that as the engine shader parameter **`fogColour`** - entry 7
    /// of the 81-entry parameter table, its value pointer written at a
    /// computed offset that lands on the nose. Confidence 90 on the binding,
    /// 80 on this key being the source; see
    /// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
    /// twenty-second pass.
    ///
    /// **The name the engine gives it and the name the file gives it
    /// disagree**, and the file's values say the file is right: `Track.Texture
    /// Colour` reaches `9.0` and `Scene.Texture Colour` runs to `0.96`, which
    /// is a multiplier's range, not a fog colour's. What the fragment program
    /// does with `fogColour` is unread, so nothing here draws it as fog.
    pub const SCENE_TEXTURE_COLOUR: &str = "Scene.Texture Colour";
    /// One float, authored `0.0` or `20.0` in `zonemode.effectsettings`.
    ///
    /// A separate key in the *file*, but the fourth lane of
    /// [`SCENE_TEXTURE_COLOUR`] in the runtime's per-stage struct - one of only
    /// two such overlaps in the measured key-to-offset table. That overlap is
    /// why HD can reassemble the two into a single `float4`.
    pub const SCENE_EQ_BRIGHTNESS: &str = "Scene.EQ brightness";
    /// Three floats. The second of the three fields HD's own cross-fade reads.
    pub const SCENE_BASE_COLOUR: &str = "Scene.Base Colour";
    /// Three floats. The third of them.
    pub const SCENE_BASE_COLOUR_HIGHLIGHT: &str = "Scene.Base Colour Highlight";
    /// **`zoneEffectInner`/`zoneEffectOuter` for the *track* material group**,
    /// which is the group whose textures carry Zone's real art.
    ///
    /// HD publishes the Zone shader's parameters **twice**, and the two
    /// publications are paired end to end: one block binds the
    /// `zoneMode{0..14}.gtf` textures together with the `Scene.*` colours, the
    /// other binds `zoneModeTrack{0..14}.gtf` together with these. Read out of
    /// `FUN_003ff860`'s own two blocks, every constant a TOC-resolved
    /// `lwz rX,-N(r2)` at the exact store - `0x00c81368` beside `0x00c81490`
    /// in the first, `0x00c813e0` beside `0x00c81500` in the second.
    /// Confidence 85; see
    /// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
    /// twenty-fifth pass.
    ///
    /// **So a port that binds the track texture set must use this key and not
    /// [`SCENE_TEXTURE_COLOUR`]** - the mixed pair is one the original never
    /// publishes. The two are not interchangeable in magnitude either: this
    /// one reaches `9.0` where its `Scene` sibling runs to `3.0`.
    pub const TRACK_TEXTURE_COLOUR: &str = "Track.Texture Colour";
    /// One float, the exponent of the rim term the Zone shader raises
    /// `1 - dot(N, -V)` to.
    ///
    /// **`zoneAnisoPower` is a float2 and this is its `.x`.**
    /// `Environment_UpdateStageBlend` (`0x003da540`) writes exactly two lanes
    /// of it - stage `n`'s value in lane 0 and stage `n - 1`'s in lane 1 - and
    /// leaves lanes 2 and 3 holding whatever integer a register happened to
    /// carry. That is an independent confirmation of the shader page's own
    /// reading that the parameter is a float2. Confidence 84; see
    /// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
    /// twenty-fourth pass.
    pub const SCENE_ANISO_POWER: &str = "Scene.Aniso Power";

    /// **The `.x` of `zoneColourTint`, and one of only two keys in HD's whole
    /// schema with no stage prefix at all.**
    ///
    /// `Environment_RegisterStageSchema` (`0x003d0b98`) hands this key the
    /// address `0x00c81470` directly, which is shader parameter 52's own value
    /// pointer - so the U scale is written once when the file is parsed and
    /// never again. Confidence 85. The shader multiplies it into the zone
    /// texture's coordinate: `zoneUV = zoneColourTint.xy * (1 - meshUV)`.
    ///
    /// This is why `zoneColourTint` is **not a colour**: the engine's own
    /// schema calls its first two lanes a texture scale, and the microcode
    /// uses them as one.
    pub const TEXTURE_U_SCALE: &str = "Texture U scale";
    /// The `.y` of `zoneColourTint`, registered at `0x00c81474` on the same
    /// terms as [`TEXTURE_U_SCALE`].
    pub const TEXTURE_V_SCALE: &str = "Texture V scale";

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
    /// [`key::SCENE_TEXTURE_COLOUR`] - the one field here whose consumer in
    /// the original is traced: it becomes the shader parameter `fogColour`.
    pub scene_texture_colour: Option<[f32; 3]>,
    /// [`key::SCENE_EQ_BRIGHTNESS`], which rides in `fogColour`'s `.w`.
    pub scene_eq_brightness: Option<f32>,
    /// [`key::SCENE_BASE_COLOUR`]. Cross-faded by the original; no traced
    /// consumer past the blend.
    pub scene_base_colour: Option<[f32; 3]>,
    /// [`key::SCENE_BASE_COLOUR_HIGHLIGHT`], on the same terms.
    pub scene_base_colour_highlight: Option<[f32; 3]>,
    /// [`key::TRACK_TEXTURE_COLOUR`] - `zoneEffect<Inner|Outer>` for the track
    /// material group, the one whose texture set carries Zone's real art.
    pub track_texture_colour: Option<[f32; 3]>,
    /// [`key::SCENE_ANISO_POWER`] - `zoneAnisoPower.x`, the exponent of the
    /// Zone shader's rim term. Read and reported; the term it indexes
    /// (`zoneAnisoPalette`) has no located filler, so nothing draws it.
    pub scene_aniso_power: Option<f32>,
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
/// Which of the two a given runtime field used is still not recovered, but
/// the fields themselves now are: HD's cross-fade reads the three 16-byte
/// records at per-stage offsets `+0x00`, `+0x20` and `+0x40`, which the
/// measured key-to-offset table names as [`key::SCENE_TEXTURE_COLOUR`],
/// [`key::SCENE_BASE_COLOUR_HIGHLIGHT`] and [`key::SCENE_BASE_COLOUR`] - all
/// three float-authored, so this domain is the right one for them.
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
    /// - **Recovered since**: *which* three fields the original blends -
    ///   [`key::SCENE_TEXTURE_COLOUR`], [`key::SCENE_BASE_COLOUR`] and
    ///   [`key::SCENE_BASE_COLOUR_HIGHLIGHT`], from the measured
    ///   key-to-offset table. They are read and faded here now; before this
    ///   they were not parsed at all.
    /// - **Not recovered**: what drives `weight` during a race, and whether
    ///   the *other* fields faded here are faded by the original at all or
    ///   snapped on commit by some path that has not been traced. They are
    ///   kept faded because a snap is the stronger claim of the two and
    ///   nothing has been read that supports it.
    #[must_use]
    pub fn cross_fade(self, previous: Self, weight: f32) -> Self {
        let fade3 =
            |c: [f32; 3], p: [f32; 3]| std::array::from_fn(|i| fade_scalar(c[i], p[i], weight));
        Self {
            // The three fields the original's own cross-fade reads, plus the
            // scalar that shares the first one's storage.
            scene_texture_colour: fade_field(
                self.scene_texture_colour,
                previous.scene_texture_colour,
                fade3,
            ),
            scene_eq_brightness: fade_field(
                self.scene_eq_brightness,
                previous.scene_eq_brightness,
                |c, p| fade_scalar(c, p, weight),
            ),
            scene_base_colour: fade_field(
                self.scene_base_colour,
                previous.scene_base_colour,
                fade3,
            ),
            scene_base_colour_highlight: fade_field(
                self.scene_base_colour_highlight,
                previous.scene_base_colour_highlight,
                fade3,
            ),
            track_texture_colour: fade_field(
                self.track_texture_colour,
                previous.track_texture_colour,
                fade3,
            ),
            // Not faded by the original at all: the blend hands the two
            // stages' exponents to the shader as two separate lanes rather
            // than mixing them. Faded here for the same reason the fields
            // below it are - see this function's own note - and the Zone
            // draw reads the unblended stage palette instead.
            scene_aniso_power: fade_field(
                self.scene_aniso_power,
                previous.scene_aniso_power,
                |c, p| fade_scalar(c, p, weight),
            ),
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
