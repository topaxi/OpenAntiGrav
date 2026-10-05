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
//! **The pure resolution lives in `oag_game::scoreboard::records_table`, not
//! here.** It used to live in this module, split out the same way
//! `oag_raceplay::pilots::axis_preview_for` is - but that kept it reachable only
//! from this binary, and `crate::capture::menu_page` (the `--menu-page
//! records` still, in the library crate) needed the identical resolution so
//! the live page and its own capture cannot silently disagree about which
//! classes exist or which column a mode draws. This module is now the thin
//! part left behind: supplying `records_table` with the one thing that
//! differs between the two callers, a track lookup drawn from
//! [`Shell::tracks_for`], which carries a distinct Zone list a capture's own
//! boot survey does not.
//!
//! `crate::menu_stage::MenuStage::render` draws the result as one
//! `Draw::Text` pair per row, continuing the page's own row pitch straight
//! past its last real entry (MODE, TRACK, BACK) rather than through the
//! single-line note slot `oag_ui_screens::prompt::axis_preview_draw` reserves for
//! AI PILOTS - four lines do not fit in a reservation sized for one, and
//! widening that reservation would have to touch `crates/game/src/capture/
//! menu_page.rs` to keep the headless and live paths agreeing, the same
//! mismatch `docs/architecture/menus.md`'s AI PILOTS section already
//! records paying for once. Three rows (MODE, TRACK, BACK) plus four class
//! rows is seven - exactly the row budget Pulse's own skin already fits
//! with nothing reserved, so nothing here needs a reservation at all.

use oag_game::records::Store;
use oag_game::scoreboard;
use oag_ui::menu;

use crate::session::Shell;

/// One `(label, time)` pair per speed class this title's ladder offers -
/// `label` off `oag_game::scoreboard::speed_class_choices`, the same list
/// the RACE page's own SPEED CLASS row is built from, so the two can never
/// name a different set of classes. `None` off any page but RECORDS, or
/// while nothing has resolved a track to look records up against yet -
/// which is unreached in practice once a shell exists, since `Menu::seed`
/// always leaves a `choice` row on some value, but real all the same for a
/// menu opened before a shell finished loading.
///
/// This is a thin wrapper over `scoreboard::records_table`, supplying the
/// one thing that differs from `crate::capture::menu_page`'s own still of
/// this page: `Shell::tracks_for(mode)` carries a distinct Zone track list,
/// which a `--menu-page records` capture's own boot survey does not - see
/// that function's own doc. Not unit-tested here for the same reason
/// `oag_raceplay::pilots::axis_preview_for`'s own live callers are not: building
/// a real `Shell` needs a font atlas and a sprite sheet, neither practical to
/// fabricate in a unit test. `scoreboard::records_table`'s own tests cover
/// the resolution itself, with a hand-built track lookup standing in for
/// `Shell::tracks_for`.
#[must_use]
pub(crate) fn table_for(
    model: &menu::Menu,
    shell: &Shell,
    store: &Store,
) -> Option<Vec<(String, String)>> {
    scoreboard::records_table(model, shell.title, store, |mode, track_id| {
        shell
            .tracks_for(mode)
            .iter()
            .map(|(track, _)| track)
            .find(|track| track.id == track_id)
            .map(oag_raceplay::catalogue::Track::entry_name)
    })
}
