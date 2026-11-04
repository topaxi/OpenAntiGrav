//! The pad rumble patterns a race fires, read from the title's archive.
//!
//! The map names the patterns ([`oag_input::rumble::map::pattern_names`]); this
//! finds each in the archive and reports the ones the title does not have.

use anyhow::{Result, bail};
use oag_assets::Archives;
use oag_tables::rumble::{Library, Pattern};

/// Spellings of the pattern directory across the titles' archives
/// (`docs/formats/rumble-xml.md`, "Census"); each archive looks entries up by
/// the digest of one exact path.
const DIRECTORIES: [&str; 5] = [
    "Data/XML/Rumble",
    "Data/xml/rumble",
    "Data/xml/Rumble",
    "data/xml/rumble",
    "data/xml/Rumble",
];

/// Reads the archive's pattern called `name`, trying each spelling of its path.
///
/// # Errors
///
/// No spelling resolves, or the file is refused by the reader.
pub fn disc_pattern(archives: &mut Archives, name: &str) -> Result<Pattern> {
    for directory in DIRECTORIES {
        let path = format!("{directory}/{name}.xml");
        for candidate in [path.replace('/', "\\"), path] {
            if let Ok(blob) = archives.read_name(&candidate) {
                return oag_tables::rumble::parse(&blob)
                    .map_err(|e| anyhow::anyhow!("{candidate}: refused: {e}"));
            }
        }
    }
    bail!("no rumble pattern called {name:?} on this disc")
}

/// Every pattern the map fires that the archive has. A title with none gets an
/// empty library and one report line; a pattern the reader refuses is named in
/// `report` and left out, so its rows play nothing.
pub(crate) fn load(archives: &mut Archives, report: &mut Vec<String>) -> Library {
    let mut library = Library::new();
    let mut missing = 0usize;
    for name in oag_input::rumble::map::pattern_names() {
        match disc_pattern(archives, name) {
            Ok(pattern) => {
                library.insert(name, pattern);
            }
            Err(e) if e.to_string().starts_with("no rumble pattern") => missing += 1,
            Err(e) => report.push(format!("rumble: {e}")),
        }
    }
    if library.is_empty() {
        report.push("rumble: this title ships no pad patterns".to_string());
    } else if missing > 0 {
        report.push(format!(
            "rumble: {missing} of the mapped patterns are not in this title's archive"
        ));
    }
    library
}
