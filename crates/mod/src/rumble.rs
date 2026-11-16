//! A rumble pattern written in TOML (`<name>.toml`, `schemas/v1/rumble.tosd`).
//!
//! The same [`Pattern`] the disc's XML reads into (`oag_tables::rumble`), so a
//! TOML pattern overrides a disc pattern of the same name and everything after
//! the reader sees one shape. The format is `docs/formats/rumble-xml.md`'s
//! "Our own TOML form".
//!
//! ```toml
//! [toml-schema]
//! location = "https://oag.topaxi.com/mods/schemas/v1/rumble.tosd"
//! version = "1.0.0"
//!
//! [pattern]
//! type = "Looping"          # or "OneShot"
//! class = "Game"            # optional: "Game" or "UI"
//! priority = "High"         # optional: "Low", "Medium" or "High"
//! loop = [1, 3]             # optional, "Looping" only
//!
//! [pattern.positional]      # optional: its presence is the Positional behaviour
//! fall_off_inner = 30
//! fall_off_outer = 20
//!
//! [tracks]
//! large = [255, 200, 0]     # one byte per frame
//! small = [1, 0]
//! ```

use oag_tables::rumble::{Class, FallOff, Kind, Pattern, Priority};
use serde::Deserialize;

/// The schema kind the header names.
pub const SCHEMA_KIND: &str = "rumble";
/// The one schema major this build reads.
pub const SCHEMA_MAJOR: u32 = 1;

/// Why a pattern was not read.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    /// No `[toml-schema]` header, or one naming another kind or major.
    #[error(transparent)]
    Schema(#[from] oag_schema::Error),
    /// Not TOML, or a key this reader does not know, or a value of the wrong
    /// type or outside a byte.
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    /// The pattern's own rules (`oag_tables::rumble`) refused it.
    #[error(transparent)]
    Pattern(#[from] oag_tables::rumble::Error),
    /// `loop` is not `[start, end]`.
    #[error("loop must be [start, end]")]
    LoopShape,
    /// A mod ships `<name>.xml` and `<name>.toml`.
    #[error("rumble pattern {0:?} is shipped as both .xml and .toml")]
    BothForms(String),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(rename = "toml-schema")]
    header: Option<oag_schema::Header>,
    pattern: Table,
    #[serde(default)]
    tracks: Tracks,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Table {
    #[serde(rename = "type")]
    kind: KindWord,
    class: Option<ClassWord>,
    priority: Option<PriorityWord>,
    #[serde(rename = "loop")]
    loop_range: Option<Vec<u32>>,
    positional: Option<Positional>,
}

#[derive(Deserialize)]
enum KindWord {
    OneShot,
    Looping,
}

#[derive(Deserialize)]
enum ClassWord {
    Game,
    #[serde(rename = "UI")]
    Ui,
}

#[derive(Deserialize)]
enum PriorityWord {
    Low,
    Medium,
    High,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Positional {
    #[serde(default)]
    fall_off_inner: Number,
    #[serde(default)]
    fall_off_outer: Number,
}

/// A TOML number, integer or float.
#[derive(Deserialize, Default, Clone, Copy)]
#[serde(untagged)]
enum Number {
    #[default]
    Absent,
    Int(i32),
    Float(f32),
}

impl Number {
    #[allow(clippy::cast_precision_loss)]
    fn get(self) -> f32 {
        match self {
            Self::Absent => 0.0,
            Self::Int(i) => i as f32,
            Self::Float(f) => f,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Tracks {
    #[serde(default)]
    large: Vec<u8>,
    #[serde(default)]
    small: Vec<u8>,
}

/// Reads a TOML pattern.
///
/// # Errors
///
/// An [`Error`] naming the missing header, the unknown key or value, or the
/// combination the pattern rules refuse.
pub fn parse(text: &str) -> Result<Pattern, Error> {
    let file: File = toml::from_str(text)?;
    oag_schema::check(file.header.as_ref(), SCHEMA_KIND, SCHEMA_MAJOR)?;
    let table = file.pattern;
    let (loop_start, loop_end) = match table.loop_range.as_deref() {
        None => (0, 0),
        Some(&[start, end]) => (start, end),
        Some(_) => return Err(Error::LoopShape),
    };
    let pattern = Pattern {
        kind: match table.kind {
            KindWord::OneShot => Kind::OneShot,
            KindWord::Looping => Kind::Looping,
        },
        loop_start,
        loop_end,
        class: table.class.map(|c| match c {
            ClassWord::Game => Class::Game,
            ClassWord::Ui => Class::Ui,
        }),
        priority: table.priority.map(|p| match p {
            PriorityWord::Low => Priority::Low,
            PriorityWord::Medium => Priority::Medium,
            PriorityWord::High => Priority::High,
        }),
        positional: table.positional.map(|p| FallOff {
            inner: p.fall_off_inner.get(),
            outer: p.fall_off_outer.get(),
        }),
        large: file.tracks.large,
        small: file.tracks.small,
    };
    pattern.validate()?;
    Ok(pattern)
}

/// The pattern a mod ships under one name, whichever form it is in.
///
/// # Errors
///
/// [`Error::BothForms`] when both are given; otherwise the form's own error.
pub fn from_forms(
    name: &str,
    xml: Option<&[u8]>,
    toml: Option<&str>,
) -> Result<Option<Pattern>, Error> {
    match (xml, toml) {
        (Some(_), Some(_)) => Err(Error::BothForms(name.to_owned())),
        (Some(xml), None) => Ok(Some(oag_tables::rumble::parse(xml)?)),
        (None, Some(toml)) => parse(toml).map(Some),
        (None, None) => Ok(None),
    }
}

#[cfg(test)]
mod tests;
