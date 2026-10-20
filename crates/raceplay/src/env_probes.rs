//! The per-craft environment probes a Wipeout HD hull is lit from.
//!
//! The circuit's own baked probes ([`oag_vex::probes`]) blended at each craft's
//! position, two 128x64 layers per grid slot: the irradiance map (the original's
//! `paraboloidIblTex`, 64x32, bilinearly resized to 128x64 here so one array holds both)
//! and the reflection map (`paraboloidReflectionTex`). **Not a render of our scene**: the original's maps are a
//! blend of disc data too, so nothing here draws, and a circuit with no `.probes`
//! binds no array and leaves every hull on its constant ambient.
//!
//! The array is bound to the scene group beside the shadow maps
//! (`mesh_render::ShadowMaps::env_probes`) and a hull names its craft in its model
//! uniform; the shader reads layer `2 * craft` for the ambient and `2 * craft + 1` for the
//! reflection of the materials that declare one.

use oag_gpu::writes::Writes;
use oag_mesh::mesh_render::{ENV_PROBE_FORMAT, ENV_PROBE_LAYERS, ENV_PROBE_SIZE};
use oag_vex::probes::{IRRADIANCE_SIZE, Probes, REFLECTION_SIZE};

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
                width: ENV_PROBE_SIZE.0,
                height: ENV_PROBE_SIZE.1,
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

    /// Blends the probes at `position` into the craft's two layers; `craft` is its grid slot.
    pub fn write(&self, queue: &dyn Writes, craft: usize, position: [f32; 3]) {
        if craft * 2 + 1 >= ENV_PROBE_LAYERS as usize {
            return;
        }
        let taps = self.probes.locate(position);
        let irradiance = resize_2x(&self.probes.blend_irradiance(&taps));
        let reflection = self.probes.blend_reflection(&taps);
        for (layer, texels) in [(craft * 2, &irradiance), (craft * 2 + 1, &reflection)] {
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
                texels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(ENV_PROBE_SIZE.0 * 4),
                    rows_per_image: Some(ENV_PROBE_SIZE.1),
                },
                wgpu::Extent3d {
                    width: ENV_PROBE_SIZE.0,
                    height: ENV_PROBE_SIZE.1,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
}

/// A 64x32 `RGBA8` image resized to 128x64 the way a linear sampler reads it at the new
/// texel centres, clamped at the edges.
fn resize_2x(src: &[u8]) -> Vec<u8> {
    let (sw, sh) = IRRADIANCE_SIZE;
    let (dw, dh) = REFLECTION_SIZE;
    let mut out = vec![0u8; dw * dh * 4];
    for y in 0..dh {
        let fy = ((y as f32 + 0.5) * 0.5 - 0.5).clamp(0.0, (sh - 1) as f32);
        let (y0, ty) = (fy as usize, fy.fract());
        let y1 = (y0 + 1).min(sh - 1);
        for x in 0..dw {
            let fx = ((x as f32 + 0.5) * 0.5 - 0.5).clamp(0.0, (sw - 1) as f32);
            let (x0, tx) = (fx as usize, fx.fract());
            let x1 = (x0 + 1).min(sw - 1);
            for c in 0..4 {
                let at = |xx: usize, yy: usize| f32::from(src[(yy * sw + xx) * 4 + c]);
                let top = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * tx;
                let bottom = at(x0, y1) + (at(x1, y1) - at(x0, y1)) * tx;
                out[(y * dw + x) * 4 + c] = (top + (bottom - top) * ty + 0.5) as u8;
            }
        }
    }
    out
}
