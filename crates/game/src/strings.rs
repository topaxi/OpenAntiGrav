//! Project-owned UI strings, layered over the disc's own [`StringTable`].
//!
//! The disc's localisation works and is already read - see
//! [`crate::boot::load_strings`] - but it covers only the disc's own ids.
//! Everything this project has added on top (`assets/ui/menu.toml`'s labels,
//! [`crate::race::mode::fallback_label`]-style fallbacks, the loading screen's
//! prose, ...) is plain English with no id-based indirection at all, and the
//! disc's own table has no way to be *overridden* either - a wrong or missing
//! entry can currently only be fixed by shipping different disc data, which
//! this project cannot do (ADR-0006). See
//! `handover/invented-ui-text-has-no-translation-and-the.md`.
//!
//! This module is the file format and the merge, not the fix: nothing yet
//! looks up a project-invented string by id (that is still literal `&str` at
//! every one of the four call sites the handover thread names), so what lands
//! here only starts mattering once something does. What it already does for
//! free, because [`crate::boot::load_strings`] wires it in: a project file
//! can **override** a disc idstring an existing widget already resolves - see
//! [`overlay`].
//!
//! # Format
//!
//! One TOML file per language, named after [`crate::language::Language::name`]
//! lowercased - `assets/ui/strings/english.toml` - under a single `[strings]`
//! table of `id = "value"` pairs, the same shape `menu.toml`'s own header
//! reserves for a `string_id`. The id space is shared with the disc's own
//! `idstring`s on purpose: a project file names a disc id to override it, or a
//! new id of its own to give a translatable home to text this project
//! invented.
//!
//! Embedded with [`include_str!`] rather than read from disk, for the same
//! reason [`crate::menu::BUILT_IN`] is: the binary works from anywhere. A
//! language with no file yet - every language but English, today - overlays
//! nothing, which is the honest state of a translation not yet written rather
//! than something to guess at.

use std::collections::HashMap;

use serde::Deserialize;

use crate::language::StringTable;

#[derive(Debug, Default, Deserialize)]
struct File {
    #[serde(default)]
    strings: HashMap<String, String>,
}

/// The embedded file for `language`, matched case-insensitively against
/// [`crate::language::Language::name`]. `None` for a language this build
/// ships no strings for yet.
fn built_in(language: &str) -> Option<&'static str> {
    if language.eq_ignore_ascii_case("English") {
        Some(include_str!("../../../assets/ui/strings/english.toml"))
    } else {
        None
    }
}

/// Merges `language`'s project file into `table`, project entries winning
/// over whatever `table` already holds - so this belongs *after*
/// [`StringTable::from_xml`], not before it. A line is added to `report`
/// only when there was something to say: a language with no file overlays
/// nothing and stays silent, the same way [`crate::boot::load_strings`]
/// already treats "no strings" as unremarkable.
pub fn overlay(table: &mut StringTable, language: &str, report: &mut Vec<String>) {
    let Some(text) = built_in(language) else {
        return;
    };
    match toml::from_str::<File>(text) {
        Ok(file) if file.strings.is_empty() => {}
        Ok(file) => {
            report.push(format!(
                "assets/ui/strings/{}.toml: {} override(s)",
                language.to_lowercase(),
                file.strings.len()
            ));
            table.merge(file.strings);
        }
        Err(e) => report.push(format!(
            "assets/ui/strings/{}.toml: {e}",
            language.to_lowercase()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_language_with_no_file_overlays_nothing() {
        let mut table = StringTable::from_xml(r#"<StringTable></StringTable>"#);
        let mut report = Vec::new();
        overlay(&mut table, "Klingon", &mut report);
        assert!(table.is_empty());
        assert!(report.is_empty());
    }

    #[test]
    fn english_parses_and_is_empty_today() {
        // Not a claim that it always will be - once something looks up a
        // project string by id, this file is where its English text lands,
        // and this test starts failing in a way that says so.
        let mut table = StringTable::default();
        let mut report = Vec::new();
        overlay(&mut table, "English", &mut report);
        assert!(table.is_empty());
        assert!(report.is_empty());
    }

    #[test]
    fn a_project_entry_overrides_a_disc_entry_of_the_same_id() {
        let mut table = StringTable::from_xml(
            r#"<StringTable><Entry ID="FE_CONFIRM" String="Confirm"></Entry></StringTable>"#,
        );
        table.merge(HashMap::from([(
            "FE_CONFIRM".to_string(),
            "OK".to_string(),
        )]));
        assert_eq!(table.get("FE_CONFIRM"), Some("OK"));
    }

    #[test]
    fn a_malformed_file_is_an_error_not_a_panic() {
        assert!(toml::from_str::<File>("not = [valid").is_err());
    }
}
