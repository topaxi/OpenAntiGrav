//! Which simulation modules a race runs: the rules modules and the track modules of the
//! mods that are on (`oag_mod::Extension`), for the title, never in an original campaign
//! (`docs/architecture/wasm-extensions/README.md`, section 1.2).
//!
//! Only the manifests are read here; each `.wasm` is read by the race load
//! (`oag_raceplay::extensions::prepare`, on the load thread), so a menu, the MODS page
//! and a vanilla race open no module and build no wasm engine (README 4.5).

use std::sync::Arc;

use oag_raceplay::extensions::{Scope, Source};

use crate::settings::mod_settings::ModSettings;

/// The rules and track modules of `mods`, in id order, with no saved picks. The race load keeps
/// the ones for its title.
#[must_use]
pub fn sources<'a>(mods: impl IntoIterator<Item = &'a oag_mod::Mod>) -> Vec<Source> {
    sources_with(mods, &ModSettings::new())
}

/// [`sources`] with the picks the mods' own menu rows saved ([`options_of`]).
#[must_use]
pub fn sources_with<'a>(
    mods: impl IntoIterator<Item = &'a oag_mod::Mod>,
    settings: &ModSettings,
) -> Vec<Source> {
    let mut found: Vec<Source> = mods
        .into_iter()
        .flat_map(|m| {
            let Some(extension) = m.manifest.extension.as_ref() else {
                return Vec::new();
            };
            let source = |module: &String, scope: Scope| Source {
                mod_id: m.manifest.id.clone(),
                module: module.clone(),
                titles: m.manifest.titles.clone(),
                api: extension.api,
                memory_pages: extension.memory_pages,
                state_bytes: extension.state_bytes,
                drives_ai_pickup: extension.drives_ai_pickup,
                scope,
                read: reader(m, module),
                strings: strings_reader(m),
                icons: m
                    .manifest
                    .icons
                    .iter()
                    .map(|(name, path)| (name.clone(), reader(m, path)))
                    .collect(),
                effects: m
                    .manifest
                    .effects
                    .iter()
                    .map(|(name, path)| (name.clone(), path.clone()))
                    .collect(),
                shaders: m
                    .manifest
                    .shaders
                    .iter()
                    .map(|(name, path)| (name.clone(), path.clone()))
                    .collect(),
                sounds: m
                    .manifest
                    .sounds
                    .iter()
                    .map(|(name, path)| (name.clone(), path.clone()))
                    .collect(),
                files: files_reader(m),
                options: options_of(m, settings),
                pickup: m.manifest.pickup.clone(),
                pickup_titles: m.manifest.pickup_titles.clone(),
                pickup_odds: m.manifest.pickup_odds.clone(),
                channels: oag_raceplay::extensions::Declared {
                    publishes: m.manifest.channels.publishes.clone(),
                    listens: m.manifest.channels.listens.clone(),
                },
            };
            let rules = extension
                .rules
                .iter()
                .map(|rules| source(&rules.module, Scope::Rules));
            let track = extension
                .track
                .iter()
                .map(|t| source(&t.module, Scope::Track(m.local_path(&t.track))));
            rules.chain(track).collect()
        })
        .collect();
    found.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));
    found
}

/// The player's picks on the mod's own `choice` rows, as indices into each row's `values`:
/// the rows of its `menu.toml` pages whose `setting` is `mod.<id>.<key>`, in file order, the
/// first [`oag_raceplay::extensions::OPTIONS`] of them. A row with nothing saved takes its `default`,
/// and a value the row does not list is index 0. What `Observation::option` reads.
fn options_of(
    module: &oag_mod::Mod,
    settings: &ModSettings,
) -> [u8; oag_raceplay::extensions::OPTIONS] {
    let mut options = [0; oag_raceplay::extensions::OPTIONS];
    let Ok(Some(text)) = module.menu() else {
        return options;
    };
    let Ok(menu) = text.parse::<toml::Table>() else {
        return options;
    };
    let id = &module.manifest.id;
    let prefix = format!("mod.{id}.");
    let rows = menu
        .get("page")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|page| page.get("entry")?.as_array())
        .flatten()
        .filter(|row| row.get("kind").and_then(toml::Value::as_str) == Some("choice"))
        .filter_map(|row| {
            let key = row.get("setting")?.as_str()?.strip_prefix(&prefix)?;
            let values = row.get("values")?.as_array()?;
            let default = row.get("default").and_then(toml::Value::as_str);
            Some((key, values, default))
        });
    for (slot, (key, values, default)) in options.iter_mut().zip(rows) {
        let saved = settings.get(id).and_then(|keys| keys.get(key));
        let pick = saved.map(String::as_str).or(default);
        *slot = values
            .iter()
            .position(|value| value.as_str() == pick)
            .map_or(0, |at| u8::try_from(at).unwrap_or(0));
    }
    options
}

/// The mod's own text for a language: `strings/english.toml` with `strings/<language>.toml`
/// over it, the files its menu rows already use ([`oag_mod::Mod::strings`]). Opened when
/// the race loads; a file that is missing or does not parse is skipped.
fn strings_reader(module: &oag_mod::Mod) -> oag_raceplay::extensions::StringsReader {
    let label = std::path::PathBuf::from(module.label());
    Arc::new(move |language| {
        let Ok(module) = oag_mod::Mod::open(&label) else {
            return std::collections::BTreeMap::new();
        };
        let mut table = oag_ui::language::StringTable::default();
        let mut languages = vec!["english".to_string()];
        languages.extend(
            language
                .map(str::to_ascii_lowercase)
                .filter(|l| l != "english"),
        );
        for language in languages {
            if let Ok(Some(text)) = module.strings(&language) {
                let _ = oag_ui::strings::overlay_text(&mut table, &text);
            }
        }
        table
            .ids()
            .filter_map(|id| table.get(id).map(|text| (id.to_string(), text.to_string())))
            .collect()
    })
}

/// Reads `module` later: from disk for a folder mod, else by reopening the archive.
fn reader(module: &oag_mod::Mod, name: &str) -> oag_raceplay::extensions::Reader {
    let label = format!("{}: {name}", module.label());
    if let Some(path) = module.local_path(name) {
        return Arc::new(move || {
            std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
        });
    }
    let archive = std::path::PathBuf::from(module.label());
    let name = name.to_string();
    Arc::new(move || {
        oag_mod::Mod::open(&archive)
            .map_err(|e| format!("{label}: {e}"))?
            .read(&name)
            .map_err(|e| format!("{label}: {e}"))?
            .ok_or_else(|| format!("{label}: not in the mod"))
    })
}

/// Reads any file of the mod by its path, reopening the mod on the load thread.
fn files_reader(module: &oag_mod::Mod) -> oag_raceplay::extensions::Files {
    let label = std::path::PathBuf::from(module.label());
    Arc::new(move |name| {
        oag_mod::Mod::open(&label)
            .map_err(|e| format!("{}: {e}", label.display()))?
            .read(name)
            .map_err(|e| format!("{name}: {e}"))?
            .ok_or_else(|| format!("{name}: not in the mod"))
    })
}

/// Sets `options.find_extensions` for a race: the load thread searches the mods that are
/// on (as `switches` has them) for rules modules, or none for a race that stays
/// `vanilla` (an original campaign, a network race). Nothing is read here.
pub fn attach(
    options: &mut oag_raceplay::Options,
    switches: &crate::settings::Mods,
    picks: &ModSettings,
    vanilla: bool,
) {
    options.find_extensions = (!vanilla).then(|| {
        let switches = switches.clone();
        let picks = picks.clone();
        oag_raceplay::extensions::Finder(Arc::new(move || {
            crate::mods::Mods::load(&switches)
                .map(|mods| sources_with(mods.player(), &picks))
                .map_err(|e| format!("{e:#}"))
        }))
    });
}

/// The dev overlay's `mods` block for `race`, `None` in a race with no simulation mod.
#[must_use]
pub fn readout(race: &oag_raceplay::Race) -> Option<oag_present::perf::mods::Readout> {
    race.is_modded().then(|| oag_present::perf::mods::Readout {
        modules: race
            .extension_reports()
            .into_iter()
            .map(|r| oag_present::perf::mods::Module {
                id: r.mod_id,
                fuel_used: r.fuel_used,
                fuel_peak: r.fuel_peak,
                fuel_budget: r.fuel_budget,
                call_us: r.call_us,
                faulted: r.faulted,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mod's `[strings]` and `[icons]` reach the race's source, and an icon is read
    /// through its reader, from a folder and from a zip alike.
    #[test]
    fn a_mods_strings_files_and_icons_reach_its_source() {
        let dir = std::env::temp_dir().join(format!("oag-modext-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("ext")).unwrap();
        std::fs::create_dir_all(dir.join("ui")).unwrap();
        std::fs::write(dir.join("ext/m.wasm"), b"\0asm").unwrap();
        std::fs::write(dir.join("ui/badge.png"), b"png bytes").unwrap();
        std::fs::create_dir_all(dir.join("strings")).unwrap();
        std::fs::write(
            dir.join("strings/english.toml"),
            "[strings]\nboost = { text = \"BOOST\", human = true }\nonly_english = { text = \"E\", human = true }\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("strings/german.toml"),
            "[strings]\nboost = { text = \"SCHUB\", human = true }\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("oag-mod.toml"),
            r#"
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "test.hudmod"
name = "Hud mod"
version = "1.0.0"
authors = ["A. Modder"]
license = "MIT"
description = "Strings and icons."

[extension]
simulation = true
api = 1

[extension.rules]
module = "ext/m.wasm"

[icons]
badge = "ui/badge.png"
"#,
        )
        .unwrap();
        let module = oag_mod::Mod::open(&dir).unwrap();
        let found = sources([&module]);
        assert_eq!(found.len(), 1);
        let text = |language: Option<&str>, id: &str| (found[0].strings)(language).get(id).cloned();
        assert_eq!(text(Some("German"), "boost").as_deref(), Some("SCHUB"));
        assert_eq!(
            text(Some("French"), "boost").as_deref(),
            Some("BOOST"),
            "English"
        );
        assert_eq!(text(None, "boost").as_deref(), Some("BOOST"));
        assert_eq!(
            text(Some("german"), "only_english").as_deref(),
            Some("E"),
            "over English"
        );
        assert_eq!(text(Some("german"), "nope"), None);
        assert_eq!(found[0].icons.len(), 1);
        assert_eq!(found[0].icons[0].0, "badge");
        assert_eq!((found[0].icons[0].1)().unwrap(), b"png bytes");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A mod's choice rows reach its source as indices into each row's `values`: the saved
    /// value, else the row's `default`, else the first value for one the row does not list.
    #[test]
    fn a_mods_choice_rows_reach_its_source_as_indices_into_their_values() {
        let dir = std::env::temp_dir().join(format!("oag-modopt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("ext")).unwrap();
        std::fs::write(dir.join("ext/m.wasm"), b"\0asm").unwrap();
        std::fs::write(
            dir.join("menu.toml"),
            r#"
[[page]]
id = "p"
title = "P"
[[page.entry]]
kind = "choice"
label = "A"
setting = "mod.test.opt.quality"
values = ["low", "medium", "high"]
default = "high"
[[page.entry]]
kind = "toggle"
label = "B"
setting = "mod.test.opt.flag"
[[page.entry]]
kind = "choice"
label = "C"
setting = "mod.test.opt.mode"
values = ["x", "y"]
[[page.entry]]
kind = "choice"
label = "Other mod's"
setting = "mod.other.thing"
values = ["a", "b"]
"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("oag-mod.toml"),
            r#"
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }
[mod]
id = "test.opt"
name = "Opt"
version = "1.0.0"
authors = ["x"]
license = "MIT"
description = "d"
[extension]
simulation = true
api = 1
memory_pages = 2
state_bytes = 8
[extension.rules]
module = "ext/m.wasm"
"#,
        )
        .unwrap();
        let module = oag_mod::Mod::open(&dir).unwrap();
        let picks = |saved: &[(&str, &str)]| {
            let mut table = ModSettings::new();
            for (key, value) in saved {
                crate::settings::mod_settings::set(
                    &mut table,
                    &format!("mod.test.opt.{key}"),
                    value,
                );
            }
            sources_with([&module], &table)[0].options
        };
        assert_eq!(
            picks(&[]),
            [2, 0, 0, 0],
            "high by default, the other rows first"
        );
        assert_eq!(picks(&[("quality", "low"), ("mode", "y")]), [0, 1, 0, 0]);
        assert_eq!(picks(&[("quality", "medium")]), [1, 0, 0, 0]);
        assert_eq!(
            picks(&[("quality", "gone")]),
            [0, 0, 0, 0],
            "a value the row does not list"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
