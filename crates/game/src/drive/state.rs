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
    /// What an `action` row fires (`launch_race`, `quit`): the same in every language, unlike the label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
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
    /// The campaign's `Selector` pulse clock in whole milliseconds, on the two screens that have
    /// one (`Grid Selection`, `Cell Selection`); how a drive sees a re-entry resume it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pulse_ms: Option<u32>,
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
    /// The simulation mods the race runs (`oag_raceplay::extensions`), on `stage=race`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mods: Vec<String>,
    /// What the first person's craft holds (`Weapon::as_type`), on `stage=race`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pickup: Option<String>,
    /// The kills or points an Eliminator ends on, on `stage=race` in that mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<u32>,
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
    /// The event the campaign map's cursor is on (Wipeout 2048), on `stage=frontend`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_event: Option<String>,
    /// The menu page under whatever overlay is open, on `stage=menu`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// The screen that takes input, on `stage=menu`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen: Option<Screen>,
    /// `landed` once the reads a selection screen opens on are in, `reading` before, on
    /// `stage=menu`: a script waits for it to see a screen open with nothing to wait for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_reads: Option<String>,
    /// The resolved race, from the frame a launch begins loading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<Resolved>,
    /// Photo mode's lens (`internal`, `close`, `track`), or `none` when it is closed, on
    /// `stage=race`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo: Option<String>,
    /// Photo mode's picture controls as `exposure=33 saturation=20 speed_blur=0 dof=off
    /// aperture=40 focus=20`, while it is open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo_fx: Option<String>,
    /// A photo mode shot of several frames under way as `drawn/frames`, or `none`, while the mode
    /// is open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo_job: Option<String>,
    /// The replay viewer while a recording is being watched on `stage=race`: `opening`,
    /// `playing`, `paused`, `ended` or `stopped` (the build stops matching the recording).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay: Option<String>,
    /// The tick the viewer stands at, the length of the recording and the last tick built, as
    /// `at/length/built` (`-` before the first keyframe).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_at: Option<String>,
    /// The grid slot the viewer's picture follows, while a recording is being watched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_follow: Option<String>,
    /// `on` or `off`: whether the watched race's HUD is drawn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_hud: Option<String>,
    /// The end-of-race screen on top, `photo` for `Race End Photo` and `other` for the rest,
    /// on `stage=race` once the race has ended with an end-of-race flow built.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endrace: Option<String>,
    /// The race's state hash, hex, on `stage=race`. It moves with every tick, so a line is
    /// written for each tick the race steps and none while it stands still.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// The simulation's own tick count, on `stage=race`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sim_tick: Option<u64>,
    /// The Pilot Assist level the player's craft flies with (`off`, `normal`, `extreme`), on
    /// `stage=race`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pilot_assist: Option<String>,
    /// Whether the race draws its ghost ship, on `stage=race`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ghost_visible: Option<bool>,
    /// How many out-of-tick changes (a Pilot Assist switch, a handed-over weapon) the race's
    /// recording holds, on `stage=race`; absent when the race records nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_events: Option<usize>,
    /// `loading` while an attract demo's race loads and `running` once it is live
    /// (`session::demo`); absent in every other race.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo: Option<String>,
    /// `on` or `off`: whether the race on screen draws its HUD, on `stage=race`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hud: Option<String>,
    /// How many slots a person flies, on `stage=race`: `0` in a demo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub humans: Option<usize>,
}

impl State {
    /// The same state without the tick, for "did anything change".
    #[must_use]
    pub fn same_as(&self, other: &Self) -> bool {
        Self {
            tick: 0,
            ..self.clone()
        } == Self {
            tick: 0,
            ..other.clone()
        }
    }

    /// One compact JSON line.
    #[must_use]
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    /// The texts a script condition may compare for `key`. Most keys have one; `focus` is the
    /// focused row's id, setting and label, so `focus=launch` and `focus="LAUNCH RACE"` both hold.
    ///
    /// Keys: `stage`, `frontend`, `map_event`, `page`, `screen` (the overlay kind, `page` when none),
    /// `screen.id`, `title`, `focus`, `opening_reads`, `demo`, `humans`, and `race.title|track|track_file|team|variant|mode|source|craft_source|target`.
    #[must_use]
    pub fn get(&self, key: &str) -> Vec<String> {
        let screen = self.screen.as_ref();
        let one = |text: Option<String>| text.into_iter().collect::<Vec<_>>();
        match key {
            "stage" => vec![self.stage.clone()],
            "frontend" => one(self.frontend.clone()),
            "map_event" => one(self.map_event.clone()),
            "page" => one(self.page.clone()),
            "opening_reads" => one(self.opening_reads.clone()),
            "screen" => one(screen.map(|s| s.kind.clone())),
            "screen.id" => one(screen.map(|s| s.id.clone())),
            "title" => one(screen.map(|s| s.title.clone())),
            "photo" => one(self.photo.clone()),
            "photo_fx" => one(self.photo_fx.clone()),
            "photo_job" => one(self.photo_job.clone()),
            "replay" => one(self.replay.clone()),
            "replay_at" => one(self.replay_at.clone()),
            "replay_follow" => one(self.replay_follow.clone()),
            "replay_hud" => one(self.replay_hud.clone()),
            "endrace" => one(self.endrace.clone()),
            "demo" => one(self.demo.clone()),
            "hud" => one(self.hud.clone()),
            "humans" => one(self.humans.map(|humans| humans.to_string())),
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
                    Some("target") => race.target.iter().map(u32::to_string).collect(),
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
            "stage"
                | "frontend"
                | "map_event"
                | "page"
                | "screen"
                | "screen.id"
                | "title"
                | "focus"
                | "photo"
                | "photo_fx"
                | "photo_job"
                | "replay"
                | "replay_at"
                | "replay_follow"
                | "replay_hud"
                | "endrace"
                | "demo"
                | "hud"
                | "humans"
                | "opening_reads"
        ) || key.strip_prefix("race.").is_some_and(|rest| {
            matches!(
                rest,
                "title"
                    | "track"
                    | "track_file"
                    | "team"
                    | "variant"
                    | "mode"
                    | "source"
                    | "craft_source"
                    | "target"
            )
        })
    }
}
