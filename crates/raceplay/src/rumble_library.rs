//! The pad rumble patterns a race fires, read from the title's archive.
//!
//! The map names the patterns ([`oag_input::rumble::map::pattern_names`]); this
//! finds each in the archive and reports the ones the title does not have.

use anyhow::{Result, bail};
use oag_assets::Archives;
use oag_assets::layers::Kind as LayerKind;
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

/// Where the override and fallback layers keep a TOML pattern.
fn toml_entry(name: &str) -> String {
    format!("Data\\XML\\Rumble\\{name}.toml")
}

/// A TOML pattern out of the override or fallback layer, `None` when no layer
/// holds it or it will not read (named in `report`).
fn layer_pattern(archives: &mut Archives, name: &str, report: &mut Vec<String>) -> Option<Pattern> {
    let entry = toml_entry(name);
    archives.layer_of(&entry)?;
    let read = archives.read_name(&entry).ok()?;
    let text = String::from_utf8_lossy(&read);
    match oag_mod::rumble::parse(&text) {
        Ok(pattern) => Some(pattern),
        Err(e) => {
            report.push(format!("rumble: {entry}: refused: {e}"));
            None
        }
    }
}

/// Every pattern the map fires, each from the first layer that holds it: a
/// player's override (`.xml` or `.toml`), the disc's `.xml`, then this project's
/// own `.toml` in the fallback layer (ADR-0061), which answers only a name the
/// disc lacks. A pattern the reader refuses is named in `report` and left out,
/// so its rows play nothing; a title with no pattern at all plays nothing and
/// the report says so.
pub fn load(archives: &mut Archives, report: &mut Vec<String>) -> Library {
    let mut library = Library::new();
    let (mut disc, mut fallback, mut overridden) = (0usize, 0usize, 0usize);
    let mut missing = Vec::new();
    for name in oag_input::rumble::map::pattern_names() {
        let over = archives.layer_of(&toml_entry(name)) == Some(LayerKind::Override);
        let pattern = if over {
            layer_pattern(archives, name, report).inspect(|_| overridden += 1)
        } else {
            None
        }
        .or_else(|| match disc_pattern(archives, name) {
            Ok(pattern) => {
                disc += 1;
                Some(pattern)
            }
            Err(e) => {
                if !e.to_string().starts_with("no rumble pattern") {
                    report.push(format!("rumble: {e}"));
                }
                None
            }
        })
        .or_else(|| layer_pattern(archives, name, report).inspect(|_| fallback += 1));
        match pattern {
            Some(pattern) => {
                library.insert(name, pattern);
            }
            None => missing.push(name),
        }
    }
    report.push(summary(disc, fallback, overridden, &missing));
    library
}

/// The loader-report line for a library.
fn summary(disc: usize, fallback: usize, overridden: usize, missing: &[&str]) -> String {
    if disc + fallback + overridden == 0 {
        return "rumble: no pattern found for this title, so the pad stays still".to_string();
    }
    let mut parts = Vec::new();
    if disc > 0 {
        parts.push(format!("{disc} from the disc"));
    }
    if fallback > 0 {
        parts.push(format!("{fallback} from this project's own fallback"));
    }
    if overridden > 0 {
        parts.push(format!("{overridden} from overrides"));
    }
    let mut line = format!("rumble: patterns {}", parts.join(", "));
    if !missing.is_empty() {
        line.push_str(&format!("; none for {}", missing.join(", ")));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::summary;

    #[test]
    fn a_title_with_no_pattern_says_so() {
        assert!(summary(0, 0, 0, &["a"]).contains("no pattern found"));
    }

    #[test]
    fn a_title_that_needs_no_fallback_does_not_name_it() {
        assert!(!summary(64, 0, 0, &[]).contains("fallback"));
    }

    #[test]
    fn the_report_counts_each_layer_and_names_what_is_missing() {
        let line = summary(2, 3, 1, &["x", "y"]);
        assert!(
            line.contains("2 from the disc, 3 from this project's own fallback, 1 from overrides")
        );
        assert!(line.ends_with("none for x, y"));
    }
}
