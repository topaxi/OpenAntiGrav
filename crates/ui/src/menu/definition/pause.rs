//! The pause screen: a page built from a title's authored pause screen
//! ([`oag_title::PauseMenu`]) and put at the root of a menu tree that opens over a parked
//! race.

use oag_title::{PauseAction, PauseMenu};

use super::{Action, Choice, Definition, Entry, Page, RowMeta};
use crate::language::StringTable;

/// The id of the page [`Definition::open_on_pause_screen`] adds.
pub const PAUSE_PAGE: &str = "pause";

/// The row id of the project's PHOTO MODE row, on the pause page and where the extras attach it.
pub const PHOTO_ROW: &str = "photo_mode";

/// The row id the page gives the row that does `action`: how a drawer that lays the page out
/// from the disc's screen finds the row for a widget.
#[must_use]
pub fn row_id(action: PauseAction) -> &'static str {
    match action {
        PauseAction::Resume => "pause_resume",
        PauseAction::Options => "pause_options",
        PauseAction::Audio => "pause_audio",
        PauseAction::Photo => PHOTO_ROW,
        PauseAction::PilotAssist => "pause_pilot_assist",
        PauseAction::Ghost => "pause_ghost",
        PauseAction::Inert => "pause_inert",
        PauseAction::Restart => "pause_restart",
        PauseAction::Quit => "pause_quit",
    }
}

/// What [`Definition::open_on_pause_screen`] built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pause {
    /// The disc's screen the page was built from.
    pub screen: &'static str,
    /// Rows the disc authors that this build has nothing behind, by `idstring`.
    pub unbuilt: Vec<&'static str>,
    /// Rows this build has something behind whose disc words did not resolve, so the row is
    /// not drawn.
    pub unworded: Vec<&'static str>,
}

impl Definition {
    /// Adds the pause page for the race in `mode` and makes it the root, so a menu opened on
    /// this definition starts on the pause screen and backing out of it leaves the menus.
    ///
    /// The rows are the title's authored ones in the disc's order, worded from `strings`
    /// (the disc's table); a row whose words are missing is left off and named in
    /// [`Pause::unworded`]. A screen with no photo row of its own gets the project's PHOTO
    /// MODE row after OPTIONS when [`PauseMenu::offers_photo`]. GAME OPTIONS opens this
    /// tree's OPTIONS page, AUDIO OPTIONS its AUDIO page. The PHOTO MODE row is taken off every other page: a race is
    /// parked and the pause page is where it lives.
    pub fn open_on_pause_screen(
        &mut self,
        pause: &PauseMenu,
        mode: &str,
        strings: &StringTable,
    ) -> Pause {
        let screen = pause.screen_for(mode);
        let options = self.pages.iter().position(|page| page.id == "options");
        let audio = self.pages.iter().position(|page| page.id == "audio");
        let mut page = Page {
            id: PAUSE_PAGE.to_string(),
            title: strings.get(pause.heading).unwrap_or_default().to_string(),
            entries: Vec::new(),
            subtitles: Vec::new(),
            meta: Vec::new(),
            parked: Vec::new(),
        };
        let mut built = Pause {
            screen: screen.name,
            unbuilt: Vec::new(),
            unworded: Vec::new(),
        };
        let mut has_photo = false;
        let push = |page: &mut Page, id: &str, entry: Entry| {
            page.entries.push(entry);
            page.subtitles.push(None);
            page.meta.push(RowMeta {
                id: Some(id.to_string()),
                parent: None,
            });
        };
        for row in screen.rows {
            let Some(action) = row.builds else {
                built.unbuilt.push(row.disc_id);
                continue;
            };
            let Some(label) = strings.get(row.disc_id).map(str::to_string) else {
                built.unworded.push(row.disc_id);
                continue;
            };
            match action {
                PauseAction::Resume => push(
                    &mut page,
                    row_id(PauseAction::Resume),
                    Entry::Back { label },
                ),
                PauseAction::Options => {
                    let Some(target) = options else {
                        built.unbuilt.push(row.disc_id);
                        continue;
                    };
                    push(
                        &mut page,
                        row_id(PauseAction::Options),
                        Entry::Submenu { label, target },
                    );
                }
                PauseAction::Audio => {
                    let Some(target) = audio else {
                        built.unbuilt.push(row.disc_id);
                        continue;
                    };
                    push(
                        &mut page,
                        row_id(PauseAction::Audio),
                        Entry::Submenu { label, target },
                    );
                }
                PauseAction::Photo => {
                    has_photo = true;
                    push(&mut page, PHOTO_ROW, run(label, Action::PhotoMode));
                }
                PauseAction::PilotAssist => {
                    let (on, off) = (on_off(strings, "FE_ON"), on_off(strings, "FE_OFF"));
                    push(
                        &mut page,
                        row_id(PauseAction::PilotAssist),
                        list(
                            label,
                            "controls.pilot_assist",
                            [("extreme", on), ("off", off)],
                        ),
                    );
                }
                PauseAction::Ghost => {
                    let (on, off) = (on_off(strings, "FE_ON"), on_off(strings, "FE_OFF"));
                    push(
                        &mut page,
                        row_id(PauseAction::Ghost),
                        list(
                            label,
                            "display.ghost_ship_visible",
                            [("off", off), ("on", on)],
                        ),
                    );
                }
                PauseAction::Inert => push(
                    &mut page,
                    row_id(PauseAction::Inert),
                    Entry::Inert { label },
                ),
                PauseAction::Restart => {
                    push(
                        &mut page,
                        row_id(PauseAction::Restart),
                        run(label, Action::RestartRace),
                    );
                }
                PauseAction::Quit => push(
                    &mut page,
                    row_id(PauseAction::Quit),
                    run(label, Action::QuitRace),
                ),
            }
        }
        if pause.offers_photo && !has_photo {
            let label = strings
                .get_or_fallback("OAG_MENU_PHOTO_MODE", "PHOTO MODE")
                .to_string();
            let at = page
                .meta
                .iter()
                .position(|meta| meta.id.as_deref() == Some(row_id(PauseAction::Options)))
                .or_else(|| {
                    page.meta
                        .iter()
                        .position(|meta| meta.id.as_deref() == Some(row_id(PauseAction::Resume)))
                })
                .map_or(0, |index| index + 1);
            page.entries.insert(at, run(label, Action::PhotoMode));
            page.subtitles.insert(at, None);
            page.meta.insert(
                at,
                RowMeta {
                    id: Some(PHOTO_ROW.to_string()),
                    parent: None,
                },
            );
        }
        for other in &mut self.pages {
            other.retain_rows(|entry| {
                !matches!(
                    entry,
                    Entry::Run {
                        action: Action::PhotoMode,
                        ..
                    }
                )
            });
        }
        self.root = self.pages.len();
        self.pages.push(page);
        built
    }
}

fn run(label: String, action: Action) -> Entry {
    Entry::Run {
        label,
        action,
        inputs: None,
    }
}

/// The words of an on/off list's entry, from the disc's table.
fn on_off(strings: &StringTable, id: &str) -> String {
    strings.get(id).unwrap_or(id).to_string()
}

/// An on/off list row: a [`Entry::Choice`] over `values` (`stored value`, `shown words`), in
/// the order the disc's `<List>` spells its entries.
fn list<const N: usize>(label: String, setting: &str, values: [(&str, String); N]) -> Entry {
    Entry::Choice {
        label,
        setting: setting.to_string(),
        values: values
            .into_iter()
            .map(|(value, label)| Choice {
                value: value.to_string(),
                label,
                unavailable: None,
            })
            .collect(),
        source: None,
        query: None,
        default: None,
        current: 0,
        disabled_by: None,
        pins: Vec::new(),
        warnings: Vec::new(),
        restart: None,
    }
}
