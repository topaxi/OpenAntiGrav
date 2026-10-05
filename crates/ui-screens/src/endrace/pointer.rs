//! `EndRace Menu` answering a mouse or a finger - the same vocabulary
//! [`crate::campaign::pointer`] and [`crate::picker::pointer`] both use.
//!
//! **`EndRace Results`/`EndRace Rewards` need no target list of their own.**
//! Neither screen authors anything to select - a click anywhere is the same
//! `ContinueButton` press cross/start already is, which
//! [`super::EndRaceMenu`]'s own caller (the composition root) reads directly
//! off [`Pointer::clicked`] rather than a hit-tested rect, the same way a
//! results table with no rows of its own would.

use oag_ui::pointer::{Pointer, contains};

use super::{EndRaceMenu, Event, Layout, draw};

/// One `Endrace Options` row's own rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub index: usize,
    pub rect: [f32; 4],
}

/// `EndRace Menu`'s own row targets - one per option, stepping down from the
/// `Endrace Options` widget's own `x`/`y` by a line of its font. The row's
/// own width is not authored (the widget lists strings of any length), so
/// it runs from the widget's `x` to the screen's own right edge - **chosen,
/// not measured**, the same gap `crate::picker::pointer::targets` names for
/// the identical shape of widget.
#[must_use]
pub fn menu_targets(model: &EndRaceMenu, layout: &Layout) -> Vec<Target> {
    let Some(menu) = layout.screen.menu.as_ref() else {
        return Vec::new();
    };
    let pitch = draw::font_line_height(&menu.font) * draw::face_scale(layout, &menu.font);
    let screen_width = 480.0 * layout.scale[0];
    let width = (screen_width - menu.x).max(pitch);
    (0..model.options().len())
        .map(|index| Target {
            index,
            rect: [menu.x, menu.y + index as f32 * pitch, width, pitch],
        })
        .collect()
}

/// The first target under `at`.
#[must_use]
pub fn hit(targets: &[Target], at: (f32, f32)) -> Option<Target> {
    targets
        .iter()
        .copied()
        .find(|target| contains(target.rect, at))
}

impl EndRaceMenu {
    /// Consumes a tick of pointer input against `targets` and says what
    /// happened, on the same [`Event`] terms as [`Self::update`] - the
    /// identical two-tap idiom
    /// [`crate::campaign::GridSelection::pointer`] uses: a hover selects, a
    /// second click on the same row confirms.
    pub fn pointer(&mut self, pointer: &Pointer, targets: &[Target]) -> Vec<Event> {
        let mut out = Vec::new();
        if pointer.is_idle() {
            return out;
        }
        let hit = pointer.at.and_then(|at| hit(targets, at));
        let was = self.index();
        if pointer.moved
            && let Some(Target { index, .. }) = hit
        {
            out.extend(self.select(index));
        }
        if pointer.clicked
            && let Some(Target { index, .. }) = hit
        {
            if index == was {
                out.push(Event::Confirmed);
            } else {
                out.extend(self.select(index));
            }
        }
        if pointer.back {
            out.push(Event::Back);
        }
        out
    }
}
