//! Builds the mesh pipeline with MSAA on a real device, draws a triangle into
//! a multisampled attachment and resolves it, then reads the resolved pixels
//! back.
//!
//! `#[ignore]`d for the same reason `exhaust_pipeline.rs` is - it needs a GPU
//! adapter. Run it by hand:
//!
//! ```sh
//! cargo nextest run -p oag-render --run-ignored all -E 'test(msaa_resolve)' --no-capture
//! ```
//!
//! `[graphics] anti_aliasing`'s MSAA levels were, until this test existed,
//! verified only by type-checked compilation: every pipeline in a scene
//! derives its `sample_count` from the one setting, so a mismatch between the
//! colour attachment, the depth attachment and the pipelines themselves would
//! be caught. That does not cover whether the *format* actually supports
//! multisample resolve at a given count - a device-level capability, not
//! something `cargo check` can see - or whether the resolve step produces a
//! picture rather than a black or garbage target. This is the only place that
//! runs the resolve at all.

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model};
use oag_mesh::mesh_render::{self, Anisotropy, UNIFORMS_SIZE, write_uniforms};
use oag_mesh::orbit::Orbit;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

/// A single triangle in the X=0 plane, facing the camera `matrices` (in
/// `mesh_render`) places at `(distance, 0, 0)` looking at the origin for
/// `yaw = pitch = 0`. Large enough relative to `radius` below to cover a good
/// fraction of the frame, so a resolved pixel sampling anywhere near centre
/// lands on it.
fn triangle_model() -> Model {
    let vertices = vec![
        GpuVertex {
            position: [0.0, -1.0, -1.0],
            normal: [1.0, 0.0, 0.0],
            colour: [1.0, 1.0, 1.0, 1.0],
            texcoord: [0.0, 0.0],
            lightmap_texcoord: [0.0, 0.0],
            lit: 0.0,
            anim: 0,
            xform: 0,
            sun_mask: 1.0,
            slots: oag_mesh::mesh::slots::DEFAULT,
            specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
            glow: 0.0,
        },
        GpuVertex {
            position: [0.0, 1.0, -1.0],
            normal: [1.0, 0.0, 0.0],
            colour: [1.0, 1.0, 1.0, 1.0],
            texcoord: [0.0, 0.0],
            lightmap_texcoord: [0.0, 0.0],
            lit: 0.0,
            anim: 0,
            xform: 0,
            sun_mask: 1.0,
            slots: oag_mesh::mesh::slots::DEFAULT,
            specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
            glow: 0.0,
        },
        GpuVertex {
            position: [0.0, 0.0, 1.0],
            normal: [1.0, 0.0, 0.0],
            colour: [1.0, 1.0, 1.0, 1.0],
            texcoord: [0.0, 0.0],
            lightmap_texcoord: [0.0, 0.0],
            lit: 0.0,
            anim: 0,
            xform: 0,
            sun_mask: 1.0,
            slots: oag_mesh::mesh::slots::DEFAULT,
            specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
            glow: 0.0,
        },
    ];
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "msaa_resolve test triangle".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            moving: false,
            blend: None,
            blend_state: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..3,
            texture: None,
            bounds: Bounds {
                centre: [0.0, 0.0, 0.0],
                radius: 1.5,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: Vec::new(),
        lightmaps: Vec::new(),
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),

        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,

        flame: None,
        absorb_shell: false,
        alpha_test_ref: None,
        centre: [0.0, 0.0, 0.0],
        radius: 2.0,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
        mesh_count: 1,
        vertices,
    }
}

/// Renders `model` through a pipeline built at `sample_count`, resolving into
/// a plain single-sample target of `FORMAT`, and returns the resolved pixels
/// as tightly packed RGBA8 rows (padding stripped).
///
/// Mirrors `race::Scene::render`'s attachment wiring: a multisampled colour
/// and depth attachment when `sample_count > 1`, resolving into the caller's
/// target at the end of one pass - see `race::msaa_color_texture` and
/// `race::depth_texture`.
fn render_and_resolve(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    sample_count: u32,
) -> Vec<u8> {
    let mesh_render::Built {
        pipeline,
        bind_group: _placeholder,
        vertex_buffer,
        index_buffer,
        texture_binds,
        fog_bind,
        anim_bind,
        ..
    } = mesh_render::build(
        device,
        queue,
        model,
        FORMAT,
        Anisotropy::Off,
        sample_count,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        mesh_render::GlowMask::Protected,
        mesh_render::Velocity::None,
        // No Zone stage: this test draws a model, not a race.
        &mesh_render::zone::StageArt::NONE,
        // No shadow map, no depth map and no receiver: this test draws one
        // model against nothing.
        mesh_render::ShadowMaps::NONE,
        mesh_render::ShadowReceiver::Never,
    )
    .expect("building the mesh pipeline");

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("msaa_resolve uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("msaa_resolve uniforms"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    write_uniforms(queue, &uniform_buffer, model, 1.0, Orbit::default());

    let resolve_target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa_resolve target"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let resolve_view = resolve_target.create_view(&wgpu::TextureViewDescriptor::default());

    let msaa_colour = (sample_count > 1).then(|| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa_resolve colour"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    });
    let msaa_view = msaa_colour
        .as_ref()
        .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()));
    let (attachment_view, resolve_target_ref) = match &msaa_view {
        Some(msaa_view) => (msaa_view, Some(&resolve_view)),
        None => (&resolve_view, None),
    };

    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa_resolve depth"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: mesh_render::DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("msaa_resolve"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("msaa_resolve"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: attachment_view,
                depth_slice: None,
                resolve_target: resolve_target_ref,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        // Group 2 is fog, left at `Fog::off`. The pipeline layout requires it
        // bound even when it does nothing, which is what this line is for - and
        // the first run of this test after fog landed is what caught that.
        pass.set_bind_group(2, &fog_bind, &[]);
        // Group 3 is the texture-transform table. Left at the all-identity
        // buffer `build` initialises: this test is about MSAA resolve, not
        // animation, and an identity transform draws the authored UVs.
        pass.set_bind_group(3, &anim_bind, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_bind_group(1, &texture_binds[0], &[]);
        for draw in &model.draws {
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
    }
    queue.submit(Some(encoder.finish()));

    let unpadded = (SIZE * 4) as usize;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("msaa_resolve readback"),
        size: (padded * SIZE as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("msaa_resolve readback"),
    });
    encoder.copy_texture_to_buffer(
        resolve_target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(SIZE),
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let mapped = readback.slice(..).get_mapped_range().expect("mapping");

    let mut pixels = Vec::with_capacity(unpadded * SIZE as usize);
    for row in 0..SIZE as usize {
        pixels.extend_from_slice(&mapped[row * padded..row * padded + unpadded]);
    }
    pixels
}

/// MSAA 4x is universally supported by `wgpu` for a colour-renderable,
/// resolvable format like `Rgba8Unorm` without any extra device feature -
/// unlike 2x, which is not (see
/// `msaa_2x_fails_validation_without_the_adapter_specific_format_features_device_feature`
/// below, and why `display::AntiAliasing` has no `Msaa2x` variant). This is
/// the case `[graphics] anti_aliasing`'s `Msaa4x` exercises on every adapter
/// this game ships on.
#[test]
#[ignore = "needs a GPU adapter"]
fn msaa_4x_resolves_a_triangle_onto_a_single_sample_target() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .expect("no GPU adapter available");
    let features = adapter.get_texture_format_features(FORMAT);
    assert!(
        features.flags.sample_count_supported(4)
            && features
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE),
        "{FORMAT:?} must support 4x multisample resolve on every adapter this \
         game ships on - if this fails, `Msaa4x` needs the same fate `Msaa2x` \
         got: removed from `display::AntiAliasing` rather than shipped broken"
    );

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("msaa_resolve test"),
        ..Default::default()
    }))
    .expect("requesting the device");
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));

    let model = triangle_model();
    let single = render_and_resolve(&device, &queue, &model, 1);
    let msaa4x = render_and_resolve(&device, &queue, &model, 4);

    assert!(
        single.chunks(4).any(|p| p != [0, 0, 0, 255]),
        "the single-sample control render must actually draw the triangle"
    );
    assert!(
        msaa4x.chunks(4).any(|p| p != [0, 0, 0, 255]),
        "resolving a 4x multisampled attachment into a single-sample target \
         must produce the same picture, not a black or empty one"
    );

    // The whole reason to run MSAA: some resolved pixel along the triangle's
    // edge is a blend of foreground and background, rather than a hard flip
    // straight from black to white the way an unresolved (or wrongly
    // resolved) attachment would read.
    let is_pure = |p: &[u8]| p == [0, 0, 0, 255] || p == [255, 255, 255, 255];
    assert!(
        msaa4x.chunks(4).any(|p| !is_pure(p)),
        "a 4x-resolved diagonal edge must contain a blended pixel - a fully \
         binary black/white result means the resolve did not actually \
         antialias anything"
    );
}

/// Pins *why* `display::AntiAliasing` has no `Msaa2x` variant: not because 2x
/// hardware support varies, but because this game's device is created with
/// `wgpu::DeviceDescriptor::default()` everywhere (`Gpu::bring_up` in
/// `crates/game/src/main/gpu.rs`, and `crates/game/src/capture.rs`'s headless
/// path), and without `wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`
/// the WebGPU spec guarantees only `[1, 4]` samples for a colour-renderable,
/// resolvable format - `sample_count: 2` is a validation error on *every*
/// adapter this way, not just adapters whose hardware happens to lack it.
/// Confirmed by first asking the adapter (which reports 2x as supported - the
/// hardware capability really is there) and then building the pipeline
/// anyway and watching it fail regardless.
///
/// Requesting that feature would fix this, but it is a first-of-its-kind
/// device feature negotiation in this codebase, done at two call sites, and
/// it loosens format validation renderer-wide - its own decision, not a
/// patch here. See ADR-0013's Consequences.
///
/// If this test ever starts failing, work out *which* of two different
/// things changed before touching `display::AntiAliasing`: either (a) a
/// device now requests `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` somewhere,
/// which is the real fix and the point `Msaa2x` can come back - delete this
/// test rather than "fix" it; or (b) a `wgpu` upgrade changed how this
/// failure surfaces (a returned `Result` instead of an uncaptured-error
/// panic, say), in which case the underlying validation gap may still be
/// exactly what it is today and this test just needs rewriting to catch the
/// new shape, not deleting.
#[test]
#[ignore = "needs a GPU adapter"]
fn msaa_2x_fails_validation_without_the_adapter_specific_format_features_device_feature() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .expect("no GPU adapter available");
    let features = adapter.get_texture_format_features(FORMAT);
    let adapter_reports_2x_supported = features.flags.sample_count_supported(2)
        && features
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE);
    eprintln!(
        "{FORMAT:?} 2x multisample resolve, per the adapter alone: \
         {adapter_reports_2x_supported}"
    );

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("msaa_resolve 2x test"),
        ..Default::default()
    }))
    .expect("requesting the device");

    let model = triangle_model();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));
        render_and_resolve(&device, &queue, &model, 2)
    }));
    assert!(
        result.is_err(),
        "sample_count 2 built and drew without a validation error on this \
         adapter - `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` may already be \
         requested somewhere, or wgpu's guarantee changed; either way, this \
         is the moment `Msaa2x` can come back into `display::AntiAliasing`"
    );
}
