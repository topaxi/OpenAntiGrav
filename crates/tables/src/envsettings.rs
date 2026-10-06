//! `.envsettings`: the lighting, fog and tone rig a Wipeout HD circuit authors.
//!
//! # Why it matters
//!
//! **The renderer's light rig is a stand-in** (`crates/mesh/src/mesh.wesl`: two
//! invented lights, for legibility). On HD the look is authored, in plain text,
//! one file per circuit: sun direction and colour, constant ambient, fog colour
//! and density, tonemapper parameters. `CLAUDE.md`'s rule about not inventing
//! what the assets author applies.
//!
//! # The format
//!
//! One `key=value` per line, no sections, no escaping, no comments. The key is
//! quoted and dotted; the value is one to four whitespace-separated numbers:
//!
//! ```text
//! "Lighting.Constant ambient color"=0.403922 0.392157 0.509804
//! "Lighting.Sun direction"=-0.777600 0.581520 -0.239110
//! "Lighting.Sky colour"=128 128 128 0
//! "Fog.Fog Density"=0.001000
//! ```
//!
//! The dotted prefix (`Lighting`, `Fog`, `Water`, `HDR and Bloom`) is part of
//! the key: the file has no sections and no leaf name repeats under two prefixes.
//!
//! # Two number encodings, told apart by formatting
//!
//! **Reading every value as a `0..=1` float gets one key wrong by 255.**
//! `"Lighting.Sky colour"=128 128 128 0` is a byte quadruple; every other colour
//! has a decimal point and is normalised. Over all 32 circuit files `Sky colour`
//! is the only integer-formatted key reaching 255; the other four
//! (`Radial bloom Enabled`, `Enable dynamic lights`, `Enable spu vertex lights`,
//! `Use Lens Flare`) are `0`/`1` flags. So [`Value::is_integer`] records the
//! formatting and [`EnvSettings::rgba8`] is separate from [`EnvSettings::vec3`].
//!
//! # Authored for a linear HDR pipeline; this project's is not
//!
//! `"Lighting.Sun color"` reaches **4.0** and `"Lighting.Constant ambient
//! color"` **3.0**, beside `"HDR and Bloom.Tone maximum brightness"=4.0`: only
//! meaningful with linear light and a tonemap. This renderer is
//! gamma-authoritative per
//! [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md)
//! with no tonemap stage. **Recorded, not acted on**; see
//! `docs/formats/envsettings.md` for which fields are usable today.
//!
//! # Confidence
//!
//! **90 on the syntax and key set**, from exact agreement across all 33 files:
//! 32 circuits sharing a 39-key schema, plus `/data/fe/fury.envsettings` (436
//! lines of feedback, equaliser and music-pulse keys for the Fury front end).
//! Nothing here reads HD's executable, so **what the original does with a value
//! is not decoded**; the names are the file's own.
//!
//! # Wipeout 2048: same shape, different key names
//!
//! Every base and DLC circuit ships a `track.EnvSettings` in the same syntax, so
//! the *format* needs no title code; the *keys* differ. 2048's registrar
//! (`Environment_RegisterLightingSchema`, `vita-2048-eu-v104/eboot.elf`,
//! confidence 85,
//! [`lighting-schema.md`](../../../docs/ghidra/functions/vita-2048-eu-v104/lighting-schema.md))
//! spells ambient `"Lighting.Constant ambient colour"` (HD:
//! `"... ambient color"`) and splits HD's `"Lighting.Sun color"` into `"Lighting.Sun
//! diffuse colour"` and `"Lighting.Sun specular colour"`: different keys, read
//! from the registrar's string table. [`PSP2_AMBIENT_COLOUR`] and
//! [`PSP2_SUN_DIFFUSE_COLOUR`] are 2048's spellings of [`AMBIENT_COLOUR`] and
//! [`SUN_COLOUR`]; [`SUN_DIRECTION`] is spelled the same on both.
//!
//! **2048 also authors a literal `"Lighting.Sun color"`, which is not this
//! project's diffuse term.** The registrar binds it to its own field. Five of 14
//! sampled circuits (`Anulpha_Pass`, `Chenghou_Project`, `bridge`, `park`,
//! `sol`) carry exactly the registrar's compiled-in default (`~1.5, 1.3, 1.0`),
//! an untouched default; corroborating, not proving, a vestigial field. A
//! consumer search found no reader (this binary reaches the struct through
//! `Environment_RegisterLightingSchema()`'s return pointer plus an offset, the
//! TOC-relative trap in `docs/formats/envsettings.md`'s HD notes). **Do not wire
//! it as a light-rig colour on its name alone**; see that page's 2048 section.

use std::collections::BTreeMap;

/// `"Lighting.Sun direction"`. Three numbers, unit on most circuits; see
/// [`EnvSettings::direction`].
pub const SUN_DIRECTION: &str = "Lighting.Sun direction";

/// `"Lighting.Sun color"`. Three numbers, reaching 4.0 across the corpus.
pub const SUN_COLOUR: &str = "Lighting.Sun color";

/// `"Lighting.Sun specular scale"`. One number, 1.0 to 3.75.
pub const SUN_SPECULAR_SCALE: &str = "Lighting.Sun specular scale";

/// `"Lighting.Constant ambient color"`. Three numbers, reaching 3.0.
pub const AMBIENT_COLOUR: &str = "Lighting.Constant ambient color";

/// Wipeout 2048's spelling of [`AMBIENT_COLOUR`]; see the module docs.
pub const PSP2_AMBIENT_COLOUR: &str = "Lighting.Constant ambient colour";

/// 2048's diffuse sun term, the field HD's [`SUN_COLOUR`] answers for. **Not
/// 2048's own literal `"Lighting.Sun color"`**, which is left unwired (module
/// docs).
pub const PSP2_SUN_DIFFUSE_COLOUR: &str = "Lighting.Sun diffuse colour";

/// `"Lighting.Prelit ambient colour scale"`. Three numbers: the scale in
/// `prelit = scale * lightmap.rgb ^ power`, from the circuit's fragment
/// microcode (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`).
pub const PRELIT_SCALE: &str = "Lighting.Prelit ambient colour scale";

/// `"Lighting.Prelit ambient colour power"`. Three numbers: the exponent of
/// [`PRELIT_SCALE`]'s term.
pub const PRELIT_POWER: &str = "Lighting.Prelit ambient colour power";

/// Omega's `"Lighting.Nova prelit scale bias power"`: **one triple of scalars**
/// `(scale, bias, power)`, not three per-channel vectors; its pixel shaders
/// compute `scale * pow(lightmap.rgb, power) + bias` per channel
/// (`docs/ghidra/functions/ps4-omega-eu/lightmap-prelit.md`). Authored by 88 of
/// the title's 97 files (patch and base), 20 distinct triples.
pub const NOVA_PRELIT: &str = "Lighting.Nova prelit scale bias power";

/// What [`NOVA_PRELIT`] is when a file omits it: the executable's static default
/// `(1.4, 0.2, 1.5)` (`FUN_015c2dc0`, `0x01e3de80`), not the identity.
pub const NOVA_PRELIT_DEFAULT: [f32; 3] = [1.4, 0.2, 1.5];

/// The `Tonemap.*` (or `TonemapHDR.*`) block Omega authors: the ten keys the
/// executable registers (`FUN_015c1f20`) under both prefixes. **Read, no
/// consumer located** (`lightmap-prelit.md`, "The `Tonemap.*` block"); nothing
/// here applies it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tonemap {
    /// `Exposure minimum`.
    pub exposure_minimum: f32,
    /// `Exposure maximum`.
    pub exposure_maximum: f32,
    /// `Exposure response`.
    pub exposure_response: f32,
    /// `Exposure time`.
    pub exposure_time: f32,
    /// `Luminance a-coefficient`.
    pub luminance_a: f32,
    /// `Luminance b-coefficient`.
    pub luminance_b: f32,
    /// `Source color end a-coefficient`.
    pub source_colour_end_a: f32,
    /// `Source color end b-coefficient`.
    pub source_colour_end_b: f32,
    /// `Start angle`.
    pub start_angle: f32,
    /// `End angle`.
    pub end_angle: f32,
}

/// `"Lighting.Sky colour"`. **Four bytes**, not floats - see the module docs.
pub const SKY_COLOUR: &str = "Lighting.Sky colour";

/// `"Lighting.Sky rotation"`. One number, degrees, -40 to 300.
pub const SKY_ROTATION: &str = "Lighting.Sky rotation";

/// `"Fog.Fog Color"`. Three numbers, reaching 1.85.
pub const FOG_COLOUR: &str = "Fog.Fog Color";

/// `"Fog.Fog Density"`. One number, 0.0003 to 0.03. The materials' fragment
/// microcode applies a coefficient as `exp(-(coefficient * view_depth)^2)`;
/// whether this reaches it unscaled is not read.
pub const FOG_DENSITY: &str = "Fog.Fog Density";

/// 2048's `"Lighting.Fog colour"`. **Four numbers**: a colour and a fourth,
/// 0.0001 to 0.0021 on every circuit authoring it, the order of HD's
/// [`FOG_DENSITY`] (Anulpha Pass 0.0004 on both titles). Both titles' shader
/// interface has one `float4` `fogColour`, with HD's fourth the curve
/// coefficient. That 2048's is the same quantity is an inheritance, not
/// measured.
pub const PSP2_FOG_COLOUR: &str = "Lighting.Fog colour";

/// `"HDR and Bloom.Bloom adaption rate"`. One number: the per-frame lerp rate
/// of the adapted average luminance, `adapted += rate * (luma(mean) -
/// adapted)` (`FUN_003b4690`). The settings registrar at `0x003a83d8` ties every
/// key in this block to its field, so each mapping here is a read, not a guess;
/// see `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`.
pub const BLOOM_ADAPTION_RATE: &str = "HDR and Bloom.Bloom adaption rate";

/// `"HDR and Bloom.Bloom adaption boost"`. One number: scales adapted luminance
/// in the gate's fade, `1 - min(adapted * boost * 0.25, 1)` (`0.25` an inline
/// constant).
pub const BLOOM_ADAPTION_BOOST: &str = "HDR and Bloom.Bloom adaption boost";

/// `"HDR and Bloom.Tone adaption boost"`. One number: scales adapted luminance in
/// the exposure, `scale = maximum_brightness - min(adapted * this,
/// darkening_clamp)`, the resolve pass's `scale`.
pub const TONE_ADAPTION_BOOST: &str = "HDR and Bloom.Tone adaption boost";

/// `"HDR and Bloom.Tone darkening clamp"`. One number: the cap in that formula.
pub const TONE_DARKENING_CLAMP: &str = "HDR and Bloom.Tone darkening clamp";

/// `"HDR and Bloom.Tone maximum brightness"`. One number: the exposure on a
/// black frame.
pub const TONE_MAXIMUM_BRIGHTNESS: &str = "HDR and Bloom.Tone maximum brightness";

/// `"HDR and Bloom.Bloom from alpha contribution"`. One number: the glow-mask
/// weight in `FunkLayerBloomGate_fp`'s bright pass, `frame.rgb * frame.a * this`
/// (`renderer.md`).
pub const BLOOM_ALPHA_CONTRIBUTION: &str = "HDR and Bloom.Bloom from alpha contribution";

/// `"HDR and Bloom.Bloom from frame contribution"`. One number: the same gate's
/// luminance weight, `frame.rgb * lum^exponent * this`.
pub const BLOOM_FRAME_CONTRIBUTION: &str = "HDR and Bloom.Bloom from frame contribution";

/// `"HDR and Bloom.Bloom from frame exponent"`. One number: the luminance
/// term's power.
pub const BLOOM_FRAME_EXPONENT: &str = "HDR and Bloom.Bloom from frame exponent";

/// `"HDR and Bloom.Bloom horizontal size"`. One number: the blur's tap spacing,
/// patched into `FunkLayerBloomBlurHorizontal_fp`'s nine taps.
pub const BLOOM_HORIZONTAL_SIZE: &str = "HDR and Bloom.Bloom horizontal size";

/// `"HDR and Bloom.Bloom vertical size"`. One number: the vertical blur's tap
/// spacing.
pub const BLOOM_VERTICAL_SIZE: &str = "HDR and Bloom.Bloom vertical size";

/// One entry's value: the numbers as written, and how they were written.
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    /// One to four numbers, in file order.
    pub numbers: Vec<f32>,
    /// Whether every number was written without a decimal point: the file's only
    /// signal for a byte or flag rather than a normalised float.
    pub is_integer: bool,
}

/// Everything one `.envsettings` file says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnvSettings {
    /// Every entry, keyed by the unquoted key. A `BTreeMap` so a load report's
    /// iteration never depends on a per-process hasher
    /// (`docs/architecture/determinism.md`).
    pub entries: BTreeMap<String, Value>,
}

/// A line that is neither blank nor `key=value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub text: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {:?} is not `key=value`", self.line, self.text)
    }
}

impl std::error::Error for Error {}

impl EnvSettings {
    /// Reads a whole file.
    ///
    /// Reads a whole file.
    ///
    /// **A non-numeric value drops its entry** rather than defaulting to zero (a
    /// zero ambient is a picture); nothing on the disc has one.
    ///
    /// # Errors
    ///
    /// [`Error`] for the first line that is neither blank nor `key=value`. All 33
    /// files on the disc parse.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut entries = BTreeMap::new();
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Some((key, value)) = trimmed.split_once('=') else {
                return Err(Error {
                    line: index + 1,
                    text: trimmed.to_string(),
                });
            };
            let fields: Vec<&str> = value.split_whitespace().collect();
            let numbers: Vec<f32> = fields.iter().filter_map(|f| f.parse().ok()).collect();
            if numbers.len() != fields.len() || numbers.is_empty() {
                continue;
            }
            entries.insert(
                key.trim().trim_matches('"').to_string(),
                Value {
                    is_integer: fields.iter().all(|f| !f.contains('.')),
                    numbers,
                },
            );
        }
        Ok(Self { entries })
    }

    /// One number, for a key that carries exactly one.
    #[must_use]
    pub fn scalar(&self, key: &str) -> Option<f32> {
        match self.entries.get(key)?.numbers.as_slice() {
            [only] => Some(*only),
            _ => None,
        }
    }

    /// Three numbers, for a key that carries exactly three. **Not clamped or
    /// normalised**: several exceed 1.0 (module docs) and clamping would hide it.
    #[must_use]
    pub fn vec3(&self, key: &str) -> Option<[f32; 3]> {
        match self.entries.get(key)?.numbers.as_slice() {
            [x, y, z] => Some([*x, *y, *z]),
            _ => None,
        }
    }

    /// Four numbers, for a key that carries exactly four.
    #[must_use]
    pub fn vec4(&self, key: &str) -> Option<[f32; 4]> {
        match self.entries.get(key)?.numbers.as_slice() {
            [x, y, z, w] => Some([*x, *y, *z, *w]),
            _ => None,
        }
    }

    /// The [`Tonemap`] block under `prefix` (`"Tonemap"` or `"TonemapHDR"`),
    /// or `None` unless **all ten** keys are present as one number each.
    #[must_use]
    pub fn tonemap(&self, prefix: &str) -> Option<Tonemap> {
        let key = |leaf: &str| self.scalar(&format!("{prefix}.{leaf}"));
        Some(Tonemap {
            exposure_minimum: key("Exposure minimum")?,
            exposure_maximum: key("Exposure maximum")?,
            exposure_response: key("Exposure response")?,
            exposure_time: key("Exposure time")?,
            luminance_a: key("Luminance a-coefficient")?,
            luminance_b: key("Luminance b-coefficient")?,
            source_colour_end_a: key("Source color end a-coefficient")?,
            source_colour_end_b: key("Source color end b-coefficient")?,
            start_angle: key("Start angle")?,
            end_angle: key("End angle")?,
        })
    }

    /// Four bytes, for a key written as integers in `0..=255`. `None` when
    /// absent, not four numbers, written with decimal points or out of range, so
    /// only [`SKY_COLOUR`] answers.
    #[must_use]
    pub fn rgba8(&self, key: &str) -> Option<[u8; 4]> {
        let value = self.entries.get(key)?;
        if !value.is_integer {
            return None;
        }
        let [r, g, b, a] = value.numbers.as_slice() else {
            return None;
        };
        let byte = |n: &f32| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "the range check above is what makes the cast exact"
            )]
            (n.is_finite() && (0.0..=255.0).contains(n)).then_some(*n as u8)
        };
        Some([byte(r)?, byte(g)?, byte(b)?, byte(a)?])
    }

    /// A `0`/`1` flag.
    #[must_use]
    pub fn flag(&self, key: &str) -> Option<bool> {
        let value = self.entries.get(key)?;
        match (value.is_integer, value.numbers.as_slice()) {
            (true, [only]) if *only == 0.0 || *only == 1.0 => Some(*only != 0.0),
            _ => None,
        }
    }

    /// A unit direction, or `None` when the triple has no direction.
    ///
    /// **Normalising is done here because the file does not.** Over the 32
    /// circuits [`SUN_DIRECTION`]'s length takes eight values (0.0, 1.0, 1.208,
    /// 1.784, 2.375, 4.743, 7.348, 34.641), so assuming a unit vector is wrong on
    /// most and using the raw triple scales the light by accident. Whether the
    /// length carries an intensity is **not** established.
    ///
    /// The zero-length case is real: `modesto_heights` authors a `Physical Sun
    /// direction` of `-0.000030 0.000040 -0.000060`, a disabled field; `None`
    /// stops it becoming a NaN in a shader.
    #[must_use]
    pub fn direction(&self, key: &str) -> Option<[f32; 3]> {
        let [x, y, z] = self.vec3(key)?;
        let length = (x * x + y * y + z * z).sqrt();
        // Two quantisation steps below the smallest real length (1.0); under this is an off switch.
        if !length.is_finite() || length < 1e-3 {
            return None;
        }
        Some([x / length, y / length, z / length])
    }
}

#[cfg(test)]
mod tests;
