//! A title's speed and shield gauge drawn from three `<Mode3D><Model>` widgets - Pure's.
//!
//! [`oag_title::BarModels`] names them and carries the evidence. Each is a model the loader
//! rasterised flat into the sheet (`oag_raceplay::hud`), so this module is placement and a
//! crop: the fills are drawn full and cut at the bar's fraction from the left, the outline
//! over both.
//!
//! **Where**: the block is orthographic and `y` counts up from the bottom of the screen
//! ([`oag_display::space::SCREEN`]'s height minus the authored `y`), so the widget's origin is
//! `(394, 252)` on the PSP's 272 rows. The sheet entry carries the model's bounds as its
//! [`crate::sprite::Placed::quad_extent`] and [`crate::sprite::Placed::quad_centre`], so the
//! rectangle is the model's own and not a hand measure.
//!
//! **The fill is a crop, and a fraction of the mesh.** The game scrubs the model's animation
//! by the fraction, which slides a texture that is opaque on its left half and clear on its
//! right under a clamped edge (`Hud_UpdateSpeedBar`, `0x0881eabc`; the shield's `0x0881f13c`).
//! The mesh's `u` runs 0.5 to 5.5 across its width, so the edge is at that fraction of the
//! mesh's own width, which is what [`crate::draw::crop_horizontally`] does to the sprite.
//! Not modelled: the animation's first key sits one frame in, so the original's fill stays
//! empty up to 1.7 percent of the way (about 3 px of a 169 px bar); and the shield's own
//! easing, a chase of the target at 140 percent a second.

use super::draw::Context;
use super::{Draw, Readout};

/// The gauge's draws, fills under the outline. Empty for a title with no [`oag_title::BarModels`],
/// and for a widget whose model did not make it into the sheet (the loader reports that).
pub(super) fn bar_model_draws(cx: &Context<'_>, readout: &Readout) -> Vec<Draw> {
    let Some(bars) = cx.art.bar_models else {
        return Vec::new();
    };
    let height = oag_display::space::SCREEN.1;
    let mut out = Vec::new();
    for (name, fraction) in [
        (
            bars.speed,
            Some(readout.speed_fraction(cx.art.speed_full_kmh as f32)),
        ),
        (bars.shield, Some(readout.shield_fraction())),
        (bars.outline, None),
    ] {
        let Some(model) = cx.layout.models.iter().find(|model| model.name == name) else {
            continue;
        };
        let Some(placed) = cx.sheet.get(&model.src) else {
            continue;
        };
        let [w, h] = placed
            .quad_extent
            .unwrap_or([placed.width as f32, placed.height as f32]);
        let [cx_off, cy_off] = placed.quad_centre.unwrap_or([0.0, 0.0]);
        let mut rect = [
            model.position[0] + cx_off - w * 0.5,
            height - model.position[1] + cy_off - h * 0.5,
            w,
            h,
        ];
        let mut uv = [
            placed.x as f32,
            placed.y as f32,
            placed.width as f32,
            placed.height as f32,
        ];
        if let Some(fraction) = fraction {
            if fraction <= 0.0 {
                continue;
            }
            rect[2] *= fraction;
            uv[2] *= fraction;
        }
        out.push(Draw::Sprite {
            rect,
            uv,
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    out
}
