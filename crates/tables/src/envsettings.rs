//! `.envsettings`: the lighting, fog and tone rig a Wipeout HD circuit authors.
//!
//! # Why this matters more than its size suggests
//!
//! **The renderer's light rig is a stand-in, and this is what it stands in
//! for.** `crates/mesh/src/mesh.wgsl` says so in its own header - a fixed
//! two-light rig with invented directions, chosen so that geometry reads
//! clearly rather than to reproduce the game's look. On HD the game's look is
//! authored, in plain text, one file per circuit: a sun direction, a sun
//! colour, a constant ambient, a fog colour and density, and a tonemapper's
//! parameters. [`CLAUDE.md`](../../../CLAUDE.md)'s rule about not inventing
//! what the assets already author is exactly this case.
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
//! The dotted prefix groups the keys - `Lighting`, `Fog`, `Water`,
//! `HDR and Bloom` - and is part of the key rather than a section header,
//! because the file has no section syntax and the same leaf name never repeats
//! under two prefixes.
//!
//! # Two number encodings, and the file distinguishes them by formatting
//!
//! **A parser that reads every value as a `0..=1` float gets one key wrong by a
//! factor of 255.** `"Lighting.Sky colour"=128 128 128 0` is a byte quadruple;
//! everything else that looks like a colour is written with a decimal point and
//! is already normalised. Measured over all 32 circuit files: `Sky colour` is
//! the *only* key written without a decimal point whose range reaches 255, and
//! the four other integer-formatted keys - `Radial bloom Enabled`,
//! `Enable dynamic lights`, `Enable spu vertex lights` and `Use Lens Flare` -
//! are all `0`/`1` flags. So [`Value::is_integer`] records the formatting and
//! [`EnvSettings::rgba8`] is a separate accessor from [`EnvSettings::vec3`].
//!
//! # It is authored for a linear HDR pipeline, and this project's is not
//!
//! `"Lighting.Sun color"` reaches **4.0** across the corpus and
//! `"Lighting.Constant ambient color"` reaches **3.0**, beside
//! `"HDR and Bloom.Tone maximum brightness"=4.0`. Those are only meaningful to
//! a renderer that works in linear light and tonemaps; this one is
//! gamma-authoritative by
//! [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md)
//! and has no tonemap stage. **That is a fact about the file, and it is
//! recorded here rather than acted on** - see `docs/formats/envsettings.md` for
//! which fields a caller can use today and which need headroom this pipeline
//! does not have.
//!
//! # Confidence
//!
//! **90 on the syntax and the key set**, on exact agreement across all 33 files
//! on the disc: 32 circuits sharing a 39-key schema, plus `/data/fe/fury.envsettings`,
//! which is a different thing entirely - 436 lines of feedback, equaliser and
//! music-pulse keys for the Fury front end. Nothing here reads HD's executable,
//! so **what the original does with any value is not decoded**; the names are
//! the file's own rather than recovered.
//!
//! # Wipeout 2048 authors the same shape under different key names
//!
//! Every base and DLC circuit ships a `track.EnvSettings` in the same
//! `"Key.Subkey"=float [float...]` syntax this module already parses - the
//! *format* needs no title-specific code. The *keys* do: 2048's registrar
//! (`Environment_RegisterLightingSchema` in `vita-2048-eu-v104/eboot.elf`,
//! confidence 85 - see
//! [`lighting-schema.md`](../../../docs/ghidra/functions/vita-2048-eu-v104/lighting-schema.md))
//! spells the ambient term `"Lighting.Constant ambient colour"` (British,
//! HD's is `"Lighting.Constant ambient color"`) and splits HD's single
//! `"Lighting.Sun color"` into `"Lighting.Sun diffuse colour"` and
//! `"Lighting.Sun specular colour"` - genuinely different keys, not a
//! reformatting, confirmed by reading the registrar's own string table rather
//! than guessed from the file. [`PSP2_AMBIENT_COLOUR`] and
//! [`PSP2_SUN_DIFFUSE_COLOUR`] are 2048's own spellings of the two terms
//! [`AMBIENT_COLOUR`] and [`SUN_COLOUR`] answer for HD. [`SUN_DIRECTION`] is
//! spelled identically on both titles and needs no title-specific constant.
//!
//! **2048's file additionally authors a literal `"Lighting.Sun color"` key,
//! spelled exactly like HD's, and it is not this project's diffuse term.**
//! The registrar binds it to its own field, separate from `Sun diffuse
//! colour`/`Sun specular colour`, so it is not a parser alias - but across
//! the 14 circuit files sampled, five (`Anulpha_Pass`, `Chenghou_Project`,
//! `bridge`, `park`, `sol`) carry the *exact* value the registrar's own
//! compiled-in default initialises the field to (`~1.5, 1.3, 1.0`), the kind
//! of untouched default an artist would leave behind on a field the picture
//! does not visibly depend on - corroborating, not proving, that it is a
//! vestigial field the shading path does not read. A displacement-based
//! consumer search (this binary accesses this struct's region through
//! `Environment_RegisterLightingSchema()`'s own return pointer plus a fixed
//! offset, not through the field's absolute address - the same trap
//! `docs/formats/envsettings.md`'s HD notes record for a TOC-relative load)
//! did not resolve a reader in the time this pass spent on it. **Do not wire
//! this key as a light-rig colour on the strength of its name alone** - see
//! `docs/formats/envsettings.md`'s 2048 section for the full account.

use std::collections::BTreeMap;

/// `"Lighting.Sun direction"`. Three numbers, unit on most circuits and not on
/// all - see [`EnvSettings::direction`].
pub const SUN_DIRECTION: &str = "Lighting.Sun direction";

/// `"Lighting.Sun color"`. Three numbers, reaching 4.0 across the corpus.
pub const SUN_COLOUR: &str = "Lighting.Sun color";

/// `"Lighting.Sun specular scale"`. One number, 1.0 to 3.75.
pub const SUN_SPECULAR_SCALE: &str = "Lighting.Sun specular scale";

/// `"Lighting.Constant ambient color"`. Three numbers, reaching 3.0.
pub const AMBIENT_COLOUR: &str = "Lighting.Constant ambient color";

/// Wipeout 2048's own spelling of [`AMBIENT_COLOUR`] - see the module docs'
/// "Wipeout 2048 authors the same shape under different key names" section.
pub const PSP2_AMBIENT_COLOUR: &str = "Lighting.Constant ambient colour";

/// Wipeout 2048's own diffuse sun term - the field HD's [`SUN_COLOUR`]
/// answers for, under a different key. **Not the same key as 2048's own
/// literal `"Lighting.Sun color"`** - see the module docs for why that one is
/// left unwired.
pub const PSP2_SUN_DIFFUSE_COLOUR: &str = "Lighting.Sun diffuse colour";

/// `"Lighting.Prelit ambient colour scale"`. Three numbers. The scale the
/// circuit's own fragment microcode multiplies the powed lightmap by -
/// `prelit = scale * lightmap.rgb ^ power` - see
/// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`.
pub const PRELIT_SCALE: &str = "Lighting.Prelit ambient colour scale";

/// `"Lighting.Prelit ambient colour power"`. Three numbers, the exponent in
/// the same term [`PRELIT_SCALE`] scales.
pub const PRELIT_POWER: &str = "Lighting.Prelit ambient colour power";

/// Wipeout: Omega Collection's `"Lighting.Nova prelit scale bias power"`.
/// **One triple of scalars**, `(scale, bias, power)` in file order, not three
/// per-channel vectors: its pixel shaders compute
/// `scale * pow(lightmap.rgb, power) + bias` on every channel - see
/// `docs/ghidra/functions/ps4-omega-eu/lightmap-prelit.md`. Authored by 88 of
/// the title's 97 files (patch and base), 20 distinct triples.
pub const NOVA_PRELIT: &str = "Lighting.Nova prelit scale bias power";

/// What [`NOVA_PRELIT`] is when a file omits it: the executable's own static
/// default, `(1.4, 0.2, 1.5)` (`FUN_015c2dc0`, `0x01e3de80`), not the identity.
pub const NOVA_PRELIT_DEFAULT: [f32; 3] = [1.4, 0.2, 1.5];

/// The `Tonemap.*` (or `TonemapHDR.*`) block Wipeout: Omega Collection authors.
///
/// The ten keys the executable registers (`FUN_015c1f20`) under both prefixes.
/// **Read, and no consumer is located** - see
/// `docs/ghidra/functions/ps4-omega-eu/lightmap-prelit.md`, "The `Tonemap.*`
/// block". Nothing in this project applies it.
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

/// `"Fog.Fog Density"`. One number, 0.0003 to 0.03. The circuit materials'
/// own fragment microcode applies its coefficient as
/// `exp(-(coefficient * view_depth)^2)`; whether this value reaches that
/// coefficient unscaled is not read.
pub const FOG_DENSITY: &str = "Fog.Fog Density";

/// Wipeout 2048's `"Lighting.Fog colour"`. **Four numbers**: a colour and a
/// fourth, 0.0001 to 0.0021 on every circuit that authors it, which is the
/// same order as HD's [`FOG_DENSITY`] (Anulpha Pass 0.0004 on both titles).
/// The shader interface names one `float4` `fogColour` in both titles; HD's
/// fourth component is the coefficient of its curve. Whether 2048's is the
/// same quantity is read as an inheritance, not measured - see the module
/// docs.
pub const PSP2_FOG_COLOUR: &str = "Lighting.Fog colour";

/// `"HDR and Bloom.Bloom adaption rate"`. One number: the per-frame lerp
/// rate of the adapted average luminance -
/// `adapted += rate * (luma(mean) - adapted)`, read out of `FUN_003b4690`.
/// The settings-block registrar at `0x003a83d8` ties every key in this block
/// to its field, which is what makes each mapping here a read rather than a
/// name-shaped guess. See `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`.
pub const BLOOM_ADAPTION_RATE: &str = "HDR and Bloom.Bloom adaption rate";

/// `"HDR and Bloom.Bloom adaption boost"`. One number: scales the adapted
/// luminance in the gate's fade - the luminance bloom term is multiplied by
/// `1 - min(adapted * boost * 0.25, 1)`, `0.25` an inline constant of the
/// executable.
pub const BLOOM_ADAPTION_BOOST: &str = "HDR and Bloom.Bloom adaption boost";

/// `"HDR and Bloom.Tone adaption boost"`. One number: scales the adapted
/// luminance in the read exposure -
/// `scale = maximum_brightness - min(adapted * this, darkening_clamp)` -
/// applied as the resolve pass's `scale` parameter.
pub const TONE_ADAPTION_BOOST: &str = "HDR and Bloom.Tone adaption boost";

/// `"HDR and Bloom.Tone darkening clamp"`. One number: the cap in the same
/// formula, i.e. how far below the maximum the exposure can fall.
pub const TONE_DARKENING_CLAMP: &str = "HDR and Bloom.Tone darkening clamp";

/// `"HDR and Bloom.Tone maximum brightness"`. One number: the exposure on a
/// black frame.
pub const TONE_MAXIMUM_BRIGHTNESS: &str = "HDR and Bloom.Tone maximum brightness";

/// `"HDR and Bloom.Bloom from alpha contribution"`. One number: the weight of
/// the glow-mask term in `FunkLayerBloomGate_fp`'s read bright pass -
/// `frame.rgb * frame.a * this` - see
/// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`.
pub const BLOOM_ALPHA_CONTRIBUTION: &str = "HDR and Bloom.Bloom from alpha contribution";

/// `"HDR and Bloom.Bloom from frame contribution"`. One number: the weight of
/// the same gate's luminance term - `frame.rgb * lum^exponent * this`.
pub const BLOOM_FRAME_CONTRIBUTION: &str = "HDR and Bloom.Bloom from frame contribution";

/// `"HDR and Bloom.Bloom from frame exponent"`. One number: the luminance
/// term's power.
pub const BLOOM_FRAME_EXPONENT: &str = "HDR and Bloom.Bloom from frame exponent";

/// `"HDR and Bloom.Bloom horizontal size"`. One number: the horizontal blur's
/// tap spacing, the patched parameter `FunkLayerBloomBlurHorizontal_fp` steps
/// its nine taps by.
pub const BLOOM_HORIZONTAL_SIZE: &str = "HDR and Bloom.Bloom horizontal size";

/// `"HDR and Bloom.Bloom vertical size"`. One number: the vertical blur's tap
/// spacing.
pub const BLOOM_VERTICAL_SIZE: &str = "HDR and Bloom.Bloom vertical size";

/// One entry's value: the numbers as written, and how they were written.
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    /// One to four numbers, in file order.
    pub numbers: Vec<f32>,
    /// Whether every number was written without a decimal point.
    ///
    /// The file's only signal that a value is a byte or a flag rather than a
    /// normalised float; see the module docs.
    pub is_integer: bool,
}

/// Everything one `.envsettings` file says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnvSettings {
    /// Every entry, keyed by the unquoted key.
    ///
    /// A `BTreeMap` rather than a `HashMap` because a load report iterates
    /// these and simulation state must never depend on a per-process hasher -
    /// see `docs/architecture/determinism.md`.
    pub entries: BTreeMap<String, Value>,
}

/// A line that is neither blank nor `key=value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// The 1-based line number.
    pub line: usize,
    /// The line, trimmed.
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
    /// **A value that is not a number is dropped and the entry with it**,
    /// rather than defaulting to zero: a zero ambient is a picture, and a
    /// missing key is a caller's decision to make. Nothing on the disc has one.
    ///
    /// # Errors
    ///
    /// [`Error`] for the first line that is neither blank nor `key=value`.
    /// Every line of all 33 files on the disc parses.
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

    /// Three numbers, for a key that carries exactly three.
    ///
    /// **Not clamped and not normalised.** Several of these exceed 1.0 - see
    /// the module docs - and clamping here would hide that from a caller who
    /// has to decide what to do about it.
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

    /// Four bytes, for a key written as integers in `0..=255`.
    ///
    /// `None` when the key is absent, is not four numbers, was written with
    /// decimal points, or leaves the byte range - so [`SKY_COLOUR`] answers and
    /// nothing else on the disc does.
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

    /// A unit direction, or `None` when the authored triple has no direction.
    ///
    /// **Normalising is the caller's job and this does it, because the file
    /// does not.** Across the 32 circuits the length of [`SUN_DIRECTION`] takes
    /// eight distinct values - 0.0, 1.0, 1.208, 1.784, 2.375, 4.743, 7.348 and
    /// 34.641 - so a reader that assumes a unit vector is wrong on most of the
    /// corpus and one that uses the raw triple scales the light by an accident.
    /// Whether the length carries an intensity is **not** established; nothing
    /// here reads the executable.
    ///
    /// The zero-length case is real: `modesto_heights` authors a
    /// `Physical Sun direction` of `-0.000030 0.000040 -0.000060`, which is a
    /// disabled field rather than a direction, and answering `None` is what
    /// stops it from becoming a NaN in a shader.
    #[must_use]
    pub fn direction(&self, key: &str) -> Option<[f32; 3]> {
        let [x, y, z] = self.vec3(key)?;
        let length = (x * x + y * y + z * z).sqrt();
        // Two quantisation steps below the smallest non-degenerate length the
        // corpus holds, which is 1.0; anything under this is an off switch.
        if !length.is_finite() || length < 1e-3 {
            return None;
        }
        Some([x / length, y / length, z / length])
    }
}

#[cfg(test)]
mod tests;
