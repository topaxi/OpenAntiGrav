//! A click or tap on a key or button hint presses that button.
//!
//! The footers of the front end draw `(O) Help`, `(X) Confirm`, `(Δ) Back`: a glyph and its word.
//! The renderer reads those out of the draw list it was handed (`oag_ui::hints`, filled in by
//! `Renderer::hints`), so the target is the layout that was drawn and the button is the glyph as
//! drawn, after the MENU BACK exchange (`docs/ui/back-and-help.md`, "Footers"). The click is
//! spent as that button's press through the keyboard's latch (`Controls::tap`), so the screen
//! reads it the way it reads the pad, one tick later: Back leaves, Help opens Help, Confirm
//! confirms, and a hint whose button does nothing on the screen does nothing on click either.
//!
//! The click is taken here, ahead of the stage's own pointer handling, so a hint over a row or a
//! panel is the hint and the press is not also a click on what is under it.

use oag_display::space::Space;
use oag_ui::hints::{Hint, Press};
use oag_ui::pointer::Pointer;

use super::pointer::in_grid;
use crate::app::race_stage::endrace_touch::EndRace;
use crate::app::stage::Stage;
use crate::render::{from_grid, inset::inset_point};
use oag_display::display;
use oag_gameplay::input::Button;

impl super::Session {
    /// The hints the stage on screen drew last frame, the grid they are in and the inset it draws
    /// about (`1.0` for a stage drawn as authored).
    fn hints_on_screen(&self) -> Option<(&[Hint], Space, f32)> {
        match &self.stage {
            Stage::Frontend(stage) => Some((
                stage.renderer.hints(),
                stage.frontend.space(),
                stage.renderer.inset(),
            )),
            Stage::Menu(stage) => Some((
                stage.renderer.hints(),
                stage.skin.space(),
                stage.renderer.inset(),
            )),
            Stage::Race(stage) => match stage.endrace.as_ref()? {
                EndRace::Disc(runtime) => Some((runtime.hints(), runtime.space(), 1.0)),
                EndRace::Touch(_) => None,
            },
            Stage::Launcher(_) | Stage::Loading(_) => None,
        }
    }

    /// Spends a click on a hint as a press of its button, and the click with it.
    pub(super) fn tick_hints(&mut self, pointer: &mut Pointer, rect: (f32, f32, f32, f32)) {
        if !pointer.clicked {
            return;
        }
        let Some((hints, space, inset)) = self.hints_on_screen() else {
            return;
        };
        if hints.is_empty() {
            return;
        }
        let Some(at) = in_grid(*pointer, space, rect, inset).at else {
            return;
        };
        let Some(press) = oag_ui::hints::hit(hints, at) else {
            return;
        };
        pointer.clicked = false;
        match press {
            Press::Button(button) => self.controls.tap(button),
            Press::Escape => self.escape(),
        }
    }

    /// The buttons the hints on screen depict, each once, in the order they are drawn.
    pub(crate) fn hint_buttons(&self) -> Vec<Button> {
        let mut out: Vec<Button> = Vec::new();
        for hint in self.hints_on_screen().map_or(&[][..], |(hints, ..)| hints) {
            if let Press::Button(button) = hint.press
                && !out.contains(&button)
            {
                out.push(button);
            }
        }
        out
    }

    /// The window pixel at the middle of the hint that depicts `button`: where a click presses it.
    pub(crate) fn hint_pixel(&self, button: Button) -> Option<(f32, f32)> {
        let (hints, space, inset) = self.hints_on_screen()?;
        let hint = hints
            .iter()
            .find(|hint| hint.press == Press::Button(button))?;
        let [x, y, width, height] = hint.rect;
        let middle = inset_point((x + width / 2.0, y + height / 2.0), inset, space);
        let rect = display::viewport(self.gpu.output.size(), self.aspect());
        Some(from_grid(space, rect, middle))
    }
}
