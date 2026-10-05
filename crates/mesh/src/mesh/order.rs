//! The order [`Model`]'s draw lists are submitted in, and where it comes from.
//!
//! Its own module because it is the one thing in [`super`] that is about
//! *ordering* rather than about geometry, and because the reasoning behind it is
//! longer than the code: the whole of it is one stable sort on one field, and
//! all of the value is in knowing which field and why.

use super::Model;

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
}
