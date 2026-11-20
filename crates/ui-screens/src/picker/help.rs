//! The Help overlay a selection screen authors on a face button.
//!
//! `Track Creation` and `Team Selection` each carry a `Redirect` whose `forward` is a face button
//! and whose `goto` is the screen's Help: `Track Help` and `Team Help` with `forward="triangle"`
//! on the PSP, `Track HelpSingle` and `Team HelpSingle` with `forward="circle"` on the PS2
//! (`docs/ui/back-and-help.md`, "Census of the authored redirects"). The overlay is read from the
//! disc's own XML and drawn the way `Cell Help` is ([`crate::campaign::cell_help_draw`]); a screen
//! that authors no such `Redirect`, or whose target is not in the file, has none, and
//! [`crate::picker::Picker::set_help_available`] keeps its button from opening anything.

use oag_gameplay::input::Button;
use oag_ui::frontend::{Draw, Placed};
use oag_ui::language::StringTable;
use oag_ui::screen::{Screen, Screens};

use super::FaceScales;
use crate::campaign;
use oag_ui::font::Atlas;

/// A selection screen's Help overlay and the button its own `Redirect` opens it on.
#[derive(Debug, Clone)]
pub struct Help {
    /// The overlay screen's widgets.
    pub layout: campaign::Layout,
    /// The face button the disc authors Help on: the PSP's triangle, the PS2's circle. The roles
    /// resolver turns it into the button the player presses
    /// (`oag_title::Title::menu_buttons`).
    pub button: Button,
}

impl Help {
    /// The overlay `screen` redirects to on a face button, read from `screens`.
    pub(super) fn read(
        screen: &Screen,
        screens: &Screens,
        strings: &StringTable,
        faces: FaceScales,
        grid: [f32; 2],
    ) -> Option<Self> {
        let redirect = screen.redirects.iter().find(|redirect| {
            matches!(redirect.forward, Some(Button::Triangle | Button::Circle))
                && redirect
                    .goto
                    .as_deref()
                    .is_some_and(|goto| goto.contains("Help"))
        })?;
        let layout =
            campaign::Layout::read(screens, redirect.goto.as_deref()?, strings, faces, grid)?;
        Some(Self {
            layout,
            button: redirect.forward?,
        })
    }

    /// The overlay over the page and its dimming backdrop: draw it above the page.
    #[must_use]
    pub fn draw(&self, sprites: &dyn Fn(&str) -> Option<Placed>, atlas: &Atlas) -> Vec<Draw> {
        campaign::cell_help_draw(&self.layout, sprites, atlas)
    }
}
