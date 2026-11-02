//! The `[mod_settings."<id>"]` tables: what a mod's own menu rows save
//! (`docs/architecture/adr/0082-*.md`, decision 2).
//!
//! A row names `setting = "mod.<id>.<key>"`. A mod id carries dots (`oag.example`) and
//! a key never does, so the split is at the last dot. Values are strings, validated by
//! the row that reads them. A turned-off mod's values stay in the table; nothing here
//! looks at `[mods]`.

use std::collections::BTreeMap;

/// Mod id, then key, then the stored value. A `BTreeMap` so the rewrite is stable.
pub type ModSettings = BTreeMap<String, BTreeMap<String, String>>;

const PREFIX: &str = "mod.";

/// Splits `mod.<id>.<key>` into `(id, key)`, or `None` if `setting` is not one.
#[must_use]
pub fn split(setting: &str) -> Option<(&str, &str)> {
    let (id, key) = setting.strip_prefix(PREFIX)?.rsplit_once('.')?;
    (!id.is_empty() && !key.is_empty()).then_some((id, key))
}

/// Records `value` under the setting's own mod. Returns `false` (and stores nothing)
/// if `setting` is not a well-formed `mod.<id>.<key>`.
pub fn set(table: &mut ModSettings, setting: &str, value: &str) -> bool {
    let Some((id, key)) = split(setting) else {
        return false;
    };
    table
        .entry(id.to_string())
        .or_default()
        .insert(key.to_string(), value.to_string());
    true
}

/// Every stored value as a `(setting, value)` pair, for seeding the menu's rows.
#[must_use]
pub fn seeds(table: &ModSettings) -> Vec<(String, oag_ui::menu::Value)> {
    table
        .iter()
        .flat_map(|(id, keys)| {
            keys.iter().map(move |(key, value)| {
                (
                    format!("{PREFIX}{id}.{key}"),
                    oag_ui::menu::Value::Text(value.clone()),
                )
            })
        })
        .collect()
}
