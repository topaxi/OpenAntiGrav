//! Which storage formats the chain's intermediates are held in on this device, and whether its
//! arithmetic runs in halves where upstream's does.
//!
//! Upstream holds its scalars in `R16_FLOAT` / `R8_UNORM` and its pairs in `RG16_FLOAT`. WebGPU
//! guarantees none of the three a storage binding, so [`Formats::BASELINE`] widens them
//! (`resources.rs`). A native adapter usually has them anyway, and wgpu validates against the
//! adapter's own flags once the device was created with
//! `Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` (`oag_mesh::mesh_render::optional_features`
//! asks for it where the adapter has it). [`Formats::probe`] asks the device itself, rather than
//! reading a feature name: a device with the feature and an adapter without storage on
//! `R16Float` gets the baseline.
//!
//! Each accessor names the intermediate's role, so the layouts, the targets and the shader
//! artifact (`narrow` feature in `build.rs`) cannot disagree about a format: every one of them
//! reads it from here.

use wgpu::TextureFormat;

/// A shader that stores to each of upstream's three storage formats.
const PROBE: &str = "
@group(0) @binding(0) var a: texture_storage_2d<r16float, write>;
@group(0) @binding(1) var b: texture_storage_2d<rg16float, write>;
@group(0) @binding(2) var c: texture_storage_2d<r8unorm, write>;
@compute @workgroup_size(1) fn main() {
    textureStore(a, vec2<i32>(0, 0), vec4<f32>(1.0));
    textureStore(b, vec2<i32>(0, 0), vec4<f32>(1.0));
    textureStore(c, vec2<i32>(0, 0), vec4<f32>(1.0));
}";

/// The format set a chain was built with. Plain data; `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Formats {
    narrow: bool,
    half: bool,
}

impl Formats {
    /// Widened to what every WebGPU device can store: see `resources.rs`.
    pub const BASELINE: Self = Self {
        narrow: false,
        half: false,
    };

    /// Upstream's own formats: `R16Float`, `Rg16Float`, `R8Unorm`.
    pub const NARROW: Self = Self {
        narrow: true,
        half: false,
    };

    /// This set with the passes upstream computes in `FFX_MIN16` run in `f16`.
    #[must_use]
    pub const fn with_half(self) -> Self {
        Self { half: true, ..self }
    }

    /// The formats a texture needs storage support in to be narrowed.
    pub(super) const NEEDED: [TextureFormat; 3] = [
        TextureFormat::R16Float,
        TextureFormat::Rg16Float,
        TextureFormat::R8Unorm,
    ];

    /// What `device` can do, asked of the device: [`Self::NARROW`] when a storage texture can be
    /// created in all three of upstream's formats, [`Self::BASELINE`] otherwise.
    ///
    /// `OAG_FSR3_FORMATS=baseline` forces the baseline formats and `OAG_FSR3_HALF=off` the `f32`
    /// arithmetic, for an A/B on one machine. Half arithmetic needs `SHADER_F16` on the device
    /// (`optional_features` asks for it where the adapter has it).
    #[must_use]
    pub fn probe(device: &wgpu::Device) -> Self {
        let mut formats = Self::probe_narrow(device);
        formats.half = device.features().contains(wgpu::Features::SHADER_F16)
            && std::env::var("OAG_FSR3_HALF").map_or(true, |v| v != "off");
        formats
    }

    fn probe_narrow(device: &wgpu::Device) -> Self {
        if std::env::var("OAG_FSR3_FORMATS").is_ok_and(|v| v == "baseline") {
            return Self::BASELINE;
        }
        if !device
            .features()
            .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
        {
            return Self::BASELINE;
        }
        let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        for format in Self::NEEDED {
            let _ = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("fsr3 format probe"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
        }
        // And the driver compiling a shader that stores to all three: a texture that can be
        // created says nothing about the shader capability the SPIR-V path declares for them,
        // and a phone is where that would differ from the desktop.
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fsr3 format probe"),
            source: wgpu::ShaderSource::Wgsl(PROBE.into()),
        });
        let _ = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("fsr3 format probe"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let invalid = pollster::block_on(scope.pop());
        let failed = pollster::block_on(internal.pop());
        if invalid.is_none() && failed.is_none() {
            Self::NARROW
        } else {
            Self::BASELINE
        }
    }

    /// Whether the intermediates are held in upstream's formats.
    #[must_use]
    pub fn is_narrow(self) -> bool {
        self.narrow
    }

    /// Whether the passes upstream computes in halves run in `f16` here.
    #[must_use]
    pub fn is_half(self) -> bool {
        self.half
    }

    /// A short name for the log line.
    #[must_use]
    pub fn name(self) -> &'static str {
        match (self.narrow, self.half) {
            (false, false) => "baseline",
            (true, false) => "narrow",
            (false, true) => "baseline, f16",
            (true, true) => "narrow, f16",
        }
    }

    /// `RG16_FLOAT` upstream: the dilated motion vectors and the shading-change pyramid.
    #[must_use]
    pub(super) fn pair(self) -> TextureFormat {
        if self.narrow {
            TextureFormat::Rg16Float
        } else {
            TextureFormat::Rgba16Float
        }
    }

    /// `R16_FLOAT` that is only loaded: the farthest depth, in metres clamped to the half range.
    #[must_use]
    pub(super) fn scalar(self) -> TextureFormat {
        if self.narrow {
            TextureFormat::R16Float
        } else {
            TextureFormat::R32Float
        }
    }

    /// `R16_FLOAT` that some pass reads through the linear sampler: luma, its instability, and
    /// the half-resolution farthest depth.
    #[must_use]
    pub(super) fn filterable(self) -> TextureFormat {
        if self.narrow {
            TextureFormat::R16Float
        } else {
            TextureFormat::Rgba16Float
        }
    }

    /// `R8_UNORM` upstream: accumulation, shading change and the new locks.
    #[must_use]
    pub(super) fn unorm(self) -> TextureFormat {
        if self.narrow {
            TextureFormat::R8Unorm
        } else {
            TextureFormat::Rgba8Unorm
        }
    }

    /// The raw depth the nearest sample dilates to, **`R32Float` in both sets**. Upstream holds
    /// it in `R16_FLOAT` because its projection is reversed-Z; this renderer's is not, so depth
    /// crowds toward `1.0` where a half float resolves only 2^-11, and the disocclusion test
    /// compares exactly these values.
    #[must_use]
    pub(super) fn depth(self) -> TextureFormat {
        TextureFormat::R32Float
    }
}
