//! Draws one glow-bearing surface through the mesh pipeline on **both** of
//! `mesh.wgsl`'s paths and asserts the pixel each leaves.
//!
//! # Why this test exists
//!
//! `hd_emissive_glow_ground_truth.rs` already renders a real circuit's glow
//! table and asserts that it reaches the frame - and it passed, unchanged, for
//! the nine days the layer was reaching **nothing a player ever sees**.
//! `mesh_render::capture_pixels_from` never sets `scene.light.enabled`, so
//! every pixel it draws resolves through the stand-in `plain` path, which is
//! the one branch that summed the glow. The defect lived entirely in the other
//! branch: `mix(plain_rgb, authored_rgb, scene.light.enabled * in.lit)`, with
//! HD's rig always enabled, hands every `in.lit == 1` chunk to `authored_rgb`,
//! and `lit_linear` had no glow term at all.
//!
//! So the assertion that catches it is not "the glow reaches a frame" but
//! **"the glow reaches a frame drawn under an authored rig"**, and the two
//! differ by one uniform. Measured on the disc, the layer went from 0 changed
//! pixels to about a tenth of the frame when that term was added - see
//! `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The emissive glow
//! belongs on the lit path".
//!
//! # What the constants are for
//!
//! **The tint is `2.0` on red deliberately.** `pow(x, 2.2)` and its inverse
//! cancel, so a tint of `1.0` draws the same pixel whether the sample is
//! sRGB-decoded before the sum or not, and a test built on one would pass
//! against either domain. At `2.0` the decoded answer is `174` and the
//! undecoded one saturates to `255` - the same discrimination
//! `zone_recolour.rs` makes for the Zone term, for the same reason.
//!
//! Green carries a tint of `1.0` and is the "did it arrive at all" channel;
//! blue carries `0.0` and must stay black, so a glow that ignores its tint
//! shows up as well.
//!
//! Skips when there is no adapter, like `zone_recolour.rs` and
//! `velocity_target.rs`. Needs no game content, so it runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

/// The one texel of the texture the glow samples, on all three channels.
const GLOW_TEXEL: u8 = 128;
/// Parameter `0xe8bcd7f5`, the float3 the sample is multiplied by.
const TINT: [f32; 3] = [2.0, 1.0, 0.0];

/// The glow table's slot 1, which is where [`mesh_render::Emissives::of`] puts
/// a model's first layer - slot 0 stays zero and adds nothing.
const GLOW_SLOT: u32 = 1 << slots::MATERIAL_SHIFT;

fn vertex(lit: f32, position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        // `1.0` is the authored path and `0.0` the stand-in one. Which of the
        // two a chunk takes is the whole subject of this file.
        lit,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        // **`EMISSIVE`, so the light rig is the identity**: `authored` is
        // exactly `vec3(1.0)` and the specular term is switched off, leaving
        // the pixel to measure the glow and the colour space alone. It is also
        // what the real materials declare - every one of the 33-35 chunks per
        // circuit that names neither a constant ambient nor a directional
        // light is a sign or a glow (`docs/formats/rcsmaterial.md`).
        //
        // `ADD_SECOND` says the program accumulates its second texture rather
        // than selecting between the two, and the high half carries the index
        // into the glow table.
        slots: slots::DEFAULT | slots::EMISSIVE | slots::ADD_SECOND | GLOW_SLOT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    }
}

fn texture(label: &str, rgba: [u8; 4]) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: label.into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(rgba.to_vec()),
        mip_count: None,
    })
}

/// One triangle, a **black** albedo and a glow texture in the second slot.
///
/// Black so the pixel is the glow alone: the microcode adds the layer to a
/// diffuse it has already multiplied by the light, and a zero diffuse leaves
/// the summand by itself on either path.
fn model(lit: f32) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "hd emissive lit-path quad".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            moving: false,
            blend: None,
            blend_state: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..3,
            texture: Some(0),
            bounds: Bounds {
                centre: [0.0, 0.0, 0.5],
                radius: 1.5,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: vec![Some(texture("black albedo", [0, 0, 0, 255]))],
        // The second slot, which `mesh.wgsl` samples as `glow_sample`. It is
        // the same binding the baked atlas uses, which is exactly why
        // `rcs::emissive` refuses a layer to any material whose second slot
        // *is* that atlas - adding a light term and a sun mask to an albedo
        // paints a shadow map as a glow.
        lightmaps: vec![Some(texture(
            "glow",
            [GLOW_TEXEL, GLOW_TEXEL, GLOW_TEXEL, 255],
        ))],
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_colour_factor: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),
        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,
        flame: None,
        absorb_shell: false,
        alpha_test_ref: None,
        centre: [0.0, 0.0, 0.5],
        radius: 2.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: vec![Emissive {
            tint: TINT,
            // `a = 0`, `b = 1` and a still clock, so the sample lands on the
            // texel this test bound rather than on whatever a scroll reached.
            // `rate` is `0.0` for the same reason `Emissive::rate` exists at
            // all: a material that accumulates is not necessarily one that
            // scrolls.
            offset: 0.0,
            scale: 1.0,
            rate: 0.0,
        }],
        vertices: vec![
            vertex(lit, [-0.9, -0.9, 0.5]),
            vertex(lit, [0.9, -0.9, 0.5]),
            vertex(lit, [0.0, 0.9, 0.5]),
        ],
    }
}

/// What the authored path must produce: the sample decoded, tinted, encoded.
fn decoded(tint: f32) -> u8 {
    let sample = f32::from(GLOW_TEXEL) / 255.0;
    let linear = (sample.powf(2.2) * tint).clamp(0.0, 1.0);
    (linear.powf(1.0 / 2.2) * 255.0).round() as u8
}

/// What the stand-in path must produce: the sample tinted in gamma, as it is.
fn gamma(tint: f32) -> u8 {
    ((f32::from(GLOW_TEXEL) / 255.0 * tint).clamp(0.0, 1.0) * 255.0).round() as u8
}

/// **The emissive glow reaches a surface drawn under an authored rig**, which
/// is every circuit chunk Wipeout HD draws.
#[test]
fn the_glow_is_summed_on_the_authored_path_and_not_only_the_stand_in_one() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let mut scene = Scene::off();
    // **The one uniform this whole test turns on.** HD binds an authored rig
    // for every race it draws, so this is the ordinary case and not a corner
    // of one.
    scene.light.enabled = 1.0;
    // Dead in front and far away, so the eye-dependent terms vanish - the same
    // placement and the same reason as `zone_recolour.rs`, whose note explains
    // what `Fog::off`'s origin does to a quad at `z = 0.5`.
    scene.fog.camera = [0.0, 0.0, 100.5];

    let authored = draw(&device, &queue, &model(1.0), scene);
    assert_ne!(
        (authored[0], authored[1], authored[2]),
        (0, 0, 0),
        "the glow drew nothing under an authored rig: it is being summed into \
         `plain` alone, so every `lit == 1` chunk - which on HD is the track, \
         the tubes, the signs and everything near the camera - drops it"
    );
    let want = (decoded(TINT[0]), decoded(TINT[1]), decoded(TINT[2]));
    assert_eq!(
        (authored[0], authored[1], authored[2]),
        want,
        "the glow must be summed in linear light on the authored path: the \
         sample is a picture and is decoded with the albedo beside it, while \
         the tint is a shader constant and is not"
    );
    // The whole point of a tint of 2.0: the two domains disagree here.
    assert_ne!(
        authored[0], 255,
        "red saturated - the sample was summed undecoded, in the domain the \
         stand-in path uses"
    );

    // **The stand-in path is unchanged and is deliberately the other domain.**
    // Prelit geometry takes it even under an authored rig, its light being
    // baked into vertex colours in the space the asset authors, so a linear
    // summand there would be the one term shaded twice.
    let prelit = draw(&device, &queue, &model(0.0), scene);
    assert_eq!(
        (prelit[0], prelit[1], prelit[2]),
        (gamma(TINT[0]), gamma(TINT[1]), gamma(TINT[2])),
        "the stand-in path must sum the glow in gamma, as it always did"
    );

    // **No `ADD_SECOND`, no glow**, on either path: the bit is the material's
    // own declaration that its program accumulates, and every other surface
    // indexes slot 0 of the table, whose tint is zero.
    let mut plain = model(1.0);
    for vertex in &mut plain.vertices {
        vertex.slots &= !slots::ADD_SECOND;
    }
    let off = draw(&device, &queue, &plain, scene);
    assert_eq!(
        (off[0], off[1], off[2]),
        (0, 0, 0),
        "a surface that does not declare `ADD_SECOND` must draw its black \
         albedo and nothing else"
    );
}

/// Identity camera and model, so clip space is model space.
fn uniforms() -> Vec<u8> {
    let identity: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // view_projection
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // model
    bytes.extend_from_slice(&[0u8; 16]); // the four pad floats
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // prev_mvp
    assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);
    bytes
}

/// Draws the model once into a `SIZE`x`SIZE` target and reads the centre back.
fn draw(device: &wgpu::Device, queue: &wgpu::Queue, model: &Model, scene: Scene) -> [u8; 4] {
    let built = mesh_render::build(
        device,
        queue,
        model,
        FORMAT,
        Anisotropy::Off,
        1,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        mesh_render::GlowMask::Protected,
        mesh_render::Velocity::None,
        &mesh_render::zone::StageArt::NONE,
        mesh_render::ShadowMaps::NONE,
        mesh_render::ShadowReceiver::Never,
    )
    .expect("building the mesh pipeline");
    queue.write_buffer(&built.fog_buffer, 0, bytemuck::bytes_of(&scene));

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("glow uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buffer, 0, &uniforms());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("glow uniforms"),
        layout: &built.pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let extent = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let make = |label, format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let colour = make(
        "colour",
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = make(
        "depth",
        mesh_render::DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let colour_view = colour.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());

    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("glow"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &colour_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&built.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        // Slot 1: the model's own textures. Slot 0 is `build`'s white 1x1.
        pass.set_bind_group(1, &built.texture_binds[1], &[]);
        pass.set_bind_group(2, &built.fog_bind, &[]);
        pass.set_bind_group(3, &built.anim_bind, &[]);
        pass.set_vertex_buffer(0, built.vertex_buffer.slice(..));
        pass.set_index_buffer(built.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..3, 0, 0..1);
    }

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("glow readback"),
        size: u64::from(SIZE * SIZE * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: None,
            },
        },
        extent,
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let mapped = readback.slice(..).get_mapped_range().expect("mapping");
    // Dead centre of the triangle.
    let at = (((SIZE / 2) * SIZE + SIZE / 2) * 4) as usize;
    [mapped[at], mapped[at + 1], mapped[at + 2], mapped[at + 3]]
}
