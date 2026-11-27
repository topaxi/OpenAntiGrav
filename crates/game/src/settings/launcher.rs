//! The `[launcher]` section: how the title chooser looks, and which picture each
//! of its cards was last left on (`docs/ui/launcher.md`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Cards, or the list of every disc image found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LauncherStyle {
    /// One card per title, drawn from the title's own system art.
    #[default]
    Cards,
    /// One row per disc image, with its platform, serial and file name.
    List,
}

/// The `[launcher]` table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Launcher {
    /// `cards` or `list`. `--launcher-list` asks for the list for one run.
    #[serde(default)]
    pub style: LauncherStyle,
    /// The picture each card was left on, written when a player changes it: card
    /// (the title) to `<serial or file name>:<picture>`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pictures: BTreeMap<String, String>,
}
