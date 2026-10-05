//! The adapter, device and queue a headless capture draws with.
//!
//! Split out so a caller can open them **before** the circuit loads and hand
//! the load a [`super::TextureSink`] over the same device, which is what keeps
//! the whole circuit's decoded textures off the CPU at once. A capture given no
//! [`CaptureGpu`] opens its own after the load, as it always did.

use anyhow::{Context, Result};

/// The adapter a capture chose and the device opened on it.
#[derive(Debug, Clone)]
pub struct CaptureGpu {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl CaptureGpu {
    /// The adapter `renderer` names, or the default one, with a device on it.
    ///
    /// The same adapter the window would have drawn with, so a screenshot is a
    /// picture of what a player sees; there is no surface to be compatible
    /// with, which is the only difference.
    ///
    /// # Errors
    ///
    /// No usable adapter, or the device request failing.
    pub fn request(renderer: &oag_display::display::Renderer) -> Result<Self> {
        let instance = crate::adapter::instance();
        let adapter = crate::adapter::choose(&instance, None, renderer)?.adapter;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &oag_mesh::mesh_render::device_descriptor("oag-game race offscreen", &adapter),
        ))
        .context("requesting the device")?;
        Ok(Self {
            adapter,
            device,
            queue,
        })
    }

    /// The sink a load opens to upload through this device.
    #[must_use]
    pub fn texture_sink(&self) -> super::TextureSink {
        super::TextureSink {
            device: self.device.clone(),
            queue: self.queue.clone(),
        }
    }
}

impl super::CaptureOptions {
    /// The device this capture draws with: the one the load streamed through,
    /// or a fresh one.
    pub(super) fn open_gpu(&self) -> Result<CaptureGpu> {
        match &self.gpu {
            Some(gpu) => Ok(gpu.clone()),
            None => CaptureGpu::request(&self.renderer),
        }
    }
}
