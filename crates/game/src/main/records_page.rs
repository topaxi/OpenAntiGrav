//! The RECORDS page's own live table: what `oag_game::records::Store` says
//! about whichever track and mode its own MODE/TRACK rows currently hold,
//! one row per speed class this title's ladder offers.
//!
//! **This is the disc's own shape, not an invented one.** `Data\Plugins\
//! PI001\GUI\RecordGrid_Definition.xml`'s "Speed Lap Records" screen is
//! exactly this: a track picker over a table with one row per speed class
//! and a `PRO_TIME` column - see `docs/architecture/menus.md`'s RECORDS
//! section for the read and its confidence score. What is left out (TAG,
//! TEAM, the boost/perfect-lap icons) is left out because
//! `oag_game::records::Record` carries no pilot identity or per-lap
//! breakdown to draw there - see that struct's own doc for why the schema
//! is chosen rather than measured. Drawing an invented tag or team would be
//! exactly the "plausible-looking stand-in" `CLAUDE.md`'s own rule forbids;
//! leaving the columns off is the honest absence instead.
//!
//! Split the same way `oag_game::pilots::axis_preview_for` is, for the same
//! reason its own doc gives: the pure resolution lives here so both draw
//! paths - the live session and (should it ever grow one) a headless
//! capture - can share it, even though only the live session reaches this
//! today. `crate::menu_stage::MenuStage::render` draws the result as one
//! `Draw::Text` pair per row, continuing the page's own row pitch straight
//! past its last real entry (MODE, TRACK, BACK) rather than through the
//! single-line note slot `oag_ui::prompt::axis_preview_draw` reserves for
//! AI PILOTS - four lines do not fit in a reservation sized for one, and
//! widening that reservation would have to touch `crates/game/src/capture/
//! menu_page.rs` to keep the headless and live paths agreeing, the same
//! mismatch `docs/architecture/menus.md`'s AI PILOTS section already
//! records paying for once. Three rows (MODE, TRACK, BACK) plus four class
//! rows is seven - exactly the row budget Pulse's own skin already fits
//! with nothing reserved, so nothing here needs a reservation at all.

use oag_game::records::{self, Store};
use oag_game::scoreboard;
use oag_ui::menu;

use crate::session::Shell;

/// The page id this module answers for. Checked explicitly rather than
/// inferred from which rows are on screen - RACE also carries `race.mode`/
/// `race.track` rows, so a value-based search would draw this table there
/// too, the exact bug `oag_game::pilots::axis_preview_for`'s own doc
/// describes hitting once already.
const PAGE_ID: &str = "records";

/// Whether `mode` produces a *finished* time worth showing over a lap one.
///
/// `oag_game::records::Record::best_total_ticks` is only ever set when
/// `Observation::finished` was true, and that field's own module doc names
/// Speed Lap and Zone as the two modes whose race never finishes at all -
/// `oag_race::Mode::laps_target` is `None` for both, so a total column would
/// be permanently empty there. Every other mode does finish, so its total is
/// the more useful number to show than a fastest single lap.
fn show_total(mode: oag_race::Mode) -> bool {
    !matches!(mode, oag_race::Mode::SpeedLap | oag_race::Mode::Zone)
}

/// What a `choice` row named `setting` currently holds, as plain text - the
/// row's *value*, not its display label, the same distinction
/// `oag_game::pilots::axis_preview_for` reads `Menu::Entry::chosen` for.
fn held_text(model: &menu::Menu, setting: &str) -> Option<String> {
    model
        .page()
        .entries
        .iter()
        .find(|entry| entry.setting() == Some(setting))
        .and_then(menu::Entry::chosen)
        .and_then(|value| match value {
            menu::Value::Text(text) => Some(text),
            menu::Value::Flag(_) => None,
        })
}

/// The pure half of [`table_for`]: one `(label, time)` pair per entry in
/// `classes`, given an already-resolved title, `.vex` entry name and mode.
///
/// Split out so this can be unit-tested with a hand-built `classes` list and
/// `Store`, with no `Shell` and no `'static oag_title::Title` to fabricate -
/// see [`table_for`]'s own doc for why building a real one is not practical
/// in a unit test.
fn build_table(
    title_name: &str,
    entry_name: &str,
    mode: oag_race::Mode,
    classes: &[menu::Choice],
    store: &Store,
) -> Vec<(String, String)> {
    let want_total = show_total(mode);
    classes
        .iter()
        .map(|choice| {
            let key = records::Key::new(title_name, Some(entry_name), mode.name(), &choice.value);
            let value = scoreboard::record_table_value(store.get(&key), want_total);
            (choice.label.clone(), value)
        })
        .collect()
}

/// One `(label, time)` pair per speed class this title's ladder offers -
/// `label` off `crate::session::menus::speed_class_choices`, the same list
/// the RACE page's own SPEED CLASS row is built from, so the two can never
/// name a different set of classes. `None` off any page but RECORDS, or
/// while nothing has resolved a track to look records up against yet -
/// which is unreached in practice once a shell exists, since `Menu::seed`
/// always leaves a `choice` row on some value, but real all the same for a
/// menu opened before a shell finished loading.
#[must_use]
pub(crate) fn table_for(
    model: &menu::Menu,
    shell: &Shell,
    store: &Store,
) -> Option<Vec<(String, String)>> {
    if model.page().id != PAGE_ID {
        return None;
    }
    let mode = oag_race::Mode::from_name(&held_text(model, "race.mode")?)?;
    let track_id = held_text(model, "race.track")?;
    let track = shell
        .tracks_for(mode)
        .iter()
        .map(|(track, _)| track)
        .find(|track| track.id == track_id)?;
    let entry_name = track.entry_name();
    let classes = crate::session::menus::speed_class_choices(shell.title);
    Some(build_table(
        shell.title.name,
        &entry_name,
        mode,
        &classes,
        store,
    ))
}

// `#[path]`, not a plain `mod tests;` - this file itself was loaded via
// `#[path = "main/records_page.rs"]` from `main.rs`, and a `#[path]`ed
// module's own children resolve *next to* it rather than under it (see
// `main.rs`'s own comment on its `mod` block). A plain `mod tests;` here
// would have resolved to `main/tests.rs` - `main.rs`'s *own* test module,
// already declared there - and silently recompiled that file a second time
// as this module's child, with `super::*` now missing everything `main.rs`
// only imports under `#[cfg(test)]` for its own `tests` module to reach.
// Caught by `cargo check --all-targets` failing forty of `main/tests.rs`'s
// own unrelated assertions with "cannot find X in this scope".
#[cfg(test)]
#[path = "records_page/tests.rs"]
mod tests;
