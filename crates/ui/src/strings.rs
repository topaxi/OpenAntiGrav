//! Project-owned UI strings, layered over the disc's own [`StringTable`].
//!
//! The disc's localisation works and is already read - see
//! `crate::boot::load_strings` - but it covers only the disc's own ids.
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
//! table of `ID = { text = "value", human = false }` entries. `human` is
//! required and means a human wrote or approved the text; a model-written or
//! machine-translated entry says `false`. The id space is shared with the
//! disc's own `idstring`s on purpose: a project file names a disc id to
//! override it, or a new id of its own to give a translatable home to text
//! this project invented. The inline-table shape keeps the flag on the line it
//! describes, so a diff of one string is one line and no second table has to
//! be kept in step.
//!
//! # Disc ids, per title
//!
//! `assets/ui/strings/disc/<namespace>/<language>.toml` holds this project's
//! own translations of a *title's* disc text, keyed by that title's idstrings.
//! A namespace is named by [`oag_title::FrontEnd::disc_strings`] and only a
//! title that names it reads it: Pure, HD and 2048 reuse id spellings, so one
//! shared file would turn Pulse's wording on for them. Nothing here may be a
//! copy of a disc sentence (`docs/overview/legal.md`).
//!
//! Embedded with [`include_str!`] rather than read from disk, for the same
//! reason [`crate::menu::BUILT_IN`] is: the binary works from anywhere. A
//! language with no file yet overlays nothing, which is the honest state of a
//! translation not yet written rather than something to guess at.
//!
//! # A translation that has not caught up yet
//!
//! English is the base language: every id used anywhere must have real text
//! here, and `check-strings.py` enforces that with no exception. A *second*
//! language's file is allowed to lag - a translator has not reached an id
//! yet - but only by **naming the gap**, under its own `[untranslated]` table
//! (`ids = ["OAG_...", ...]`), never by omitting the id silently. See
//! `assets/ui/strings/english.toml`'s own doc comment for the worked example.
//! A machine translation may fill a gap, but only marked `human = false`.

use std::collections::HashMap;

use serde::Deserialize;

use crate::language::StringTable;

#[derive(Debug, Default, Deserialize)]
struct File {
    #[serde(default)]
    strings: HashMap<String, Entry>,
}

/// One string and who vouches for it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    text: String,
    /// Required, no default: a missing flag is a parse error, not a silent
    /// `false`. `true` only when a human wrote or approved the text.
    #[allow(
        dead_code,
        reason = "read by scripts/check-strings.py, not by the loader"
    )]
    human: bool,
}

impl File {
    fn texts(self) -> HashMap<String, String> {
        self.strings
            .into_iter()
            .map(|(id, entry)| (id, entry.text))
            .collect()
    }
}

/// A language this build offers on every title, whether or not a disc ships it.
#[derive(Debug)]
pub struct ProjectLanguage {
    /// What it is called, which is also its `assets/ui/strings/` file stem and
    /// what a saved setting stores. Matched case-insensitively, so Omega's own
    /// on-disc `portuguesebr` plugin is the same language.
    pub name: &'static str,
    /// The language's name in itself, for the picker.
    pub native_name: &'static str,
}

/// The languages added after a disc's own. **Chosen, not measured**: Wipeout
/// Omega is the only disc that ships Brazilian Portuguese.
pub const PROJECT_LANGUAGES: &[ProjectLanguage] = &[ProjectLanguage {
    name: "PortugueseBR",
    native_name: "Português (Brasil)",
}];

/// The embedded file for `language`, matched case-insensitively against
/// [`crate::language::Language::name`]. `None` for a language this build
/// ships no strings for yet.
fn built_in(language: &str) -> Option<&'static str> {
    if language.eq_ignore_ascii_case("English") {
        Some(include_str!("../../../assets/ui/strings/english.toml"))
    } else if language.eq_ignore_ascii_case("French") {
        Some(include_str!("../../../assets/ui/strings/french.toml"))
    } else if language.eq_ignore_ascii_case("German") {
        Some(include_str!("../../../assets/ui/strings/german.toml"))
    } else if language.eq_ignore_ascii_case("PortugueseBR") {
        Some(include_str!("../../../assets/ui/strings/portuguesebr.toml"))
    } else {
        None
    }
}

/// The embedded per-title file of disc-keyed translations, for a `namespace`
/// a title's front end names and a `language`.
fn built_in_disc(namespace: &str, language: &str) -> Option<&'static str> {
    let is = |name: &str| language.eq_ignore_ascii_case(name);
    match namespace {
        "pulse" if is("PortugueseBR") => Some(include_str!(
            "../../../assets/ui/strings/disc/pulse/portuguesebr.toml"
        )),
        "pulse" if is("German") => Some(include_str!(
            "../../../assets/ui/strings/disc/pulse/german.toml"
        )),
        "pure" if is("PortugueseBR") => Some(include_str!(
            "../../../assets/ui/strings/disc/pure/portuguesebr.toml"
        )),
        "hd" if is("PortugueseBR") => Some(include_str!(
            "../../../assets/ui/strings/disc/hd/portuguesebr.toml"
        )),
        "2048" if is("PortugueseBR") => Some(include_str!(
            "../../../assets/ui/strings/disc/2048/portuguesebr.toml"
        )),
        "2048" if is("German") => Some(include_str!(
            "../../../assets/ui/strings/disc/2048/german.toml"
        )),
        _ => None,
    }
}

/// Merges `language`'s project file into `table`, project entries winning
/// over whatever `table` already holds - so this belongs *after*
/// [`StringTable::from_xml`], not before it. A line is added to `report`
/// only when there was something to say: a language with no file overlays
/// nothing and stays silent, the same way `crate::boot::load_strings`
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
            table.merge(file.texts());
        }
        Err(e) => report.push(format!(
            "assets/ui/strings/{}.toml: {e}",
            language.to_lowercase()
        )),
    }
}

/// The key a disc-keyed file uses for an id it must not spell out: `h_` and
/// the 32-bit FNV-1a of the exact id string, in eight hex digits.
///
/// Pure keys most strings by their English text, so naming such an id in a
/// committed file would commit disc text. [`overlay_disc`] hashes every id the
/// base table holds and lays a `h_xxxxxxxx` entry over each match. FNV-1a over
/// the exact bytes, not the folding CRC of `oag_formats::wad::hash_name`: Pure
/// has ids that differ only by case.
#[must_use]
pub fn hashed_key(id: &str) -> String {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in id.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("h_{hash:08x}")
}

/// Rewrites a file's `h_xxxxxxxx` keys to the base table's own ids. A plain id,
/// or a hashed key that is itself an id, is left as written.
fn resolve_hashed_keys(
    table: &StringTable,
    texts: HashMap<String, String>,
) -> HashMap<String, String> {
    let is_hashed = |key: &str| key.len() == 10 && key.starts_with("h_");
    if !texts.keys().any(|key| is_hashed(key)) {
        return texts;
    }
    let mut by_hash: HashMap<String, Vec<&str>> = HashMap::new();
    for id in table.ids() {
        by_hash.entry(hashed_key(id)).or_default().push(id);
    }
    let mut out = HashMap::new();
    for (key, text) in texts {
        if is_hashed(&key) && table.get(&key).is_none() {
            for id in by_hash.get(&key).into_iter().flatten() {
                out.insert((*id).to_string(), text.clone());
            }
        } else {
            out.insert(key, text);
        }
    }
    out
}

/// Merges the title's own disc-keyed translations into `table`, before the
/// project's `OAG_` file so that one still wins. A *project* language is
/// overlaid; a language the disc ships itself only has its gaps filled
/// ([`StringTable::fill`]), so the disc's own words are never replaced. Silent for a namespace or
/// language that has no file.
pub fn overlay_disc(
    table: &mut StringTable,
    namespace: &str,
    language: &str,
    report: &mut Vec<String>,
) {
    let Some(text) = built_in_disc(namespace, language) else {
        return;
    };
    match toml::from_str::<File>(text) {
        Ok(file) => {
            let texts = resolve_hashed_keys(table, file.texts());
            report.push(format!(
                "assets/ui/strings/disc/{namespace}/{}.toml: {} translation(s)",
                language.to_lowercase(),
                texts.len()
            ));
            if PROJECT_LANGUAGES
                .iter()
                .any(|project| project.name.eq_ignore_ascii_case(language))
            {
                table.merge(texts);
            } else {
                table.fill(texts);
            }
        }
        Err(e) => report.push(format!(
            "assets/ui/strings/disc/{namespace}/{}.toml: {e}",
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
/// `crate::boot::chosen_language`'s own fallback, since there is no
/// `languages` list here to fall further back through.
///
/// [`overlay`] stays the one merge primitive underneath this and
/// `crate::boot::load_strings` both - this only changes what it starts
/// from.
#[must_use]
pub fn project_table(language: Option<&str>) -> StringTable {
    let mut table = StringTable::default();
    let mut report = Vec::new();
    overlay(&mut table, language.unwrap_or("English"), &mut report);
    table
}

#[cfg(test)]
mod tests;
