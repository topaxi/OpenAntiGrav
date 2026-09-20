//! `EngineLightData.xml`: where Wipeout HD hangs a ship's engine light, and
//! how far it reaches.
//!
//! # What it is
//!
//! Two numbers per ship directory, and nothing else - measured on all 37
//! files the Fury disc ships (`crates/tables/tests/enginelight_ground_truth.rs`):
//!
//! ```xml
//! <?xml version="1.0" encoding="UTF-8"?>
//! <EngineLightData>
//!     <Distance>0.4f</Distance>
//!     <Radius>2.0f</Radius>
//! </EngineLightData>
//! ```
//!
//! `Ship_LoadEngineLightData` (`0x000d25e0` in `ps3-hdfury-eu`, confidence
//! 72 on the name, 88 on the read) opens `%s\EngineLightData.xml` against the
//! ship's own directory, finds the `EngineLightData` root and stores the two
//! children at `ship+0xfc`/`ship+0x100`. Their one consumer is
//! `EngineFlare_SubmitSpuLight` (`0x0029ff28`, confidence 85), which places
//! one SPU vertex light per craft per frame at
//! `flare_node.position - flare_node.z_axis * Distance` with range
//! `Radius + U(-0.1, 0.1)` - see
//! `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The captured buffer's
//! producer is found". So `Distance` is a slide *back along the nozzle
//! axis* (negative pulls the light into the hull - `feisar_c1` authors
//! `-0.4f`) and `Radius` is the light's falloff range in world units.
//!
//! # Why this is not [`fexml`](crate::fexml)
//!
//! The values are **element text**, not attributes, and [`crate::fexml::parse`]
//! discards text by design (no front-end schema carries any). This file is
//! three tags deep and has no attributes at all, so it gets its own reader
//! rather than a text-carrying mode bolted onto a parser every other table
//! relies on.
//!
//! # The number spelling is a C float literal, inconsistently
//!
//! Four spellings occur across the 37 files: `0.4f`, `1f`, `0.3` and
//! `-0.4f`. The trailing `f` is optional and so is the decimal point.
//! `str::parse::<f32>` rejects the first two, so [`number`] strips one
//! trailing `f`/`F` before parsing - and nothing else, so a value the
//! original's `atof` would not have read still fails here rather than
//! silently reading as zero.

/// One ship directory's engine light, as authored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineLightData {
    /// `<Distance>`: how far behind the flare node, along the node's own Z
    /// axis, the light sits. Authored `-0.4` to `1.5` across the disc.
    pub distance: f32,
    /// `<Radius>`: the light's range `D` in world units - the `1 - |d| / D`
    /// falloff reaches zero here. Authored `0.7` to `2.0` across the disc.
    pub radius: f32,
}

/// Why a file did not read as [`EngineLightData`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No `<EngineLightData>` element.
    NoRoot,
    /// The named child is absent.
    Missing(&'static str),
    /// The named child's text is not a float literal.
    NotANumber(&'static str, String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRoot => write!(f, "no <EngineLightData> element"),
            Self::Missing(name) => write!(f, "<EngineLightData> has no <{name}> child"),
            Self::NotANumber(name, text) => {
                write!(f, "<{name}> holds {text:?}, which is not a float literal")
            }
        }
    }
}

impl std::error::Error for Error {}

/// The archive entry name for a ship directory's engine light, under the
/// directory the title keeps its roster in - the same shape as
/// [`crate::handling::entry_name_in`], for the same reason.
///
/// The original formats `%s\EngineLightData.xml` against the ship's own
/// path; the archive lookup is case-insensitive, so the on-disc
/// `enginelightdata.xml` resolves.
#[must_use]
pub fn entry_name_in(dir: &str, team: &str) -> String {
    format!(r"{dir}\{team}\EngineLightData.xml")
}

/// [`entry_name_in`] under [`crate::handling::SHIP_DIR`].
#[must_use]
pub fn entry_name(team: &str) -> String {
    entry_name_in(crate::handling::SHIP_DIR, team)
}

/// Reads one `EngineLightData.xml`.
///
/// # Errors
///
/// [`Error`] when the root or either child is missing, or a value is not a
/// float literal. A file that reads is never partially read: both numbers or
/// neither.
pub fn parse(xml: &str) -> Result<EngineLightData, Error> {
    let root = element_text(xml, "EngineLightData").ok_or(Error::NoRoot)?;
    let distance = child(root, "Distance")?;
    let radius = child(root, "Radius")?;
    Ok(EngineLightData { distance, radius })
}

fn child(root: &str, name: &'static str) -> Result<f32, Error> {
    let text = element_text(root, name).ok_or(Error::Missing(name))?;
    number(text).ok_or_else(|| Error::NotANumber(name, text.trim().to_string()))
}

/// The text between `<name>` and `</name>`, case-insensitive on the tag, or
/// `None` when either tag is absent.
fn element_text<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let lower = xml.to_ascii_lowercase();
    let start = lower.find(&open.to_ascii_lowercase())? + open.len();
    let end = start + lower[start..].find(&close.to_ascii_lowercase())?;
    Some(&xml[start..end])
}

/// A C float literal: optional sign, digits with an optional fraction, and an
/// optional trailing `f`. `1f`, `0.3` and `-0.4f` all read.
fn number(text: &str) -> Option<f32> {
    let text = text.trim();
    let text = text
        .strip_suffix('f')
        .or_else(|| text.strip_suffix('F'))
        .unwrap_or(text);
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIRANHA: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<EngineLightData>\n\t\
        <Distance>0.4f</Distance>\n\t<Radius>2.0f</Radius>\n</EngineLightData>\n";

    #[test]
    fn a_shipped_file_reads_both_numbers() {
        assert_eq!(
            parse(PIRANHA),
            Ok(EngineLightData {
                distance: 0.4,
                radius: 2.0
            })
        );
    }

    /// The four spellings the disc uses, one file each: `0.4f`, `1f`, `0.3`
    /// and `-0.4f`.
    #[test]
    fn every_spelling_on_the_disc_reads() {
        assert_eq!(number("0.4f"), Some(0.4));
        assert_eq!(number("1f"), Some(1.0));
        assert_eq!(number("0.3"), Some(0.3));
        assert_eq!(number("-0.4f"), Some(-0.4));
        assert_eq!(number(" 1.5F "), Some(1.5));
    }

    #[test]
    fn a_missing_child_is_named() {
        let xml = "<EngineLightData><Distance>1f</Distance></EngineLightData>";
        assert_eq!(parse(xml), Err(Error::Missing("Radius")));
    }

    #[test]
    fn a_non_number_is_an_error_not_a_zero() {
        let xml = "<EngineLightData><Distance>far</Distance><Radius>1</Radius></EngineLightData>";
        assert_eq!(
            parse(xml),
            Err(Error::NotANumber("Distance", "far".to_string()))
        );
    }

    #[test]
    fn no_root_is_an_error() {
        assert_eq!(parse("<Ship/>"), Err(Error::NoRoot));
    }

    #[test]
    fn the_entry_name_is_the_originals_format_string() {
        assert_eq!(
            entry_name("feisar_c1"),
            r"Data\Ships\feisar_c1\EngineLightData.xml"
        );
    }
}
