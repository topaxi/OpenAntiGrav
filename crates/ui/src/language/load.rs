//! Which languages a source offers, and the one a boot reads its text in.
//!
//! Read off the title's archives rather than off a boot, because a race needs
//! the string table too and does not go through the boot path.

use oag_pulse as pulse;

use super::{Language, StringTable};
use crate::strings::PROJECT_LANGUAGES;

use crate::xml::expand;

/// Every language plugin this source carries.
///
/// Public because a race needs the string table too, for the HUD's `idstring`
/// captions, and it does not go through the boot path that used to be the only
/// caller. See [`load_strings`].
pub fn load_languages(
    archives: &mut oag_assets::Archives,
    plugins: &[&str],
    disc_strings: Option<&'static str>,
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
            Some(language) => out.push(Language {
                disc_strings,
                ..repair_native_name(archives, language, report)
            }),
            None => report.push(format!(
                "language plugin {plugin}: {name} declares no language"
            )),
        }
    }

    add_project_languages(&mut out, disc_strings);

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

/// A native name a plugin's `Definition.xml` carries with its letters already
/// lost: a `?` where a character the file's author could not save stood.
///
/// **Measured, not a decoding fault**: HD's, Omega's and 2048's Russian plugins
/// hold the bytes `50 3f 3f 3f 3f 3f 3f` (`P` and six `?`) in the file itself.
/// No real language is spelled with a `?`, so the test is exact enough to leave
/// the other mis-labelled names (`Svenska` on Japanese) alone.
#[must_use]
pub fn is_lossy_name(name: &str) -> bool {
    name.contains('?') || name.contains('\u{fffd}')
}

/// Replaces a lossy native name with the one the same plugin's own string table
/// authors: `OPT_<ID>`, which the Russian `entries.xml` spells `Русский`.
///
/// Only a name [`is_lossy_name`] says is lost is touched. When the table has no
/// such entry the name stays as the disc wrote it, and [`pickable`] then leaves
/// the language out of a picker rather than draw question marks.
fn repair_native_name(
    archives: &mut oag_assets::Archives,
    mut language: Language,
    report: &mut Vec<String>,
) -> Language {
    if !is_lossy_name(&language.native_name) {
        return language;
    }
    let key = format!("OPT_{}", language.name.to_uppercase());
    let own = language
        .entries
        .as_deref()
        .and_then(|path| archives.read_name(path).ok())
        .and_then(|blob| expand(&blob).ok())
        .and_then(|xml| StringTable::from_xml(&xml).get(&key).map(str::to_string))
        .filter(|name| !name.is_empty() && !is_lossy_name(name));
    match own {
        Some(name) => {
            report.push(format!(
                "language plugin {}: native name {:?} is lost in the file itself, \
                 drawn as {name:?} from its own table's {key}",
                language.plugin, language.native_name
            ));
            language.native_name = name;
        }
        None => report.push(format!(
            "language plugin {}: native name {:?} is lost in the file and its table has no {key}",
            language.plugin, language.native_name
        )),
    }
    language
}

/// The languages a picker drawn in `face` can show: those whose native name
/// every character of which `face` carries a glyph for, and which is not lossy.
///
/// **A face that lacks a glyph draws nothing for it**, so a row would read as a
/// gap in a word. HD's `Default` face is `helv.fnt` with no Cyrillic, and
/// Russian's name is Cyrillic, so on HD's picker Russian is left out and the
/// report says so; Russian's own `arialbd.fnt` carries the glyphs, but a row
/// cannot yet be drawn in a face other than the picker's. Never a substitute
/// glyph and never a substitute face.
#[must_use]
pub fn pickable(
    languages: &[Language],
    face: &crate::font::Atlas,
    report: &mut Vec<String>,
) -> Vec<Language> {
    languages
        .iter()
        .filter(|language| match undrawable(&language.native_name, face) {
            None => true,
            Some(ch) => {
                report.push(format!(
                    "language {} ({:?}) is not offered in the picker: its face has no glyph for {ch:?}",
                    language.name, language.native_name
                ));
                false
            }
        })
        .cloned()
        .collect()
}

/// The first character of `name` that `face` cannot draw, or a `?` a lost name
/// leaves behind.
#[must_use]
pub fn undrawable(name: &str, face: &crate::font::Atlas) -> Option<char> {
    name.chars()
        .find(|&ch| is_lossy_name(&ch.to_string()) || face.cell(ch).is_none())
}

/// Appends [`PROJECT_LANGUAGES`] after the disc's own, so a picker lists them
/// last and every caller of [`load_languages`] sees the same list.
///
/// A project language is the disc's English with this project's translation
/// laid over it, so it needs an English to stand on and borrows that
/// language's table source and font slots. A disc that already ships a language
/// of the same name (Omega's own `portuguesebr`) keeps its own plugin as the
/// base, and nothing is added. `disc_strings` is [`oag_title::FrontEnd::disc_strings`].
fn add_project_languages(out: &mut Vec<Language>, disc_strings: Option<&'static str>) {
    let Some(base) = english_base(out).cloned() else {
        return;
    };
    for project in PROJECT_LANGUAGES {
        if out
            .iter()
            .any(|l| l.name.eq_ignore_ascii_case(project.name))
        {
            continue;
        }
        out.push(Language {
            name: project.name.to_string(),
            native_name: project.native_name.to_string(),
            disc_strings,
            ..base.clone()
        });
    }
}

/// The language a project language stands on, and the one a boot falls back to:
/// the disc's `English`, else its `American`. A release whose executable reaches
/// only `American` (2048 USA offers American, French and Spanish) has no plain
/// English plugin to offer.
fn english_base(languages: &[Language]) -> Option<&Language> {
    ["English", "American"]
        .iter()
        .find_map(|name| languages.iter().find(|l| l.name == *name))
}

/// Which language a boot reads its text in.
///
/// The saved language first, then English (or American), then whatever comes first. The
/// fallback chain used to end at English with a note that there was nothing
/// saved to prefer; there is now. A saved name this source does not carry falls
/// through rather than failing - the same rule the picker's own preselection
/// follows, and for the same reason: a settings file written against the EU
/// disc must not stop the USA one booting.
///
/// Its own function so that [`load_strings`] and
/// `load_circuit_names` in `oag_game::boot` cannot answer it differently and put
/// half the front end in one language and the circuit list in another.
#[must_use]
pub fn chosen_language<'a>(
    languages: &'a [Language],
    preferred: Option<&str>,
) -> Option<&'a Language> {
    preferred
        .and_then(|name| languages.iter().find(|l| l.name.eq_ignore_ascii_case(name)))
        .or_else(|| english_base(languages))
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
            if let Some(namespace) = language.disc_strings {
                crate::strings::overlay_disc(&mut table, namespace, &language.name, report);
            }
            crate::strings::overlay(&mut table, &language.name, report);
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

/// The `.fnt` a role resolves to on this source, from its language plugins.
///
/// **`preferred`'s own slot first.** A role that plugin does not fill falls
/// through to a scan of every plugin in source order - not because two
/// plugins are known to name a shared role's file differently, but because a
/// role the chosen language leaves silent (Pure declares four slots to
/// Pulse's eight) still deserves the answer *some* plugin on this source
/// gives, rather than 5x7. `None` only when no plugin at all fills `role` -
/// an ordinary answer, not a failure.
///
/// **`preferred` used to not exist at all here** - every caller scanned
/// every plugin with no preference, so a face this asked for came off
/// whichever plugin happened to load first rather than the language the
/// player chose. See `oag_raceplay`'s `hud_font`'s own doc for the sibling
/// this mirrors and the bug both used to share.
pub fn role_font(
    languages: &[Language],
    preferred: Option<&Language>,
    role: &str,
) -> Option<String> {
    preferred
        .and_then(|language| language.font(role))
        .or_else(|| languages.iter().find_map(|language| language.font(role)))
        .map(str::to_string)
}

/// The `borderExtendPixels` a role authors on this source, resolved the same
/// way [`role_font`] resolves its file: the chosen language's slot first, then
/// the first plugin that authors one. `0` when none does.
#[must_use]
pub fn role_border(languages: &[Language], preferred: Option<&Language>, role: &str) -> u32 {
    preferred
        .map(|language| language.border_extend(role))
        .filter(|px| *px > 0)
        .or_else(|| {
            languages
                .iter()
                .map(|language| language.border_extend(role))
                .find(|px| *px > 0)
        })
        .unwrap_or(0)
}
