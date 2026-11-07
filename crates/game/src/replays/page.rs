//! The REPLAYS page: the REPLAY row's choices, the detail lines under it, and what the list says
//! about a file it will not play.
//!
//! Resolved from the files' headers alone (`oag_replay::Replay::read_head`), a few hundred bytes
//! a file. **What a header cannot know is whether the race rebuilds**: that is the tick-zero hash
//! after a load, so a mode this build has not proved is listed as NOT VERIFIED and played, and a
//! rebuild that disagrees is refused by name when WATCH asks (the row is then dimmed with the
//! reason, [`Context::refused`]). Nothing is ever played wrong (ADR-0087, decision 7).
//!
//! The pure half, shared by the live page and the `--menu-page` still the way
//! `scoreboard::records_table` is, so the two cannot disagree about a row.

use std::collections::BTreeMap;
use std::path::PathBuf;

use oag_ui::language::StringTable;
use oag_ui::menu::{Choice, Menu};

use super::{Entry, Folder, Head};

/// The page, and the row that holds which file the page is about. Never persisted.
pub const PAGE: &str = "replays";
/// See [`PAGE`].
pub const SELECTED: &str = "replays.selected";

/// The string id and English text of each word the page draws.
pub mod words {
    /// The page's one row of files.
    pub const NONE: (&str, &str) = ("OAG_REPLAYS_NONE", "NONE YET");
    /// Why there are none, with recording on.
    pub const NONE_WHY: (&str, &str) = (
        "OAG_REPLAYS_NONE_WHY",
        "EVERY RACE YOU DRIVE IS RECORDED HERE",
    );
    /// Why there are none, with recording off in the settings.
    pub const NONE_OFF: (&str, &str) = (
        "OAG_REPLAYS_NONE_OFF",
        "RECORDING IS OFF: [REPLAY] RECORD IN THE SETTINGS FILE",
    );
    /// The detail line labels.
    pub const CIRCUIT: (&str, &str) = ("OAG_REPLAYS_CIRCUIT", "CIRCUIT");
    /// See [`CIRCUIT`].
    pub const MODE: (&str, &str) = ("OAG_REPLAYS_MODE", "RACE");
    /// See [`MODE`].
    pub const FINISH: (&str, &str) = ("OAG_REPLAYS_FINISH", "FINISH");
    /// See [`MODE`].
    pub const DATE: (&str, &str) = ("OAG_REPLAYS_DATE", "DATE");
    /// See [`MODE`].
    pub const SIZE: (&str, &str) = ("OAG_REPLAYS_SIZE", "SIZE");
    /// See [`MODE`].
    pub const NOTE: (&str, &str) = ("OAG_REPLAYS_NOTE", "NOTE");
    /// A file the player kept.
    pub const SAVED: (&str, &str) = ("OAG_REPLAYS_SAVED", "SAVED");
    /// A replay of a race that was taken over.
    pub const TAKEN_OVER: (&str, &str) = ("OAG_REPLAYS_TAKEN_OVER", "TAKEN OVER AT %s");
    /// A race that did not end with a result.
    pub const NOT_FINISHED: (&str, &str) = ("OAG_REPLAYS_NOT_FINISHED", "NOT FINISHED");
    /// A mode whose rebuild this build has not proved.
    pub const NOT_VERIFIED: (&str, &str) = ("OAG_REPLAYS_NOT_VERIFIED", "MODE NOT VERIFIED");
    /// Why a file cannot be played: another game's.
    pub const OTHER_TITLE: (&str, &str) =
        ("OAG_REPLAYS_OTHER_TITLE", "RECORDED ON %s: OPEN THAT DISC");
    /// Why a file cannot be played: written by a later version.
    pub const NEWER: (&str, &str) = ("OAG_REPLAYS_NEWER", "RECORDED BY A NEWER VERSION");
    /// Why a file cannot be played: not a replay or cut short.
    pub const DAMAGED: (&str, &str) = ("OAG_REPLAYS_DAMAGED", "DAMAGED FILE");
    /// Why a file cannot be played: no launch request in its header.
    pub const NO_SETUP: (&str, &str) = (
        "OAG_REPLAYS_NO_SETUP",
        "CANNOT BE REBUILT: NO LAUNCH REQUEST",
    );
    /// Why a file cannot be played: the race it rebuilds is not the recorded one.
    pub const REFUSED: (&str, &str) = ("OAG_REPLAYS_REFUSED", "CANNOT BE REBUILT HERE: %s");
}

/// Longest value the page draws before it cuts with `...`.
const VALUE_CHARS: usize = 22;

/// The REPLAY row's value column, in letters: about ten on 2048's face, the narrowest.
const ROW_CHARS: usize = 10;

fn fit(text: &str) -> String {
    fit_to(text, VALUE_CHARS)
}

fn fit_to(text: &str, width: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= width {
        return text.to_string();
    }
    let keep: String = text.chars().take(width - 3).collect();
    format!("{keep}...")
}

/// What the page needs to say about a list of files.
#[allow(missing_debug_implementations)]
pub struct Context<'a> {
    /// The booted title's name, as a header spells it.
    pub title: &'a str,
    /// The words.
    pub strings: &'a StringTable,
    /// The race modes with their localised labels, as the RACE page's MODE row has them.
    pub modes: &'a [Choice],
    /// A circuit's localised name from the mode of a race and the entry a header names.
    pub circuit: Box<dyn Fn(oag_race::Mode, &str) -> Option<String> + 'a>,
    /// Files WATCH opened and the rebuild refused, with the reason.
    pub refused: &'a BTreeMap<PathBuf, String>,
    /// Whether `[replay] record` is on, for what an empty list says.
    pub recording: bool,
}

impl Context<'_> {
    fn say(&self, (id, english): (&str, &str)) -> String {
        self.strings.get_or_fallback(id, english).to_string()
    }

    fn mode_label(&self, token: &str) -> String {
        self.modes
            .iter()
            .find(|choice| choice.value == token)
            .map_or_else(
                || token.replace('_', " ").to_uppercase(),
                |c| c.label.clone(),
            )
    }

    fn circuit_name(&self, header: &oag_replay::Header) -> String {
        let mode = oag_race::Mode::from_name(&header.mode).unwrap_or_default();
        // The setup spells the entry as the disc does; the key's copy is lower case.
        let entry = header
            .setup
            .as_ref()
            .and_then(|setup| setup.track.as_deref())
            .unwrap_or(&header.track);
        (self.circuit)(mode, entry).unwrap_or_else(|| {
            // The entry's own name when the source lists no such circuit.
            entry
                .rsplit(['\\', '/'])
                .nth(1)
                .unwrap_or(entry)
                .to_string()
        })
    }
}

/// Whether a build proves the rebuild of a header's race: Time Trial, Speed Lap and Single Race
/// on every title, and Head to Head with two people on Pulse (`docs/formats/replay-file.md`).
#[must_use]
pub fn claimed(header: &oag_replay::Header) -> bool {
    match header.mode.as_str() {
        "time_trial" | "speed_lap" | "single_race" => true,
        "head_to_head" => {
            header.title.eq_ignore_ascii_case("Wipeout Pulse")
                && header.setup.as_ref().is_some_and(|setup| setup.people == 2)
        }
        _ => false,
    }
}

/// Why `entry` cannot be played, or `None` when it can be tried.
fn unusable(entry: &Entry, cx: &Context<'_>) -> Option<String> {
    let header = match &entry.head {
        Head::Read(header) => header,
        Head::Newer => return Some(cx.say(words::NEWER)),
        Head::Damaged => return Some(cx.say(words::DAMAGED)),
    };
    if let Some(why) = cx.refused.get(&entry.path) {
        return Some(cx.say(words::REFUSED).replace("%s", why));
    }
    if !header.title.eq_ignore_ascii_case(cx.title) {
        return Some(
            cx.say(words::OTHER_TITLE)
                .replace("%s", &header.title.to_uppercase()),
        );
    }
    if header.setup.is_none() {
        return Some(cx.say(words::NO_SETUP));
    }
    None
}

/// The REPLAY row's value for `entry`: the day of the month and the time it was recorded, `*`
/// first for a kept one. The files are listed newest first and the DATE line under the rows has
/// the whole date: the value column is ten letters wide on 2048's face and a longer one is cut.
/// The circuit is the first detail line, for the same reason.
#[must_use]
pub fn choice(entry: &Entry, cx: &Context<'_>) -> Choice {
    let (_, _, day, hour, minute) = civil(entry.unix);
    let when = format!("{day:02} {hour:02}:{minute:02}");
    let saved = if entry.folder == Folder::Saved {
        "*"
    } else {
        ""
    };
    let label = fit_to(&format!("{saved}{when}"), ROW_CHARS);
    let choice = Choice::labelled(entry.key(), label);
    match unusable(entry, cx) {
        Some(why) => choice.unusable(why),
        None => choice,
    }
}

/// The choices for `ValueSource::Replays`: one per file, or one dimmed value saying why none.
#[must_use]
pub fn choices(entries: &[Entry], cx: &Context<'_>) -> Vec<Choice> {
    if entries.is_empty() {
        let why = if cx.recording {
            words::NONE_WHY
        } else {
            words::NONE_OFF
        };
        return vec![Choice::labelled("", cx.say(words::NONE)).unusable(cx.say(why))];
    }
    entries.iter().map(|entry| choice(entry, cx)).collect()
}

/// The file the page's REPLAY row is on.
#[must_use]
pub fn selected<'a>(model: &Menu, entries: &'a [Entry]) -> Option<&'a Entry> {
    let held = model
        .definition()
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some(SELECTED))
        .and_then(oag_ui::menu::Entry::chosen)?;
    let oag_ui::menu::Value::Text(name) = held else {
        return None;
    };
    entries.iter().find(|entry| entry.key() == *name)
}

/// The detail lines under the page's rows, in the order they are dropped from the end when the
/// page has no room for them all; `None` off the page.
#[must_use]
pub fn table_for(
    model: &Menu,
    entries: &[Entry],
    cx: &Context<'_>,
) -> Option<Vec<(String, String)>> {
    if model.page().id != PAGE {
        return None;
    }
    let room = model
        .visible_rows()
        .saturating_sub(model.page().entries.len());
    let entry = selected(model, entries)?;
    let mut lines = lines(entry, cx);
    lines.truncate(room.max(1));
    Some(lines)
}

/// All of `entry`'s detail lines, most useful first.
#[must_use]
pub fn lines(entry: &Entry, cx: &Context<'_>) -> Vec<(String, String)> {
    let Head::Read(header) = &entry.head else {
        return Vec::new();
    };
    let ticks = |ticks: u64| oag_hud::format_lap_time(ticks, oag_hud::Precision::Tenths);
    let mut out = Vec::new();
    out.push((
        cx.say(words::CIRCUIT),
        fit(&cx.circuit_name(header).to_uppercase()),
    ));
    let finish = match header.finish {
        Some(finish) => format!(
            "{}  {}",
            ordinal(finish.place),
            oag_hud::format_lap_time(u64::from(finish.ticks), oag_hud::Precision::Hundredths)
        ),
        None => cx.say(words::NOT_FINISHED),
    };
    out.push((cx.say(words::FINISH), fit(&finish)));
    out.push((
        cx.say(words::MODE),
        fit(&format!(
            "{} {}",
            cx.mode_label(&header.mode),
            header.class.to_uppercase()
        )),
    ));
    let mut notes = Vec::new();
    if entry.folder == Folder::Saved {
        notes.push(cx.say(words::SAVED));
    }
    if let Some(fork) = &header.fork {
        notes.push(cx.say(words::TAKEN_OVER).replace("%s", &ticks(fork.tick)));
    }
    if !claimed(header) {
        notes.push(cx.say(words::NOT_VERIFIED));
    }
    if !notes.is_empty() {
        out.push((cx.say(words::NOTE), fit(&notes.join(", "))));
    }
    let (y, month, day, hour, minute) = civil(entry.unix);
    out.push((
        cx.say(words::DATE),
        format!("{y}-{month:02}-{day:02} {hour:02}:{minute:02} UTC"),
    ));
    out.push((
        cx.say(words::SIZE),
        format!("{}  {}", ticks(header.ticks), kib(entry.bytes)),
    ));
    out
}

fn kib(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

fn ordinal(place: u8) -> String {
    let suffix = match (place % 10, place % 100) {
        (_, 11..=13) => "TH",
        (1, _) => "ST",
        (2, _) => "ND",
        (3, _) => "RD",
        _ => "TH",
    };
    format!("{place}{suffix}")
}

/// `(year, month, day, hour, minute)` in UTC of `unix` seconds: the proleptic Gregorian
/// calendar by the days-from-civil algorithm.
#[must_use]
pub fn civil(unix: u64) -> (u64, u64, u64, u64, u64) {
    let days = unix / 86_400;
    let rest = unix % 86_400;
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day, rest / 3_600, rest % 3_600 / 60)
}

#[cfg(test)]
mod tests;
