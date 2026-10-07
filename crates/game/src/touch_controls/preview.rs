//! A picture of the overlay for the TOUCH CONTROLS page: the real draw list,
//! shrunk into a panel under the rows, following the settings the rows set.
//!
//! **Chosen, not measured.** No title ships a touch scheme, so the panel, the
//! phone shape it stands for and the demo poses are this project's own. The
//! picture behind the panel is whatever the front end already draws there:
//! nothing is painted to stand in for a track.
//!
//! The overlay is laid out for a phone-shaped window ([`PHONE`]) by the same
//! [`super::draw`] a race calls, in the grid a race draws it in, and only then
//! scaled and moved into the panel with [`Draw::zoom`] - so the preview cannot
//! drift from the overlay, and a new control shows up here without an edit.

use oag_display::space::Space;
use oag_input::touch::{Scheme, Setup};
use oag_ui::frontend::Draw;

use super::{Art, Demo};

/// The page the preview belongs to, in `assets/ui/menu.toml`.
pub const PAGE: &str = "touch_controls";

/// The window the preview stands for: the S24's landscape size.
pub const PHONE: (f32, f32) = (2340.0, 1080.0);

/// Seconds each pose is held before the demo moves to the next.
pub const HOLD: f32 = 1.6;

/// The panel's top and bottom as shares of the screen's height: under the
/// rows and above the footer on the front end's skins.
const TOP: f32 = 0.53;
const BOTTOM: f32 = 0.86;

const PANEL: [f32; 4] = [0.0, 0.0, 0.0, 0.50];
const FRAME: [f32; 4] = [1.0, 1.0, 1.0, 0.35];

/// The poses that show off `setup`, idle first. Every pose here draws with
/// exactly `setup` ([`Demo::setup`]), which a test pins.
#[must_use]
pub fn poses(setup: Setup) -> &'static [Demo] {
    match (setup.scheme, setup.zones) {
        (Scheme::Standard, true) => &[
            Demo::Idle,
            Demo::GoLeft,
            Demo::GoRight,
            Demo::StickAndGo,
            Demo::GoFire,
            Demo::GoLeftPast,
        ],
        (Scheme::Standard, false) => &[Demo::IdleButtons],
        (Scheme::Easy, true) => &[Demo::EasyIdle, Demo::EasyBrake],
        (Scheme::Easy, false) => &[Demo::EasyZonesOff, Demo::EasyBarBrake],
    }
}

/// The pose on show `clock` seconds into the page; `0.0` is the idle one.
#[must_use]
pub fn pose_at(setup: Setup, clock: f32) -> Demo {
    let poses = poses(setup);
    let step = (clock.max(0.0) / HOLD) as usize;
    poses[step % poses.len()]
}

/// Where the panel sits on a screen of `space`: centred, phone-shaped.
#[must_use]
pub fn panel(space: Space) -> [f32; 4] {
    let (w, h) = space.size;
    let height = h * (BOTTOM - TOP);
    let width = (height * PHONE.0 / PHONE.1).min(w * 0.9);
    let height = width * PHONE.1 / PHONE.0;
    [(w - width) / 2.0, h * BOTTOM - height, width, height]
}

/// The panel and the overlay in it, for `setup` at `opacity` (1.0 is the
/// design), `clock` seconds into the page.
#[must_use]
pub fn draw(art: &Art, space: Space, setup: Setup, opacity: f32, clock: f32) -> Vec<Draw> {
    let [x, y, w, h] = panel(space);
    let demo = pose_at(setup, clock);
    let grid = oag_present::perf::GRID_H / PHONE.1;
    let mut list = super::draw(
        art,
        &demo.touches(PHONE),
        PHONE,
        grid,
        false,
        setup,
        opacity,
    );
    let k = h / oag_present::perf::GRID_H;
    let origin = (x / (1.0 - k), y / (1.0 - k));
    for item in &mut list {
        item.zoom(origin, k, 1.0);
    }
    let edge = 0.6;
    let mut out = vec![Draw::Fill {
        rect: [x, y, w, h],
        color: PANEL,
    }];
    for rect in [
        [x, y, w, edge],
        [x, y + h - edge, w, edge],
        [x, y, edge, h],
        [x + w - edge, y, edge, h],
    ] {
        out.push(Draw::Fill { rect, color: FRAME });
    }
    out.extend(list);
    out
}

/// The preview for whichever page is open: empty anywhere but [`PAGE`].
#[must_use]
pub fn for_page(
    page: &str,
    art: &Art,
    space: Space,
    controls: &crate::settings::Controls,
    clock: f32,
) -> Vec<Draw> {
    if page != PAGE {
        return Vec::new();
    }
    draw(
        art,
        space,
        controls.touch_setup(),
        controls.touch_alpha(),
        clock,
    )
}

#[cfg(test)]
mod tests;
