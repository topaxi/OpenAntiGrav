//! Bind group 1's layout, split out of `mesh_render.rs` under the 1,000-line rule.

/// Bind group 1's layout: a material's albedo, the sampler every texture shares,
/// its lightmap, and the third to sixth pictures - what every mesh pipeline binds
/// per material slot.
///
/// A function rather than an inline descriptor because a second pipeline
/// draws through the same bind groups: the sun-occlusion pass
/// (`oag_render::shadow::occlusion`) takes a track drawable's own material bind
/// groups and needs a layout equal to the one they were built against. Two
/// layouts built from one descriptor are equal in wgpu's eyes; two written
/// out by hand drift.
#[must_use]
pub fn material_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("albedo"),
        entries: &entries(),
    })
}

/// [`material_bind_group_layout`]'s entries.
pub(super) fn entries() -> [wgpu::BindGroupLayoutEntry; 7] {
    [
        texture(0),
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        },
        // The lightmap shares the albedo's sampler: it is the same filtering on
        // the same kind of texture, and a second sampler would be a second thing
        // to keep in step for no difference in the picture.
        texture(2),
        // HD's pad mask, `_ne`, and whatever else a family binds third: sampled
        // only where its flag is set, a flat placeholder everywhere else.
        texture(3),
        // HD's magstrip wave, and whatever else a family binds fourth.
        texture(4),
        // A material's fifth and sixth pictures, for the HD programs that sample
        // more than four units (`Model::fifth_maps`, `Model::sixth_maps`); a
        // white 1x1 everywhere else.
        texture(5),
        texture(6),
    ]
}

/// A fragment-stage 2D float texture at `binding`.
fn texture(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    /// Sampled textures the fragment stage of the per-slot mesh pipeline binds: group 1's
    /// six and group 2's ten (`zone`'s two layered arrays and its visualiser lookup, `shadow_map`'s seven).
    fn fragment_sampled_textures() -> usize {
        super::entries()
            .iter()
            .chain(&super::super::zone::layout_entries())
            .chain(&super::super::shadow_map::layout_entries())
            .filter(|e| {
                e.visibility.contains(wgpu::ShaderStages::FRAGMENT)
                    && matches!(e.ty, wgpu::BindingType::Texture { .. })
            })
            .map(|e| e.count.map_or(1, std::num::NonZeroU32::get) as usize)
            .sum()
    }

    /// **The per-slot layout fits WebGPU's guaranteed 16 sampled textures per stage**, which
    /// is also wgpu's default limit and GL's fixed one: a layout over it fails validation on
    /// such an adapter and no mesh draws at all. The six material textures fit only because
    /// the Zone stage's Inner and Outer sets are one 2-layer array each (`zone::layered`).
    #[test]
    fn the_fragment_stage_binds_no_more_sampled_textures_than_the_default_limit() {
        let bound = fragment_sampled_textures();
        assert_eq!(bound, 16, "group 1's six, group 2's ten");
        assert!(
            bound as u32 <= wgpu::Limits::default().max_sampled_textures_per_shader_stage,
            "{bound} sampled textures"
        );
    }
}
