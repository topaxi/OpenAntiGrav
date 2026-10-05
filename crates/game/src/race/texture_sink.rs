//! A device to upload textures through while a circuit loads.
//!
//! [`super::load`] produces plain data and holds no GPU, so every texture an
//! `.rcsmodel` names was decoded and kept on the CPU until the scene was built:
//! Tech De Ra's peak was all 2.3 GiB of its BC7 blocks at once, plus the craft
//! and the sky. A caller that already has the device the scene will be built
//! from opens this around the load, and each texture goes up the moment it is
//! decoded - see [`oag_mesh::mesh_render::TextureSinkScope`].
//!
//! **Only a caller whose scene is built from the same device may open one.**
//! The menus' launch and the headless `--race --screenshot` do; the windowed
//! `--race` loads before its window, and so its device, exists, and the trace
//! and `--dry-run` routes never build a scene at all.

use oag_mesh::mesh_render::TextureSinkScope;

/// The device and queue a load uploads its textures through.
#[derive(Debug, Clone)]
pub struct TextureSink {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl TextureSink {
    /// Opens the sink on the current thread until the returned scope drops.
    #[must_use]
    pub fn open(&self) -> TextureSinkScope {
        TextureSinkScope::open(&self.device, &self.queue)
    }

    /// [`Self::open`] for a load that may have no device: `None` opens nothing
    /// and the load keeps every texture on the CPU, as it always did.
    #[must_use]
    pub fn open_if(sink: Option<&Self>) -> Option<TextureSinkScope> {
        sink.map(Self::open)
    }
}
