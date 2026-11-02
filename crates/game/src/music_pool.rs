//! The MUSIC page: one row that walks the music sources and one that switches
//! the source it is on, the shape of the MODS page (`crate::mods`).
//!
//! The sources are `oag_sound::pool`'s: the booted disc, other titles' discs and
//! the folders named in `settings.toml`. The rows are re-supplied whenever the
//! pool's [`oag_sound::pool::Pool::revision`] moves, so a scan that lands while
//! the page is open shows up without leaving it.

use oag_sound::pool::{Pool, SourceRow, SourceStatus};
use oag_ui::language::StringTable;
use oag_ui::menu::{Choice, Menu, Value, ValueSource};

/// The page the sources are listed on.
pub const PAGE: &str = "music";
/// The SOURCE row. Never persisted: it only says which source ENABLED is about.
pub const SELECTED: &str = "audio.music_pool";
/// The ENABLED row: the switch of the source [`SELECTED`] is on, kept in
/// `[audio] music_switches` rather than under this key.
pub const ENABLED: &str = "audio.music_pool_enabled";

/// Words the page says, with their English fallback.
pub mod words {
    /// A source whose scan has not answered.
    pub const SCANNING: (&str, &str) = ("OAG_MUSIC_SCANNING", "SCANNING");
    /// A source that is in the pool.
    pub const ON: (&str, &str) = ("OAG_MODS_ON", "ON");
    /// A source that is out of it.
    pub const OFF: (&str, &str) = ("OAG_MODS_OFF", "OFF");
}

/// The width a source's label may take in a row.
const LABEL_CHARS: usize = 22;

fn fit(text: &str) -> String {
    if text.chars().count() <= LABEL_CHARS {
        return text.to_string();
    }
    let keep: String = text
        .chars()
        .skip(text.chars().count() - (LABEL_CHARS - 3))
        .collect();
    format!("...{keep}")
}

/// The choices for `ValueSource::MusicPool`: every source, by id.
#[must_use]
pub fn choices(rows: &[SourceRow]) -> Vec<Choice> {
    rows.iter()
        .map(|row| Choice::labelled(&row.id, fit(&row.label)))
        .collect()
}

/// The ENABLED row's choices for `row`, and the value to seed: `on`/`off`, or the one
/// dimmed value a source that cannot be switched yet holds.
#[must_use]
pub fn switch_for(row: &SourceRow, strings: &StringTable) -> (Vec<Choice>, &'static str) {
    let say = |(id, english): (&str, &str)| strings.get_or_fallback(id, english).to_string();
    let (value, word) = if row.enabled {
        ("on", words::ON)
    } else {
        ("off", words::OFF)
    };
    match &row.status {
        SourceStatus::Scanning => (
            vec![Choice::labelled(value, say(word)).unusable(say(words::SCANNING))],
            value,
        ),
        SourceStatus::Unavailable(why) => (
            vec![Choice::labelled(value, say(word)).unusable(why.clone())],
            value,
        ),
        SourceStatus::Ready(_) => (
            vec![
                Choice::labelled("on", say(words::ON)),
                Choice::labelled("off", say(words::OFF)),
            ],
            value,
        ),
    }
}

/// The source the SOURCE row is on.
#[must_use]
pub fn selected<'a>(model: &Menu, rows: &'a [SourceRow]) -> Option<&'a SourceRow> {
    let held = model
        .definition()
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some(SELECTED))
        .and_then(oag_ui::menu::Entry::chosen)?;
    let Value::Text(id) = held else {
        return None;
    };
    rows.iter().find(|row| row.id == id)
}

/// Supplies both rows from the pool as it stands, keeping the source the SOURCE row is on.
pub fn supply(model: &mut Menu, pool: &Pool, strings: &StringTable) {
    let rows = pool.rows();
    model.supply(ValueSource::MusicPool, &choices(&rows));
    let Some(row) = selected(model, &rows) else {
        model.supply(ValueSource::MusicPoolSwitch, &[]);
        return;
    };
    let (switch, value) = switch_for(row, strings);
    model.supply(ValueSource::MusicPoolSwitch, &switch);
    model.seed(ENABLED, &Value::Text(value.to_string()));
}
