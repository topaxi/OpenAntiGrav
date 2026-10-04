//! Which of the gantry's draws stand on the panel at a moment of its
//! timeline, for a title whose clock reaches the later board states.
//!
//! [`super::clip_to_panel`] answers this once, at load, across the panel's
//! width alone: right for Pulse, whose clock never leaves the countdown, and
//! for HD before the first line crossing. HD's race manager moves the clock
//! on to three later windows (`docs/ghidra/functions/ps3-hdfury-eu/gantry-clock.md`),
//! and in each a different set of nodes stands on the panel: the `FX-350`
//! art between laps, `FINAL LAP` on the lap before the last, the chequered
//! flag on the last. What has left the panel then has left it upwards as
//! well as sideways (the countdown glyph and its backdrop rise about ten
//! units at 6.000 s), so this tests both axes of the board's own plane.
//!
//! **Measured on the original, 2026-10-04** (RPCS3, Talon's Junction, each
//! window forced from the grid): in none of the three windows does anything
//! show beside or above the board. Every draw here is keyed by its index
//! range's first index, [`key`], which is unique within one model.

use oag_core::math::{Mat4, Vec3};

use crate::mesh::{DrawCall, Model};

use super::{FX350_TEXTURE, basename};

/// The key a draw is named by in a hidden set: its index range's start.
#[must_use]
pub fn key(draw: &DrawCall) -> u32 {
    draw.range.start
}

/// Every draw of `model`, from all three lists.
fn all_draws(model: &Model) -> impl Iterator<Item = &DrawCall> {
    model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
}

/// The keys of every draw bound to [`FX350_TEXTURE`]: what
/// [`super::strip_fx350_art`] would remove, named instead of removed.
#[must_use]
pub fn fx350_draws(model: &Model) -> Vec<u32> {
    all_draws(model)
        .filter(|draw| {
            draw.texture
                .and_then(|slot| model.textures.get(slot)?.as_ref())
                .is_some_and(|t| basename(&t.label).eq_ignore_ascii_case(FX350_TEXTURE))
        })
        .map(key)
        .collect()
}

/// The midpoint of each draw's own extent in the board's plane (model `x`
/// and `y`), with its `Anim Transform` applied at `seconds`.
///
/// The extent's midpoint and not the vertex mean, for the reason
/// [`super::clip_to_panel`] gives: a board's glyphs crowd one end.
#[must_use]
pub fn midpoints(model: &Model, seconds: f32) -> Vec<(u32, [f32; 2])> {
    let matrices = model.sample_anim_nodes(seconds);
    all_draws(model)
        .filter_map(|draw| {
            let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
            for i in draw.range.clone() {
                let v = model.vertices[model.indices[i as usize] as usize];
                let p = Vec3::from(v.position);
                let p = match v
                    .xform
                    .checked_sub(1)
                    .and_then(|s| matrices.get(s as usize))
                {
                    Some(m) => Mat4::from_cols_array(m).transform_point3(p),
                    None => p,
                };
                lo = [lo[0].min(p.x), lo[1].min(p.y)];
                hi = [hi[0].max(p.x), hi[1].max(p.y)];
            }
            (lo[0] <= hi[0]).then(|| (key(draw), [0.5 * (lo[0] + hi[0]), 0.5 * (lo[1] + hi[1])]))
        })
        .collect()
}

/// The keys of the draws whose midpoint at `seconds` is off a panel
/// `2 half_width` by `2 half_height` centred on the model's origin.
#[must_use]
pub fn off_panel(model: &Model, half_width: f32, half_height: f32, seconds: f32) -> Vec<u32> {
    midpoints(model, seconds)
        .into_iter()
        .filter(|(_, [x, y])| x.abs() > half_width || y.abs() > half_height)
        .map(|(key, _)| key)
        .collect()
}
