//! A screen filter: one fullscreen pass over the *finished* frame, running a
//! WGSL fragment a preset file supplies, with last frame's output to hand.
//!
//! What a CRT or a PSP's LCD did to the picture happened to *all* of it - the
//! HUD, the menus and the front end as much as the track - so this is not one
//! more rung on `oag_game::upscale`'s scene-resolution ladder beside FXAA and
//! FSR. It reads the presentation target after the UI has composited into it,
//! and hands its output on to be graded; the game side of that is
//! `oag_game::upscale::screen`. [ADR-0053](../../../../docs/architecture/adr/0053-screen-filters-are-loadable-wgsl-after-the-composite.md)
//! is the decision.
//!
//! # A preset is a file
//!
//! A preset is one `.wgsl` file whose leading `//!` lines are a TOML document
//! naming it and its tunables, and whose body defines
//!
//! ```wgsl
//! fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32>
//! ```
//!
//! against the bindings [`screen.wgsl`](./screen.wgsl) declares. The prelude
//! is this crate's; the body is the preset's; the two are concatenated and
//! compiled as one module. A player's own preset in their config directory is
//! compiled by exactly the same path as a built-in, because *being* the same
//! path is what lets a copied built-in be tuned in place and what keeps the
//! contract honest - there is no second, wider interface the shipped presets
//! quietly use.
//!
//! The header:
//!
//! ```text
//! //! name = "PSP-3000 LCD"
//! //! description = "One sentence for the menu, optional."
//! //!
//! //! [[param]]
//! //! name = "persistence"
//! //! default = 0.12
//! //! min = 0.0
//! //! max = 0.8
//! //! step = 0.02
//! ```
//!
//! Each `[[param]]` becomes `fn param_<name>() -> f32` in the compiled module,
//! in declaration order, up to [`MAX_PARAMS`]. `min`, `max` and `step` are for
//! a future tuning row and are checked, not yet drawn.
//!
//! # A shader that will not build is a message, not a crash
//!
//! A player's file is untrusted input. It is parsed and validated with naga
//! before the device ever sees it, so a syntax error arrives as a
//! [`Preset::validate`] `Err` carrying naga's own annotated report - line,
//! column and a caret - rather than as wgpu's uncaptured-error panic. The
//! device's own pipeline construction runs inside an error scope for the
//! failures validation cannot see, a `@group(1)` the layout has no slot for
//! being one. Either way `oag_game::upscale::screen` logs it once and draws
//! the frame unfiltered.

use std::borrow::Cow;

use anyhow::{Context, Result, anyhow, bail, ensure};
use serde::Deserialize;

/// How many tunables a preset may declare: four `vec4<f32>`s of the uniform.
pub const MAX_PARAMS: usize = 16;

/// The prelude every preset is compiled after.
pub const PRELUDE: &str = include_str!("screen.wgsl");

/// One tunable a preset's header declared.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Param {
    /// A WGSL identifier: `param_<name>()` is generated from it.
    pub name: String,
    pub default: f32,
    #[serde(default)]
    pub min: f32,
    #[serde(default = "one")]
    pub max: f32,
    #[serde(default = "hundredth")]
    pub step: f32,
}

fn one() -> f32 {
    1.0
}

fn hundredth() -> f32 {
    0.01
}

/// The header, as TOML sees it.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    name: Option<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    param: Vec<Param>,
}

/// A parsed preset: its header, and the body it appends to the prelude.
#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    /// The file stem, which is what the settings file and the menu row hold.
    pub id: String,
    /// What the menu row shows. The header's `name`, or the id when it has none.
    pub name: String,
    pub description: String,
    pub params: Vec<Param>,
    /// The file after its header - the WGSL that defines `screen_filter`.
    pub body: String,
    /// Bumped by the catalogue each time the file changes on disk, so a pass
    /// built from an older revision knows to rebuild. Two presets with the same
    /// `id` and `revision` are the same shader.
    pub revision: u64,
}

impl Preset {
    /// Parses a preset file: the `//!` header, then the body.
    ///
    /// # Errors
    ///
    /// A header that is not TOML, an unknown key in it, more than
    /// [`MAX_PARAMS`] tunables, a parameter whose name is not a WGSL
    /// identifier or is declared twice, or a default outside its own
    /// `min..=max`.
    pub fn parse(id: &str, source: &str) -> Result<Self> {
        let (header, body) = split_header(source);
        let header: Header = if header.trim().is_empty() {
            Header::default()
        } else {
            toml::from_str(&header)
                .with_context(|| format!("the header of screen filter {id:?}"))?
        };
        ensure!(
            header.param.len() <= MAX_PARAMS,
            "screen filter {id:?} declares {} tunables; the uniform holds {MAX_PARAMS}",
            header.param.len()
        );
        for (index, param) in header.param.iter().enumerate() {
            ensure!(
                is_identifier(&param.name),
                "screen filter {id:?}: tunable {:?} is not a WGSL identifier",
                param.name
            );
            ensure!(
                !header.param[..index].iter().any(|p| p.name == param.name),
                "screen filter {id:?}: tunable {:?} is declared twice",
                param.name
            );
            ensure!(
                param.min <= param.default && param.default <= param.max,
                "screen filter {id:?}: tunable {:?} defaults to {} outside {}..={}",
                param.name,
                param.default,
                param.min,
                param.max
            );
        }
        Ok(Self {
            id: id.to_string(),
            name: header.name.unwrap_or_else(|| id.to_string()),
            description: header.description,
            params: header.param,
            body: body.to_string(),
            revision: 0,
        })
    }

    /// The whole module: prelude, the generated accessors, then the body.
    #[must_use]
    pub fn wgsl(&self) -> String {
        let mut out = String::with_capacity(PRELUDE.len() + self.body.len() + 512);
        out.push_str(PRELUDE);
        out.push_str("\n// Generated from the preset's header.\n");
        for (index, param) in self.params.iter().enumerate() {
            out.push_str(&format!(
                "fn param_{}() -> f32 {{ return screen.params[{}u][{}u]; }}\n",
                param.name,
                index / 4,
                index % 4
            ));
        }
        out.push_str("\n// The preset.\n");
        out.push_str(&self.body);
        out.push('\n');
        out
    }

    /// The tunables' defaults, laid out as the uniform holds them.
    #[must_use]
    pub fn defaults(&self) -> [[f32; 4]; 4] {
        let mut out = [[0.0; 4]; 4];
        for (index, param) in self.params.iter().enumerate().take(MAX_PARAMS) {
            out[index / 4][index % 4] = param.default;
        }
        out
    }

    /// Parses and validates the compiled module with naga, without a device.
    ///
    /// # Errors
    ///
    /// naga's annotated report - the source with the offending line marked -
    /// for a module that does not parse or does not validate, and a plain
    /// message for a body that defines no `screen_filter`.
    pub fn validate(&self) -> Result<()> {
        // Textual, and before the parse: the parse's own report for a missing
        // `screen_filter` points at the prelude's call site, which is the one
        // line of the module the author did not write.
        ensure!(
            self.body
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .contains("fn screen_filter("),
            "screen filter {:?} defines no `fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32>`",
            self.id
        );
        let source = self.wgsl();
        let module = naga::front::wgsl::parse_str(&source)
            .map_err(|error| anyhow!("{}", error.emit_to_string_with_path(&source, &self.id)))?;
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .map_err(|error| anyhow!("{}", error.emit_to_string_with_path(&source, &self.id)))?;
        Ok(())
    }
}

/// Splits a file into its `//!` header, with the markers stripped, and the rest.
///
/// The header is the run of lines at the top that start with `//!`; blank
/// lines inside it are kept, being TOML's own paragraph breaks. The first line
/// that is neither ends it.
fn split_header(source: &str) -> (String, &str) {
    let mut header = String::new();
    let mut consumed = 0;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("//!") {
            header.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            if !rest.ends_with('\n') {
                header.push('\n');
            }
            consumed += line.len();
        } else if trimmed.trim().is_empty() && !header.is_empty() {
            // A blank line inside the header is a TOML paragraph break. The
            // one that ends the header is swallowed too, which costs the body
            // a blank line and nothing else.
            header.push('\n');
            consumed += line.len();
        } else {
            break;
        }
    }
    (header, &source[consumed..])
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The uniform, field for field as `screen.wgsl` declares it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub output_size: [f32; 2],
    pub texel: [f32; 2],
    pub native_size: [f32; 2],
    pub frame: f32,
    pub time: f32,
    pub strength: f32,
    pub padding: [f32; 3],
    pub params: [[f32; 4]; 4],
}

/// One frame's worth of input to [`Screen::render`].
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    /// The finished frame - the presentation target, UI and all.
    pub source: &'a wgpu::TextureView,
    /// Its size, and the size the output is built at.
    pub size: (u32, u32),
    /// The grid the running title authors in. See `Screen.native_size`.
    pub native: (f32, f32),
    /// Seconds since the filter was built.
    pub time: f32,
    /// The player's strength row, `0..=1`.
    pub strength: f32,
    /// The tunables. [`Preset::defaults`] unless something overrides them.
    pub params: [[f32; 4]; 4],
}

/// The compiled pass and its two targets.
///
/// Two, because a filter reads what it drew last frame: the pass writes one
/// while binding the other as `previous`, and swaps every frame. Both are
/// cleared on a resize, which is what makes the first frame's `previous`
/// black rather than stale.
#[derive(Debug)]
pub struct Screen {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
    uniform: wgpu::Buffer,
    written: Option<Uniform>,
    targets: Option<[Target; 2]>,
    /// Which of `targets` was written last; the other is next.
    current: usize,
    /// How many frames this pass has drawn: the uniform's `frame`.
    frames: u32,
    format: wgpu::TextureFormat,
    size: (u32, u32),
}

#[derive(Debug)]
struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Screen {
    /// Compiles `preset` against a target `format`.
    ///
    /// Validates with naga first - see the module doc - and builds the pipeline
    /// inside an error scope, so nothing a preset file can contain reaches
    /// wgpu's uncaptured-error handler.
    ///
    /// # Errors
    ///
    /// The preset's own compile error, annotated, or the device's.
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        preset: &Preset,
    ) -> Result<Self> {
        preset.validate()?;
        let source = preset.wgsl();

        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("screen filter {}", preset.id)),
            source: wgpu::ShaderSource::Wgsl(Cow::Owned(source)),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("screen filter"),
            entries: &[
                super::texture_entry(0),
                super::sampler_entry(1),
                super::uniform_entry(2),
                super::texture_entry(3),
                super::sampler_entry(4),
            ],
        });
        let sampler = |label: &str, filter: wgpu::FilterMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        let linear = sampler("screen filter linear", wgpu::FilterMode::Linear);
        let nearest = sampler("screen filter nearest", wgpu::FilterMode::Nearest);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("screen filter"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        // Non-sRGB, as every pass in this module writes: the value is passed
        // on to the grade untouched, and since ADR-0020 the presentation
        // target is non-sRGB at every real call site anyway.
        let target_format = format.remove_srgb_suffix();
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("screen filter"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        if let Some(error) = pollster::block_on(scope.pop()) {
            bail!("screen filter {:?} did not build: {error}", preset.id);
        }

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen filter uniform"),
            size: size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            pipeline,
            layout,
            linear,
            nearest,
            uniform,
            written: None,
            targets: None,
            current: 0,
            frames: 0,
            format: target_format,
            size: (0, 0),
        })
    }

    /// The filtered frame, in the same space it was read in. `None` until the
    /// first [`render`](Self::render).
    #[must_use]
    pub fn output(&self) -> Option<&wgpu::TextureView> {
        self.targets
            .as_ref()
            .map(|targets| &targets[self.current].view)
    }

    /// The texture behind [`output`](Self::output), for a readback.
    #[must_use]
    pub fn output_texture(&self) -> Option<&wgpu::Texture> {
        self.targets
            .as_ref()
            .map(|targets| &targets[self.current].texture)
    }

    /// Runs the pass, resizing both targets if `frame.size` moved.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
    ) {
        self.resize(device, frame.size);
        let Some(targets) = &self.targets else { return };

        let (w, h) = (self.size.0 as f32, self.size.1 as f32);
        let wanted = Uniform {
            output_size: [w, h],
            texel: [1.0 / w, 1.0 / h],
            native_size: [frame.native.0.max(1.0), frame.native.1.max(1.0)],
            frame: self.frames as f32,
            time: frame.time,
            strength: frame.strength.clamp(0.0, 1.0),
            padding: [0.0; 3],
            params: frame.params,
        };
        if self.written != Some(wanted) {
            queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&wanted));
            self.written = Some(wanted);
        }

        let next = 1 - self.current;
        let bind_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("screen filter"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(frame.source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.linear),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&targets[self.current].view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.nearest),
                    },
                ],
            },
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("screen filter"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &targets[next].view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, Some(&bind_group), &[]);
        pass.draw(0..3, 0..1);
        drop(pass);

        self.current = next;
        self.frames = self.frames.wrapping_add(1);
    }

    fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.size == size && self.targets.is_some() {
            return;
        }
        self.size = size;
        let target = |label: &str| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            Target { texture, view }
        };
        // A fresh texture is zeroed by wgpu, so the first `previous` is black
        // and a persistence filter starts from nothing rather than from
        // whatever a smaller earlier target held.
        self.targets = Some([target("screen filter a"), target("screen filter b")]);
        self.current = 0;
        self.frames = 0;
    }
}

#[cfg(test)]
mod tests;
