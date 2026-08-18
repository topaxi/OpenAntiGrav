//! `.envsettings`: the lighting, fog and tone rig a Wipeout HD circuit authors.
//!
//! # Why this matters more than its size suggests
//!
//! **The renderer's light rig is a stand-in, and this is what it stands in
//! for.** `crates/render/src/mesh.wgsl` says so in its own header - a fixed
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

/// `"Lighting.Sky colour"`. **Four bytes**, not floats - see the module docs.
pub const SKY_COLOUR: &str = "Lighting.Sky colour";

/// `"Lighting.Sky rotation"`. One number, degrees, -40 to 300.
pub const SKY_ROTATION: &str = "Lighting.Sky rotation";

/// `"Fog.Fog Color"`. Three numbers, reaching 1.85.
pub const FOG_COLOUR: &str = "Fog.Fog Color";

/// `"Fog.Fog Density"`. One number, 0.0003 to 0.03 - an **exponential**
/// coefficient, where this project's fog uniform is a linear near/far ramp.
pub const FOG_DENSITY: &str = "Fog.Fog Density";

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
