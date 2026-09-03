//! The binding vocabulary every FSR 3.1 pass is declared in: five kinds of
//! entry, and the one layout they all share as group 0.
//!
//! Its own file rather than the top of `fsr3.rs`, under the 1,000-line rule in
//! `scripts/check-file-size.py`. It is a real seam rather than an arbitrary
//! cut: **the binding numbers here have to agree with every `@group(1)
//! @binding(n)` in the matching WGSL**, and a pass's layout is the only place
//! that can be checked against its shader.

/// The `Fsr3Constants` uniform, the two samplers, and nothing else - group 0 of
/// every pass.
///
/// One shared layout rather than one per pass, for the reason
/// [`super::motion_blur`] gives for its own: a binding a pass does not read
/// costs nothing, and four near-identical layouts is four places for a binding
/// number to drift from the WGSL.
pub(super) fn shared_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let sampler = |binding, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Sampler(ty),
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("fsr3 shared"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            sampler(1, wgpu::SamplerBindingType::NonFiltering),
            sampler(2, wgpu::SamplerBindingType::Filtering),
        ],
    })
}

/// [`load_entry`], but multisampled when the scene is.
///
/// Only the two scene attachments ever need it: everything else this port reads
/// is a target it wrote itself, and those are never multisampled.
pub(super) fn load_multisampled_entry(
    binding: u32,
    multisampled: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled,
        },
        ..load_entry(binding)
    }
}

/// A texture read with `textureLoad`, which is what every pass here does.
///
/// `filterable: false` because the depth attachment binds through this too and
/// a depth format is unfilterable - the same entry
/// [`super::motion_blur`] needed for the same reason.
pub(super) fn load_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

/// A write-only storage texture at `binding`.
///
/// **Write-only, never `ReadWrite`.** Read-write access carries no baseline
/// guarantee, and a pass that needs to read what an earlier one wrote binds it
/// through [`load_entry`] instead - every target here carries
/// `TEXTURE_BINDING` as well as `STORAGE_BINDING` for exactly that.
pub(super) fn store_entry(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    }
}

/// A texture read with `textureSampleLevel` through the filtering sampler,
/// across its whole mip chain.
///
/// **The one binding kind here that is not `textureLoad`.** `shading_change`
/// samples the pyramid at three explicit mip levels, interpolating across each
/// level's coarser grid rather than snapping to it, so the binding has to
/// declare itself filterable - which `Rgba16Float` is, in the WebGPU baseline,
/// and which a depth format is not. Nothing depth-shaped reaches this entry.
pub(super) fn sample_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

/// A small uniform at `binding`, for the one thing that changes *between*
/// dispatches of the same frame.
///
/// The shared constants at group 0 are written once a frame; a pyramid's level
/// index is not, so it cannot live there. One buffer per level, written at
/// build time rather than per frame, because the values are `0..PYRAMID_MIPS`
/// and never move.
pub(super) fn level_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// A read-only storage buffer at `binding` - the atomic scatter target again,
/// seen by the passes that consume rather than write it.
pub(super) fn read_buffer_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// A read-write storage buffer at `binding` - the atomic scatter target, and
/// the one resource in this port that is not a texture.
pub(super) fn atomic_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
