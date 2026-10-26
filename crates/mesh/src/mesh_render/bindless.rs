//! Bind group 1 as binding arrays: one group per drawable instead of one per
//! material slot, for a device that has [`FEATURES`].
//!
//! Every draw of a drawable used to switch group 1 to its slot's group, and wgpu
//! validates and tracks each switch when the encoder finishes: 55 to 65% of a
//! view's `encoder.finish`. Here a drawable binds one group: every **distinct**
//! texture its slots name in one array and its distinct samplers in another. Each slot
//! has a row of the seven indices its group named ([`Bindless::row`]), which a draw
//! pushes as immediates when its slot changes, and the shader (`shaders/bindings.wesl`,
//! the `bindless` feature) samples the same texture with the same sampler the slot's
//! group bound, so the pictures are the same by construction.
//!
//! **Immediates, so the index is uniform.** A slot index carried from the vertex stage
//! (the first instance, as a flat varying) is not provably uniform, so every array read
//! through it took the non-uniform path: on the Arrow Lake iGPU (Mesa ANV, which has no
//! native non-uniform indexing) the pipelined two-view frame rose 6.5% on HD and 16% on
//! Pure, and NVIDIA's scene passes about 15% on Pulse and Pure. An immediate is uniform:
//! naga emits no `NonUniform` decoration for it (four for the varying, in a probe of the
//! same reads), and the variant does not ask for non-uniform indexing at all.
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
//! **Only the slots a draw names** ([`used_slots`]). A drawable built over a circuit's
//! whole material table (HD's speed and weapon pads) bound all 269 of Anulpha Pass's
//! textures to draw a few pads; keeping the slots its own draws reach took the elements
//! bound per frame there from 1,370 to 842, the same picture byte for byte.
//!
//! **The samplers are a small array too.** A drawable binds one sampler for every slot
//! (`build`'s repeat sampler), and the only other a mesh draw binds is the billboard
//! card's sampler ([`super::card_material`]), which an advert slot swaps in:
//! two descriptors a race, one or two a drawable.
//!
//! Native only: WebGPU has no binding arrays, and a device without [`FEATURES`] keeps
//! the per-slot groups and the default shader, byte for byte.

/// The three device features the variant needs: texture and sampler arrays, bound
/// shorter than the layout's length, and indexed by immediates.
pub const FEATURES: wgpu::Features = wgpu::Features::TEXTURE_BINDING_ARRAY
    .union(wgpu::Features::PARTIALLY_BOUND_BINDING_ARRAY)
    .union(wgpu::Features::IMMEDIATES);

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

/// One slot's row, `MaterialSlot` in `shaders/bindings.wesl`: indices into the texture
/// array of its albedo, lightmap, pad mask and wave map, then into the sampler array,
/// then into the texture array of its fifth and sixth pictures.
pub type Row = [u32; 7];

/// The bytes of immediates the variant's pipelines declare: one [`Row`].
pub const IMMEDIATE_SIZE: u32 = std::mem::size_of::<Row>() as u32;

/// Whether `device` can draw the variant: the features, and limits that hold both arrays.
#[must_use]
pub fn supported(device: &wgpu::Device) -> bool {
    let limits = device.limits();
    device.features().contains(FEATURES)
        && limits.max_binding_array_elements_per_shader_stage >= CAPACITY + SAMPLERS
        && limits.max_binding_array_sampler_elements_per_shader_stage >= SAMPLERS
        && limits.max_immediate_size >= IMMEDIATE_SIZE
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
        &model.fifth_maps,
        &model.sixth_maps,
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
        max_immediate_size: own.max_immediate_size.min(IMMEDIATE_SIZE),
        ..limits
    }
}

/// What one material slot binds: group 1's seven resources.
#[derive(Clone, Debug)]
pub struct Material {
    pub albedo: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub lightmap: wgpu::TextureView,
    pub pad_mask: wgpu::TextureView,
    pub wave: wgpu::TextureView,
    pub fifth: wgpu::TextureView,
    pub sixth: wgpu::TextureView,
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
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&self.fifth),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&self.sixth),
                },
            ],
        })
    }
}

/// The variant's group 1 layout: the texture array and the sampler array.
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
        ],
    })
}

/// A drawable's group 1 in the variant, and the per-slot materials it was built
/// from, kept so one slot can be replaced ([`Self::set`]).
#[derive(Clone, Debug)]
pub struct Bindless {
    materials: Vec<Material>,
    used: Vec<bool>,
    group: wgpu::BindGroup,
    rows: Vec<Row>,
    elements: usize,
}

impl Bindless {
    /// The group over the slots of `materials` that `used` marks (see [`used_slots`]), or
    /// `None` past [`CAPACITY`] distinct textures or [`SAMPLERS`] samplers (or for no slot
    /// at all, which `build` never makes: slot 0 is its white). A slot `used` leaves out
    /// reads slot 0's row.
    #[must_use]
    pub fn new(device: &wgpu::Device, used: Vec<bool>, materials: Vec<Material>) -> Option<Self> {
        let Some((group, rows, elements)) = group(device, &materials, &used) else {
            log::info!(
                "bindless: {} material slots name more than {CAPACITY} textures or \
                 {SAMPLERS} samplers: per-slot groups",
                materials.len()
            );
            return None;
        };
        Some(Self {
            materials,
            used,
            group,
            rows,
            elements,
        })
    }

    /// The group, bound once per drawable at group 1.
    #[must_use]
    pub fn group(&self) -> &wgpu::BindGroup {
        oag_gpu::perfprobe::array_bind(self.elements);
        &self.group
    }

    /// Slot `slot`'s row as the immediates a draw of it pushes, at offset 0. Callers
    /// clamp the slot to the drawable's own first, as for the per-slot groups; one past
    /// them anyway reads slot 0, the white fallback.
    #[must_use]
    pub fn row(&self, slot: usize) -> &[u8] {
        bytemuck::cast_slice(self.rows.get(slot).unwrap_or(&self.rows[0]))
    }

    /// Replaces slot `slot` with `material` and rebuilds the group. A slot this
    /// drawable does not have, or one that would take the arrays past their length,
    /// is left alone.
    pub fn set(&mut self, device: &wgpu::Device, slot: usize, material: Material) {
        if slot >= self.materials.len() {
            return;
        }
        let previous = std::mem::replace(&mut self.materials[slot], material);
        match group(device, &self.materials, &self.used) {
            Some((group, rows, elements)) => {
                (self.group, self.rows, self.elements) = (group, rows, elements);
            }
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

/// Which of a model's `slots` material slots a draw can bind: slot 0 (the white
/// fallback, and the row [`Bindless::row`] falls back to) and each slot a draw of
/// any of the model's lists names, clamped as the drawing side clamps it.
///
/// **Only these go into the arrays.** wgpu tracks every element of the group each
/// time it is bound in a pass, whether a draw reads it or not, and a drawable built
/// over a circuit's whole material table draws a few of its slots: HD's Anulpha
/// Pass builds its speed pads and its weapon pads that way, and each bound the
/// circuit's 269 distinct textures to draw a handful of pads.
#[must_use]
pub fn used_slots(model: &crate::mesh::Model, slots: usize) -> Vec<bool> {
    let mut used = vec![false; slots];
    if let Some(white) = used.first_mut() {
        *white = true;
    }
    let last = slots.saturating_sub(1);
    for draw in model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
        .chain(&model.shine_draws)
    {
        if let Some(slot) = used.get_mut(draw.texture.map_or(0, |t| t + 1).min(last)) {
            *slot = true;
        }
    }
    used
}

/// The group over the slots of `materials` that `used` marks, their distinct textures
/// and samplers, and each slot's [`Row`] of indices into them (slot 0's for a slot
/// `used` leaves out), with the group's element count.
fn group(
    device: &wgpu::Device,
    materials: &[Material],
    used: &[bool],
) -> Option<(wgpu::BindGroup, Vec<Row>, usize)> {
    let kept: Vec<&Material> = materials
        .iter()
        .zip(used.iter().chain(std::iter::repeat(&true)))
        .filter_map(|(m, &used)| used.then_some(m))
        .collect();
    let (views, view_index) = distinct(kept.iter().flat_map(|m| {
        [
            &m.albedo,
            &m.lightmap,
            &m.pad_mask,
            &m.wave,
            &m.fifth,
            &m.sixth,
        ]
    }));
    let (samplers, sampler_index) = distinct(kept.iter().map(|m| &m.sampler));
    log::debug!(
        "bindless: {} of {} slots, {} textures, {} samplers",
        kept.len(),
        materials.len(),
        views.len(),
        samplers.len()
    );
    if views.is_empty() || views.len() > CAPACITY as usize || samplers.len() > SAMPLERS as usize {
        return None;
    }
    let mut kept_rows = view_index
        .as_chunks::<6>()
        .0
        .iter()
        .zip(&sampler_index)
        .map(
            |(&[albedo, lightmap, pad_mask, wave, fifth, sixth], &sampler)| {
                [albedo, lightmap, pad_mask, wave, sampler, fifth, sixth]
            },
        );
    // Slot 0 is always kept, so the first kept row is its row.
    let mut white = None;
    let mut rows = Vec::with_capacity(materials.len());
    for &used in used
        .iter()
        .chain(std::iter::repeat(&true))
        .take(materials.len())
    {
        let row = if used { kept_rows.next() } else { white };
        let row = row?;
        white.get_or_insert(row);
        rows.push(row);
    }
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        ],
    });
    Some((group, rows, views.len() + samplers.len()))
}

#[cfg(test)]
mod tests {
    use super::used_slots;

    /// A drawable over a circuit's whole material table keeps only white and the slots its
    /// own draws name, in every list, clamped as the drawing side clamps them.
    #[test]
    fn the_arrays_keep_only_the_slots_a_draw_names() {
        let mut model = super::super::pipeline_cache::tests::triangle_model();
        let draw = model.draws[0].clone();
        let with = |texture| crate::mesh::DrawCall {
            texture,
            ..draw.clone()
        };
        model.draws = vec![with(Some(2)), with(None)];
        model.alpha_tested_draws = vec![with(Some(4))];
        model.transparent_draws = vec![with(Some(40))];
        let used = used_slots(&model, 8);
        assert_eq!(
            used,
            [true, false, false, true, false, true, false, true],
            "white, texture 2 as slot 3, texture 4 as slot 5, and 40 clamped to the last"
        );
    }
}
