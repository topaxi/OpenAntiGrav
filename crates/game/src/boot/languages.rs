//! Which languages a source offers, and the one a boot reads its text in.
//!
//! Split out of `boot.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change - and with
//! room left for it, the same reason every other split in this directory
//! gives.

use oag_pulse as pulse;

use oag_ui::language::{Language, StringTable};

use super::expand;

/// Every language plugin this source carries.
///
/// Public because a race needs the string table too, for the HUD's `idstring`
/// captions, and it does not go through the boot path that used to be the only
/// caller. See [`load_strings`].
pub fn load_languages(
    archives: &mut oag_assets::Archives,
    plugins: &[&str],
    report: &mut Vec<String>,
) -> Vec<Language> {
    let mut out = Vec::new();
    for plugin in plugins {
        // **Pulse's path convention, applied to every title.** It is genuinely
        // shared today - HD's plugins resolve through it - but it lives in the
        // Pulse crate rather than on `oag_title`, so a title that keeps its
        // plugins elsewhere would load zero languages. That used to happen
        // *silently*, one `continue` per miss (finding G4 of the 2026-08-18
        // review); each miss now names the entry it asked for, so the shape of
        // the failure is legible from the load report rather than only from an
        // empty picker. Promoting the convention to an axis waits for the title
        // that disagrees, which is ADR-0022's rule and the same call S6 makes.
        let name = pulse::names::language_definition(plugin);
        let Ok(blob) = archives.read_name(&name) else {
            report.push(format!(
                "language plugin {plugin}: no {name} in this source"
            ));
            continue;
        };
        let Ok(xml) = expand(&blob) else {
            report.push(format!(
                "language plugin {plugin}: {name} is not readable XML"
            ));
            continue;
        };
        match Language::from_definition(plugin, &xml) {
            Some(language) => out.push(language),
            None => report.push(format!(
                "language plugin {plugin}: {name} declares no language"
            )),
        }
    }

    if out.is_empty() {
        report.push("no language plugins resolved; the picker will be empty".to_string());
    } else {
        report.push(format!(
            "{} language(s): {}",
            out.len(),
            out.iter()
                .map(|l| format!("{} ({}, {})", l.name, l.native_name, l.plugin))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out
}

/// Which language a boot reads its text in.
///
/// The saved language first, then English, then whatever comes first. The
/// fallback chain used to end at English with a note that there was nothing
/// saved to prefer; there is now. A saved name this source does not carry falls
/// through rather than failing - the same rule the picker's own preselection
/// follows, and for the same reason: a settings file written against the EU
/// disc must not stop the USA one booting.
///
/// Its own function so that [`load_strings`] and
/// [`super::roster::load_circuit_names`] cannot answer it differently and put
/// half the front end in one language and the circuit list in another.
#[must_use]
pub fn chosen_language<'a>(
    languages: &'a [Language],
    preferred: Option<&str>,
) -> Option<&'a Language> {
    preferred
        .and_then(|name| languages.iter().find(|l| l.name.eq_ignore_ascii_case(name)))
        .or_else(|| languages.iter().find(|l| l.name == "English"))
        .or_else(|| languages.first())
}

/// The chosen language's string table.
///
/// Public for the same reason [`load_languages`] is: the HUD resolves its own
/// `idstring` keys through this (`IG_HUD_LAP` on Pulse, `HUD_Lap` on Pure),
/// and a race reaches it without booting the front end.
pub fn load_strings(
    archives: &mut oag_assets::Archives,
    languages: &[Language],
    preferred: Option<&str>,
    report: &mut Vec<String>,
) -> StringTable {
    let Some(language) = chosen_language(languages, preferred) else {
        return StringTable::default();
    };
    // No `Dynamic Entry File Source` does not always mean no strings: Pure's
    // `PI000` (English) states every string inline in `Definition.xml`
    // instead of naming a separate file - see `docs/formats/pure-status.md`.
    // Re-parsing the definition is safe for a plugin that really names
    // nothing too: `<Font>`/`<Values>` carry no `<Entry>` tag.
    let name = match language.entries.as_deref() {
        Some(entries) => entries.to_string(),
        None => pulse::names::language_definition(&language.plugin),
    };

    match archives
        .read_name(&name)
        .and_then(|blob| expand(&blob).map_err(|e| oag_assets::Error::BadSpec(e.to_string())))
    {
        Ok(xml) => {
            let mut table = StringTable::from_xml(&xml);
            oag_ui::strings::overlay(&mut table, &language.name, report);
            if table.is_empty() {
                report.push(format!("{} names no string table", language.name));
            } else {
                report.push(format!(
                    "{name}: {} strings for {}",
                    table.len(),
                    language.name
                ));
            }
            table
        }
        Err(e) => {
            report.push(format!("{name}: {e}"));
            StringTable::default()
        }
    }
}
