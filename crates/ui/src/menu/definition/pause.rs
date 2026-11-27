//! The pause screen: a page built from a title's authored pause screen
//! ([`oag_title::PauseMenu`]) and put at the root of a menu tree that opens over a parked
//! race.

use oag_title::{PauseAction, PauseMenu, PauseRow, PauseScreen};

use super::{Action, Choice, Definition, Entry, Page, RowMeta};
use crate::language::StringTable;

/// The id of the page [`Definition::open_on_pause_screen`] adds.
pub const PAUSE_PAGE: &str = "pause";

/// The row id of a row the disc authors that this build has nothing behind, where one is drawn
/// ([`Definition::open_on_pause_screen_as_authored`]).
pub const UNBUILT_ROW: &str = "pause_unbuilt";

/// What the id of a page built from a screen a pause row opens (a confirm screen, the ghost
/// list) starts with, before the screen's name.
pub const SCREEN_PAGE_PREFIX: &str = "pause_screen:";

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
        PauseAction::GhostList => "pause_ghost_list",
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
        self.build_pause_screen(pause, mode, strings, false)
    }

    /// [`Self::open_on_pause_screen`] for a comparison with the original's frame: a row the
    /// disc authors that this build has nothing behind (the game manual) is drawn too, as a row
    /// that opens nothing, so the page lists what the original lists.
    pub fn open_on_pause_screen_as_authored(
        &mut self,
        pause: &PauseMenu,
        mode: &str,
        strings: &StringTable,
    ) -> Pause {
        self.build_pause_screen(pause, mode, strings, true)
    }

    fn build_pause_screen(
        &mut self,
        pause: &PauseMenu,
        mode: &str,
        strings: &StringTable,
        draw_unbuilt: bool,
    ) -> Pause {
        let screen = pause.screen_for(mode);
        let mut page = Page {
            id: PAUSE_PAGE.to_string(),
            title: strings.get(pause.heading).unwrap_or_default().to_string(),
            entries: Vec::new(),
            subtitles: Vec::new(),
            meta: Vec::new(),
            parked: Vec::new(),
            opens_on: None,
            item_page: None,
        };
        let mut built = Pause {
            screen: screen.name,
            unbuilt: Vec::new(),
            unworded: Vec::new(),
        };
        let before = self.pages.len();
        let has_photo = self.build_rows(
            pause,
            screen.rows,
            strings,
            draw_unbuilt,
            &mut page,
            &mut built,
        );
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
        for other in &mut self.pages[..before] {
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
        for (entry, meta) in page.entries.iter_mut().zip(&page.meta) {
            if let (Entry::Submenu { target, .. }, Some(UNBUILT_ROW)) = (entry, meta.id.as_deref())
            {
                *target = self.root;
            }
        }
        self.pages.push(page);
        built
    }
    /// Pushes `rows` onto `page`; returns whether one of them was the disc's own PHOTO MODE row.
    fn build_rows(
        &mut self,
        pause: &PauseMenu,
        rows: &[PauseRow],
        strings: &StringTable,
        draw_unbuilt: bool,
        page: &mut Page,
        built: &mut Pause,
    ) -> bool {
        let options = self.pages.iter().position(|page| page.id == "options");
        let audio = self.pages.iter().position(|page| page.id == "audio");
        let mut has_photo = false;
        for row in rows {
            let Some(action) = row.builds else {
                built.unbuilt.push(row.disc_id);
                if draw_unbuilt && let Some(label) = strings.get(row.word) {
                    // Opens the page it is on (patched once it has an index): a still never
                    // activates a row.
                    let target = usize::MAX;
                    push_row(
                        page,
                        UNBUILT_ROW,
                        Entry::Submenu {
                            label: label.to_string(),
                            target,
                        },
                    );
                }
                continue;
            };
            let Some(label) = strings.get(row.word).map(str::to_string) else {
                built.unworded.push(row.disc_id);
                continue;
            };
            match action {
                PauseAction::Resume => {
                    push_row(page, row_id(PauseAction::Resume), Entry::Back { label })
                }
                PauseAction::Options => {
                    let Some(target) = options else {
                        built.unbuilt.push(row.disc_id);
                        continue;
                    };
                    push_row(
                        page,
                        row_id(PauseAction::Options),
                        Entry::Submenu { label, target },
                    );
                }
                PauseAction::Audio => {
                    let Some(target) = audio else {
                        built.unbuilt.push(row.disc_id);
                        continue;
                    };
                    push_row(
                        page,
                        row_id(PauseAction::Audio),
                        Entry::Submenu { label, target },
                    );
                }
                PauseAction::Photo => {
                    has_photo = true;
                    push_row(page, PHOTO_ROW, run(label, Action::PhotoMode));
                }
                PauseAction::PilotAssist => {
                    let (on, off) = (on_off(strings, "FE_ON"), on_off(strings, "FE_OFF"));
                    push_row(
                        page,
                        row_id(PauseAction::PilotAssist),
                        list(
                            label,
                            "controls.pilot_assist",
                            [("extreme", on), ("off", off)],
                        ),
                    );
                }
                PauseAction::Ghost => {
                    if let Some(opened) = row.opens.and_then(|name| pause.screen(name)) {
                        let target = self.screen_page(pause, opened, strings, built);
                        push_row(
                            page,
                            row_id(PauseAction::Ghost),
                            Entry::Submenu { label, target },
                        );
                        continue;
                    }
                    let (on, off) = (on_off(strings, "FE_ON"), on_off(strings, "FE_OFF"));
                    push_row(
                        page,
                        row_id(PauseAction::Ghost),
                        list(
                            label,
                            "display.ghost_ship_visible",
                            [("off", off), ("on", on)],
                        ),
                    );
                }
                PauseAction::GhostList => {
                    let (Some(none), Some(best)) =
                        (strings.get("GHOST_NONE"), strings.get("GHOST_MY_BEST"))
                    else {
                        built.unworded.push(row.disc_id);
                        continue;
                    };
                    push_row(
                        page,
                        row_id(PauseAction::GhostList),
                        list(
                            label,
                            "display.ghost_ship_visible",
                            [("off", none.to_string()), ("on", best.to_string())],
                        ),
                    );
                }
                PauseAction::Inert => {
                    push_row(page, row_id(PauseAction::Inert), Entry::Inert { label })
                }
                PauseAction::Restart | PauseAction::Quit => {
                    let (id, act) = if action == PauseAction::Restart {
                        (row_id(PauseAction::Restart), Action::RestartRace)
                    } else {
                        (row_id(PauseAction::Quit), Action::QuitRace)
                    };
                    let entry = match row.opens.and_then(|name| pause.screen(name)) {
                        Some(opened) => {
                            let target = self.screen_page(pause, opened, strings, built);
                            Entry::Submenu { label, target }
                        }
                        None => run(label, act),
                    };
                    push_row(page, id, entry);
                }
            }
        }
        has_photo
    }

    /// The page of a screen a row opens: `opened`'s rows, behind the id [`SCREEN_PAGE_PREFIX`]
    /// gives the screen. It opens on its way back, the row that resumes (a confirm screen: a second
    /// press of the key that opened it cancels), or on its list. A word the screen lacks is
    /// reported in `built` as on the pause page. Returns the page's index.
    fn screen_page(
        &mut self,
        pause: &PauseMenu,
        opened: &PauseScreen,
        strings: &StringTable,
        built: &mut Pause,
    ) -> usize {
        let mut page = Page {
            id: format!("{SCREEN_PAGE_PREFIX}{}", opened.name),
            title: strings.get(pause.heading).unwrap_or_default().to_string(),
            entries: Vec::new(),
            subtitles: Vec::new(),
            meta: Vec::new(),
            parked: Vec::new(),
            opens_on: None,
            item_page: None,
        };
        self.build_rows(pause, opened.rows, strings, false, &mut page, built);
        // The way back is never dropped for want of its words: a page that asks must always have
        // the row to cancel on, so it takes the fallback words and the gap is reported.
        let back = row_id(PauseAction::Resume);
        if opened
            .rows
            .iter()
            .any(|row| row.builds == Some(PauseAction::Resume))
            && page.row_with_id(back).is_none()
        {
            let label = strings.get_or_fallback("FE_CANCEL", "CANCEL").to_string();
            push_row(&mut page, back, Entry::Back { label });
        }
        let list = row_id(PauseAction::GhostList);
        page.opens_on = Some(if page.row_with_id(list).is_some() {
            list.to_string()
        } else {
            row_id(PauseAction::Resume).to_string()
        });
        self.pages.push(page);
        self.pages.len() - 1
    }
}

fn push_row(page: &mut Page, id: &str, entry: Entry) {
    page.entries.push(entry);
    page.subtitles.push(None);
    page.meta.push(RowMeta {
        id: Some(id.to_string()),
        parent: None,
    });
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
