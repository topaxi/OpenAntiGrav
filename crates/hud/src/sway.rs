//! The race HUD sliding across the screen with the ship's drawn lean.
//!
//! Wipeout HD's `Hud_Update` (`0x0009e704`-`0x0009e754`) stores one translation, `(-15 *
//! ship+0x6a58, 15 * ship+0x6a5c)`, into the two screens it looked up by name at load, `"HUD"`
//! and `"InGame"`. Every race layout's root element is `<Screen name="HUD">` (Omega's two HUD
//! sets read alike), so the whole HUD moves, not a widget: on RPCS3 under player input a held
//! full stick slid the bars, the clocks, the pickup slot and the fixed reticle by the same 13.3
//! px at 1280x720, five regions measured within 0.1 px of each other, and every plateau of the
//! follower did the same at its own size (`docs/ghidra/functions/ps3-hdfury-eu/ship-lean.md`,
//! "The HUD sway").
//!
//! This is the **x** term: `ship+0x6a58` is the drawn lean's follower, which the simulation
//! already runs for the hull (`oag_physics::ShipState::steer_lean_follower`), so the HUD reads the
//! same number. The y term is `15 * ship+0x6a5c`, the far camera rig's spring lag along the
//! craft's up axis, and is drawn as **zero**: its law is read there but it rides on
//! `body+0x1b0` and `ship+0x6b00`, two terms nobody has read.
//!
//! The pickup slot is an `<Image>` of the layout on HD (`pickup_icon_models` is `None`) and a
//! sprite in `draw_list` on every title, so it slides with the rest. The lock-on sight, which
//! `draw_list` also builds and which follows its target through the world, slides **by
//! structure** only: it is a widget of the HUD screen, so the translation of the screen carries
//! it, and no lock was ever captured with the stick held.
//!
//! Display only. Nothing here is hashed or read back by the simulation.

use oag_ui::frontend::Draw;

use super::Readout;
use super::draw::Frame;
use super::placement::{Placement, place};

/// How far the whole HUD slides this frame, in the layout's own grid: `[x, y]`.
///
/// `[0.0, 0.0]` for a title with no measured sway, which is every title but HD and the two
/// that inherit its law.
pub(super) fn offset(art: &oag_title::HudArt, readout: &Readout) -> [f32; 2] {
    match art.sway {
        Some(sway) => [
            f32::from(sway.value.x_per_lean) * readout.steer_lean_follower,
            0.0,
        ],
        None => [0.0, 0.0],
    }
}

/// Moves every draw of `frame` by `offset`, the same translation [`Placement`] gives a split
/// view's HUD, with a clip so wide that nothing is dropped. The frame's spans are left alone:
/// a whole-screen frame has none, and a split view's frame is placed after this.
pub(super) fn slide(frame: &mut Frame, offset: [f32; 2]) {
    if offset == [0.0, 0.0] {
        return;
    }
    let placement = Placement {
        scale: 1.0,
        offset,
        clip: [-1.0e9, -1.0e9, 2.0e9, 2.0e9],
    };
    let slid = |list: &mut Vec<Draw>| {
        *list = list
            .iter()
            .filter_map(|draw| place(draw, &placement))
            .collect();
    };
    slid(&mut frame.sprites);
    slid(&mut frame.hud_text);
    slid(&mut frame.small_text);
    slid(&mut frame.default_text);
    slid(&mut frame.skin_text);
}

#[cfg(test)]
mod tests;
