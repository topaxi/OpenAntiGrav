//! The bloom's glow mask is what a material's own fragment program writes into
//! the scene target's alpha, and **only an opaque draw writes it**.
//!
//! Wipeout HD's scene target is `A8R8G8B8` and the gate multiplies the frame
//! colour by its alpha: a neon sign's program ends `MOV H0.w, 1`, the lit road's
//! `MOV H0.w, 0`, an emissive pad's `MOV H0.w, <_ne>.w`. `oag_mesh::mesh::rcs::glow`
//! reads which off the microcode and hands the result to `GpuVertex::glow` under
//! `GlowMask::StampedByProgram`; this draws one triangle for each encoding and
//! reads the alpha back, so dropping the wiring (the model flag, the pipeline's
//! write mask, the `glow_program` override or the decode) fails here and not
//! only in a frame nobody compares.
//!
//! Needs no game content and skips when there is no adapter.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

/// The one texel of the texture the glow samples, on all three channels.
const GLOW_TEXEL: u8 = 128;
/// Parameter `0xe8bcd7f5`, the float3 the sample is multiplied by.
const TINT: [f32; 3] = [2.0, 1.0, 0.0];

fn vertex(glow: f32, position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        // `1.0` is the authored path and `0.0` the stand-in one. Which of the
        // two a chunk takes is the whole subject of this file.
        lit: 1.0,
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
        slots: slots::DEFAULT | slots::EMISSIVE,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow,
        texcoord2: [0.0, 0.0],
        tangent: 0,
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
fn model(glow: f32) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "hd program glow quad".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            moving: false,
            blend: None,
            blend_state: None,
            depth_point: None,
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
            [GLOW_TEXEL, GLOW_TEXEL, GLOW_TEXEL, SECOND_ALPHA],
        ))],
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_glow: Vec::new(),
        material_colour_factor: Vec::new(),
        material_quake: Vec::new(),
        material_tangent: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),
        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,
        glow_from_program: true,
        flame: None,
        absorb_shell: false,
        material_lod_bias: 0.0,
        material_state: Vec::new(),
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
            program: [0.0; 2],
        }],
        vertices: vec![
            vertex(glow, [-0.9, -0.9, 0.5]),
            vertex(glow, [0.9, -0.9, 0.5]),
            vertex(glow, [0.0, 0.9, 0.5]),
        ],
    }
}

const SECOND_ALPHA: u8 = 128;

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    Some(
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device"),
    )
}

fn alpha_of(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    glow: f32,
    mask: mesh_render::GlowMask,
) -> u8 {
    let mut scene = Scene::off();
    scene.light.enabled = 1.0;
    scene.fog.camera = [0.0, 0.0, 100.5];
    draw(device, queue, &model(glow), scene, mask)[3]
}

/// `mesh::rcs::glow::texel_glow(Source::Second, 3, 1.0)`: the second texture's alpha.
const SECOND_CHANNEL_3: f32 = -(1.0 + 8.0 * (4.0 * 1.0 + 3.0));

#[test]
fn an_opaque_draw_writes_the_alpha_its_program_computes() {
    let Some((device, queue)) = device() else {
        return;
    };
    let program = mesh_render::GlowMask::StampedByProgram;
    assert_eq!(
        alpha_of(&device, &queue, 1.0, program),
        255,
        "MOV H0.w, 1: a neon sign"
    );
    assert_eq!(
        alpha_of(&device, &queue, 0.0, program),
        0,
        "MOV H0.w, 0: a lit road"
    );
    assert_eq!(
        alpha_of(&device, &queue, 122.0 / 255.0, program),
        122,
        "a constant is written as it is (Metropia's sky: 122)"
    );
    assert_eq!(
        alpha_of(&device, &queue, SECOND_CHANNEL_3, program),
        SECOND_ALPHA,
        "a texel-sourced mask reads the second texture's alpha"
    );
    // A model that does not carry the flag keeps its mask closed: colour only, the clear stays.
    let mut other = model(1.0);
    other.glow_from_program = false;
    let mut scene = Scene::off();
    scene.light.enabled = 1.0;
    scene.fog.camera = [0.0, 0.0, 100.5];
    assert_eq!(
        draw(
            &device,
            &queue,
            &other,
            scene,
            mesh_render::GlowMask::Protected
        )[3],
        0,
        "a model without `glow_from_program` leaves the alpha channel alone"
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
    bytes.extend_from_slice(&[0u8; 32]); // the four pad floats, then the environment-probe layer and its padding
    bytes.extend_from_slice(&[0u8; 32]); // hull point lights
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // prev_mvp
    assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);
    bytes
}

/// Draws the model once into a `SIZE`x`SIZE` target and reads the centre back.
fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    scene: Scene,
    glow: mesh_render::GlowMask,
) -> [u8; 4] {
    let built = mesh_render::build(
        device,
        queue,
        model,
        FORMAT,
        Anisotropy::Off,
        1,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        glow,
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
        pass.set_bind_group(0, &bind_group, &[0]);
        // Slot 1: the model's own textures. Slot 0 is `build`'s white 1x1.
        pass.set_bind_group(1, &built.texture_binds[1], &[]);
        pass.set_bind_group(2, &built.fog_bind, &[0]);
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
