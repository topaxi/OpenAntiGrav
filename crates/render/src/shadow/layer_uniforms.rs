//! Per-layer matrices staged on the CPU and written once a frame.
//!
//! The sun-occlusion and self-shadow passes draw one array layer per craft, each through
//! a matrix of its own in a 256-byte slot of one uniform buffer. A queue write per layer is
//! a write (and its staging copy) per craft per frame; staged here, a frame's layers go up
//! in one write over the span of slots they cover.

use oag_core::math::Mat4;

/// A CPU copy of a buffer of slots, and the span of slots staged since the last write.
#[derive(Debug)]
pub(super) struct LayerUniforms {
    stride: usize,
    bytes: Vec<u8>,
    dirty: Option<std::ops::Range<usize>>,
}

/// The bytes one slot's matrix takes; the rest of the slot is padding to the stride.
const MATRIX: usize = 64;

impl LayerUniforms {
    /// `layers` slots of `stride` bytes, zeroed as a new buffer is.
    pub(super) fn new(stride: u64, layers: u32) -> Self {
        let stride = stride as usize;
        Self {
            stride,
            bytes: vec![0; stride * layers as usize],
            dirty: None,
        }
    }

    /// Puts `matrix` in `layer`'s slot for the next [`Self::write`]. A layer past the
    /// buffer is ignored, as the pass that would read it has no view to draw into.
    pub(super) fn stage(&mut self, layer: usize, matrix: Mat4) {
        let at = self.stride * layer;
        let Some(slot) = self.bytes.get_mut(at..at + MATRIX) else {
            return;
        };
        slot.copy_from_slice(bytemuck::cast_slice(&matrix.to_cols_array()));
        self.dirty = Some(match self.dirty.take() {
            Some(span) => span.start.min(layer)..span.end.max(layer + 1),
            None => layer..layer + 1,
        });
    }

    /// The staged span as a buffer offset and its bytes, or `None` when nothing was staged;
    /// the span is clean afterwards.
    ///
    /// A slot inside the span that nothing staged this time carries the bytes it last
    /// had, which are what the buffer already holds, so the write leaves it unchanged.
    fn take(&mut self) -> Option<(u64, &[u8])> {
        let span = self.dirty.take()?;
        let start = span.start * self.stride;
        let end = (span.end - 1) * self.stride + MATRIX;
        Some((start as u64, &self.bytes[start..end]))
    }

    /// Writes what [`Self::stage`] put since the last call to `buffer`, in one write.
    pub(super) fn write(&mut self, queue: &dyn oag_gpu::writes::Writes, buffer: &wgpu::Buffer) {
        if let Some((offset, bytes)) = self.take() {
            queue.write_buffer(buffer, offset, bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(bytes: &[u8], at: usize) -> Mat4 {
        Mat4::from_cols_slice(bytemuck::cast_slice(&bytes[at..at + MATRIX]))
    }

    #[test]
    fn one_write_covers_the_staged_layers_and_keeps_what_lies_between() {
        let mut uniforms = LayerUniforms::new(256, 8);
        assert!(uniforms.take().is_none());
        let earlier = Mat4::from_scale(oag_core::math::Vec3::splat(2.0));
        uniforms.stage(2, earlier);
        let (offset, bytes) = uniforms.take().expect("layer 2 staged");
        assert_eq!((offset, bytes.len()), (512, MATRIX));
        assert!(uniforms.take().is_none(), "a written span is clean");

        let (one, four) = (
            Mat4::from_translation(oag_core::math::Vec3::X),
            Mat4::from_translation(oag_core::math::Vec3::Z),
        );
        let ((), allocations) = crate::alloc_count::allocations(|| {
            uniforms.stage(4, four);
            uniforms.stage(1, one);
        });
        assert_eq!(allocations, 0);
        let (offset, bytes) = uniforms.take().expect("layers 1 and 4 staged");
        assert_eq!((offset, bytes.len()), (256, 3 * 256 + MATRIX));
        assert_eq!(slot(bytes, 0), one);
        assert_eq!(slot(bytes, 256), earlier, "layer 2 keeps its last matrix");
        assert_eq!(slot(bytes, 512), Mat4::ZERO, "layer 3 was never written");
        assert_eq!(slot(bytes, 768), four);
    }

    #[test]
    fn a_layer_past_the_buffer_stages_nothing() {
        let mut uniforms = LayerUniforms::new(256, 8);
        uniforms.stage(8, Mat4::IDENTITY);
        assert!(uniforms.take().is_none());
    }
}
