//! The player's own hands on `Team Selection`'s craft: the pad's right stick, zoom button and
//! left stick, and a pointer's drag and wheel over the craft's frame, as
//! [`crate::preview::ship_model::control`] reads them.

use oag_gameplay::input::Button;
use oag_input::Controls;
use oag_ui::pointer::{Pointer, contains};

use super::PickerStage;
use crate::preview::ship_model::control::Craft;

impl PickerStage {
    /// The law the player's control follows on this screen, where the title has one and the
    /// screen is a craft's.
    fn craft_law(&self) -> Option<oag_title::ShipControl> {
        if self.model.kind() != oag_ui_screens::picker::Kind::Ship {
            return None;
        }
        self.previews.front_end?.ship_control
    }

    /// Whether a modal has the screen: the Help overlay, or a screen that takes no input.
    fn craft_blocked(&self) -> bool {
        self.model.help_open() || self.model.is_locked()
    }

    /// Whether the player can turn, zoom and pan the craft here: what asks the window to give a
    /// held mouse button's travel to the screen.
    pub(crate) fn has_craft_control(&self) -> bool {
        self.craft_law().is_some()
    }

    /// Reads this tick's pad into the craft's input, and spends the presses a held zoom button
    /// makes of the left stick, so a pan does not also step the team. A split-screen seat reads
    /// its own device, which is not the pad read here.
    pub(crate) fn read_craft_pad(&mut self, controls: &mut Controls) {
        let held = self.craft_law().is_some() && self.seat.is_none() && !self.craft_blocked();
        if !held {
            return;
        }
        let view = controls.view();
        let zoom = f32::from(u8::from(controls.view_held(Button::R)));
        self.craft_input.right = view.right_raw;
        self.craft_input.left = view.left;
        self.craft_input.zoom = zoom;
        if zoom > 0.0
            || self
                .ship_player
                .as_ref()
                .is_some_and(|player| player.zoomed())
        {
            for button in [Button::Left, Button::Right, Button::Up, Button::Down] {
                controls.buttons_mut().consume_press(button);
            }
        }
    }

    /// Takes the part of `pointer` that is the craft's out of it: a wheel over the craft's frame
    /// zooms instead of stepping the list, and a drag that began on the frame turns (or, zoomed,
    /// pans) it instead of the grid beneath.
    pub(crate) fn take_craft_pointer(&mut self, pointer: &mut Pointer) {
        if self.craft_law().is_none() || self.craft_blocked() {
            return;
        }
        let frame = self.layout.preview;
        if pointer.pressed {
            self.craft_grab = pointer.press_at.is_some_and(|at| contains(frame, at));
        }
        if self.craft_grab && pointer.drag != (0.0, 0.0) {
            self.craft_input.drag = Some(pointer.drag);
            pointer.drag = (0.0, 0.0);
        }
        if pointer.released {
            self.craft_grab = false;
        }
        if pointer.scroll != 0 && pointer.at.is_some_and(|at| contains(frame, at)) {
            self.craft_input.wheel += pointer.scroll;
            pointer.scroll = 0;
        }
    }

    /// Advances the craft's state by the fixed ticks `dt` seconds make, one frame of the
    /// original's update each: the depth fit where the title has one, and the player's control.
    /// Runs while the craft is hidden and on every step, as the original's does. `aspect` is the
    /// screen's own.
    pub(crate) fn tick_craft(&mut self, aspect: f32, dt: f32) {
        let law = self.craft_law();
        let fit = self.ship_fit().map(|(fit, box_z)| (*fit, box_z));
        if fit.is_none() && law.is_none() {
            return;
        }
        let input = std::mem::take(&mut self.craft_input);
        self.ship_player
            .get_or_insert_with(|| {
                crate::preview::ship_model::ShipPlayer::open(fit.as_ref().map(|(fit, _)| fit), law)
            })
            .tick(
                fit.as_ref().map(|(fit, box_z)| (fit, aspect, *box_z)),
                &input,
                dt,
            );
    }

    /// The zoom and turn terms as text, for a drive script to read.
    pub(crate) fn craft_report(&self) -> Option<(String, String)> {
        let player = self.ship_player.as_ref()?;
        let craft = player.craft();
        Some((
            format!("{:.2}", -craft.fov_delta),
            format!("{:.2},{:.2}", craft.turn[0], craft.turn[1]),
        ))
    }

    /// What the player has done to the craft, for the draw.
    pub(crate) fn craft(&self) -> Craft {
        self.ship_player
            .as_ref()
            .map_or_else(Craft::default, |player| player.craft())
    }
}
