//! Rows attached to another, on a front end that has no room for them in the
//! row of tabs: HD's and Omega's horizontal top level.
//!
//! A flat-list front end (Pulse, Pure, 2048) draws an attached row inline after
//! its parent and needs nothing of this module. A strip title
//! ([`oag_title::Attached::Revealed`]) draws only the top-level rows as tabs and
//! shows a row's attached rows **below it while the row, or one of its attached
//! rows, has focus**. Left and right step between top-level rows; up and down
//! step inside the focused row's family and stop at its ends. All of it is
//! **chosen, not measured**: the discs author no second level on the main menu.
//!
//! Hidden rows are not drawn, not given a pointer region and not reachable by the
//! pad.

use crate::menu::nav::{Dir, Nav};
use oag_core::buttons::{Button, Input};

use super::{Menu, MenuEvent};

/// One tab of a revealing strip: a row, and how far below the top level it sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Tab {
    /// Index into the page's entries.
    pub row: usize,
    /// `0` on the strip itself, `1..` for the first, second... attached row below.
    pub depth: usize,
}

impl Menu {
    /// Whether attached rows are revealed under their parent rather than drawn
    /// inline. Set by whoever is drawing, from the title
    /// ([`oag_title::MainMenu::attached`]), as [`Self::set_strip_layout`] is.
    pub fn set_reveal_attached(&mut self, reveal: bool) {
        self.reveal = reveal;
        let page = self.current();
        self.focus[page] = (0..self.focus[page].len())
            .map(|row| self.focus_target(page, row))
            .collect();
    }

    /// Puts the cursor on the row of the page on screen with this `id`, with its
    /// block grown as at rest. `false` when there is none.
    pub fn focus_row(&mut self, id: &str) -> bool {
        let page = self.current();
        let Some(row) = self.definition.pages[page].row_with_id(id) else {
            return false;
        };
        self.cursor[page] = row;
        self.snap_focus();
        true
    }

    /// Whether this page is drawn as a strip that reveals attached rows.
    pub(super) fn reveals(&self) -> bool {
        self.reveal && super::strip::suits(self.page())
    }

    /// The tabs to draw, in order: the top level, with the focused family's
    /// attached rows after their head. Every row at depth `0` when nothing is
    /// revealed.
    pub(super) fn tabs(&self) -> Vec<Tab> {
        let page = self.page();
        if !self.reveals() {
            return (0..page.entries.len())
                .map(|row| Tab { row, depth: 0 })
                .collect();
        }
        let focused = page.family_of(self.selected());
        let mut out = Vec::new();
        for head in page.top_level() {
            out.push(Tab {
                row: head,
                depth: 0,
            });
            if head == focused {
                out.extend(
                    page.family(head)
                        .filter(|&row| row != head)
                        .enumerate()
                        .map(|(k, row)| Tab { row, depth: k + 1 }),
                );
            }
        }
        out
    }

    /// Whether `row`'s tab draws as the selected one: it is, or it heads the
    /// family the selection is in.
    pub(super) fn tab_selected(&self, row: usize) -> bool {
        row == self.selected()
            || (self.reveals() && {
                let page = self.page();
                !page.is_attached(row) && page.family_of(self.selected()) == row
            })
    }

    /// What `row`'s block grows toward: `1.0` for the selected row and, when
    /// attached rows are revealed, for the head of its family.
    pub(super) fn focus_target(&self, page: usize, row: usize) -> f32 {
        let definition = &self.definition.pages[page];
        let selected = self.cursor[page];
        let head = self.reveal
            && !definition.is_attached(row)
            && definition.family_of(selected) == row
            && super::strip::suits(definition);
        if row == selected || head { 1.0 } else { 0.0 }
    }

    fn step_within(&mut self, delta: i64) -> bool {
        let page = self.current();
        let family: Vec<usize> = {
            let definition = &self.definition.pages[page];
            definition
                .family(definition.family_of(self.cursor[page]))
                .collect()
        };
        let Some(at) = family.iter().position(|&row| row == self.cursor[page]) else {
            return false;
        };
        let to = i64::try_from(at).unwrap_or(0) + delta;
        let Some(&row) = usize::try_from(to).ok().and_then(|to| family.get(to)) else {
            return false;
        };
        self.cursor[page] = row;
        true
    }

    fn step_family(&mut self, delta: i64) -> bool {
        let page = self.current();
        let heads: Vec<usize> = self.definition.pages[page].top_level().collect();
        let here = self.definition.pages[page].family_of(self.cursor[page]);
        let Some(at) = heads.iter().position(|&head| head == here) else {
            return false;
        };
        let count = i64::try_from(heads.len()).unwrap_or(1);
        let to = (i64::try_from(at).unwrap_or(0) + delta).rem_euclid(count);
        self.cursor[page] = heads[usize::try_from(to).unwrap_or(0)];
        heads.len() > 1
    }

    /// [`Self::update`] on a page that reveals attached rows.
    pub(super) fn update_revealed(&mut self, input: &mut Input) -> Vec<MenuEvent> {
        let mut out = Vec::new();
        let mut moved = None;
        if input.take(Button::Down) && self.step_within(1) {
            moved = Some(Dir::Down);
        }
        if input.take(Button::Up) && self.step_within(-1) {
            moved = Some(Dir::Up);
        }
        if input.take(Button::Right) && self.step_family(1) {
            moved = Some(Dir::Right);
        }
        if input.take(Button::Left) && self.step_family(-1) {
            moved = Some(Dir::Left);
        }
        if let Some(dir) = moved {
            self.nav.push(Nav::Moved(Some(dir)));
        }
        if input.take(Button::Cross) || input.take(Button::Start) {
            out.extend(self.activate());
        }
        if input.take(Button::Circle) {
            out.extend(self.back());
        }
        out
    }
}
