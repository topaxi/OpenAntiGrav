//! The chooser as cards: one per title, drawn from the title's own system art
//! (`docs/ui/launcher.md`).
//!
//! A card is a view over [`Launcher::rows`]: every row of a title is one of the
//! card's *variants*, and a row with two pictures is two variants. The cursor
//! stays a row index, so what picking, the disc-key prompt and a dropped file do
//! is the list's own code. The focused card's shown variant is what the cursor
//! sits on.
//!
//! # The art
//!
//! Pixels live in the sprite sheet the stage builds ([`super::art`]), under
//! [`picture_key`] and [`logo_key`]. This module knows only which pictures each
//! source has, and a card whose picture is not in the sheet says its name.

use std::collections::BTreeMap;

use oag_hud::sprite::Sheet;
use oag_ui::frontend::{Align, Draw};
use oag_ui::hints::{Entry, Press};
use oag_ui::pointer::Pointer;

use oag_display::space::SCREEN;
use oag_gameplay::input::{Button, Input};

use super::{Candidate, Launcher, State};

/// The sheet key of `source`'s picture `name`.
#[must_use]
pub fn picture_key(source: &str, name: &str) -> String {
    format!("card:{source}:{name}")
}

/// The sheet key of the logo drawn over that picture.
#[must_use]
pub fn logo_key(source: &str, name: &str) -> String {
    format!("card:{source}:{name}:logo")
}

/// The sheet key of the cog that marks a card with more than one picture.
pub const COG_KEY: &str = "card:cog";

/// One thing a card can show: a row, and which of its pictures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// Index into [`Launcher::rows`].
    pub row: usize,
    /// The row's picture, `None` when it has none.
    pub picture: Option<String>,
}

/// One card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// What a remembered picture is filed under: the title's slug, or the file
    /// name of a row that would not open.
    pub key: String,
    /// What the card can show, in the order the cog steps through it.
    pub variants: Vec<Variant>,
}

/// What the cards say, resolved from the project's string table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Words {
    choose: String,
    r#move: String,
    start: String,
    picture: String,
    quit: String,
    encrypted: String,
}

impl Words {
    /// Looks the ids up in `table`, falling back to the id itself so a gap
    /// is seen.
    #[must_use]
    pub fn from_table(table: &oag_ui::language::StringTable) -> Self {
        let say = |id: &str| table.get(id).unwrap_or(id).to_string();
        Self {
            choose: say("OAG_LAUNCHER_CHOOSE"),
            r#move: say("OAG_LAUNCHER_MOVE"),
            start: say("OAG_LAUNCHER_START"),
            picture: say("OAG_LAUNCHER_PICTURE"),
            quit: say("OAG_LAUNCHER_QUIT"),
            encrypted: say("OAG_LAUNCHER_ENCRYPTED"),
        }
    }
}

/// The card mode's own state, beside the list's.
#[derive(Debug)]
pub(super) struct Cards {
    pub(super) words: Words,
    /// Source to the names of its pictures, as they were read.
    pictures: BTreeMap<String, Vec<String>>,
    /// Card key to the variant it shows, remembered and chosen alike.
    shown: BTreeMap<String, String>,
    /// The focused card's index.
    focus: usize,
    /// A choice the player made that nothing has saved yet.
    changed: Option<(String, String)>,
}

impl Launcher {
    /// Shows cards instead of the list, with each card left where `remembered`
    /// (settings' `[launcher] pictures`) says.
    #[must_use]
    pub fn with_cards(mut self, words: Words, remembered: BTreeMap<String, String>) -> Self {
        self.cards = Some(Cards {
            words,
            pictures: BTreeMap::new(),
            shown: remembered,
            focus: 0,
            changed: None,
        });
        let first = self.rows.iter().position(Candidate::is_playable);
        self.follow_row(first.unwrap_or(self.cursor), false);
        // The cursor may sit on a row no card holds (a locked copy listed
        // ahead of the one that plays).
        self.sync();
        self
    }

    /// Whether this chooser draws cards.
    #[must_use]
    pub fn is_cards(&self) -> bool {
        self.cards.is_some()
    }

    /// The cards, in the order they are drawn. Empty in list mode.
    #[must_use]
    pub fn card_list(&self) -> Vec<Card> {
        self.cards
            .as_ref()
            .map_or_else(Vec::new, |cards| build(&self.rows, &cards.pictures))
    }

    /// The focused card's index.
    #[must_use]
    pub fn focused_card(&self) -> usize {
        self.cards.as_ref().map_or(0, |cards| cards.focus)
    }

    /// The variant `card` shows.
    #[must_use]
    pub fn shown_variant(&self, card: &Card) -> usize {
        let Some(cards) = &self.cards else { return 0 };
        shown_index(&self.rows, card, cards.shown.get(&card.key))
    }

    /// The sources whose art is worth reading: those that open. A locked disc's
    /// pictures are ciphertext until its key is in.
    pub fn art_sources(&self) -> impl Iterator<Item = &str> {
        self.rows
            .iter()
            .filter(|row| row.is_playable())
            .map(|row| row.source.as_str())
    }

    /// Records the pictures `source` has, once they have been read.
    pub fn set_pictures(&mut self, source: &str, names: Vec<String>) {
        if let Some(cards) = &mut self.cards {
            cards.pictures.insert(source.to_string(), names);
        }
        self.sync();
    }

    /// The choice the player made since the last call, as `(card, variant)`, for
    /// the session to save.
    pub fn take_choice(&mut self) -> Option<(String, String)> {
        self.cards.as_mut()?.changed.take()
    }

    /// Puts the cursor on the shown variant of the focused card.
    fn sync(&mut self) {
        let list = self.card_list();
        let Some(cards) = &mut self.cards else { return };
        cards.focus = cards.focus.min(list.len().saturating_sub(1));
        let Some(card) = list.get(cards.focus) else {
            return;
        };
        let at = shown_index(&self.rows, card, cards.shown.get(&card.key));
        self.cursor = card.variants[at].row;
    }

    /// Focuses the card holding `row`. With `show`, the card also switches to
    /// that row (a dropped disc), which is not a choice to remember.
    pub(super) fn follow_row(&mut self, row: usize, show: bool) {
        let list = self.card_list();
        let Some(cards) = &mut self.cards else { return };
        let found = list.iter().enumerate().find_map(|(at, card)| {
            let variant = card.variants.iter().find(|v| v.row == row)?;
            Some((at, card, variant))
        });
        // A row no card holds (a locked copy of a title that already plays)
        // leaves the cursor alone, so the key prompt opens on it.
        let Some((at, card, variant)) = found else {
            return;
        };
        cards.focus = at;
        if show {
            cards
                .shown
                .insert(card.key.clone(), variant_key(&self.rows, variant));
        }
        self.sync();
    }

    /// Steps card `at` to its next variant, which is the player's choice.
    fn cycle(&mut self, at: usize) {
        let list = self.card_list();
        let Some(card) = list.get(at) else { return };
        if card.variants.len() < 2 {
            return;
        }
        let next = (self.shown_variant(card) + 1) % card.variants.len();
        let key = variant_key(&self.rows, &card.variants[next]);
        let Some(cards) = &mut self.cards else { return };
        cards.focus = at;
        cards.shown.insert(card.key.clone(), key.clone());
        cards.changed = Some((card.key.clone(), key));
        self.sync();
    }

    fn note_lines(&self) -> Vec<String> {
        let encrypted = self
            .cards
            .as_ref()
            .map_or("", |c| c.words.encrypted.as_str());
        notes(&self.rows, encrypted)
    }

    fn selectable_card(&self, card: &Card) -> bool {
        card.variants
            .iter()
            .any(|v| self.rows[v.row].is_selectable())
    }

    /// Moves the focus `step` cards, wrapping, stepping over cards that will
    /// not open.
    fn move_focus(&mut self, step: isize) {
        let list = self.card_list();
        let count = list.len();
        let Some(cards) = &self.cards else { return };
        let mut at = cards.focus;
        for _ in 0..count {
            at = at
                .wrapping_add_signed(step)
                .wrapping_add(count)
                .rem_euclid(count);
            if self.selectable_card(&list[at]) {
                self.focus_card(at);
                return;
            }
        }
    }

    /// Moves a row up or down, to the card nearest below or above.
    fn move_row(&mut self, down: bool) {
        let list = self.card_list();
        let Some(cards) = &self.cards else { return };
        let rects = layout(list.len(), self.note_lines().len());
        let Some(from) = rects.get(cards.focus).copied() else {
            return;
        };
        let centre = |r: [f32; 4]| r[0] + r[2] / 2.0;
        let target = rects
            .iter()
            .enumerate()
            .filter(|(at, r)| {
                self.selectable_card(&list[*at])
                    && if down {
                        r[1] > from[1] + 1.0
                    } else {
                        r[1] < from[1] - 1.0
                    }
            })
            .min_by(|(_, a), (_, b)| {
                let key =
                    |r: &[f32; 4]| ((r[1] - from[1]).abs(), (centre(*r) - centre(from)).abs());
                let (a, b) = (key(a), key(b));
                a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1))
            })
            .map(|(at, _)| at);
        if let Some(at) = target {
            self.focus_card(at);
        }
    }

    fn focus_card(&mut self, at: usize) {
        if let Some(cards) = &mut self.cards {
            cards.focus = at;
        }
        self.sync();
    }

    /// Confirms the focused card: boots it, or opens the key prompt.
    fn confirm_card(&mut self) -> Option<String> {
        if self.open_entry() {
            return None;
        }
        self.pick()
    }

    /// [`Launcher::update`], in card mode.
    pub(super) fn update_cards(&mut self, input: &mut Input) -> Option<String> {
        if input.take(Button::Right) {
            self.move_focus(1);
        }
        if input.take(Button::Left) {
            self.move_focus(-1);
        }
        if input.take(Button::Down) {
            self.move_row(true);
        }
        if input.take(Button::Up) {
            self.move_row(false);
        }
        if input.take(Button::Square) {
            self.cycle(self.focused_card());
        }
        let confirmed = input.take(Button::Cross) | input.take(Button::Start);
        if confirmed { self.confirm_card() } else { None }
    }

    /// [`Launcher::pointer`], in card mode: hover focuses, a click on the cog
    /// switches the picture, a click anywhere else on a card boots it.
    pub(super) fn pointer_cards(&mut self, pointer: &Pointer) -> Option<String> {
        if pointer.scroll != 0 {
            self.move_focus(pointer.scroll.signum() as isize);
        }
        let list = self.card_list();
        let rects = layout(list.len(), self.note_lines().len());
        let at = pointer.at?;
        if pointer.clicked
            && let Some(card) = rects.iter().enumerate().find_map(|(i, r)| {
                (list[i].variants.len() > 1 && inside(at, cog_hit(*r))).then_some(i)
            })
        {
            self.focus_card(card);
            self.cycle(card);
            return None;
        }
        let card = rects
            .iter()
            .position(|r| inside(at, *r))
            .filter(|&i| self.selectable_card(&list[i]))?;
        if pointer.moved || pointer.clicked {
            self.focus_card(card);
        }
        if pointer.clicked {
            return self.confirm_card();
        }
        None
    }

    /// The card chooser's draw list.
    pub(super) fn draw_cards(&self, sheet: &Sheet) -> Vec<Draw> {
        let Some(cards) = &self.cards else {
            return Vec::new();
        };
        let list = self.card_list();
        let notes = self.note_lines();
        let rects = layout(list.len(), notes.len());
        let mut out = vec![Draw::Fill {
            rect: [0.0, 0.0, SCREEN.0, SCREEN.1],
            color: super::BACKDROP,
        }];
        out.push(super::text(
            super::MARGIN,
            12.0,
            1.0,
            super::HEADING,
            "OPENANTIGRAV",
        ));
        out.push(super::text(
            super::MARGIN,
            24.0,
            1.0,
            super::DIM,
            &cards.words.choose,
        ));
        out.push(Draw::Text {
            x: SCREEN.0 - super::MARGIN,
            y: 12.0,
            scale: 1.0,
            color: super::DIM,
            border: None,
            align: Align::Right,
            text: super::build_label(),
            wrap_width: None,
        });
        for (index, (card, rect)) in list.iter().zip(&rects).enumerate() {
            self.draw_card(&mut out, sheet, card, *rect, index == cards.focus);
        }
        for (index, note) in notes.iter().enumerate() {
            out.push(super::text(
                super::MARGIN,
                NOTES_BOTTOM - (notes.len() - index) as f32 * NOTE,
                1.0,
                super::UNAVAILABLE_COLOUR,
                note,
            ));
        }
        out.push(super::text(
            super::MARGIN,
            SCREEN.1 - 20.0,
            1.0,
            super::DIM,
            &oag_ui::hints::line_text(&self.card_hints(cards)),
        ));
        out
    }

    fn draw_card(
        &self,
        out: &mut Vec<Draw>,
        sheet: &Sheet,
        card: &Card,
        rect: [f32; 4],
        focused: bool,
    ) {
        let selectable = self.selectable_card(card);
        let variant = &card.variants[self.shown_variant(card)];
        let row = &self.rows[variant.row];
        let [x, y, w, h] = rect;
        if focused && selectable {
            out.push(Draw::Fill {
                rect: [x - 2.0, y - 2.0, w + 4.0, h + 4.0],
                color: FRAME,
            });
        }
        out.push(Draw::Fill {
            rect,
            color: CARD_BACK,
        });
        let tint = if focused && selectable {
            [1.0, 1.0, 1.0, 1.0]
        } else {
            [0.76, 0.76, 0.8, 1.0]
        };
        let picture = variant
            .picture
            .as_ref()
            .and_then(|name| Some((name, sheet.get(&picture_key(&row.source, name))?)));
        match picture {
            Some((name, placed)) => {
                out.push(Draw::Sprite {
                    rect: contain(rect, placed.width as f32 / placed.height as f32),
                    uv: [
                        placed.x as f32,
                        placed.y as f32,
                        placed.width as f32,
                        placed.height as f32,
                    ],
                    color: tint,
                });
                if let Some(logo) = sheet.get(&logo_key(&row.source, name)) {
                    let width = w * LOGO_WIDTH;
                    let height = width * logo.height as f32 / logo.width as f32;
                    out.push(Draw::Sprite {
                        rect: [x + w * LOGO_LEFT, y + (h - height) / 2.0, width, height],
                        uv: [
                            logo.x as f32,
                            logo.y as f32,
                            logo.width as f32,
                            logo.height as f32,
                        ],
                        color: tint,
                    });
                }
            }
            None => {
                let colour = if !selectable {
                    super::UNAVAILABLE_COLOUR
                } else if focused {
                    super::SELECTED
                } else {
                    super::TEXT
                };
                out.push(Draw::Text {
                    x: x + w / 2.0,
                    y: y + h / 2.0 - 8.0,
                    scale: 1.0,
                    color: colour,
                    border: None,
                    align: Align::Centre,
                    text: row.title().to_string(),
                    wrap_width: Some(w - 8.0),
                });
            }
        }
        if card.variants.len() > 1 {
            self.draw_cog(out, sheet, rect, focused);
        }
        let colour = if focused && selectable {
            super::SELECTED
        } else {
            super::DIM
        };
        out.push(super::text(x, y + h + 3.0, 1.0, colour, &caption(row)));
        if card.variants.len() > 1 {
            out.push(Draw::Text {
                x: x + w,
                y: y + h + 3.0,
                scale: 1.0,
                color: colour,
                border: None,
                align: Align::Right,
                text: format!("{}/{}", self.shown_variant(card) + 1, card.variants.len()),
                wrap_width: None,
            });
        }
    }

    fn draw_cog(&self, out: &mut Vec<Draw>, sheet: &Sheet, rect: [f32; 4], focused: bool) {
        let [x, y, w, _] = rect;
        let cog = [x + w - COG - 3.0, y + 3.0, COG, COG];
        out.push(Draw::Fill {
            rect: [cog[0] - 1.0, cog[1] - 1.0, cog[2] + 2.0, cog[3] + 2.0],
            color: COG_BACK,
        });
        if let Some(placed) = sheet.get(COG_KEY) {
            out.push(Draw::Sprite {
                rect: cog,
                uv: [
                    placed.x as f32,
                    placed.y as f32,
                    placed.width as f32,
                    placed.height as f32,
                ],
                color: if focused {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.75, 0.78, 0.82, 1.0]
                },
            });
        }
    }

    /// The footer: what moves, boots, switches and quits. The switch entry is
    /// there only when the focused card has somewhere to switch to.
    fn card_hints(&self, cards: &Cards) -> Vec<Entry> {
        let mut entries = vec![
            Entry::new(None, &cards.words.r#move),
            Entry::button(Button::Cross, &cards.words.start).after("   "),
        ];
        let list = self.card_list();
        if list.get(cards.focus).is_some_and(|c| c.variants.len() > 1) {
            entries.push(Entry::button(Button::Square, &cards.words.picture).after("   "));
        }
        entries.push(Entry::new(Some(Press::Escape), &cards.words.quit).after("   "));
        entries
    }

    /// [`super::hints`], in card mode.
    pub(super) fn card_hint_entries(&self) -> Option<Vec<Entry>> {
        self.cards.as_ref().map(|cards| self.card_hints(cards))
    }
}

/// The row's own identity under the picture: the serial, which says which
/// pressing is shown, or the file name where the source carries none.
fn caption(row: &Candidate) -> String {
    match &row.serial {
        Some(_) => row.provenance(),
        None => row.name.clone(),
    }
}

/// Why each row that will not open will not, for the line under the cards.
fn notes(rows: &[Candidate], encrypted: &str) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match &row.state {
            State::Unavailable(why) => Some(format!("{}: {why}", row.name)),
            State::NeedsKey if !has_playable_copy(rows, row) => {
                Some(format!("{}: {encrypted}", row.name))
            }
            State::NeedsKey | State::Playable(_) => None,
        })
        .take(MAX_NOTES)
        .collect()
}

/// Whether a row with the same serial already plays.
fn has_playable_copy(rows: &[Candidate], locked: &Candidate) -> bool {
    locked.serial.is_some()
        && rows
            .iter()
            .any(|row| row.is_playable() && row.serial == locked.serial)
}

/// The cards `rows` make: one per title in the order the titles first appear,
/// then one for each row that does not open as a title.
#[must_use]
pub fn build(rows: &[Candidate], pictures: &BTreeMap<String, Vec<String>>) -> Vec<Card> {
    let count = |row: usize| pictures.get(&rows[row].source).map_or(0, Vec::len);
    let mut titles: Vec<(String, Vec<usize>)> = Vec::new();
    let mut closed: Vec<usize> = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        match row.playable() {
            Some(title) => {
                let slug = title.slug().to_string();
                match titles.iter_mut().find(|(key, _)| *key == slug) {
                    Some((_, members)) => {
                        // The same pressing twice (an encrypted disc beside its
                        // decrypted copy) is one variant: the one with art.
                        let twin = row.serial.is_some().then(|| {
                            members.iter().position(|&m| {
                                rows[m].serial == row.serial && rows[m].platform == row.platform
                            })
                        });
                        match twin.flatten() {
                            Some(at) if count(members[at]) == 0 && count(index) > 0 => {
                                members[at] = index;
                            }
                            Some(_) => {}
                            None => members.push(index),
                        }
                    }
                    None => titles.push((slug, vec![index])),
                }
            }
            None if matches!(row.state, State::NeedsKey) && has_playable_copy(rows, row) => {}
            None => closed.push(index),
        }
    }
    let variants_of = |row: usize| -> Vec<Variant> {
        match pictures.get(&rows[row].source) {
            Some(names) if !names.is_empty() => names
                .iter()
                .map(|name| Variant {
                    row,
                    picture: Some(name.clone()),
                })
                .collect(),
            _ => vec![Variant { row, picture: None }],
        }
    };
    titles
        .into_iter()
        .map(|(key, members)| Card {
            key,
            variants: members.into_iter().flat_map(variants_of).collect(),
        })
        .chain(closed.into_iter().map(|row| Card {
            key: format!("row:{}", rows[row].name),
            variants: vec![Variant { row, picture: None }],
        }))
        .collect()
}

/// What a variant is remembered as: the pressing's serial (or the file's name)
/// and the picture.
fn variant_key(rows: &[Candidate], variant: &Variant) -> String {
    let row = &rows[variant.row];
    format!(
        "{}:{}",
        row.serial.as_deref().unwrap_or(&row.name),
        variant.picture.as_deref().unwrap_or("none")
    )
}

/// The variant of `card` that `remembered` names. Before the pictures are read a
/// remembered one still finds its pressing, so confirming early boots it. With
/// nothing remembered, the first with a picture.
fn shown_index(rows: &[Candidate], card: &Card, remembered: Option<&String>) -> usize {
    if let Some(wanted) = remembered {
        if let Some(at) = card
            .variants
            .iter()
            .position(|v| variant_key(rows, v) == *wanted)
        {
            return at;
        }
        let pressing = wanted.rsplit_once(':').map_or(wanted.as_str(), |(p, _)| p);
        if let Some(at) = card.variants.iter().position(|v| {
            let row = &rows[v.row];
            row.serial.as_deref().unwrap_or(&row.name) == pressing
        }) {
            return at;
        }
    }
    card.variants
        .iter()
        .position(|v| v.picture.is_some())
        .unwrap_or(0)
}

/// Where each of `count` cards goes: the largest 16:9 size that fits, rows
/// centred. `notes` lines are kept free below.
#[must_use]
pub fn layout(count: usize, notes: usize) -> Vec<[f32; 4]> {
    if count == 0 {
        return Vec::new();
    }
    let area_h = AREA_BOTTOM - AREA_TOP - notes as f32 * NOTE;
    let area_w = SCREEN.0 - AREA_SIDE * 2.0;
    let (mut best_w, mut best_cols) = (0.0f32, 1usize);
    for cols in 1..=count {
        let rows = count.div_ceil(cols);
        let across = (area_w - (cols - 1) as f32 * GAP) / cols as f32;
        let down = ((area_h - (rows - 1) as f32 * GAP) / rows as f32 - CAPTION) * ASPECT;
        let width = across.min(down);
        if width > best_w {
            (best_w, best_cols) = (width, cols);
        }
    }
    let (w, h) = (best_w.floor(), (best_w.floor() / ASPECT).floor());
    let rows = count.div_ceil(best_cols);
    let block = rows as f32 * (h + CAPTION) + (rows - 1) as f32 * GAP;
    let top = AREA_TOP + ((area_h - block) / 2.0).max(0.0).floor();
    (0..count)
        .map(|index| {
            let row = index / best_cols;
            let in_row = (count - row * best_cols).min(best_cols);
            let line = in_row as f32 * w + (in_row - 1) as f32 * GAP;
            let left = ((SCREEN.0 - line) / 2.0).floor();
            [
                left + (index % best_cols) as f32 * (w + GAP),
                top + row as f32 * (h + CAPTION + GAP),
                w,
                h,
            ]
        })
        .collect()
}

/// The area a click or tap on the cog covers: larger than the glyph, so a finger
/// finds it.
#[must_use]
pub fn cog_hit(card: [f32; 4]) -> [f32; 4] {
    [card[0] + card[2] - COG_HIT, card[1], COG_HIT, COG_HIT]
}

fn inside(at: (f32, f32), rect: [f32; 4]) -> bool {
    at.0 >= rect[0] && at.0 < rect[0] + rect[2] && at.1 >= rect[1] && at.1 < rect[1] + rect[3]
}

/// `aspect` fitted inside `rect`, centred.
fn contain(rect: [f32; 4], aspect: f32) -> [f32; 4] {
    let [x, y, w, h] = rect;
    if aspect > w / h {
        let height = w / aspect;
        [x, y + (h - height) / 2.0, w, height]
    } else {
        let width = h * aspect;
        [x + (w - width) / 2.0, y, width, h]
    }
}

const AREA_TOP: f32 = 40.0;
const AREA_BOTTOM: f32 = 244.0;
const AREA_SIDE: f32 = 12.0;
const NOTES_BOTTOM: f32 = AREA_BOTTOM;
const NOTE: f32 = 11.0;
const MAX_NOTES: usize = 2;
const GAP: f32 = 8.0;
const CAPTION: f32 = 12.0;
const ASPECT: f32 = 16.0 / 9.0;
const COG: f32 = 13.0;
const COG_HIT: f32 = 24.0;
/// The logo's left edge and width as fractions of the card: over the side of
/// HD's picture the swirl leaves clear.
const LOGO_LEFT: f32 = 0.05;
const LOGO_WIDTH: f32 = 0.5;

const CARD_BACK: [f32; 4] = [0.08, 0.10, 0.16, 1.0];
const FRAME: [f32; 4] = [0.36, 0.66, 1.0, 1.0];
const COG_BACK: [f32; 4] = [0.0, 0.0, 0.0, 0.6];

#[cfg(test)]
mod tests;
