//! A click or tap on a key or button hint presses that button.
//!
//! The footers of the front end draw `(O) Help`, `(X) Confirm`, `(Δ) Back`: a glyph and its word.
//! The renderer reads those out of the draw list it was handed (`oag_ui::hints`, filled in by
//! `Renderer::hints`), so the target is the layout that was drawn and the button is the glyph as
//! drawn, after the MENU BACK exchange (`docs/ui/back-and-help.md`, "Footers"). The screens of
//! our own that spell their keys out (photo mode, the replay viewer, the disc chooser) list their
//! hints beside the lines they draw them from. The click is spent as that press through the
//! keyboard's latch (`Controls::tap`), so the screen reads it the way it reads the pad, one tick
//! later: Back leaves, Help opens Help, Confirm confirms, and a hint whose button does nothing on
//! the screen does nothing on click either.
//!
//! The click is taken here, ahead of the stage's own pointer handling, so a hint over a row or a
//! panel is the hint and the press is not also a click on what is under it.

use std::borrow::Cow;

use oag_display::display;
use oag_display::space::Space;
use oag_ui::hints::{Hint, Press};
use oag_ui::pointer::Pointer;

use super::pointer::in_grid;
use crate::app::race_stage::endrace_touch::EndRace;
use crate::app::stage::Stage;
use crate::render::{from_grid, inset::inset_point};

/// The hints on screen, with what a pointer needs to land on them.
struct OnScreen<'a> {
    hints: Cow<'a, [Hint]>,
    /// The grid the hints are in.
    space: Space,
    /// The aspect rectangle the picture is drawn into, in window pixels.
    rect: (f32, f32, f32, f32),
    /// The scale the stage draws its widgets at about the grid's centre.
    inset: f32,
}

impl super::Session {
    /// The hints the stage on screen drew last frame.
    fn hints_on_screen(&mut self) -> Option<OnScreen<'_>> {
        let surface = self.gpu.output.size();
        let picture = display::viewport(surface, self.aspect());
        let overlay = oag_present::perf::grid(surface);
        let whole = (0.0, 0.0, surface.0 as f32, surface.1 as f32);
        // Photo mode and the replay viewer draw over the race in the overlay grid, and take the
        // pointer in it whole.
        if self.photo.is_some() {
            return Some(OnScreen {
                hints: Cow::Owned(self.photo_hints(overlay.size)),
                space: overlay,
                rect: whole,
                inset: 1.0,
            });
        }
        if self.viewer.is_some() && matches!(self.stack.top(), Stage::Race(_)) {
            return Some(OnScreen {
                hints: Cow::Owned(self.viewer_hints(overlay.size)),
                space: overlay,
                rect: whole,
                inset: 1.0,
            });
        }
        let (hints, space, inset) = match self.stack.top() {
            Stage::Frontend(stage) => (
                Cow::Borrowed(stage.renderer.hints()),
                stage.frontend.space(),
                stage.renderer.inset(),
            ),
            // A prompt takes the whole input, so what is under its scrim is not a target: a click
            // on the page's footer would press that button into the keyboard.
            Stage::Menu(stage) => (
                match &stage.prompt {
                    Some(prompt) => {
                        Cow::Owned(prompt.hints(&stage.skin, &stage.text_atlas, self.roles().back))
                    }
                    None => Cow::Borrowed(stage.renderer.hints()),
                },
                stage.skin.space(),
                stage.renderer.inset(),
            ),
            Stage::Race(stage) => match stage.endrace.as_ref()? {
                EndRace::Disc(runtime) => (Cow::Borrowed(runtime.hints()), runtime.space(), 1.0),
                EndRace::Touch(_) => return None,
            },
            Stage::Launcher(stage) => (
                Cow::Owned(crate::launcher::hints(&stage.launcher)),
                oag_display::space::Space::PSP,
                1.0,
            ),
            Stage::Loading(_) => return None,
        };
        Some(OnScreen {
            hints,
            space,
            rect: picture,
            inset,
        })
    }

    /// Spends a click on a hint as that press, and the click with it.
    pub(super) fn tick_hints(&mut self, pointer: &mut Pointer) {
        if !pointer.clicked {
            return;
        }
        let press = {
            let Some(screen) = self.hints_on_screen() else {
                return;
            };
            let Some(at) = in_grid(*pointer, screen.space, screen.rect, screen.inset).at else {
                return;
            };
            oag_ui::hints::hit(&screen.hints, at)
        };
        let Some(press) = press else { return };
        pointer.clicked = false;
        match press {
            Press::Button(button) => self.controls.tap(button),
            Press::Escape => self.escape(),
        }
    }

    /// The presses the hints on screen make, each once, in the order they are drawn.
    pub(crate) fn hint_presses(&mut self) -> Vec<Press> {
        let Some(screen) = self.hints_on_screen() else {
            return Vec::new();
        };
        let mut out: Vec<Press> = Vec::new();
        for hint in screen.hints.iter() {
            if !out.contains(&hint.press) {
                out.push(hint.press);
            }
        }
        out
    }

    /// The window pixel at the middle of the hint that presses `press`: where a click makes it.
    pub(crate) fn hint_pixel(&mut self, press: Press) -> Option<(f32, f32)> {
        let screen = self.hints_on_screen()?;
        let hint = screen.hints.iter().find(|hint| hint.press == press)?;
        let [x, y, width, height] = hint.rect;
        let middle = inset_point(
            (x + width / 2.0, y + height / 2.0),
            screen.inset,
            screen.space,
        );
        Some(from_grid(screen.space, screen.rect, middle))
    }
}
