//! Project-owned UI strings, layered over the disc's own [`StringTable`].
//!
//! The disc's localisation works and is already read - see
//! [`crate::boot::load_strings`] - but it covers only the disc's own ids.
//! Everything this project has added on top of it (`assets/ui/menu.toml`'s
//! `label`/`title`, [`oag_race::Mode::fallback_label`]-style fallbacks, the
//! loading screen's prose, ...) is wired through this module's [`overlay`]
//! now, project entries winning over a matching disc id.
//!
//! **`just check-strings` (`scripts/check-strings.py`) is the gate that keeps
//! it wired.** It fails if a `string_id`/`title_string_id` referenced
//! anywhere resolves to nothing in this file, or if a *new*
//! `assets/ui/menu.toml` row or page names none at all - offline, at gate
//! time, the same shape `check-size`'s file-length ceiling already is for a
//! different kind of debt. English text with no id is only tolerated on the
//! rows that script's own `BASELINE` names.
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
//! language with no file yet - every language but English and French, today -
//! overlays nothing, which is the honest state of a translation not yet
//! written rather than something to guess at.
//!
//! # A translation that has not caught up yet
//!
//! English is the base language: every id used anywhere must have real text
//! here, and `check-strings.py` enforces that with no exception. A *second*
//! language's file is allowed to lag - a translator has not reached an id
//! yet - but only by **naming the gap**, under its own `[untranslated]` table
//! (`ids = ["OAG_...", ...]`), never by omitting the id silently. See
//! `assets/ui/strings/english.toml`'s own doc comment for the worked example,
//! and never fill either kind of gap with a machine translation - a marked
//! placeholder is honest, an invented one is not.

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
    } else if language.eq_ignore_ascii_case("French") {
        Some(include_str!("../../../assets/ui/strings/french.toml"))
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

/// A disc-independent table: `language`'s project file, and nothing else.
///
/// For the three sources that have no disc-derived `StringTable` to merge
/// into at all - `menu.toml`, the loading screen's prose, the window's own
/// title - because every one of them runs before a disc is open. Falls back
/// to `"English"` on `None`, the one-language analogue of
/// [`crate::boot::chosen_language`]'s own fallback, since there is no
/// `languages` list here to fall further back through.
///
/// [`overlay`] stays the one merge primitive underneath this and
/// [`crate::boot::load_strings`] both - this only changes what it starts
/// from.
#[must_use]
pub fn project_table(language: Option<&str>) -> StringTable {
    let mut table = StringTable::default();
    let mut report = Vec::new();
    overlay(&mut table, language.unwrap_or("English"), &mut report);
    table
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

    /// **This is the test that used to assert the file was empty**, with a
    /// comment saying it would start failing the moment something looked a
    /// project string up by id. That happened: the pilot editor's CRUD rows
    /// and the on-screen keyboard it opens are the first screen in this build
    /// whose text is looked up rather than written into the widget.
    ///
    /// What it pins now is the property that still matters - every id in the
    /// file is one of ours. The `OAG_` prefix is what keeps a project entry
    /// from colliding with a disc idstring, and this file shares one id space
    /// with the disc's own by design. An id landing here without it would
    /// silently override a real front-end string on somebody's disc.
    ///
    /// It deliberately does **not** list the ids: that would be a second
    /// place to update for every string added, which is exactly the kind of
    /// bookkeeping nobody does.
    #[test]
    fn every_english_entry_is_one_of_ours_and_reads_back() {
        let mut table = StringTable::default();
        let mut report = Vec::new();
        overlay(&mut table, "English", &mut report);
        // The report is a *count*, not an error list - it used to be empty
        // only because the file was.
        assert_eq!(report.len(), 1, "{report:?}");
        assert!(report[0].contains("english.toml"), "{report:?}");
        assert!(!table.is_empty(), "the file has entries now");
        // One of them, spot-checked, so "not empty" cannot pass on a file
        // that parsed into something unrelated.
        assert_eq!(table.get("OAG_PILOT_RENAME"), Some("RENAME"));

        let text = built_in("English").expect("English ships a file");
        let file: File = toml::from_str(text).expect("the shipped file must parse");
        for id in file.strings.keys() {
            assert!(
                id.starts_with("OAG_"),
                "{id:?} has no OAG_ prefix, so it would override a disc idstring \
                 of that name rather than adding one of ours"
            );
        }
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

    #[test]
    fn project_table_defaults_to_english_with_no_language_named() {
        // Both empty today, so this only proves they take the same path -
        // see `english_parses_and_is_empty_today` for why that is expected.
        assert_eq!(
            project_table(None).len(),
            project_table(Some("English")).len()
        );
    }

    #[test]
    fn project_table_is_empty_for_a_language_this_build_ships_no_file_for() {
        assert!(project_table(Some("Klingon")).is_empty());
    }

    /// **The check that catches a shipped file `built_in()` forgot to embed.**
    /// `check-strings.py` validates every `assets/ui/strings/*.toml` on disk,
    /// but nothing there proves the *binary* ever reads one back - a file
    /// this script covers and `built_in()` does not match by name would pass
    /// the gate while shipping no translation at all, the same "kept in step
    /// by hand" gap that script's own module doc already admits to for
    /// `STRING_CONSUMERS`.
    #[test]
    fn every_file_under_assets_ui_strings_is_reachable_through_built_in() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/ui/strings");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).expect("assets/ui/strings must exist") {
            let path = entry.expect("a readable dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .expect("a .toml file has a stem");
            assert!(
                built_in(stem).is_some(),
                "{stem}.toml exists under assets/ui/strings/ but built_in() does not embed it"
            );
            checked += 1;
        }
        assert!(checked >= 2, "expected at least english and french here");
    }

    /// French translates what the maintainer was confident about and defers
    /// the rest under `[untranslated]` - see `french.toml`'s own header. A
    /// deferred id must not appear in the merged table at all, so a caller's
    /// own English fallback (or the disc's own French entry, on the boot
    /// path) is what a player actually sees for it.
    #[test]
    fn french_translates_a_confident_id_and_defers_an_unconfident_one() {
        let mut table = StringTable::default();
        let mut report = Vec::new();
        overlay(&mut table, "French", &mut report);
        assert_eq!(table.get("OAG_MENU_RACE"), Some("COURSE"));
        assert_eq!(table.get("OAG_CONTROLS_SIDESHIFT"), None);
    }
}
