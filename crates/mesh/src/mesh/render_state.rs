//! What a PS3 `.rcsmodel` material's render-state word does to the draw lists,
//! once a title's rule says to honour it.
//!
//! `Material_ApplyRenderState` (`0x005d8f68` in `ps3-hdfury-eu`) feeds the
//! state word's bits to the RSX, and a live Moa Therma frame shows what the race
//! pass makes of them (`docs/ghidra/functions/ps3-hdfury-eu/material-state.md`,
//! "The race pass sets pass bits 3, 5 and 6"):
//!
//! - **Bit 7 turns blending and the alpha test off and alpha-to-coverage on,
//!   and the draw writes depth.** All 31 of the frame's `cf_tree` draws
//!   (`0xbd`) are issued that way, among the opaque draws, before the first
//!   blended one. Here they move from [`Model::transparent_draws`] to
//!   [`Model::alpha_tested_draws`], the list that is drawn where the original
//!   issues them and writes depth the same way.
//! - **Bit 2 beside the blend bit is a blended draw that writes depth.** The
//!   frame's `0x2d` corner arrows and `0x3d` start line blend with the depth
//!   mask on; the `0x39` glass blends with it off. Those keep their place in
//!   the blended list and get [`DrawCall::writes_depth`](super::DrawCall).
//!
//! **Coverage itself is chosen, not measured.** The console's scene target has
//! two samples, so a texel's alpha keeps 0, 1 or 2 of them and the resolve
//! softens the edge. This renderer's scene has one sample by default, where
//! coverage degenerates to keeping the pixel or not; [`COVERAGE_REFERENCE`] is
//! the threshold that stands in for it.

use super::Model;

/// The state bit that turns the depth write on (`Rsx_SetDepthMask`).
pub const STATE_DEPTH_WRITE: u32 = 0x04;

/// The state bit that selects the alpha-to-coverage path.
pub const STATE_COVERAGE: u32 = 0x80;

/// The alpha a coverage draw's texel must exceed to be kept, standing in for
/// alpha-to-coverage on a single-sample target. **Chosen, not measured**: the
/// midpoint, where two-sample coverage goes from keeping none or one sample to
/// keeping one or two. It is the same `GL_GREATER` / `0.5` pair every mode-2
/// material on the HD disc authors for its own alpha test (`rcs::cutout`).
pub const COVERAGE_REFERENCE: f32 = 0.5;

impl Model {
    /// The state word of the material `slot` names, `0` when the model records
    /// none.
    #[must_use]
    pub fn state_of(&self, slot: Option<usize>) -> u32 {
        slot.and_then(|slot| self.material_state.get(slot))
            .copied()
            .unwrap_or(0)
    }

    /// Applies the render-state bits a title's rule honours: every blended draw
    /// whose material carries [`STATE_COVERAGE`] leaves the blended list for
    /// the cutout list, unblended, at [`COVERAGE_REFERENCE`], and every
    /// blended draw left whose material carries [`STATE_DEPTH_WRITE`] writes
    /// depth. Order within each list is kept. Returns how many draws moved.
    ///
    /// Keyed on the state bit and never on the `_atoc` texture names: two
    /// disc materials sample an `_atoc` texture without the bit
    /// (`docs/formats/rcsmodel.md`).
    pub fn honour_render_state(&mut self) -> usize {
        let (coverage, blended): (Vec<_>, Vec<_>) = std::mem::take(&mut self.transparent_draws)
            .into_iter()
            .partition(|draw| self.state_of(draw.texture) & STATE_COVERAGE != 0);
        let mut blended = blended;
        for draw in &mut blended {
            draw.writes_depth = self.state_of(draw.texture) & STATE_DEPTH_WRITE != 0;
        }
        self.transparent_draws = blended;
        let moved = coverage.len();
        self.alpha_tested_draws
            .extend(coverage.into_iter().map(|mut draw| {
                draw.blend_state = None;
                draw.alpha_test_ref = Some(COVERAGE_REFERENCE);
                draw
            }));
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{Bounds, DrawCall};

    fn blended(slot: usize, first: u32) -> DrawCall {
        DrawCall {
            range: first..first + 3,
            texture: Some(slot),
            bounds: Bounds {
                centre: [0.0; 3],
                radius: 1.0,
            },
            moving: false,
            culled: false,
            blend: None,
            blend_state: Some(wgpu::BlendState::ALPHA_BLENDING),
            depth_point: None,
            layer: 0,
            alpha_test_ref: None,
            writes_depth: false,
            node: None,
            chunk: None,
        }
    }

    /// Moa Therma's pair: a `cf_tree` (`0xbd`) and the glass behind it
    /// (`0x39`). Only the tree leaves the blended list, unblended, and the
    /// glass keeps its blend and its place.
    #[test]
    fn a_coverage_material_leaves_the_blended_list_and_the_glass_stays() {
        let mut model = Model::none("moa");
        model.material_state = vec![0xbd, 0x39];
        model.transparent_draws = vec![blended(0, 0), blended(1, 3), blended(0, 6)];
        assert_eq!(model.honour_render_state(), 2);
        assert_eq!(model.transparent_draws.len(), 1);
        assert_eq!(model.transparent_draws[0].texture, Some(1));
        assert!(model.transparent_draws[0].blend_state.is_some());
        assert!(!model.transparent_draws[0].writes_depth);
        let trees: Vec<_> = model
            .alpha_tested_draws
            .iter()
            .map(|d| d.range.start)
            .collect();
        assert_eq!(trees, [0, 6]);
        for tree in &model.alpha_tested_draws {
            assert_eq!(tree.blend_state, None);
            assert_eq!(tree.alpha_test_ref, Some(COVERAGE_REFERENCE));
        }
    }

    /// A model with no recorded states (every builder but HD's PS3 one) is
    /// left exactly as it was.
    #[test]
    fn a_model_without_states_is_untouched() {
        let mut model = Model::none("pulse");
        model.transparent_draws = vec![blended(0, 0)];
        assert_eq!(model.honour_render_state(), 0);
        assert_eq!(model.transparent_draws.len(), 1);
        assert!(model.alpha_tested_draws.is_empty());
    }

    /// The start line (`0x3d`) blends and writes depth, as the live frame's
    /// draw 551 does; the glass (`0x39`) beside it does not.
    #[test]
    fn a_blended_material_with_bit_2_writes_depth() {
        let mut model = Model::none("moa");
        model.material_state = vec![0x39, 0x3d];
        model.transparent_draws = vec![blended(0, 0), blended(1, 3)];
        assert_eq!(model.honour_render_state(), 0);
        let writes: Vec<_> = model
            .transparent_draws
            .iter()
            .map(|d| d.writes_depth)
            .collect();
        assert_eq!(writes, [false, true]);
    }
}
