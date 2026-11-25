//! The settings presets a benchmark can draw with (`--preset NAME`).
//!
//! One file per title, `Data\XML\BenchmarkPresets_<title key>.toml`, read through the asset layers
//! like every other file this project authors for a title: the embedded one is the fallback, and a
//! file of the same name in a mod's or the player's `overrides/<title>/Data/XML/` replaces it. The
//! shape is `schemas/v1/benchmark-presets.tosd`. A preset is a name and the graphics settings it
//! sets, by the key the graphics page uses, so it says in data what a player could set by hand;
//! the run applies them in memory, never to `settings.toml`.

use std::collections::BTreeMap;

use serde::Deserialize;

/// The graphics settings a preset may set: the keys of `[graphics]` that change what a frame
/// costs and that the page applies the moment they are given.
pub const KEYS: &[&str] = &[
    "render_scale",
    "msaa",
    "shadows",
    "motion_blur",
    "motion_blur_resolution",
    "anisotropy",
    "model_detail",
    "texture_detail",
    "trail_order",
    "scenery_sort",
    "trail_length",
    "weather",
];

/// The file a title's presets are in.
#[must_use]
pub fn file_name(title_key: &str) -> String {
    format!("Data\\XML\\BenchmarkPresets_{}.toml", title_key.to_uppercase_first())
}

trait UppercaseFirst {
    fn to_uppercase_first(&self) -> String;
}

impl UppercaseFirst for str {
    fn to_uppercase_first(&self) -> String {
        let mut chars = self.chars();
        chars.next().map_or_else(String::new, |first| {
            first.to_uppercase().collect::<String>() + chars.as_str()
        })
    }
}

/// One preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    /// What `--preset` names, matched ignoring case.
    pub name: String,
    /// One line for a person choosing.
    pub summary: String,
    /// `(graphics key, value)` in the file's key order.
    pub settings: Vec<(String, String)>,
}

/// A title's presets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Presets {
    /// In file order.
    pub presets: Vec<Preset>,
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    preset: Vec<Raw>,
}

#[derive(Deserialize)]
struct Raw {
    name: String,
    #[serde(default)]
    summary: String,
    #[serde(flatten)]
    settings: BTreeMap<String, toml::Value>,
}

impl Presets {
    /// Reads a file. A key that is not one of [`KEYS`], or a value that is not text, is refused by
    /// name, so a misspelt setting never passes for one that was applied.
    ///
    /// # Errors
    ///
    /// The file is not TOML of this shape, a preset repeats a name, or it sets an unknown key.
    pub fn parse(text: &str) -> Result<Self, String> {
        let file: File = toml::from_str(text).map_err(|error| error.to_string())?;
        let mut presets: Vec<Preset> = Vec::new();
        for raw in file.preset {
            if presets
                .iter()
                .any(|seen| seen.name.eq_ignore_ascii_case(&raw.name))
            {
                return Err(format!("preset {:?} appears twice", raw.name));
            }
            let mut settings = Vec::new();
            for (key, value) in raw.settings {
                if !KEYS.contains(&key.as_str()) {
                    return Err(format!(
                        "preset {:?}: {key:?} is not a setting a preset sets (one of {})",
                        raw.name,
                        KEYS.join(", ")
                    ));
                }
                let toml::Value::String(value) = value else {
                    return Err(format!("preset {:?}: {key} is not text", raw.name));
                };
                settings.push((key, value));
            }
            presets.push(Preset {
                name: raw.name,
                summary: raw.summary,
                settings,
            });
        }
        Ok(Self { presets })
    }

    /// The preset called `name`, ignoring case.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Preset> {
        self.presets
            .iter()
            .find(|preset| preset.name.eq_ignore_ascii_case(name))
    }

    /// The names, for a message that says what there is.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.presets.iter().map(|preset| preset.name.as_str()).collect()
    }
}

/// Reads the title's presets through the asset layers: the embedded file unless a layer above it
/// has one. `None` when it will not read; the report says why, and `--preset` then names nothing.
pub fn load(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    report: &mut Vec<String>,
) -> Option<Presets> {
    let name = file_name(&super::title_key(title.name));
    let read = archives
        .read_name(&name)
        .map_err(|error| format!("{error:#}"))
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string()))
        .and_then(|text| Presets::parse(&text));
    match read {
        Ok(presets) => {
            report.push(format!(
                "{name}: benchmark presets {}",
                presets.names().join(", ")
            ));
            Some(presets)
        }
        Err(error) => {
            report.push(format!(
                "{name}: {error}, so the benchmark takes your own settings"
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
        toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/benchmark-presets.tosd", version = "1.0.0" }
        [[preset]]
        name = "low"
        summary = "cheap"
        msaa = "off"
        render_scale = "50"
    "#;

    #[test]
    fn a_preset_is_a_name_and_the_settings_it_sets() {
        let presets = Presets::parse(GOOD).unwrap();
        let low = presets.get("LOW").unwrap();
        assert_eq!(low.summary, "cheap");
        assert_eq!(
            low.settings,
            [("msaa".into(), "off".into()), ("render_scale".into(), "50".into())]
        );
        assert!(presets.get("high").is_none());
    }

    #[test]
    fn an_unknown_key_or_a_repeated_name_is_refused() {
        assert!(Presets::parse("[[preset]]\nname = \"a\"\nvsync = \"on\"\n").is_err());
        assert!(Presets::parse("[[preset]]\nname = \"a\"\n[[preset]]\nname = \"A\"\n").is_err());
        assert!(Presets::parse("[[preset]]\nname = \"a\"\nmsaa = 4\n").is_err());
    }

    #[test]
    fn the_file_is_named_for_the_title() {
        assert_eq!(file_name("pulse"), "Data\\XML\\BenchmarkPresets_Pulse.toml");
        assert_eq!(file_name("2048"), "Data\\XML\\BenchmarkPresets_2048.toml");
    }
}
