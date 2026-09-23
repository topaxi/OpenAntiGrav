//! What a Pulse surface stamps into the bloom's glow mask.
//!
//! The GE keeps the stencil in the framebuffer's alpha and never blends it,
//! so the original's mask holds **stencil references**, not coverage. Read in
//! `Gfx_BuildBatchStateList` (`0x0891f890`) and `FUN_089307b4`, and measured
//! out of EDRAM on a live race - see `docs/rendering/glow-mask.md`:
//!
//! - a batch with `pass_mask & 0xc0` stamps its texture's glow byte, the
//!   `Texture` node payload's byte `+0x1e` (`0xff` when it has no texture);
//! - every other opaque or alpha-tested batch stamps [`BASE`];
//! - a transparent batch without `0xc0` stamps nothing.
//!
//! [`batch_value`] is that rule, and [`super::GpuVertex::glow`] carries its
//! answer to the shader.

use oag_vex::vex;

/// What an opaque batch without the glow bits stamps: `g_display+0x1178`,
/// read live as `4`. Its writer is not read, so this is a measurement of the
/// running game rather than a reading of the code that sets it.
pub const BASE: u8 = 4;

/// The `pass_mask` bits that route a batch's stencil reference to its
/// texture's glow byte.
pub const STAMP_BITS: u16 = 0xc0;

/// Where a `Texture` node's payload keeps its glow byte.
const TEXTURE_GLOW_AT: usize = 0x1e;

/// Each `Texture` node's glow byte, by ordinal - the index a material names.
///
/// `None` for a node whose payload is too short to carry one.
#[must_use]
pub fn texture_bytes(data: &[u8]) -> Vec<Option<u8>> {
    let Ok(classes) = vex::classes_of(data) else {
        return Vec::new();
    };
    let Ok(nodes) = vex::nodes(data) else {
        return Vec::new();
    };
    nodes
        .iter()
        .filter(|n| Some(n.class_id) == classes.texture)
        .map(|n| data[n.payload()].get(TEXTURE_GLOW_AT).copied())
        .collect()
}

/// The value one batch stamps into the mask, `0..=1`.
///
/// `texture` is the ordinal its material names, if any. A transparent batch
/// with the glow bits answers its texture's byte too, though no pipeline here
/// writes it yet - see `mesh_render::GlowMask::Stamped`.
#[must_use]
pub fn batch_value(batch: &vex::Batch, texture: Option<u32>, bytes: &[Option<u8>]) -> f32 {
    let byte = if batch.pass_mask & STAMP_BITS != 0 {
        texture
            .and_then(|t| bytes.get(t as usize).copied().flatten())
            .unwrap_or(0xff)
    } else if batch.is_transparent() {
        0
    } else {
        BASE
    };
    f32::from(byte) / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(pass_mask: u16) -> vex::Batch {
        vex::Batch {
            pass_mask,
            material_index: 0,
            primitive_type: vex::PRIM_TRIANGLES,
            vertex_type: 0,
            scale: 1.0,
            vertices: Vec::new(),
            declared_vertex_count: 0,
            bounds: ([0.0; 3], [0.0; 3]),
            header_flags: 0,
        }
    }

    /// The three rules on `docs/rendering/glow-mask.md`, with the values the
    /// live EDRAM read found: `walls002_sb_GLOW`'s `0x5c` under a glow
    /// batch, the base `4` under an ordinary one, nothing under a blend.
    #[test]
    fn a_batch_stamps_its_textures_byte_the_base_or_nothing() {
        let bytes = [Some(0x5c), None];
        let value = |pass_mask, texture| {
            (batch_value(&batch(pass_mask), texture, &bytes) * 255.0).round() as u8
        };
        assert_eq!(value(0x0880, Some(0)), 0x5c);
        assert_eq!(
            value(0x0880, Some(1)),
            0xff,
            "no byte: the state list's 0xff"
        );
        assert_eq!(value(0x0880, None), 0xff);
        assert_eq!(value(0x0800, Some(0)), BASE);
        assert_eq!(value(0x0000, Some(0)), BASE);
        assert_eq!(
            value(0x1232, Some(0)),
            0,
            "the PSP plume: transparent, no glow bits"
        );
    }
}
