//! Shared by the AI-table ground truths: where a title keeps its two AI files
//! and what the readers make of each one. Names, structure and outcomes only;
//! no tuning value is written anywhere in these tests (ADR-0006).

#![allow(dead_code)]

use oag_assets::Archive;
use oag_assets::psarc;
use oag_tables::ai_control_stats;
use oag_tables::ai_race_stats::{self, ClassRace, Error};
use oag_tables::fexml;
use oag_tables::handling::SpeedClass;

/// The class elements an AI file can carry, in the order the files list them.
pub const CLASS_ELEMENTS: [&str; 6] = [
    "VectorStats",
    "VenomStats",
    "FlashStats",
    "RapierStats",
    "PhantomStats",
    "SuperPhantomStats",
];

/// A PSP/PS2 WAD on a disc image, `None` when the image is absent.
pub fn wad(image: &str, path: &str) -> Option<Archive> {
    let disc = oag_testdata::image(image)?;
    let spec = format!("{}:{path}", disc.display());
    Some(Archive::open(&spec).unwrap_or_else(|e| panic!("{spec}: {e}")))
}

pub fn pulse_psp(image: &str) -> Option<Archive> {
    wad(image, "PSP_GAME/USRDIR/Data.wad")
}

pub fn pulse_ps2() -> Option<Archive> {
    wad("pulse-ps2-eu.chd", "54748/WADS2.WAD")
}

pub fn hd_disc(archive: &str) -> Option<psarc::Archive> {
    let iso = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", iso.display());
    Some(psarc::Archive::open(&spec).unwrap_or_else(|e| panic!("{spec}: {e}")))
}

pub fn loose(path: &str) -> Option<psarc::Archive> {
    let file = oag_testdata::exact(path)?;
    Some(psarc::Archive::open_file(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display())))
}

/// Every `AIControlStats*`/`AIRaceStats*` entry of a PSARC, by lower-cased base
/// name, sorted. A PSARC lists its paths, so this is a census of the archive
/// itself rather than a lookup of names the project already knows.
pub fn psarc_ai_files(archive: &mut psarc::Archive) -> Vec<(String, Vec<u8>)> {
    let mut paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| {
            let base = base_name(p);
            base.starts_with("aicontrolstats") || base.starts_with("airacestats")
        })
        .cloned()
        .collect();
    paths.sort_by_key(|p| base_name(p));
    paths
        .into_iter()
        .map(|p| {
            let blob = archive.read_path(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
            (base_name(&p), blob)
        })
        .collect()
}

pub fn base_name(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .next()
        .unwrap()
        .to_ascii_lowercase()
}

/// The class elements `text` carries.
pub fn classes_in(text: &str) -> Vec<&'static str> {
    CLASS_ELEMENTS
        .into_iter()
        .filter(|c| text.contains(&format!("<{c}>")))
        .collect()
}

/// What the controller reader makes of one file: `ok(<classes>)` or the first
/// refusal, by name.
pub fn control_outcome(blob: &[u8]) -> String {
    let text = fexml::text(blob).expect("expands");
    let classes = classes_in(&text);
    for class in &classes {
        if let Err(e) = ai_control_stats::parse_element(&text, class) {
            return e.to_string();
        }
    }
    format!("ok({})", classes.len())
}

/// The same for the race reader.
pub fn race_outcome(blob: &[u8]) -> String {
    let text = fexml::text(blob).expect("expands");
    let classes = classes_in(&text);
    for class in &classes {
        if let Err(e) = ai_race_stats::parse_class_race(&text, class) {
            return e.to_string();
        }
    }
    format!("ok({})", classes.len())
}

/// `(base name, outcome)` for every AI file of the matching family in `files`.
pub fn outcomes(
    files: &[(String, Vec<u8>)],
    family: &str,
    outcome: fn(&[u8]) -> String,
) -> Vec<(String, String)> {
    files
        .iter()
        .filter(|(name, _)| name.starts_with(family))
        .map(|(name, blob)| (name.clone(), outcome(blob)))
        .collect()
}

pub fn race_class(blob: &[u8], class: SpeedClass) -> Result<ClassRace, Error> {
    ai_race_stats::full_from_blob(blob, class)
}
