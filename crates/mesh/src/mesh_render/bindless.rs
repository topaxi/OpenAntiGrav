//! Bind group 1 as binding arrays: one group per drawable instead of one per
//! material slot, for a device that has [`FEATURES`].
//!
//! Every draw of a drawable used to switch group 1 to its slot's group, and wgpu
//! validates and tracks each switch when the encoder finishes: 55 to 65% of a
//! view's `encoder.finish`. Here a drawable binds one group: every **distinct**
//! texture its slots name in one array, its distinct samplers in another, and a
//! table (a storage buffer) giving each slot the five indices its group named. A draw
//! names its slot by its first instance, and the shader (`shaders/bindings.wesl`, the
//! `bindless` feature) reads the slot's row and samples the same texture with the same
//! sampler the slot's group bound, so the pictures are the same by construction.
//!
//! **Distinct, and only the elements used.** wgpu walks one texture-initialisation
//! check per array element each time the group is (re)bound in a pass, so an array of
//! one element per slot and role (HD's Talon's Junction: 443 slots, four textures each)
//! or one padded to a fixed length cost `finish` more than the switches it replaced
//! (measured: Pulse's second view 288 to 440 us padded to 1,024, 318 to 216 padded to
//! 256). So the arrays hold each texture once, the layout's length is a ceiling and a
//! group supplies only what it uses, which is what [`wgpu::Features::PARTIALLY_BOUND_BINDING_ARRAY`]
//! allows; and one fixed layout keeps a pipeline [`super::pipeline_cache`] built for one
//! drawable valid for another's group.
//!
//! **The samplers are a small array too.** A drawable binds one sampler for every slot
//! (`build`'s repeat sampler), and the only other a mesh draw binds is the billboard
//! card's clamp sampler ([`super::card_material`]), which an advert slot swaps in:
//! two descriptors a race, one or two a drawable.
//!
//! Native only: WebGPU has no binding arrays, and a device without [`FEATURES`] keeps
//! the per-slot groups and the default shader, byte for byte.

/// The three device features the variant needs: texture and sampler arrays, indexed by
/// a value the shader cannot prove uniform (a flat varying), and bound shorter than the
/// layout's length.
pub const FEATURES: wgpu::Features = wgpu::Features::TEXTURE_BINDING_ARRAY
    .union(wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING)
    .union(wgpu::Features::PARTIALLY_BOUND_BINDING_ARRAY);

/// The texture array's length: the most distinct textures one drawable may bind. A
/// drawable with more keeps the per-slot groups (and says so in the log).
///
/// Chosen, not measured, with room over the circuits counted: material slots (the white
/// fallback included) on each title's default are 2048's Altima 524, Omega's Tech de Ra
/// 462, HD's Talon's Junction 443 and Zone 18, Pure's Vineta K 189, Pulse's `16_Track`
/// 136, and the distinct textures are fewer.
pub const CAPACITY: u32 = 1024;

/// The sampler array's length.
pub const SAMPLERS: u32 = 16;

/// The `u32`s of one slot's row in the table: `MaterialSlot` in `shaders/bindings.wesl`.
const ROW: usize = 8;

/// Whether `device` can draw the variant: the features, and limits that hold both arrays.
#[must_use]
pub fn supported(device: &wgpu::Device) -> bool {
    let limits = device.limits();
    device.features().contains(FEATURES)
        && limits.max_binding_array_elements_per_shader_stage >= CAPACITY + SAMPLERS
        && limits.max_binding_array_sampler_elements_per_shader_stage >= SAMPLERS
}

/// Whether `model`'s slots fit the arrays: its distinct decoded textures, plus the three
/// 1x1 fallbacks `build` shares between the slots that lack one, within [`CAPACITY`]. An
/// upper bound on what [`Bindless::new`] places (a texture that does not upload becomes a
/// fallback), so a model this passes always builds its group.
#[must_use]
pub fn fits(model: &crate::mesh::Model) -> bool {
    let textures: std::collections::HashSet<_> = [
        &model.textures,
        &model.lightmaps,
        &model.pad_masks,
        &model.wave_maps,
    ]
    .into_iter()
    .flatten()
    .flatten()
    .map(std::sync::Arc::as_ptr)
    .collect();
    let fits = textures.len() + 3 <= CAPACITY as usize;
    if !fits {
        log::info!(
            "{}: {} distinct textures, over the bindless array's {CAPACITY}: per-slot groups",
            model.label,
            textures.len()
        );
    }
    fits
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
            .min(CAPACITY + SAMPLERS),
        max_binding_array_sampler_elements_per_shader_stage: own
            .max_binding_array_sampler_elements_per_shader_stage
            .min(SAMPLERS),
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

/// The variant's group 1 layout: the texture array, the sampler array and the table.
#[must_use]
pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("material arrays"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: std::num::NonZeroU32::new(CAPACITY),
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: std::num::NonZeroU32::new(SAMPLERS),
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new((ROW * 4) as u64),
                },
                count: None,
            },
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
    /// The group over `materials`, slot for slot, or `None` past [`CAPACITY`] distinct
    /// textures or [`SAMPLERS`] samplers (or for no slot at all, which `build` never
    /// makes: slot 0 is its white).
    #[must_use]
    pub fn new(device: &wgpu::Device, materials: Vec<Material>) -> Option<Self> {
        let Some(group) = group(device, &materials) else {
            log::info!(
                "bindless: {} material slots name more than {CAPACITY} textures or \
                 {SAMPLERS} samplers: per-slot groups",
                materials.len()
            );
            return None;
        };
        Some(Self { materials, group })
    }

    /// The group, bound once per drawable at group 1.
    #[must_use]
    pub fn group(&self) -> &wgpu::BindGroup {
        &self.group
    }

    /// Replaces slot `slot` with `material` and rebuilds the group. A slot this
    /// drawable does not have, or one that would take the arrays past their length,
    /// is left alone.
    pub fn set(&mut self, device: &wgpu::Device, slot: usize, material: Material) {
        if slot >= self.materials.len() {
            return;
        }
        let previous = std::mem::replace(&mut self.materials[slot], material);
        match group(device, &self.materials) {
            Some(group) => self.group = group,
            None => {
                log::warn!("bindless: slot {slot}'s new material does not fit; kept the old");
                self.materials[slot] = previous;
            }
        }
    }
}

/// The distinct items of `all`, in first-seen order, and each item's index among them.
fn distinct<T: Clone + Eq + std::hash::Hash>(all: impl Iterator<Item = T>) -> (Vec<T>, Vec<u32>) {
    let mut seen = std::collections::HashMap::new();
    let mut items = Vec::new();
    let indices = all
        .map(|item| {
            *seen.entry(item.clone()).or_insert_with(|| {
                items.push(item);
                (items.len() - 1) as u32
            })
        })
        .collect();
    (items, indices)
}

/// The group over `materials`: their distinct textures and samplers, and the table of
/// each slot's indices into them, `[albedo, lightmap, pad_mask, wave, sampler, 0, 0, 0]`.
fn group(device: &wgpu::Device, materials: &[Material]) -> Option<wgpu::BindGroup> {
    let (views, view_index) = distinct(
        materials
            .iter()
            .flat_map(|m| [&m.albedo, &m.lightmap, &m.pad_mask, &m.wave]),
    );
    let (samplers, sampler_index) = distinct(materials.iter().map(|m| &m.sampler));
    log::debug!(
        "bindless: {} slots, {} textures, {} samplers",
        materials.len(),
        views.len(),
        samplers.len()
    );
    if views.is_empty() || views.len() > CAPACITY as usize || samplers.len() > SAMPLERS as usize
    {
        return None;
    }
    let table: Vec<u32> = view_index
        .chunks_exact(4)
        .zip(&sampler_index)
        .flat_map(|(roles, &sampler)| {
            let mut row = [0; ROW];
            row[..4].copy_from_slice(roles);
            row[4] = sampler;
            row
        })
        .collect();
    let table_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("material slots"),
        size: std::mem::size_of_val(&table[..]) as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: true,
    });
    table_buffer
        .slice(..)
        .get_mapped_range_mut()
        .expect("mapped at creation")
        .copy_from_slice(bytemuck::cast_slice(&table));
    table_buffer.unmap();
    Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("material arrays"),
        layout: &layout(device),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureViewArray(&views),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::SamplerArray(&samplers),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: table_buffer.as_entire_binding(),
            },
        ],
    }))
}
