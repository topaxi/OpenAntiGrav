//! The per-craft environment probes a Wipeout HD hull is lit from.
//!
//! The circuit's own baked probes ([`oag_vex::probes`]) blended at each craft's
//! position, two 128x64 layers per grid slot: the irradiance map (the original's
//! `paraboloidIblTex`, 64x32, in the layer's top-left corner) and the reflection map
//! (`paraboloidReflectionTex`). **Not a render of our scene**: the original's maps are a
//! blend of disc data too, so nothing here draws, and a circuit with no `.probes`
//! binds no array and leaves every hull on its constant ambient.
//!
//! The array is bound to the scene group beside the shadow maps
//! (`mesh_render::ShadowMaps::env_probes`) and a hull names its craft in its model
//! uniform; the shader reads layer `2 * craft` for the ambient and `2 * craft + 1` for the
//! reflection of the materials that declare one.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use oag_gpu::writes::Writes;
use oag_mesh::mesh_render::{ENV_PROBE_FORMAT, ENV_PROBE_LAYERS, ENV_PROBE_SIZE};
use oag_vex::probes::{self, IRRADIANCE_SIZE, Probes, REFLECTION_SIZE, Taps};

/// The engine's probe tint, parameter `0x7480de6d` (`iblScalePower`): what both probe maps
/// are multiplied by, and how far the result is pulled towards its own luma.
///
/// Read live on RPCS3 (Zone mode, zones 3 and 4, two frames; Talon's Junction and Metropia
/// before): the engine writes `(4, 4, 4, 0)` in a race and `(0.698, 0.698, 0.698, 1)` in a
/// Zone race. What moves it inside a Zone race (the stage, the zone number) is not read:
/// both frames were at the `Sub Flash` stage, so the one value is carried for the whole mode.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ProbeTint {
    /// The rgb scale, the same on all three channels everywhere it was read.
    pub(crate) gain: f32,
    /// The `.w`: `0` keeps the colour, `1` replaces it by its luma.
    pub(crate) desaturate: f32,
}

impl ProbeTint {
    /// A race outside Zone mode.
    pub(crate) const ENGINE: Self = Self {
        gain: 4.0,
        desaturate: 0.0,
    };
    /// A Zone race: `0.69804` is `178 / 255`.
    pub(crate) const ZONE: Self = Self {
        gain: 178.0 / 255.0,
        desaturate: 1.0,
    };

    pub(crate) const fn to_array(self) -> [f32; 2] {
        [self.gain, self.desaturate]
    }
}

/// The probes of a circuit and the array they are blended into.
#[derive(Debug)]
pub struct EnvProbes {
    probes: Probes,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    /// Decoded probes, by index: a craft blends the same few every frame, and decoding a
    /// probe costs more than blending it. Cleared when it outgrows [`CACHE_LIMIT`].
    decoded: RefCell<HashMap<usize, Arc<Decoded>>>,
    /// What each craft's layers were last written from, so a craft that has not moved
    /// uploads nothing.
    written: RefCell<[Option<Taps>; ENV_PROBE_LAYERS as usize / 2]>,
}

/// One probe's two images decoded to `RGBA8`.
#[derive(Debug)]
struct Decoded {
    irradiance: Vec<u8>,
    reflection: Vec<u8>,
}

/// How many decoded probes to keep: 8 crafts blend four each, and 64 probes are 2.5 MB.
const CACHE_LIMIT: usize = 64;

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
            decoded: RefCell::default(),
            written: RefCell::new(Default::default()),
        }
    }

    /// The array, for `ShadowMaps::env_probes`.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    fn decoded(&self, probe: usize) -> Arc<Decoded> {
        let mut cache = self.decoded.borrow_mut();
        if cache.len() >= CACHE_LIMIT && !cache.contains_key(&probe) {
            cache.clear();
        }
        Arc::clone(cache.entry(probe).or_insert_with(|| {
            Arc::new(Decoded {
                irradiance: self.probes.irradiance(probe),
                reflection: self.probes.reflection(probe),
            })
        }))
    }

    /// Blends the probes at `position` into the craft's two layers; `craft` is its grid slot.
    ///
    /// The irradiance map goes into the corner of its layer at its own 64x32 size and the
    /// shader addresses it there, so nothing is resized.
    pub fn write(&self, queue: &dyn Writes, craft: usize, position: [f32; 3]) {
        if craft * 2 + 1 >= ENV_PROBE_LAYERS as usize {
            return;
        }
        let taps = self.probes.locate(position);
        if self.written.borrow()[craft].as_ref() == Some(&taps) {
            return;
        }
        let used: Vec<(Arc<Decoded>, f32)> = taps
            .probes
            .iter()
            .zip(&taps.weights)
            .filter(|(_, weight)| **weight > 0.0)
            .map(|(&probe, &weight)| (self.decoded(probe), weight))
            .collect();
        let irradiance: Vec<(&[u8], f32)> = used
            .iter()
            .map(|(d, w)| (d.irradiance.as_slice(), *w))
            .collect();
        let reflection: Vec<(&[u8], f32)> = used
            .iter()
            .map(|(d, w)| (d.reflection.as_slice(), *w))
            .collect();
        self.upload(queue, craft * 2, &probes::mix(&irradiance), IRRADIANCE_SIZE);
        self.upload(
            queue,
            craft * 2 + 1,
            &probes::mix(&reflection),
            REFLECTION_SIZE,
        );
        self.written.borrow_mut()[craft] = Some(taps);
    }

    fn upload(&self, queue: &dyn Writes, layer: usize, texels: &[u8], size: (usize, usize)) {
        if texels.is_empty() {
            return;
        }
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
                bytes_per_row: Some(size.0 as u32 * 4),
                rows_per_image: Some(size.1 as u32),
            },
            wgpu::Extent3d {
                width: size.0 as u32,
                height: size.1 as u32,
                depth_or_array_layers: 1,
            },
        );
    }
}
