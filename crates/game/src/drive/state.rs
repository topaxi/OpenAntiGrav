//! What the game says about itself while it is driven.
//!
//! One [`State`] per frame, written as a JSON line whenever it changed (`--state-log`, or stderr).
//! It is built by the composition root from the stage on screen; nothing in it is parsed from a
//! picture.

use serde::Serialize;

/// One option of a choice row.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Opt {
    /// What the setting stores.
    pub value: String,
    /// What the row shows.
    pub label: String,
    /// Why this option cannot be used yet or at all, when it cannot. The menus still offer it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
}

/// One row of the screen that has focus: a menu row, a picker entry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Row {
    /// Position on the screen.
    pub index: usize,
    /// The row's stable id (`menu.toml`'s `id`, or a picker entry's id), when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// What the row says.
    pub label: String,
    /// `submenu`, `action`, `choice`, `toggle`, `binding`, `back` or `entry`.
    pub kind: &'static str,
    /// The settings key a `choice` or `toggle` row edits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
    /// What a `choice` row is set to (the stored value, not the label), or `on`/`off`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The options a `choice` row steps through.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<Opt>,
    /// `false` for a row greyed out.
    pub enabled: bool,
}

/// The focused row, repeated outside the list so a reader need not index into it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Focus {
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
}

/// The screen that takes input: a menu page, or whatever is open over it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Screen {
    /// `page` for an ordinary menu page, else what is open over it (`picker`, `campaign`, `prompt`...).
    pub kind: String,
    /// The page id, or the overlay's own (`track-select`, `ship-select`).
    pub id: String,
    /// The heading.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// The focused row; `None` on a screen that reports no rows (`rows` is then empty).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<Focus>,
    /// The buttons that move focus forward and back, when the screen knows them (a picker does);
    /// `None` leaves the driver to find out by pressing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<(&'static str, &'static str)>,
    /// Every row, in order.
    pub rows: Vec<Row>,
    /// A transition is running and presses are not taken yet.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub busy: bool,
}

/// The race the game resolved: what it launched, or is racing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Resolved {
    /// The disc image the track and rules come from.
    pub source: String,
    /// Where the craft come from, when that is another release (RACE REMIX).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub craft_source: Option<String>,
    /// The title the source is, once loaded (`stage=race`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The circuit's entry name; `None` is the source's own default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<String>,
    /// A foreign track file raced instead of any circuit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// The mode, as the menus name it (`time_trial`).
    pub mode: String,
}

/// Everything reported after one frame.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct State {
    /// The simulation tick the frame ended on. Not part of the change test.
    pub tick: u64,
    /// `launcher`, `loading`, `frontend`, `menu` or `race`.
    pub stage: String,
    /// What the boot sequence is showing, on `stage=frontend`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontend: Option<String>,
    /// The menu page under whatever overlay is open, on `stage=menu`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// The screen that takes input, on `stage=menu`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen: Option<Screen>,
    /// The resolved race, from the frame a launch begins loading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<Resolved>,
}

impl State {
    /// The same state without the tick, for "did anything change".
    #[must_use]
    pub fn same_as(&self, other: &Self) -> bool {
        Self { tick: 0, ..self.clone() } == Self { tick: 0, ..other.clone() }
    }

    /// One compact JSON line.
    #[must_use]
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    /// The texts a script condition may compare for `key`. Most keys have one; `focus` is the
    /// focused row's id, setting and label, so `focus=launch` and `focus="LAUNCH RACE"` both hold.
    ///
    /// Keys: `stage`, `frontend`, `page`, `screen` (the overlay kind, `page` when none),
    /// `screen.id`, `title`, `focus`, and `race.title|track|track_file|team|variant|mode|source|craft_source`.
    #[must_use]
    pub fn get(&self, key: &str) -> Vec<String> {
        let screen = self.screen.as_ref();
        let one = |text: Option<String>| text.into_iter().collect::<Vec<_>>();
        match key {
            "stage" => vec![self.stage.clone()],
            "frontend" => one(self.frontend.clone()),
            "page" => one(self.page.clone()),
            "screen" => one(screen.map(|s| s.kind.clone())),
            "screen.id" => one(screen.map(|s| s.id.clone())),
            "title" => one(screen.map(|s| s.title.clone())),
            "focus" => screen
                .and_then(|s| s.focus.as_ref())
                .map(|f| {
                    let mut all = vec![f.label.clone()];
                    all.extend(f.id.clone());
                    all.extend(f.setting.clone());
                    all
                })
                .unwrap_or_default(),
            other => {
                let Some(race) = self.race.as_ref() else {
                    return Vec::new();
                };
                match other.strip_prefix("race.") {
                    Some("title") => one(race.title.clone()),
                    Some("track") => one(race.track.clone()),
                    Some("track_file") => one(race.track_file.clone()),
                    Some("team") => one(race.team.clone()),
                    Some("variant") => one(race.variant.clone()),
                    Some("mode") => vec![race.mode.clone()],
                    Some("source") => vec![race.source.clone()],
                    Some("craft_source") => one(race.craft_source.clone()),
                    _ => Vec::new(),
                }
            }
        }
    }

    /// Whether every condition holds.
    #[must_use]
    pub fn satisfies(&self, conds: &[super::Cond]) -> bool {
        conds.iter().all(|cond| cond.holds(&self.get(&cond.key)))
    }

    /// The keys that name nothing this build reports, for a script that misspells one.
    #[must_use]
    pub fn is_key(key: &str) -> bool {
        matches!(
            key,
            "stage" | "frontend" | "page" | "screen" | "screen.id" | "title" | "focus"
        ) || key.strip_prefix("race.").is_some_and(|rest| {
            matches!(
                rest,
                "title" | "track" | "track_file" | "team" | "variant" | "mode" | "source" | "craft_source"
            )
        })
    }
}
