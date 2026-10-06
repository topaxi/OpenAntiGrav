//! `.effectSettings`/`.effectsettings`: the per-stage colour-grade table Zone
//! (and Detonator) read to reskin the same environment stage by stage.
//!
//! One colour palette per Zone stage (`Start`, `Sub Flash`, ..., `Supersonic`),
//! keyed by the zone number and layered over whichever circuit is racing. 2048
//! has no dedicated Zone circuit (`docs/gameplay/race-modes.md#zone`); HD/Fury
//! races one of four `/data/environments/zone_N/` circuits. Either way the look
//! escalates with the race: the disc's `MSC_EVENT_ZONE` text says top speed
//! rises every ten seconds.
//!
//! # Format
//!
//! [`envsettings`]'s tokeniser (one quoted, dotted `"key"=floats` per line), so
//! [`EffectSettings::parse`] goes through [`EnvSettings::parse`]:
//!
//! ```text
//! "0 Start.Lighting.Sun colour"=1.000000 1.000000 1.000000 0.000000
//! "Zone 0 Start.Colour 1.Colour"=1.000000 1.000000 1.000000
//! ```
//!
//! Most keys carry a **stage prefix** before the first `.`: HD writes
//! `"<n> <Name>"`, 2048 `"Zone <n> <Name>"`. A few title-wide keys (2048's
//! `"Sky Radius"`, HD's `"Texture U scale"`) carry none and read off
//! [`EffectSettings::table`].
//!
//! # Confidence
//!
//! **85** on the syntax and prefix shape: all five files (HD's
//! `zonemode`, `zonemodedlc3`, `detonatormode`, `detonatormodedlc3`, and 2048's
//! ten identical `ZoneMode2048.effectSettings`) parse in full and every stage
//! name recovers (`effectsettings_ground_truth.rs`). Not 90, because **the
//! stage-index-to-zone-number mapping is inferred from names**, not checked
//! against `Zone_Update`'s timer; see `docs/formats/effectsettings.md`'s
//! `## Open`.
//!
//! [`cross_fade_rgba8`] does read the executable: HD/Fury's runtime cross-fade
//! between adjacent stages (confidence 80). [`StagePalette`] is what it blends
//! and `oag_raceplay::zone_grade` applies it. 2048 drives it off its recovered
//! zone-number ladder; **HD/Fury still takes an explicit stage index, because
//! what selects the stage during a race is unrecovered there**.

use std::collections::BTreeMap;

use crate::envsettings::{self, EnvSettings};

/// One stage's own identity, as the file names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    /// The zone number, the key's leading integer (`0` for `Start`).
    pub index: u32,
    /// The stage's authored name (`"Start"`, `"Sub Venom"`, `"Supersonic"`).
    pub name: String,
    /// The exact text before the first `.` of this stage's keys (`"0 Start"` on
    /// HD, `"Zone 0 Start"` on 2048), kept whole so [`EffectSettings::stage_key`]
    /// never guesses a title's spelling.
    prefix: String,
}

/// The zone index and stage name a key's leading segment names, or `None` for a
/// key with no stage prefix (`"Sky Radius"`, a plain group like `"Lighting"`).
fn parse_stage_head(head: &str) -> Option<(u32, &str)> {
    let head = head.strip_prefix("Zone ").unwrap_or(head);
    let (number, name) = head.split_once(' ')?;
    let index: u32 = number.parse().ok()?;
    Some((index, name))
}

/// Everything one `.effectSettings`/`.effectsettings` file says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectSettings {
    /// Every entry as [`EnvSettings::parse`] read it, stage prefix included.
    pub table: EnvSettings,
    /// Every stage this file names, by zone number. A `BTreeMap` for
    /// determinism, like [`EnvSettings::entries`].
    pub stages: BTreeMap<u32, Stage>,
}

impl EffectSettings {
    /// Reads a whole file.
    ///
    /// # Errors
    ///
    /// [`envsettings::Error`] for the first line that is neither blank nor
    /// `key=value`.
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

    /// The full key one stage's `key` reads under, or `None` for an unnamed stage.
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
    /// Not clamped, like [`EnvSettings::vec3`]: 2048's `"Edge Colour"` reaches 1.098.
    #[must_use]
    pub fn stage_vec3(&self, stage: u32, key: &str) -> Option<[f32; 3]> {
        self.table.vec3(&self.stage_key(stage, key)?)
    }

    /// Four numbers, for a per-stage key that carries exactly four.
    ///
    /// [`EnvSettings::rgba8`] cannot answer these: HD's four-number colour keys
    /// are written with decimal points, not as the byte quadruple
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
    /// Only [`key::SKY_REFLECTION_COLOUR`] and the two `EQ ... tint` keys are
    /// written this way on HD; the rest answer [`Self::stage_vec4`].
    #[must_use]
    pub fn stage_rgba8(&self, stage: u32, key: &str) -> Option<[u8; 4]> {
        self.table.rgba8(&self.stage_key(stage, key)?)
    }

    /// A colour key's red, green and blue, whether written with three numbers or
    /// four.
    ///
    /// **The fourth lane is dropped**: every four-number colour key in
    /// `zonemode.effectsettings` authors `0.000000` there on all fifteen stages.
    /// 2048's two fog keys carry a density there and use [`Self::stage_fourth`].
    #[must_use]
    pub fn stage_rgb(&self, stage: u32, key: &str) -> Option<[f32; 3]> {
        let key = self.stage_key(stage, key)?;
        match self.table.entries.get(&key)?.numbers.as_slice() {
            [r, g, b] | [r, g, b, _] => Some([*r, *g, *b]),
            _ => None,
        }
    }

    /// The fourth number of a four-number per-stage key, else `None`.
    ///
    /// For [`key::ENVIRONMENT_FOG_COLOUR_2048`], whose fourth lane is the fog
    /// density. Answers `None` for three-number keys rather than substituting.
    #[must_use]
    pub fn stage_fourth(&self, stage: u32, key: &str) -> Option<f32> {
        let key = self.stage_key(stage, key)?;
        match self.table.entries.get(&key)?.numbers.as_slice() {
            [_, _, _, fourth] => Some(*fourth),
            _ => None,
        }
    }

    /// The Zone shader's `zoneColourTint.xy`: the two prefix-free keys
    /// [`key::TEXTURE_U_SCALE`] and [`key::TEXTURE_V_SCALE`].
    ///
    /// `None` unless the file authors **both**. Only HD's Zone files carry them;
    /// 2048 reauthored the vocabulary and drops them.
    #[must_use]
    pub fn zone_uv_scale(&self) -> Option<[f32; 2]> {
        Some([
            self.table.scalar(key::TEXTURE_U_SCALE)?,
            self.table.scalar(key::TEXTURE_V_SCALE)?,
        ])
    }

    /// One stage's [`StagePalette`], or `None` for a stage the file does not name.
    ///
    /// A named stage authoring none of these keys gives
    /// `Some(StagePalette::default())`, which differs from an absent stage.
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
            track_eq_brightness: self.stage_scalar(stage, key::TRACK_EQ_BRIGHTNESS),
            track_base_colour_highlight: self.stage_rgb(stage, key::TRACK_BASE_COLOUR_HIGHLIGHT),
            track_base_colour: self.stage_rgb(stage, key::TRACK_BASE_COLOUR),
            // HD's spelling first, then 2048's; a file carries one vocabulary.
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
            eq_colour_tint: self.stage_rgba8(stage, key::EQ_COLOUR_TINT),
            eq_analogue_colour_tint: self.stage_rgba8(stage, key::EQ_ANALOGUE_COLOUR_TINT),
        })
    }

    /// The palette that applies at `stage`, cross-faded against the stage before
    /// it as HD/Fury's runtime pairs them.
    ///
    /// **`stage - 1`, clamped at zero, is the recovered pairing**: both
    /// functions that compute this blend index the table by the entity's stage
    /// field and by that minus one, saturating (`ps3-hdfury-eu`, `FUN_003ce2c0`
    /// and `Environment_UpdateStageBlend` at `0x003da540`, confidence 80). At
    /// stage `0` the blend is the identity whatever `weight` reads.
    ///
    /// `None` when the file does not name `stage`. A stage whose predecessor is
    /// skipped blends against itself.
    #[must_use]
    pub fn blended_palette(&self, stage: u32, weight: f32) -> Option<StagePalette> {
        let current = self.stage_palette(stage)?;
        let previous = self
            .stage_palette(stage.saturating_sub(1))
            .unwrap_or(current);
        Some(current.cross_fade(previous, weight))
    }
}

/// Cross-fades two stages' packed-byte colour channels, byte for byte, as
/// HD/Fury's runtime does.
///
/// `weight` is the current stage's share (`1.0` when a stage becomes current,
/// falling toward `0.0`), expected in `0.0..=1.0` and not clamped, matching the
/// traced code.
///
/// # Evidence
///
/// Recovered from two functions in HD/Fury's executable that compute this
/// arithmetic against the same `0x250`-stride stage table: `FUN_003da540` and
/// `FUN_003ce2c0`, both `ps3-hdfury-eu`, confidence 80 (corroborated twice).
/// Per channel: `(int)(current * weight) + (int)(previous * other_weight)`, each
/// term truncated toward zero before summing, then clamped to `0xff` on the high
/// side only. See
/// [zone-effectsettings-loader.md](../../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-a-fourth-pass-who-writes-0x008b7944--n0x38---one-near-repeat-of-the-opd-trap-caught-before-it-shipped-one-real-correction-one-new-lead).
///
/// Which stage is "current" and what `weight` reads at runtime are driven by
/// callers (`oag_raceplay::zone_grade`); on HD/Fury the stage selector is
/// unrecovered.
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

/// The per-stage keys this project reads, spelled as HD/Fury's executable spells
/// them.
///
/// Every name appears verbatim in `g_EffectSettingsSchemaKeyNames`
/// (`0x008b79cc`, `ps3-hdfury-eu`), the 73-entry schema table
/// `FwKeyedText_ParseEntry` matches against; see
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`. The
/// British spellings are the schema's own; `.envsettings`' American
/// [`envsettings::SUN_COLOUR`] belongs to a different table.
pub mod key {
    /// **The one key whose consumer is traced end to end.**
    ///
    /// Three floats. HD/Fury cross-fades it per frame, stores its rgb and
    /// [`SCENE_EQ_BRIGHTNESS`] separately, reassembles them into one `float4` in
    /// `Scene_PrepareFrame` and publishes that as the shader parameter
    /// **`fogColour`** (entry 7 of the 81-entry parameter table). Confidence 90
    /// on the binding, 80 on this key being the source; see
    /// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
    /// twenty-second pass.
    ///
    /// **The engine's name and the file's disagree**, and the values say the
    /// file is right: `Track.Texture Colour` reaches `9.0`, `Scene.Texture
    /// Colour` `0.96`, a multiplier's range. What the fragment program does with
    /// `fogColour` is unread, so nothing here draws it as fog.
    pub const SCENE_TEXTURE_COLOUR: &str = "Scene.Texture Colour";
    /// One float, authored `0.0` or `20.0` in `zonemode.effectsettings`.
    ///
    /// A separate key in the file but the fourth lane of
    /// [`SCENE_TEXTURE_COLOUR`] in the runtime's stage struct, which is why HD
    /// can reassemble the two into one `float4`.
    pub const SCENE_EQ_BRIGHTNESS: &str = "Scene.EQ brightness";
    /// Three floats. The second of the three fields HD's cross-fade reads.
    pub const SCENE_BASE_COLOUR: &str = "Scene.Base Colour";
    /// Three floats. The third of them.
    pub const SCENE_BASE_COLOUR_HIGHLIGHT: &str = "Scene.Base Colour Highlight";
    /// **`zoneEffectInner`/`zoneEffectOuter` for the *track* material group**,
    /// whose textures carry Zone's real art.
    ///
    /// HD publishes the Zone shader's parameters twice: one block binds
    /// `zoneMode{0..14}.gtf` with the `Scene.*` colours, the other
    /// `zoneModeTrack{0..14}.gtf` with these. Read from `FUN_003ff860`'s two
    /// blocks (`0x00c81368` beside `0x00c81490`, `0x00c813e0` beside
    /// `0x00c81500`). Confidence 85; see
    /// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
    /// twenty-fifth pass.
    ///
    /// **A port binding the track texture set must use this key, not
    /// [`SCENE_TEXTURE_COLOUR`]**: the mixed pair is never published, and this
    /// reaches `9.0` where its `Scene` sibling runs to `3.0`.
    pub const TRACK_TEXTURE_COLOUR: &str = "Track.Texture Colour";
    /// The `.w` of [`TRACK_TEXTURE_COLOUR`], on the terms [`SCENE_EQ_BRIGHTNESS`]
    /// is the `.w` of its sibling.
    pub const TRACK_EQ_BRIGHTNESS: &str = "Track.EQ brightness";
    /// `zoneBase<Inner|Outer>` for the track group: the `rim^10` summand of the
    /// Zone surface colour `zoneTex * zoneEffect + zoneBase.rgb * rim^10 +
    /// zoneBaseAlt.rgb * rim^5` (both exponents inline literals in the microcode).
    ///
    /// **Not confined to an untextured family**: 20,084 of the 20,214
    /// Zone-bearing fragment blocks consume it, 18,032 alongside a sampled
    /// `zoneTex*`. See `oag_mesh::mesh_render::Zone` for the census.
    pub const TRACK_BASE_COLOUR_HIGHLIGHT: &str = "Track.Base Colour Highlight";
    /// `zoneBaseAlt<Inner|Outer>` for the track group: the `rim^5` summand's
    /// colour, as [`TRACK_BASE_COLOUR_HIGHLIGHT`].
    pub const TRACK_BASE_COLOUR: &str = "Track.Base Colour";

    /// One float, the exponent of the rim term the Zone shader raises
    /// `1 - dot(N, -V)` to.
    ///
    /// **`zoneAnisoPower` is a float2 and this is its `.x`.**
    /// `Environment_UpdateStageBlend` (`0x003da540`) writes only two lanes:
    /// stage `n` in lane 0, stage `n - 1` in lane 1. Confidence 84; see
    /// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
    /// twenty-fourth pass.
    pub const SCENE_ANISO_POWER: &str = "Scene.Aniso Power";

    /// **The `.x` of `zoneColourTint`, and one of only two keys in HD's schema
    /// with no stage prefix.**
    ///
    /// `Environment_RegisterStageSchema` (`0x003d0b98`) hands this key the
    /// address `0x00c81470`, shader parameter 52's value pointer, so it is
    /// written once at parse. Confidence 85. The shader computes `zoneUV =
    /// zoneColourTint.xy * (1 - meshUV)`, so `zoneColourTint` is a texture
    /// scale, **not a colour**.
    pub const TEXTURE_U_SCALE: &str = "Texture U scale";
    /// The `.y` of `zoneColourTint`, registered at `0x00c81474`; see
    /// [`TEXTURE_U_SCALE`].
    pub const TEXTURE_V_SCALE: &str = "Texture V scale";

    /// The stage's distance fog colour. Four floats, alpha authored `0`.
    pub const FOG_COLOUR: &str = "Lighting.Fog colour";
    /// The coefficient of the `exp(-(density * depth)^2)` curve `.envsettings`'
    /// `Fog.Fog Density` also feeds.
    pub const FOG_DENSITY: &str = "Lighting.Fog density";
    /// Four floats, alpha authored `0`. **No `Sun direction` key exists**, so a
    /// stage can tint a rig, never aim one.
    pub const SUN_COLOUR: &str = "Lighting.Sun colour";
    /// Four floats, alpha authored `0`.
    pub const AMBIENT_COLOUR: &str = "Lighting.Constant Ambient Colour";
    /// Four floats, alpha authored `0`.
    pub const PRELIT_SCALE: &str = "Lighting.Prelit Colour Scale";
    /// Three floats.
    pub const PRELIT_POWER: &str = "Lighting.Prelit Colour Power";
    /// **The one colour key the file writes as bytes**, so the one
    /// [`super::cross_fade_rgba8`] blends in its recovered domain.
    pub const SKY_REFLECTION_COLOUR: &str = "Lighting.Sky reflection colour";
    /// Three floats. Parsed, not drawn: HD's sky is the `sky.gtf` cubemap, with
    /// no horizon/zenith term.
    pub const SKY_HORIZON_COLOUR: &str = "Sky horizon colour";
    /// Three floats, as [`SKY_HORIZON_COLOUR`].
    pub const SKY_ZENITH_COLOUR: &str = "Sky zenith colour";

    /// The visualiser's two colours, byte-written like [`SKY_REFLECTION_COLOUR`]:
    /// `%s.EQ colour tint`, schema entry 62 (`0x008b79cc`); see
    /// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`. The
    /// maintainer's play
    /// (`docs/formats/effectsettings.md#the-eq-keys-are-an-audio-spectrum-observed-in-play`)
    /// calls the `EQ` cluster an equaliser. **No consumer is traced** (the
    /// loader page's `+0x150` row reads "nothing"), so the mapping to the banded
    /// spectrum's colour is naming and shape alone, confidence 70.
    pub const EQ_COLOUR_TINT: &str = "EQ colour tint";
    /// `%s.EQ analogue colour tint`, schema entry 63, `+0x154` of the stage
    /// struct: the smoothed variant beside [`EQ_COLOUR_TINT`], equally
    /// unconfirmed.
    pub const EQ_ANALOGUE_COLOUR_TINT: &str = "EQ analogue colour tint";

    /// 2048's spelling of the environment's distance fog, and the one key here
    /// **not** in HD's schema table.
    ///
    /// 2048's stage blocks carry `Fog.Environment Fog Colour` and `Fog.Track Fog
    /// Colour` and **no density key** (`grep -ci density` over a shipped
    /// `ZoneMode2048.effectSettings` is `0`). Both are written with **four**
    /// numbers where other colours have three, and the fourth reads
    /// `0.0025`/`0.0015` (environment) and `0.007` (track): the magnitude of
    /// HD's [`FOG_DENSITY`] (`0.0021`), and nothing like an alpha. So the fourth
    /// lane is the density. **Confidence 74**: missing key, odd arity,
    /// magnitude; no traced consumer in 2048's executable.
    pub const ENVIRONMENT_FOG_COLOUR_2048: &str = "Fog.Environment Fog Colour";
    /// 2048's second fog block, on the same four-number terms. **Read, not
    /// drawn**: what selects environment or track is unrecovered, so the primary
    /// pair is used, as `.envsettings`' reader does.
    pub const TRACK_FOG_COLOUR_2048: &str = "Fog.Track Fog Colour";
    /// 2048's spelling of [`SKY_HORIZON_COLOUR`]. Read, not drawn.
    pub const SKY_HORIZON_COLOUR_2048: &str = "Sky.Horizon Colour";
    /// 2048's spelling of [`SKY_ZENITH_COLOUR`]. Read, not drawn.
    pub const SKY_ZENITH_COLOUR_2048: &str = "Sky.Zenith Colour";
}

/// One stage's palette: the subset of its keys that has somewhere to go.
///
/// **Every field is `Option`** and nothing substitutes for an absent key:
/// "this stage does not author that" is an ordinary answer. Colours are
/// **unclamped** (`Track.Texture Colour` reaches `9.0`), like
/// [`EnvSettings::vec3`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StagePalette {
    /// [`key::SCENE_TEXTURE_COLOUR`], the one field whose consumer is traced: it
    /// becomes the shader parameter `fogColour`.
    pub scene_texture_colour: Option<[f32; 3]>,
    /// [`key::SCENE_EQ_BRIGHTNESS`], which rides in `fogColour`'s `.w`.
    pub scene_eq_brightness: Option<f32>,
    /// [`key::SCENE_BASE_COLOUR`]. Cross-faded by the original; no traced
    /// consumer past the blend.
    pub scene_base_colour: Option<[f32; 3]>,
    /// [`key::SCENE_BASE_COLOUR_HIGHLIGHT`], on the same terms.
    pub scene_base_colour_highlight: Option<[f32; 3]>,
    /// [`key::TRACK_TEXTURE_COLOUR`]: `zoneEffect<Inner|Outer>` for the track
    /// group.
    pub track_texture_colour: Option<[f32; 3]>,
    /// [`key::TRACK_EQ_BRIGHTNESS`], the track group's `zoneEffect.w`.
    pub track_eq_brightness: Option<f32>,
    /// [`key::TRACK_BASE_COLOUR_HIGHLIGHT`] - `zoneBase<Inner|Outer>` for the
    /// track group.
    pub track_base_colour_highlight: Option<[f32; 3]>,
    /// [`key::TRACK_BASE_COLOUR`] - `zoneBaseAlt<Inner|Outer>` for the track
    /// group.
    pub track_base_colour: Option<[f32; 3]>,
    /// [`key::SCENE_ANISO_POWER`]: `zoneAnisoPower.x`. Read, not drawn: the term
    /// it indexes (`zoneAnisoPalette`) has no located filler.
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
    /// [`key::EQ_COLOUR_TINT`], byte-written like [`Self::sky_reflection_colour`].
    pub eq_colour_tint: Option<[u8; 4]>,
    /// [`key::EQ_ANALOGUE_COLOUR_TINT`], on the same terms.
    pub eq_analogue_colour_tint: Option<[u8; 4]>,
}

/// One float channel of the blend, in the float domain the key is authored in.
///
/// The weighting is the recovered one; the domain is the key's own.
/// [`cross_fade_rgba8`] truncates and clamps at `0xff` on packed bytes, which
/// would send a float key like `1.800000` to white (three of
/// `zonemode.effectsettings`' float colours exceed `1.0`). HD's cross-fade reads
/// the 16-byte records at stage offsets `+0x00`, `+0x20` and `+0x40`, which the
/// key-to-offset table names [`key::SCENE_TEXTURE_COLOUR`],
/// [`key::SCENE_BASE_COLOUR_HIGHLIGHT`] and [`key::SCENE_BASE_COLOUR`]; all
/// three are float-authored, so this domain is right for them.
fn fade_scalar(current: f32, previous: f32, weight: f32) -> f32 {
    // Two multiplies and an add, not `mul_add`, per `docs/architecture/determinism.md`.
    current * weight + previous * (1.0 - weight)
}

/// One field of the blend, in the two-stage `Option` shape the palettes have.
///
/// A field blends only where both stages author it. If only the current stage
/// does, its value stands (inventing a zero to fade from would put a value in
/// the picture no stage wrote). If the current stage authors none the result is
/// `None`: which stage a key is missing from is the file's own statement.
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
    /// `weight` is **this** palette's share, as in [`cross_fade_rgba8`]: `1.0`
    /// is this stage alone, `0.0` is `previous` alone.
    ///
    /// - **Recovered (confidence 80)**: HD/Fury cross-fades stage `n` against
    ///   `n - 1` by a weight, with the byte arithmetic of [`cross_fade_rgba8`].
    /// - **Recovered**: *which* three fields the original blends
    ///   ([`key::SCENE_TEXTURE_COLOUR`], [`key::SCENE_BASE_COLOUR`],
    ///   [`key::SCENE_BASE_COLOUR_HIGHLIGHT`]), from the key-to-offset table.
    /// - **Not recovered**: what drives `weight` during a race, and whether the
    ///   *other* fields faded here are faded by the original or snapped on
    ///   commit. A fade is kept as the weaker claim.
    #[must_use]
    pub fn cross_fade(self, previous: Self, weight: f32) -> Self {
        let fade3 =
            |c: [f32; 3], p: [f32; 3]| std::array::from_fn(|i| fade_scalar(c[i], p[i], weight));
        Self {
            // The three fields the original's cross-fade reads, plus the scalar
            // sharing the first one's storage.
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
            track_eq_brightness: fade_field(
                self.track_eq_brightness,
                previous.track_eq_brightness,
                |c, p| fade_scalar(c, p, weight),
            ),
            track_base_colour_highlight: fade_field(
                self.track_base_colour_highlight,
                previous.track_base_colour_highlight,
                fade3,
            ),
            track_base_colour: fade_field(
                self.track_base_colour,
                previous.track_base_colour,
                fade3,
            ),
            // Not faded by the original (the blend hands the two exponents to the
            // shader as two lanes); faded here like the fields below, and the Zone
            // draw reads the unblended palette.
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
            // The one field stored as bytes, so the recovered arithmetic applies unaltered.
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
            eq_colour_tint: fade_field(self.eq_colour_tint, previous.eq_colour_tint, |c, p| {
                cross_fade_rgba8(c, p, weight)
            }),
            eq_analogue_colour_tint: fade_field(
                self.eq_analogue_colour_tint,
                previous.eq_analogue_colour_tint,
                |c, p| cross_fade_rgba8(c, p, weight),
            ),
        }
    }
}

#[cfg(test)]
mod tests;
