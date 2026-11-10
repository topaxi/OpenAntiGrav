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

use oag_gpu::writes::Writes;
use oag_mesh::mesh_render::{ENV_PROBE_FORMAT, ENV_PROBE_LAYERS, ENV_PROBE_SIZE};
use oag_render::probe_blend::{self, Job, ProbeBlend};
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
    /// What scenery glass multiplies the sky reflection map by: `(1, 1, 1, 0)`, read live on RPCS3
    /// off Moa Therma's `dc_windows_a` (the lightmapped variant, `MUL H1.xyz, H1, {1, 1, 1, 0}` after
    /// the luma `DP3`), where the hulls beside it in the same frame read the engine's `(4, 4, 4, 0)`.
    /// One circuit and one family measured; whether the value follows the draw's class or the
    /// circuit is not read, and a Zone race's is not read at all, so every race takes this one.
    pub(crate) const SCENERY: Self = Self {
        gain: 1.0,
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
    blend: Blend,
    /// What each craft's layers were last written from, so a craft that has not moved
    /// blends nothing.
    written: RefCell<[Option<Taps>; CRAFTS]>,
}

/// How many crafts the array has layers for.
const CRAFTS: usize = ENV_PROBE_LAYERS as usize / 2;

/// Set to `cpu` to blend on the frame thread, as a picture to hold the GPU blend against.
const BLEND_VAR: &str = "OAG_PROBE_BLEND";

/// Where the blend runs. Both write the same bytes into the same array layout
/// (`oag_render::probe_blend`'s tests), so a hull cannot tell them apart.
#[derive(Debug)]
enum Blend {
    /// One compute pass a frame over probes uploaded once each.
    Gpu(Gpu),
    /// [`probes::mix_into`] on the frame thread and an upload per craft: asked for by
    /// [`BLEND_VAR`], and what wgpu's GL backend gets (best effort there; its storage
    /// textures and reinterpreted views are not ones this pass was run on).
    Cpu(Cpu),
}

#[derive(Debug)]
struct Gpu {
    pass: ProbeBlend,
    /// Which probe each slot of the pass's source array holds.
    slots: RefCell<Kept<()>>,
    /// The crafts to blend at the next [`EnvProbes::flush`].
    jobs: RefCell<Vec<Job>>,
}

#[derive(Debug)]
struct Cpu {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    /// Decoded probes: a craft blends the same few every frame, and decoding a probe costs
    /// more than blending it.
    decoded: RefCell<Kept<Decoded>>,
    /// [`probes::mix_into`]'s accumulator and its output, kept so a blend allocates nothing.
    scratch: RefCell<(Vec<f32>, Vec<u8>)>,
}

/// One probe's two images decoded to `RGBA8`.
#[derive(Debug)]
struct Decoded {
    irradiance: Vec<u8>,
    reflection: Vec<u8>,
}

impl Decoded {
    fn of(probes: &Probes, probe: usize) -> Self {
        Self {
            irradiance: probes.irradiance(probe),
            reflection: probes.reflection(probe),
        }
    }
}

/// Up to [`probe_blend::SLOTS`] probes in numbered slots, the one used longest ago giving
/// its slot up for a new one: the field keeps moving onto new probes, and clearing the lot
/// would decode every probe in use again. Eight crafts use at most 32 in a frame, so a probe
/// a blend of this frame names is never the one that goes.
#[derive(Debug)]
struct Kept<T> {
    slots: Vec<Slot<T>>,
    /// Counts uses, so a slot says how recently it was named.
    uses: u64,
}

#[derive(Debug)]
struct Slot<T> {
    probe: usize,
    used: u64,
    value: T,
}

impl<T> Default for Kept<T> {
    fn default() -> Self {
        Self {
            slots: Vec::with_capacity(probe_blend::SLOTS as usize),
            uses: 0,
        }
    }
}

impl<T> Kept<T> {
    /// The slot `probe` is in, taking one (and calling `fill` with it) if it is in none.
    fn slot(&mut self, probe: usize, fill: impl FnOnce(usize) -> T) -> usize {
        self.uses += 1;
        let uses = self.uses;
        if let Some(at) = self.slots.iter().position(|slot| slot.probe == probe) {
            self.slots[at].used = uses;
            return at;
        }
        let at = if self.slots.len() < probe_blend::SLOTS as usize {
            self.slots.len()
        } else {
            let oldest = self
                .slots
                .iter()
                .enumerate()
                .min_by_key(|(_, slot)| slot.used);
            oldest.map_or(0, |(at, _)| at)
        };
        let slot = Slot {
            probe,
            used: uses,
            value: fill(at),
        };
        if at < self.slots.len() {
            self.slots[at] = slot;
        } else {
            self.slots.push(slot);
        }
        at
    }
}

impl EnvProbes {
    /// The array of [`ENV_PROBE_LAYERS`] layers, undefined until a craft writes its own.
    #[must_use]
    pub fn new(device: &wgpu::Device, probes: Probes) -> Self {
        let asked_cpu = std::env::var(BLEND_VAR).is_ok_and(|value| value == "cpu");
        let blend = if asked_cpu || device.adapter_info().backend == wgpu::Backend::Gl {
            log::debug!("environment probes: blended on the frame thread");
            Blend::Cpu(Cpu::new(device))
        } else {
            Blend::Gpu(Gpu {
                pass: ProbeBlend::new(device),
                slots: RefCell::default(),
                jobs: RefCell::new(Vec::with_capacity(CRAFTS)),
            })
        };
        Self {
            probes,
            blend,
            written: RefCell::new(Default::default()),
        }
    }

    /// The array, for `ShadowMaps::env_probes`.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        match &self.blend {
            Blend::Gpu(gpu) => gpu.pass.view(),
            Blend::Cpu(cpu) => &cpu.view,
        }
    }

    /// Blends the probes at `position` into the craft's two layers; `craft` is its grid slot.
    /// The layers hold the blend once [`Self::flush`] has run and its pass is submitted.
    ///
    /// The irradiance map goes into the corner of its layer at its own 64x32 size and the
    /// shader addresses it there, so nothing is resized.
    pub fn write(&self, queue: &dyn Writes, craft: usize, position: [f32; 3]) {
        if craft >= CRAFTS {
            return;
        }
        let taps = self.probes.locate(position);
        if self.written.borrow()[craft].as_ref() == Some(&taps) {
            return;
        }
        match &self.blend {
            Blend::Gpu(gpu) => gpu.queue(queue, &self.probes, craft, &taps),
            Blend::Cpu(cpu) => cpu.blend(queue, &self.probes, craft, &taps),
        }
        self.written.borrow_mut()[craft] = Some(taps);
    }

    /// Records the pass that blends every craft [`Self::write`] took since the last flush,
    /// into `encoder`: ahead of the scene pass that samples the layers, in the same
    /// submission or an earlier one.
    pub fn flush(&self, queue: &dyn Writes, encoder: &mut wgpu::CommandEncoder) {
        if let Blend::Gpu(gpu) = &self.blend {
            let mut jobs = gpu.jobs.borrow_mut();
            gpu.pass.blend(queue, encoder, &jobs);
            jobs.clear();
        }
    }
}

impl Gpu {
    fn queue(&self, queue: &dyn Writes, probes: &Probes, craft: usize, taps: &Taps) {
        let mut slots = self.slots.borrow_mut();
        let scales = taps.scales();
        let mut job = Job {
            craft: craft as u32,
            slots: [0; 4],
            scales,
        };
        for ((slot, &probe), scale) in job.slots.iter_mut().zip(&taps.probes).zip(scales) {
            if scale > 0.0 {
                *slot = slots.slot(probe, |at| {
                    let decoded = Decoded::of(probes, probe);
                    self.pass.write_source(
                        queue,
                        at as u32,
                        &decoded.irradiance,
                        &decoded.reflection,
                    );
                }) as u32;
            }
        }
        self.jobs.borrow_mut().push(job);
    }
}

impl Cpu {
    fn new(device: &wgpu::Device) -> Self {
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
            texture,
            view,
            decoded: RefCell::default(),
            scratch: RefCell::default(),
        }
    }

    fn blend(&self, queue: &dyn Writes, probes: &Probes, craft: usize, taps: &Taps) {
        let mut decoded = self.decoded.borrow_mut();
        // Every tap is in a slot before any is read: taking a slot may move another.
        let mut used = [(0usize, 0.0f32); 4];
        let mut count = 0;
        for (&probe, &weight) in taps.probes.iter().zip(&taps.weights) {
            if weight > 0.0 {
                used[count] = (probe, weight);
                count += 1;
            }
        }
        let mut at = [0usize; 4];
        for (at, &(probe, _)) in at.iter_mut().zip(&used[..count]) {
            *at = decoded.slot(probe, |_| Decoded::of(probes, probe));
        }
        let scratch = &mut *self.scratch.borrow_mut();
        let (acc, out) = (&mut scratch.0, &mut scratch.1);
        let images: [fn(&Decoded) -> &[u8]; 2] = [|d| &d.irradiance, |d| &d.reflection];
        for (which, (image, size)) in images
            .into_iter()
            .zip([IRRADIANCE_SIZE, REFLECTION_SIZE])
            .enumerate()
        {
            let mut taps = [(&[][..], 0.0f32); 4];
            for ((tap, &at), &(_, weight)) in taps.iter_mut().zip(&at).zip(&used[..count]) {
                *tap = (image(&decoded.slots[at].value), weight);
            }
            probes::mix_into(&taps[..count], acc, out);
            self.upload(queue, craft * 2 + which, out, size);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The field moves onto one new probe: only that one is filled, it takes the slot of the
    /// probe used longest ago, and the probes still in use keep theirs.
    #[test]
    fn a_full_cache_gives_up_the_slot_of_the_probe_used_longest_ago() {
        let limit = probe_blend::SLOTS as usize;
        let mut kept = Kept::default();
        for probe in 0..limit {
            assert_eq!(kept.slot(probe, |at| at), probe);
        }
        assert_eq!(kept.slot(0, |_| panic!("probe 0 is kept")), 0);
        assert_eq!(kept.slot(limit, |at| at), 1, "probe 1 was used longest ago");
        assert_eq!(kept.slots.len(), limit);
        assert_eq!(kept.slot(2, |_| panic!("probe 2 is kept")), 2);
        assert_eq!(
            kept.slot(1, |at| at),
            3,
            "probe 1 comes back into probe 3's slot"
        );
    }
}
