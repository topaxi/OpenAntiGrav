//! Batches as geometry: triangle lists and strips expanded, and the pass mask
//! and blend flag read off a batch.
//!
//! Split out of `vex.rs`'s `#[cfg(test)] mod tests`, which was 1,108
//! lines - past the 200 an inline test module may hold, and past the 1,000
//! a file may. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use crate::vex::*;

fn batch_with(primitive_type: u8, vertex_count: usize) -> Batch {
    Batch {
        pass_mask: 1,
        material_index: 0,
        primitive_type,
        vertex_type: 0x100,
        scale: 1.0,
        vertices: vec![
            Vertex {
                position: [0.0; 3],
                normal: None,
                texcoord: None,
                colour: None,
            };
            vertex_count
        ],
        declared_vertex_count: u16::try_from(vertex_count).unwrap_or(u16::MAX),
        bounds: ([0.0; 3], [0.0; 3]),
        header_flags: 0,
    }
}

#[test]
fn expands_a_triangle_list() {
    let t = batch_with(PRIM_TRIANGLES, 9).triangles();
    assert_eq!(t, vec![[0, 1, 2], [3, 4, 5], [6, 7, 8]]);
}

#[test]
fn a_triangle_list_ignores_a_trailing_partial_triangle() {
    assert_eq!(batch_with(PRIM_TRIANGLES, 8).triangles().len(), 2);
}

#[test]
fn expands_a_strip_with_alternating_winding() {
    // Getting the flip wrong makes every other face point inwards, which
    // with culling on riddles the model with holes.
    let t = batch_with(PRIM_TRIANGLE_STRIP, 5).triangles();
    assert_eq!(t, vec![[0, 1, 2], [2, 1, 3], [2, 3, 4]]);
}

#[test]
fn a_strip_shorter_than_a_triangle_yields_nothing() {
    assert!(batch_with(PRIM_TRIANGLE_STRIP, 2).triangles().is_empty());
    assert!(batch_with(PRIM_TRIANGLE_STRIP, 0).triangles().is_empty());
}

#[test]
fn unsupported_primitives_yield_nothing_rather_than_garbage() {
    for prim in [0u8, 1, 2, 5, 6] {
        assert!(batch_with(prim, 12).triangles().is_empty(), "prim {prim}");
    }
}

#[test]
fn classifies_pass_masks() {
    let batch = |pass_mask| Batch {
        pass_mask,
        material_index: 0,
        primitive_type: 3,
        vertex_type: 0x100,
        scale: 1.0,
        vertices: Vec::new(),
        declared_vertex_count: 0,
        bounds: ([0.0; 3], [0.0; 3]),
        header_flags: 0,
    };
    assert!(batch(0x0101).is_transparent());
    assert!(!batch(0x0021).is_transparent());
    assert!(batch(0x0801).is_alpha_tested());
    // `0x20` set is the `Gu_Disable(GU_CULL_FACE)` branch - two-sided - so
    // the batch that carries the bit is the one that is *not* culled.
    assert!(!batch(0x0021).is_culled());
    assert!(batch(0x0001).is_culled());

    // The three bits inside `0x0700`, and the branch order that separates
    // them: `Gfx_BuildBatchStateList` tests `0x100` first, then `0x200`,
    // then `0x400`, so a batch carrying more than one takes the earliest.
    assert_eq!(batch(0x0100).blend_class(), Some(BlendClass::AlphaOver));
    assert_eq!(batch(0x0200).blend_class(), Some(BlendClass::Additive));
    assert_eq!(batch(0x0400).blend_class(), Some(BlendClass::None));
    assert_eq!(batch(0x0300).blend_class(), Some(BlendClass::AlphaOver));
    assert_eq!(batch(0x0600).blend_class(), Some(BlendClass::Additive));
    assert_eq!(batch(0x0021).blend_class(), None);
    // The boost plume's own measured mask, all 32 batches across 8 teams.
    assert_eq!(batch(0x1232).blend_class(), Some(BlendClass::Additive));
}

#[test]
fn decodes_the_additive_blend_flag() {
    let batch = |header_flags| Batch {
        pass_mask: 1,
        material_index: 0,
        primitive_type: 3,
        vertex_type: 0x100,
        scale: 1.0,
        vertices: Vec::new(),
        declared_vertex_count: 0,
        bounds: ([0.0; 3], [0.0; 3]),
        header_flags,
    };
    assert!(batch(0x00).is_additive_blend());
    assert!(!batch(0x10).is_additive_blend());
    // The extended-header bit (0x40) is independent of the blend bit.
    assert!(batch(0x40).is_additive_blend());
    assert!(!batch(0x50).is_additive_blend());
}
