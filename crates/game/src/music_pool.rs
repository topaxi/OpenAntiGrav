//! The MUSIC page: one on/off row per music source.
//!
//! The sources are `oag_sound::pool`'s: the booted disc, other titles' discs and the
//! folders named in `settings.toml`. `menu.toml` holds one template row
//! ([`TEMPLATE`]); [`expand`] turns it into a row per source before the menu model is
//! made (`oag_ui::menu::Definition::expand_rows`), and [`refresh`] moves a row's
//! status afterwards. With fewer than two sources there is nothing to choose and
//! neither the page nor the row that opens it is left in the tree.

use oag_sound::pool::{Pool, SourceRow, SourceStatus};
use oag_ui::language::StringTable;
use oag_ui::menu::{Choice, Definition, Menu, RowItem, ValueSource};

/// The page the sources are listed on.
pub const PAGE: &str = "music";
/// The template row's setting; a source's row edits `<TEMPLATE>.<source id>`.
pub const TEMPLATE: &str = "audio.music_pool";

/// Words the page says, with their English fallback.
pub mod words {
    /// A source whose scan has not answered.
    pub const SCANNING: (&str, &str) = ("OAG_MUSIC_SCANNING", "SCANNING");
    /// A source that is in the pool.
    pub const ON: (&str, &str) = ("OAG_MODS_ON", "ON");
    /// A source that is out of it.
    pub const OFF: (&str, &str) = ("OAG_MODS_OFF", "OFF");
}

/// The source a row's setting edits, when it is one of the page's rows.
#[must_use]
pub fn source_of(setting: &str) -> Option<&str> {
    setting.strip_prefix(TEMPLATE)?.strip_prefix('.')
}

fn choices(row: &SourceRow, strings: &StringTable) -> Vec<Choice> {
    let say = |(id, english): (&str, &str)| strings.get_or_fallback(id, english).to_string();
    let on = Choice::labelled("on", say(words::ON));
    let off = Choice::labelled("off", say(words::OFF));
    let reason = match &row.status {
        SourceStatus::Ready(_) => return vec![on, off],
        SourceStatus::Scanning => say(words::SCANNING),
        SourceStatus::Unavailable(why) => why.clone(),
    };
    if row.enabled {
        vec![on.unusable(reason)]
    } else {
        vec![off.unusable(reason)]
    }
}

fn items(pool: &Pool, strings: &StringTable) -> Vec<RowItem> {
    let rows = pool.rows();
    if rows.len() < 2 {
        return Vec::new();
    }
    rows.iter()
        .map(|row| RowItem {
            id: row.id.clone(),
            label: row.label.clone(),
            detail: row.detail.clone(),
            values: choices(row, strings),
            current: if row.enabled { "on" } else { "off" }.to_string(),
        })
        .collect()
}

/// Makes the page's rows from the pool as it stands, or takes the page out when the pool
/// has nothing to choose between.
pub fn expand(definition: &mut Definition, pool: &Pool, strings: &StringTable) {
    if definition.expand_rows(ValueSource::MusicPool, &items(pool, strings)) == 0 {
        definition.drop_submenus_to(PAGE);
    }
}

/// What the rows would say, without the switches: a change in it means the tree needs
/// expanding again (a scan landed, a source appeared).
#[must_use]
pub fn signature(pool: &Pool) -> String {
    pool.rows()
        .iter()
        .map(|row| format!("{}|{}|{:?}", row.id, row.label, row.status))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `menu` shows the rows [`expand`] would make for `pool` now.
#[must_use]
pub fn is_current(menu: &Menu, pool: &Pool) -> bool {
    let rows = pool.rows();
    let wanted: Vec<String> = if rows.len() < 2 {
        Vec::new()
    } else {
        rows.iter()
            .map(|row| format!("{TEMPLATE}.{}", row.id))
            .collect()
    };
    menu.expanded_settings(TEMPLATE) == wanted
}
