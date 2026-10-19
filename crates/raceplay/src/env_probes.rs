//! The per-craft environment probes a Wipeout HD hull is lit from.
//!
//! The circuit's own baked probes ([`oag_vex::probes`]) blended at each craft's
//! position, one 64x32 layer per grid slot - what the original's `paraboloidIblTex`
//! holds for that craft. **Not a render of our scene**: the original's maps are a
//! blend of disc data too, so nothing here draws, and a circuit with no `.probes`
//! binds no array and leaves every hull on its constant ambient.
//!
//! The array is bound to the scene group beside the shadow maps
//! (`mesh_render::ShadowMaps::env_probes`) and a hull names its layer in its model
//! uniform. Its 128x64 reflection map ([`oag_vex::probes::Probes::blend_reflection`])
//! is decoded and measured against the original but has no reader yet: only the
//! `specular_from_alpha` hull chunks sample it.

use oag_gpu::writes::Writes;
use oag_mesh::mesh_render::{ENV_PROBE_FORMAT, ENV_PROBE_LAYERS};
use oag_vex::probes::{IRRADIANCE_SIZE, Probes};

/// The probes of a circuit and the array they are blended into.
#[derive(Debug)]
pub struct EnvProbes {
    probes: Probes,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl EnvProbes {
    /// An array of [`ENV_PROBE_LAYERS`] layers, undefined until a craft writes its own.
    #[must_use]
    pub fn new(device: &wgpu::Device, probes: Probes) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("environment probes"),
            size: wgpu::Extent3d {
                width: IRRADIANCE_SIZE.0 as u32,
                height: IRRADIANCE_SIZE.1 as u32,
                depth_or_array_layers: ENV_PROBE_LAYERS,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: ENV_PROBE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        Self {
            probes,
            texture,
            view,
        }
    }

    /// The array, for `ShadowMaps::env_probes`.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Blends the probes at `position` into `layer`, which is the craft's grid slot.
    pub fn write(&self, queue: &dyn Writes, layer: usize, position: [f32; 3]) {
        if layer >= ENV_PROBE_LAYERS as usize {
            return;
        }
        let taps = self.probes.locate(position);
        let texels = self.probes.blend_irradiance(&taps);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer as u32,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &texels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(IRRADIANCE_SIZE.0 as u32 * 4),
                rows_per_image: Some(IRRADIANCE_SIZE.1 as u32),
            },
            wgpu::Extent3d {
                width: IRRADIANCE_SIZE.0 as u32,
                height: IRRADIANCE_SIZE.1 as u32,
                depth_or_array_layers: 1,
            },
        );
    }
}
