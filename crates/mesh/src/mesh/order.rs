//! The order [`Model`]'s draw lists are submitted in, and where it comes from.
//!
//! Its own module because it is the one thing in [`super`] that is about
//! *ordering* rather than about geometry, and because the reasoning behind it is
//! longer than the code: the whole of it is one stable sort on one field, and
//! all of the value is in knowing which field and why.

use super::{DrawCall, Model};

impl Model {
    /// Puts all three draw lists into the order the original's render queue
    /// dispatches them: **ascending by [`DrawCall::layer`], stably.**
    ///
    /// # What the original does
    ///
    /// Wipeout Pulse draws neither in file order nor by depth. Every drawable
    /// calls `Gfx_Enqueue` (`0x0891e35c`) with a 32-bit key, and
    /// `Gfx_FlushRenderManager` (`0x0891e3c0`) sorts that one queue ascending
    /// before dispatching it. A **mesh** batch set enqueues with its bare layer
    /// key and no depth term at all (`0x0892ece0`), so between two meshes the
    /// layer is the whole of the order, and within a layer it is submission
    /// order. That is exactly what a stable sort on the layer reproduces.
    ///
    /// # Why it is done once, at build
    ///
    /// The key mesh geometry is queued with does not depend on the camera, so
    /// neither does this order. Sorting here rather than per frame keeps every
    /// draw site iterating a plain slice, and keeps `pvs::DrawSections` - which
    /// is built from these lists afterwards and is index-parallel to them -
    /// correct without knowing this happened.
    ///
    /// # Two ways this is not the original
    ///
    /// Stated rather than hidden, because both are ours and neither is measured:
    ///
    /// - **The original's `qsort` (`0x08972860`, Bentley-McIlroy) is not
    ///   stable**, so for two items with equal keys it may emit either order.
    ///   A stable sort is a deterministic choice inside the freedom that leaves,
    ///   not a claim about which order the original picks.
    /// - **The original has one queue for the whole scene**; this sorts one
    ///   model's lists. For a circuit, which is one `Model`, those coincide.
    ///
    /// See `docs/rendering/draw-order.md`.
    pub fn sort_by_layer(&mut self) {
        for list in [
            &mut self.draws,
            &mut self.alpha_tested_draws,
            &mut self.transparent_draws,
        ] {
            list.sort_by_key(|draw| draw.layer);
        }
    }

    /// The order this frame draws [`Self::transparent_draws`] in, as indices
    /// into it, or `None` when no draw carries a [`DrawCall::depth_point`] and
    /// the list's own order stands.
    ///
    /// **List B is queued with a view depth.** `Mesh_SubmitNode` (`0x0890cc80`)
    /// enqueues a mesh's list-B batches at `mesh+0xc8` OR-ed with
    /// `0xfffff - trunc(-z * 349.525)`, `z` being the view-space depth of the
    /// mesh's box centre (clamped to zero past 3,000 units and for a point behind
    /// the eye), and `Gfx_FlushRenderManager` sorts the whole queue ascending: the
    /// far meshes first. Both `0x45` trees and `0x4a` arrow strips land on the
    /// same `0x4d000000` layer, so a nearer tree is drawn after the strip that a
    /// layer-only order put over it. The sort is stable here, so the batches of
    /// one mesh (one key) and meshes of equal key keep the list's order; the
    /// original's `qsort` is not stable, so equal keys are our choice.
    ///
    /// `depth_plane` is the fourth row of the view-projection matrix
    /// (`clip.w = -z_view` for a perspective projection), as `[x, y, z, w]`.
    /// **Chosen, not measured:** the key is taken in the units of the model's own
    /// positions; the original multiplies the transformed `z` by
    /// `display+0x4310 / 65`, a scale not read, which moves only where the
    /// 3,000-unit clamp falls.
    #[must_use]
    pub fn blended_order(&self, depth_plane: [f32; 4]) -> Option<Vec<u32>> {
        order_blended(&self.transparent_draws, depth_plane)
    }
}

/// [`Model::blended_order`] over a draw list.
fn order_blended(draws: &[DrawCall], depth_plane: [f32; 4]) -> Option<Vec<u32>> {
    if !draws.iter().any(|d| d.depth_point.is_some()) {
        return None;
    }
    let mut keyed: Vec<(u32, u32)> = draws
        .iter()
        .zip(0u32..)
        .map(|(draw, index)| {
            let depth = draw
                .depth_point
                .map_or(0, |p| view_depth_key(depth_plane, p));
            (draw.layer | depth, index)
        })
        .collect();
    keyed.sort_unstable();
    Some(keyed.into_iter().map(|(_, index)| index).collect())
}

/// The low twenty bits of a list-B key: `0xfffff - trunc(-z * 349.525)`, so a
/// nearer point has the larger key and draws later. `0` for a point behind the
/// eye or past 3,000 units (`0xfffff / 349.525`).
fn view_depth_key(plane: [f32; 4], point: [f32; 3]) -> u32 {
    let distance = plane[0] * point[0] + plane[1] * point[1] + plane[2] * point[2] + plane[3];
    let t = distance * 349.525;
    if t > 0.0 && t <= 1_048_575.0 {
        0xfffff - (t as u32 & 0xfffff)
    } else {
        0
    }
}

#[cfg(test)]
mod tests;
