//! Pulse's own text-entry idiom, played rather than reinvented: a row of
//! `length` character cells the player scrolls one glyph at a time, with a
//! confirm item bracketed by two gradient bars - see `docs/formats/fexml.md`'s
//! `TagInput` section.
//!
//! Two halves, like [`crate::prompt`]:
//!
//! - [`Geometry`] and [`geometry`] read the disc's own widget positions -
//!   the cell backgrounds, the confirm label and its bars - directly out of
//!   [`oag_tables::fexml::parse`]'s tree, because the screen that carries
//!   them (`Data.wad`'s `TAG_INPUT_SCREENS` entry) is an **anonymous**
//!   `Screen`, which `oag_ui::screen::Screens::collect` deliberately walks
//!   through without registering. That rule is shared code another thread
//!   is working near, so this reads the raw tree itself rather than asking
//!   for it to change - see [`geometry`]'s own doc.
//! - [`TagEntry`] is the interactive model: a buffer of cells, a cursor
//!   that can also land on the confirm slot, and the same `Input`/`Pointer`
//!   -in, `Outcome`/`Draw`-out shape [`crate::prompt::Keyboard`] has.
//!
//! # The alphabet is a parameter, not a constant here
//!
//! Pulse and Pure each author their own `<TagInput>` with a different
//! alphabet (Pure's is 43 characters, uppercase only). Baking either into
//! this crate would make it a title's own module rather than the format's,
//! so [`TagEntry::new`] takes the alphabet as `&str` - `oag_pulse::tag_input::ALPHABET`
//! is Pulse's own, filtered through `oag_game::prompt::accepts` wherever the
//! caller's own data (a pilot name) has narrower rules than the disc's.

use std::collections::HashMap;

use oag_gameplay::input::{Button, Input};
use oag_tables::fexml;

use crate::prompt::Outcome;
use oag_ui::frontend::{Align, Draw};
use oag_ui::screen::{self, Fill, Node, Screens, Text, argb_to_rgba};

/// The disc's own widget positions for one `<TagInput>` screen: the cell
/// backgrounds, the title label, the confirm label and its two gradient
/// bars.
///
/// **Not the alphabet, and not a buffer.** This is only what
/// [`TagEntry::draw`] paints things *at* - see this module's own doc for why
/// the interactive state and the alphabet are kept apart from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    /// The `<TagInput>` widget's own attributes: `x`, `y`, `length`, `color`,
    /// `scale`.
    pub tag_input: screen::TagInput,
    /// The cell background tiles, in document order - `Item`'s own
    /// `BGgradient` children, `OffsetX`/`OffsetY` already folded in.
    pub cells: Vec<Fill>,
    /// The screen's own title label (`PRO_ENTER_NAME`/`PRO_ENTER_TAG`),
    /// when the screen carries one - its position and font, not its text:
    /// see this module's own doc on labels.
    pub title: Option<Text>,
    /// The `FE_CONFIRM` label's own position and font.
    pub confirm: Option<Text>,
    /// The two gradient bars either side of the confirm label.
    pub bars: Vec<Fill>,
}

/// Reads one `<TagInput name="{widget_name}">` screen's own geometry out of
/// `document` (already-expanded front-end XML), resolving `FEGlobals->`
/// through `globals` - the caller's own read of `Skin.xml`, since neither
/// `Name` nor `Tag`'s `TagInput` declares `TitleXOffset`/`TextColor` itself.
///
/// **Walks [`fexml::parse`]'s tree directly rather than going through
/// [`Screens::from_xml`].** The screen this is measured against -
/// `Data.wad`'s `TAG_INPUT_SCREENS` entry - holds its `Name`/first `Tag`
/// `TagInput` under an **anonymous** `<Screen type="FE_Default">`, and
/// `Screens::collect` walks through an anonymous screen without registering
/// it (see that function's own doc), so `Screens::screens` never carries
/// these widgets at all. Changing that rule to reach one more widget would
/// move every other screen's own attribution and is shared code; reading the
/// tree directly here costs one small walk instead. `crates/ui-screens/tests/tag_input_ground_truth.rs`
/// checks the one `TagInput` in this same entry that *is* under a named
/// screen (`Create Profile Setup`'s own `Tag`) against `Screens` itself, so
/// the parsing this leans on - [`Screens::fill_from_node`],
/// [`Screens::text_from_node`], [`Screens::tag_input_from_node`] - is proven
/// against the real disc even though this function's own entry point is not.
#[must_use]
pub fn geometry(
    document: &str,
    globals: &HashMap<String, String>,
    widget_name: &str,
) -> Option<Geometry> {
    let root = fexml::parse(document);
    let container = find_container(&root, widget_name)?;

    let scratch = Screens {
        globals: globals.clone(),
        ..Screens::default()
    };

    let tag_node = container.children.iter().find(|c| {
        c.name.eq_ignore_ascii_case("TagInput")
            && c.attr("name")
                .is_some_and(|n| n.eq_ignore_ascii_case(widget_name))
    })?;
    let tag_input = scratch.tag_input_from_node(tag_node);

    let mut cells = Vec::new();
    for item in container.children_named("Item") {
        let offset = (
            scratch.number(item.value("OffsetX")).unwrap_or(0.0),
            scratch.number(item.value("OffsetY")).unwrap_or(0.0),
        );
        for image in item.children_named("Image") {
            if let Some(fill) = scratch.fill_from_node(image, offset) {
                cells.push(fill);
            }
        }
    }

    // The title label sits in the first `LeftLayer`, the confirm bars in the
    // second - the same document order every measured instance of this
    // screen shares. A screen with only one `LeftLayer` (none measured so
    // far) yields a title and no bars, which is a gap [`TagEntry::draw`]
    // draws nothing for rather than guessing a position.
    let mut left_layers = container.children_named("LeftLayer");
    let title = left_layers
        .next()
        .and_then(|layer| layer.children_named("Text").next())
        .map(|node| scratch.text_from_node(node, None, (0.0, 0.0)));
    let bars: Vec<Fill> = left_layers
        .next()
        .map(|layer| {
            layer
                .children_named("Image")
                .filter_map(|image| scratch.fill_from_node(image, (0.0, 0.0)))
                .collect()
        })
        .unwrap_or_default();

    let confirm = container
        .children_named("Text")
        .find(|node| {
            node.attr("name")
                .is_some_and(|n| n.eq_ignore_ascii_case("confirm"))
        })
        .map(|node| scratch.text_from_node(node, None, (0.0, 0.0)));

    Some(Geometry {
        tag_input,
        cells,
        title,
        confirm,
        bars,
    })
}

/// The nearest node (possibly `root` itself) whose direct children include a
/// `<TagInput name="{widget_name}">` - the anonymous `Screen` in
/// [`geometry`]'s case, but not tied to that element name, since nothing
/// here needs it to be one.
fn find_container<'a>(node: &'a Node, widget_name: &str) -> Option<&'a Node> {
    let holds_it = node.children.iter().any(|c| {
        c.name.eq_ignore_ascii_case("TagInput")
            && c.attr("name")
                .is_some_and(|n| n.eq_ignore_ascii_case(widget_name))
    });
    if holds_it {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|c| find_container(c, widget_name))
}

/// Labels this module draws that are not part of [`Geometry`]: already
/// resolved by the caller, never looked up here - the same rule
/// `crate::prompt`'s own module doc states for [`crate::prompt::Labels`].
///
/// **Drawn at the disc's own authored positions, with this project's own
/// text.** [`Geometry::title`]/[`Geometry::confirm`] carry where and how the
/// disc draws `PRO_ENTER_NAME`/`FE_CONFIRM`; what they say is a different
/// question, and the pilot editor's RENAME is not the disc's "enter your
/// name" screen, so it supplies its own words for the disc's own slots.
#[derive(Debug, Clone, Default)]
pub struct Labels {
    /// Replaces the disc's own title label, drawn at [`Geometry::title`]'s
    /// position - "RENAME PILOT", not "ENTER YOUR NAME".
    pub title: String,
    /// Replaces `FE_CONFIRM`, drawn at [`Geometry::confirm`]'s position.
    pub confirm: String,
    /// One line under the row saying which button does what. Positioned the
    /// same way [`crate::prompt::Keyboard`]'s own hint is - chosen, not
    /// measured, since no disc screen pairs this idiom with a hint line.
    pub hint: String,
}

/// An interactive `<TagInput>`: [`Geometry`]'s positions, an alphabet, and a
/// buffer of cells the player scrolls through - one glyph, or blank, per
/// cell.
///
/// The cursor's range is `0..=cells.len()`: every index below the last is a
/// real cell, and `cells.len()` itself is the confirm slot to its right -
/// modelling the disc's own "row of cells, then `FE_CONFIRM`" layout as one
/// strip rather than a grid with a separate accept key, the way
/// [`crate::prompt::Keyboard`] has one.
#[derive(Debug, Clone)]
pub struct TagEntry {
    labels: Labels,
    geometry: Geometry,
    alphabet: Vec<char>,
    cells: Vec<Option<char>>,
    cursor: usize,
    note: Option<String>,
}

impl TagEntry {
    /// A `TagEntry` opened on `initial`, cycling through `alphabet`.
    ///
    /// **`initial` is filtered against `alphabet`, silently**: a character
    /// the disc's own widget cannot display becomes a blank cell rather than
    /// a panic or a truncated buffer - the caller decides *whether* to open
    /// this at all (every glyph of the current text must be in the filtered
    /// alphabet, or `crate::prompt::Keyboard` is used instead - see
    /// `docs/formats/fexml.md`), not this constructor.
    #[must_use]
    pub fn new(labels: Labels, geometry: Geometry, alphabet: &str, initial: &str) -> Self {
        let alphabet: Vec<char> = alphabet.chars().collect();
        let length = geometry.tag_input.length as usize;
        let mut cells = vec![None; length];
        for (cell, c) in cells.iter_mut().zip(initial.chars()) {
            if alphabet.contains(&c) {
                *cell = Some(c);
            }
        }
        Self {
            labels,
            geometry,
            alphabet,
            cells,
            cursor: 0,
            note: None,
        }
    }

    /// What the cells currently spell - every blank cell skipped, wherever
    /// it sits. A mid-string blank is not expected on the one alphabet this
    /// has been built against (space is itself a glyph in it), but this does
    /// not assume that: chosen, not measured.
    #[must_use]
    pub fn text(&self) -> String {
        self.cells.iter().flatten().collect()
    }

    /// Sets, or clears, the live remark drawn under the row. See
    /// [`crate::prompt::Keyboard::set_note`].
    pub fn set_note(&mut self, note: Option<String>) {
        self.note = note;
    }

    /// Whether the cursor is on a real cell (`Some`) or the confirm slot
    /// (`None`).
    #[must_use]
    pub fn selected_cell(&self) -> Option<usize> {
        (self.cursor < self.cells.len()).then_some(self.cursor)
    }

    fn cycle(&mut self, forward: bool) {
        let Some(index) = self.selected_cell() else {
            return;
        };
        let positions = self.alphabet.len() + 1; // +1 for blank
        let at = self.cells[index].map_or(self.alphabet.len(), |c| {
            self.alphabet
                .iter()
                .position(|&a| a == c)
                .unwrap_or(self.alphabet.len())
        });
        let next = if forward {
            (at + 1) % positions
        } else {
            (at + positions - 1) % positions
        };
        self.cells[index] = self.alphabet.get(next).copied();
    }

    /// One edit from a desk keyboard - see `oag_game::typing`. A character
    /// outside the alphabet is ignored rather than accepted and immediately
    /// undrawable.
    ///
    /// **Chosen, not measured**: typing a character sets it on the current
    /// cell and advances the cursor by one, all the way onto the confirm
    /// slot after the last cell - nothing on the disc pairs this idiom with
    /// a desk keyboard to read the mapping off. Delete steps back one cell
    /// (from the confirm slot, onto the last) and clears it, mirroring a
    /// backspace rather than the disc's own up/down scroll.
    pub fn edit(&mut self, edit: crate::prompt::Edit) {
        match edit {
            crate::prompt::Edit::Type(c) => {
                if let Some(index) = self.selected_cell()
                    && self.alphabet.contains(&c)
                {
                    self.cells[index] = Some(c);
                    self.cursor = (self.cursor + 1).min(self.cells.len());
                }
            }
            crate::prompt::Edit::Delete => {
                if self.cursor >= self.cells.len() {
                    self.cursor = self.cells.len().saturating_sub(1);
                }
                if let Some(index) = self.selected_cell() {
                    self.cells[index] = None;
                }
            }
        }
    }

    /// Consumes a tick of input. See [`crate::prompt::Keyboard::update`] on
    /// why every button is taken as an edge.
    ///
    /// **Not measured** - see this module's own doc and
    /// `docs/formats/fexml.md`'s `TagInput` section: up/down cycles the
    /// glyph under the cursor, left/right moves along the row and onto the
    /// confirm slot, the reading the cell strip and the executable's own
    /// sign-extending selector arithmetic both support but that nothing has
    /// captured yet.
    pub fn update(&mut self, input: &mut Input) -> Outcome {
        let length = self.cells.len();
        if input.take(Button::Right) {
            self.cursor = (self.cursor + 1).min(length);
        }
        if input.take(Button::Left) {
            self.cursor = self.cursor.saturating_sub(1);
        }
        if input.take(Button::Up) {
            self.cycle(true);
        }
        if input.take(Button::Down) {
            self.cycle(false);
        }
        if input.take(Button::Square) {
            self.edit(crate::prompt::Edit::Delete);
        }
        if input.take(Button::Circle) {
            return Outcome::Cancelled;
        }
        if input.take(Button::Start) {
            return Outcome::Accepted;
        }
        if input.take(Button::Cross) && self.selected_cell().is_none() {
            return Outcome::Accepted;
        }
        Outcome::Pending
    }

    /// The confirm slot's own hit target: a box around
    /// [`Geometry::confirm`]'s own position. **Chosen, not measured** - the
    /// disc authors no rect for it, only a label position and two bars either
    /// side of it.
    fn confirm_rect(&self) -> Option<[f32; 4]> {
        let confirm = self.geometry.confirm.as_ref()?;
        Some([confirm.x - 40.0, confirm.y - 10.0, 80.0, 20.0])
    }

    fn cell_at(&self, at: (f32, f32)) -> Option<usize> {
        self.geometry
            .cells
            .iter()
            .position(|fill| oag_ui::pointer::contains(cell_rect(fill), at))
    }

    /// Consumes a tick of pointer input, on [`Outcome`] terms.
    ///
    /// **Ours.** A click on a cell selects it without changing its glyph - a
    /// mouse has a wheel, and scrolling over a cell cycles it the way up/down
    /// does on a pad. A click on the confirm slot accepts; the secondary
    /// button cancels. See `docs/architecture/menus.md`.
    pub fn pointer(&mut self, pointer: &oag_ui::pointer::Pointer) -> Outcome {
        if pointer.is_idle() {
            return Outcome::Pending;
        }
        if pointer.back {
            return Outcome::Cancelled;
        }
        let Some(at) = pointer.at else {
            return Outcome::Pending;
        };
        if let Some(index) = self.cell_at(at) {
            if pointer.moved || pointer.clicked {
                self.cursor = index;
            }
            if pointer.scroll != 0 {
                self.cursor = index;
                self.cycle(pointer.scroll > 0);
            }
            return Outcome::Pending;
        }
        if self
            .confirm_rect()
            .is_some_and(|rect| oag_ui::pointer::contains(rect, at))
        {
            if pointer.moved {
                self.cursor = self.cells.len();
            }
            if pointer.clicked {
                return Outcome::Accepted;
            }
        }
        Outcome::Pending
    }

    /// What this looks like, over whatever is already on screen - every
    /// position the disc's own [`Geometry`], every string this project's own
    /// [`Labels`].
    ///
    /// `skin` sizes every piece of text - see [`Self::glyph_scale`]'s own
    /// doc for why the disc's own authored `scale` numbers are not used for
    /// that - and its screen size draws the scrim and panel behind the row,
    /// the same reason [`crate::prompt::Keyboard::draw`] takes one: this
    /// screen sits over live menu rows (`AI PILOTS`, in this project's own
    /// use), never over the disc's own solid backdrop the way the real
    /// screen does, so without one every row behind it bleeds straight
    /// through the cell strip.
    #[must_use]
    pub fn draw(&self, skin: &oag_ui::menu::Skin) -> Vec<Draw> {
        let mut out = Vec::new();
        let (screen_w, screen_h) = skin.space().size;
        out.push(Draw::Fill {
            rect: [-4000.0, -4000.0, 8000.0, 8000.0],
            color: SCRIM,
        });
        // Same proportions as `crate::prompt::Panel`, which this duplicates
        // rather than shares (see [`fill_draw`]'s own note): tall enough to
        // cover both the authored title (near `y=0`) and the hint line this
        // project adds near the bottom, so neither draws over the live page
        // behind it.
        let panel_top = screen_h * 0.055;
        out.push(Draw::Fill {
            rect: [screen_w * 0.08, panel_top, screen_w * 0.84, screen_h * 0.90],
            color: PANEL,
        });

        let glyph_scale = self.glyph_scale(skin);

        for (index, fill) in self.geometry.cells.iter().enumerate() {
            if self.selected_cell() == Some(index) {
                out.push(Draw::Fill {
                    rect: cell_rect(fill),
                    color: CURSOR,
                });
            } else {
                out.push(fill_draw(fill));
            }
            let glyph = self
                .cells
                .get(index)
                .and_then(|c| *c)
                .map(String::from)
                .unwrap_or_default();
            if !glyph.is_empty() {
                out.push(Draw::Text {
                    x: fill.x + fill.width.unwrap_or(0.0) * 0.5,
                    y: self.geometry.tag_input.y,
                    scale: glyph_scale,
                    color: argb_to_rgba(self.geometry.tag_input.color),
                    border: None,
                    align: Align::Centre,
                    text: glyph,
                    wrap_width: None,
                });
            }
        }

        for bar in &self.geometry.bars {
            out.push(fill_draw(bar));
        }

        if let Some(title) = &self.geometry.title {
            out.push(Draw::Text {
                x: title.x,
                // The authored `y` (`FEGlobals->TitleYOffset`, `0` on
                // Pulse) sits above this panel's own top edge, which the
                // disc's real screen has no panel to clash with - nudged
                // down onto the panel instead of drawn over the live page's
                // own header behind it. Chosen, not measured.
                y: title.y.max(panel_top + skin.line_height() * 0.3),
                scale: skin.row_scale(),
                color: argb_to_rgba(title.color),
                border: None,
                align: Align::parse(&title.align),
                text: self.labels.title.clone(),
                wrap_width: None,
            });
        }

        if let Some(confirm) = &self.geometry.confirm {
            if self.selected_cell().is_none()
                && let Some(rect) = self.confirm_rect()
            {
                out.push(Draw::Fill {
                    rect,
                    color: CURSOR,
                });
            }
            out.push(Draw::Text {
                x: confirm.x,
                y: confirm.y,
                scale: skin.row_scale() * 0.8,
                color: argb_to_rgba(confirm.color),
                border: None,
                align: Align::parse(&confirm.align),
                text: self.labels.confirm.clone(),
                wrap_width: None,
            });
        }

        if !self.labels.hint.is_empty() {
            out.push(Draw::Text {
                x: self.geometry.tag_input.x,
                // Chosen, not measured - near the bottom of the panel, the
                // same shape `crate::prompt::Keyboard`'s own hint takes
                // relative to its.
                y: screen_h * 0.70,
                scale: skin.row_scale() * 0.6,
                color: [1.0, 1.0, 1.0, 0.7],
                border: None,
                align: Align::Left,
                text: self.labels.hint.clone(),
                wrap_width: Some(screen_w * 0.84 - self.geometry.tag_input.x),
            });
        }

        if let Some(note) = &self.note {
            out.push(Draw::Text {
                x: self.geometry.tag_input.x,
                y: screen_h * 0.60,
                scale: skin.row_scale() * 0.55,
                color: NOTE,
                border: None,
                align: Align::Left,
                text: note.clone(),
                wrap_width: None,
            });
        }

        out
    }

    /// The scale [`Self::draw`] draws every cell's glyph at.
    ///
    /// **Not [`screen::TagInput::scale`].** That number is authored for the
    /// disc's own renderer, which this project's does not reproduce -
    /// `2.0` there draws a glyph that roughly fills a 25-unit cell against
    /// Pulse's own `Default` atlas; fed straight into this renderer's own
    /// `Draw::Text.scale` (calibrated the way `oag_ui::menu`'s rows already
    /// are, off `skin.row_scale()`) it comes out several times too big -
    /// found by looking at a capture, not by arithmetic. So this sizes off
    /// the skin instead, the same way `crate::prompt::Keyboard`'s own grid
    /// glyphs do, chosen to fit inside the authored cell rect rather than
    /// overflow it: **chosen, not measured**.
    fn glyph_scale(&self, skin: &oag_ui::menu::Skin) -> f32 {
        let Some(cell) = self.geometry.cells.first() else {
            return skin.row_scale();
        };
        let Some(height) = cell.height else {
            return skin.row_scale();
        };
        // A glyph a little shorter than the cell it sits in, in units of the
        // skin's own line height - the same ratio a row's own text keeps
        // against its row pitch.
        (height * 0.8 / skin.line_height().max(1.0)).clamp(0.3, 3.0)
    }
}

fn cell_rect(fill: &Fill) -> [f32; 4] {
    [
        fill.x,
        fill.y,
        fill.width.unwrap_or(0.0),
        fill.height.unwrap_or(0.0),
    ]
}

/// A gradient-carrying [`Fill`] draws as [`Draw::GradientFill`], a plain one
/// as [`Draw::Fill`] - the same conversion `crate::picker::fill_draw`,
/// `crate::endrace::draw` and `crate::campaign::draw` each already carry
/// their own copy of, for their own screen family.
fn fill_draw(fill: &Fill) -> Draw {
    fill.draw()
}

/// The box behind the selected cell or the confirm slot. Ours; see
/// `crate::prompt`'s own `CURSOR` for the twin this duplicates rather than
/// shares - each screen family here keeps its own copy of its handful of
/// invented colours, the same way `fill_draw` above does.
const CURSOR: [f32; 4] = [0.20, 0.42, 0.62, 0.85];

/// A live remark about what has been typed. See `crate::prompt`'s own `NOTE`.
const NOTE: [f32; 4] = [1.0, 0.76, 0.25, 1.0];

/// What everything outside the panel is dimmed with. See `crate::prompt`'s
/// own `SCRIM` - this module's reason for one is the same: the row is drawn
/// over live menu content (`AI PILOTS`, this project's own caller), and the
/// disc's own real screen has no menu behind it to hide.
const SCRIM: [f32; 4] = [0.0, 0.0, 0.0, 0.72];

/// The panel's own fill. Ours; see [`SCRIM`].
const PANEL: [f32; 4] = [0.04, 0.06, 0.09, 0.985];

#[cfg(test)]
mod tests;
