//! Indirect submission: a list of indexed draws that share every piece of state goes out as
//! one `multi_draw_indexed_indirect` from arguments the CPU writes each frame, instead of one
//! `draw_indexed` per draw.
//!
//! **What it saves is wgpu's per-draw validation**, not GPU work. `encoder.finish()` replays
//! every recorded command through wgpu-core, and a `draw_indexed` there checks the pipeline,
//! the bound groups and the vertex buffers again each time. A multi-draw checks them once and
//! its sub-draws cost a metadata entry each for wgpu's GPU-side argument validation, which
//! stays on. Measured in a standalone wgpu 30 program on the RX 7800 XT (RADV), 1,000 draws
//! in one pass: `finish` 107 to 260 us with `draw_indexed`, 37 to 65 us with one
//! multi-draw.
//!
//! **Only lists whose draws differ by index range alone.** The material slot of a shaded draw
//! is an immediate ([`super::bindless`]), which cannot change inside one multi-draw. Carrying
//! it per draw instead (the instance index into a storage buffer) makes the array index
//! non-uniform, which the bindless lane measured at 6.5 to 16% more GPU time on ANV and
//! NVIDIA. So the user here is the depth half of the prepass ([`super::Prepass`]), which
//! reads no texture.
//!
//! **Chosen at startup, per device**: [`selects`] on the backend, and `OAG_INDIRECT=0`
//! keeps the per-draw path for an A/B. Vulkan and Dx12 always have `INDIRECT_EXECUTION`
//! and draw a multi-draw natively. GL loops over it inside wgpu-hal, WebGPU has no
//! multi-draw, and Metal's `INDIRECT_EXECUTION` depends on the GPU family and is unmeasured
//! here, so all three keep the per-draw path, which stays the tested fallback.

/// The bytes of one draw's arguments: `wgpu::util::DrawIndexedIndirectArgs`.
pub const ARGS_SIZE: u64 = std::mem::size_of::<wgpu::util::DrawIndexedIndirectArgs>() as u64;

/// Whether a device on `backend` takes the indirect path, given the `OAG_INDIRECT`
/// switch's value: Vulkan and Dx12, where wgpu has `INDIRECT_EXECUTION` and a native
/// multi-draw, unless the switch says `0`.
#[must_use]
pub fn selects(backend: wgpu::Backend, switch: Option<&str>) -> bool {
    matches!(backend, wgpu::Backend::Vulkan | wgpu::Backend::Dx12) && switch != Some("0")
}

/// [`selects`] for `device` and this process's environment.
#[must_use]
pub fn wanted(device: &wgpu::Device) -> bool {
    selects(
        device.adapter_info().backend,
        std::env::var("OAG_INDIRECT").ok().as_deref(),
    )
}

/// One argument list per view: `views` regions of `draws` entries each, so two views of a
/// frame never write the same bytes (a queue write lands before every command of the
/// submission it joins).
#[derive(Debug)]
pub struct Args {
    buffer: wgpu::Buffer,
    /// Bytes per view.
    stride: u64,
    draws: usize,
}

impl Args {
    /// Room for `draws` draws in each of `views` views.
    #[must_use]
    pub fn new(device: &wgpu::Device, label: &str, views: usize, draws: usize) -> Self {
        let stride = draws.max(1) as u64 * ARGS_SIZE;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: stride * views.max(1) as u64,
            usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            stride,
            draws,
        }
    }

    /// Packs `ranges` (index ranges of one index buffer, instance 0, base vertex 0) into
    /// `bytes`, which the caller then writes with [`Self::write`]. At most the list's capacity
    /// is packed; the number packed is returned.
    pub fn pack(
        &self,
        ranges: impl Iterator<Item = std::ops::Range<u32>>,
        bytes: &mut Vec<u8>,
    ) -> u32 {
        bytes.clear();
        let mut count = 0;
        for range in ranges.take(self.draws) {
            let args = wgpu::util::DrawIndexedIndirectArgs {
                index_count: range.end - range.start,
                instance_count: 1,
                first_index: range.start,
                base_vertex: 0,
                first_instance: 0,
            };
            bytes.extend_from_slice(args.as_bytes());
            count += 1;
        }
        count
    }

    /// Where view `view`'s list starts in [`Self::buffer`].
    #[must_use]
    pub fn offset(&self, view: usize) -> u64 {
        view as u64 * self.stride
    }

    /// The buffer every view's list is in.
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Writes `bytes`, packed by [`Self::pack`], as view `view`'s list.
    pub fn write(&self, writes: &dyn oag_gpu::writes::Writes, view: usize, bytes: &[u8]) {
        if !bytes.is_empty() {
            oag_gpu::perfprobe::write_buffer(writes, &self.buffer, self.offset(view), bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vulkan_and_dx12_take_the_indirect_path() {
        assert!(selects(wgpu::Backend::Vulkan, None));
        assert!(selects(wgpu::Backend::Dx12, None));
        assert!(selects(wgpu::Backend::Vulkan, Some("1")));
    }

    #[test]
    fn gl_metal_and_the_browser_keep_the_per_draw_path() {
        for backend in [
            wgpu::Backend::Gl,
            wgpu::Backend::Metal,
            wgpu::Backend::BrowserWebGpu,
            wgpu::Backend::Noop,
        ] {
            assert!(!selects(backend, None), "{backend:?}");
        }
    }

    #[test]
    fn the_switch_at_zero_keeps_the_per_draw_path() {
        assert!(!selects(wgpu::Backend::Vulkan, Some("0")));
        assert!(!selects(wgpu::Backend::Dx12, Some("0")));
    }
}
