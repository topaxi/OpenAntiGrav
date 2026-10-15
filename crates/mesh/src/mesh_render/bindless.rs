//! Bind group 1 as binding arrays: one group per drawable instead of one per
//! material slot, for a device that has [`FEATURES`].
//!
//! Every draw of a drawable used to switch group 1 to its slot's group, and wgpu
//! validates and tracks each switch when the encoder finishes: 55 to 65% of a
//! view's `encoder.finish`. Here the five bindings of [`super::material_bind_group_layout`]
//! become arrays with one element per slot, element `n` holding exactly what slot
//! `n`'s group binds, so a draw names its slot by its first instance and the shader
//! (`shaders/bindings.wesl`, the `bindless` feature) indexes every array by it. The
//! pictures are the same by construction: the same texture and sampler per draw.
//!
//! **The samplers are an array too, indexed by the same slot**, not a deduplicated
//! set. A drawable binds one sampler for every slot (`build`'s repeat sampler), and
//! the only other one a mesh draw binds is the billboard card's clamp sampler
//! ([`super::albedo_bind_group`]), which an advert slot swaps in: two descriptors a
//! race. A deduplicated set would need a per-slot index table, and wgpu refuses a
//! uniform buffer in a group holding a binding array (Vulkan's update-after-bind), so
//! the table would be a storage buffer read per fragment to save nothing measurable.
//!
//! Native only: WebGPU has no binding arrays, and a device without [`FEATURES`] keeps
//! the per-slot groups and the default shader, byte for byte.

/// The two device features the variant needs: texture and sampler arrays, indexed
/// by a value the shader cannot prove uniform (a flat varying).
pub const FEATURES: wgpu::Features = wgpu::Features::TEXTURE_BINDING_ARRAY.union(
    wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
);

/// Every array's length. **One fixed length**, so every bindless layout is equal and
/// a pipeline [`super::pipeline_cache`] keeps for one drawable binds another's group;
/// the elements past a drawable's own slots repeat slot 0. A drawable with more slots
/// than this keeps the per-slot groups (and says so in the log).
///
/// Chosen, not measured, with room over the circuits counted (slots, the white fallback
/// included) on each title's default: 2048's Altima 524, Omega's Tech de Ra 462, HD's
/// Talon's Junction 443 and Zone 18, Pure's Vineta K 189, Pulse's `16_Track` 136. A
/// drawable past it draws exactly as before.
pub const CAPACITY: u32 = 1024;

/// Array elements of the five bindings together, which wgpu counts against
/// `max_binding_array_elements_per_shader_stage`.
const ELEMENTS: u32 = 5 * CAPACITY;

/// Whether `device` can draw the variant: both features, and limits that hold
/// [`CAPACITY`] in every array.
#[must_use]
pub fn supported(device: &wgpu::Device) -> bool {
    let limits = device.limits();
    device.features().contains(FEATURES)
        && limits.max_binding_array_elements_per_shader_stage >= ELEMENTS
        && limits.max_binding_array_sampler_elements_per_shader_stage >= CAPACITY
}

/// Whether a drawable built on `device` uses the variant: [`supported`], unless
/// `OAG_BINDLESS=0` asks for the per-slot groups (an A/B switch for benches and the
/// pictures' comparison; nothing a player sets).
#[must_use]
pub fn wanted(device: &wgpu::Device) -> bool {
    supported(device) && std::env::var("OAG_BINDLESS").map_or(true, |v| v != "0")
}

/// The limits the variant needs, raised from `limits` to what `adapter` has.
/// Applied to every device this renderer requests: a limit no higher than the
/// adapter's never fails a request, and it is zero (the default) on an adapter
/// without binding arrays.
#[must_use]
pub fn with_limits(limits: wgpu::Limits, adapter: &wgpu::Adapter) -> wgpu::Limits {
    let own = adapter.limits();
    wgpu::Limits {
        max_binding_array_elements_per_shader_stage: own
            .max_binding_array_elements_per_shader_stage
            .min(ELEMENTS),
        max_binding_array_sampler_elements_per_shader_stage: own
            .max_binding_array_sampler_elements_per_shader_stage
            .min(CAPACITY),
        ..limits
    }
}

/// What one material slot binds: group 1's five resources.
#[derive(Clone, Debug)]
pub struct Material {
    pub albedo: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub lightmap: wgpu::TextureView,
    pub pad_mask: wgpu::TextureView,
    pub wave: wgpu::TextureView,
}

impl Material {
    /// This slot's own group against `layout`, [`super::material_bind_group_layout`]:
    /// the per-draw path, and the one the sun-occlusion pass and the ghost bind.
    #[must_use]
    pub fn bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        label: &str,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.albedo),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.lightmap),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.pad_mask),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.wave),
                },
            ],
        })
    }
}

/// The variant's group 1 layout: [`super::material_bind_group_layout`]'s five
/// bindings, each an array of [`CAPACITY`].
#[must_use]
pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let count = std::num::NonZeroU32::new(CAPACITY);
    let texture = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("albedo arrays"),
        entries: &[
            texture(0),
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count,
            },
            texture(2),
            texture(3),
            texture(4),
        ],
    })
}

/// A drawable's group 1 in the variant, and the per-slot materials it was built
/// from, kept so one slot can be replaced ([`Self::set`]).
#[derive(Clone, Debug)]
pub struct Bindless {
    materials: Vec<Material>,
    group: wgpu::BindGroup,
}

impl Bindless {
    /// The group over `materials`, slot for slot, or `None` past [`CAPACITY`]
    /// (or for no slot at all, which `build` never makes: slot 0 is its white).
    #[must_use]
    pub fn new(device: &wgpu::Device, materials: Vec<Material>) -> Option<Self> {
        if materials.is_empty() || materials.len() > CAPACITY as usize {
            return None;
        }
        let group = group(device, &materials);
        Some(Self { materials, group })
    }

    /// The group, bound once per drawable at group 1.
    #[must_use]
    pub fn group(&self) -> &wgpu::BindGroup {
        &self.group
    }

    /// How many slots are this drawable's own (the rest repeat slot 0).
    #[must_use]
    pub fn slots(&self) -> usize {
        self.materials.len()
    }

    /// Replaces slot `slot` with `material` and rebuilds the group. A slot this
    /// drawable does not have is left alone.
    pub fn set(&mut self, device: &wgpu::Device, slot: usize, material: Material) {
        let Some(current) = self.materials.get_mut(slot) else {
            return;
        };
        *current = material;
        self.group = group(device, &self.materials);
    }
}

fn group(device: &wgpu::Device, materials: &[Material]) -> wgpu::BindGroup {
    let at = |index: usize| materials.get(index).unwrap_or(&materials[0]);
    let all = 0..CAPACITY as usize;
    let views = |pick: fn(&Material) -> &wgpu::TextureView| -> Vec<&wgpu::TextureView> {
        all.clone().map(|index| pick(at(index))).collect()
    };
    let albedo = views(|m| &m.albedo);
    let lightmap = views(|m| &m.lightmap);
    let pad_mask = views(|m| &m.pad_mask);
    let wave = views(|m| &m.wave);
    let samplers: Vec<&wgpu::Sampler> = all.clone().map(|index| &at(index).sampler).collect();
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("albedo arrays"),
        layout: &layout(device),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureViewArray(&albedo),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::SamplerArray(&samplers),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureViewArray(&lightmap),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureViewArray(&pad_mask),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureViewArray(&wave),
            },
        ],
    })
}
