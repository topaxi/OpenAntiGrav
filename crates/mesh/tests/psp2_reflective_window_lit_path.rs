//! Draws one triangle through the mesh pipeline for 2048's two Moa Therma families and asserts
//! `shade.wesl` reads them, on a fixture needing no game content.
//!
//! `psp2_reflective_window_ground_truth.rs` checks the real circuit builds the entries; this one
//! checks the shader reads them (`mesh::rcs::psp2::emissive_add`):
//!
//! ```text
//! PROGRAM_REFLECT_MAP   rgb += decode(third(0.5 + 0.5 * reflect(-V, N).xy)) * picture.a
//! PROGRAM_PICTURE_ADD   rgb += decode(picture)
//! ```
//!
//! Drop either mode's block, read the gate off another lane, flip an axis of the lookup or add
//! the picture unsquared and one of the readings below moves.
//!
//! Skips when there is no adapter. Runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::{
    Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, PROGRAM_PICTURE_ADD,
    PROGRAM_REFLECT_MAP, Texels, slots,
};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;
const ENTRY: u32 = 1 << slots::MATERIAL_SHIFT;
const BLACK: [u8; 4] = [0, 0, 0, 255];
const CLEAR_BLACK: [u8; 4] = [0, 0, 0, 0];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

fn vertex(position: [f32; 3], normal: [f32; 3]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal,
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.5, 0.5],
        lightmap_texcoord: [0.5, 0.5],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT | ENTRY,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent: 0,
        depth_bias: 0.0,
    }
}

fn texture(label: &str, texels: &[[u8; 4]]) -> Option<Arc<ModelTexture>> {
    Some(Arc::new(ModelTexture {
        label: label.into(),
        width: texels.len() as u32,
        height: 1,
        texels: Texels::Rgba8(texels.concat()),
        mip_count: None,
        lod_code: 0.0,
    }))
}

/// A triangle facing `normal` on `picture`, with `third` on the third binding.
fn model(mode: f32, picture: [u8; 4], third: &[[u8; 4]], normal: [f32; 3]) -> Model {
    let vertices = vec![
        vertex([-0.9, -0.9, 0.5], normal),
        vertex([0.9, -0.9, 0.5], normal),
        vertex([0.0, 0.9, 0.5], normal),
    ];
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "2048 reflective window".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            writes_depth: true,
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
        transparent_draws: Vec::new(),
        alpha_tested_draws: Vec::new(),
        textures: vec![texture("picture", &[picture])],
        lightmaps: Vec::new(),
        pad_masks: vec![texture("third", third)],
        wave_maps: Vec::new(),
        fifth_maps: Vec::new(),
        sixth_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_glow: Vec::new(),
        material_colour_factor: Vec::new(),
        material_quake: Vec::new(),
        material_tangent: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),
        caustic: oag_mesh::mesh::caustic_pass::Caustic::default(),
        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,
        glow_from_program: false,
        flame: None,
        absorb_shell: false,
        shield_shell: false,
        shield_lit: false,
        own_shield: false,
        material_lod_bias: 0.0,
        material_state: Vec::new(),
        per_batch_cull: false,
        sky_reflection: None,
        alpha_test_ref: None,
        centre: [0.0, 0.0, 0.5],
        radius: 2.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: vec![Emissive {
            tint: [1.0; 3],
            offset: 0.0,
            scale: 0.0,
            rate: 0.0,
            program: [mode, 0.0],
        }],
        vertices,
    }
}

#[test]
fn the_reflect_map_is_read_at_the_world_reflection_and_gated_by_the_pictures_alpha() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) = pollster::block_on(adapter.request_device(
        &mesh_render::device_descriptor("reflective window", &adapter),
    ))
    .expect("requesting the device");
    let at = |model: &Model| {
        let mut scene = Scene::off();
        scene.light.enabled = 1.0;
        scene.light.direction = [0.0, 0.0, 1.0];
        scene.light.sun = [0.6, 0.6, 0.6];
        scene.fog.camera = [0.0, 0.0, 100.5];
        draw(&device, &queue, model, scene)
    };
    let window = |picture: [u8; 4], third: &[[u8; 4]], normal: [f32; 3]| {
        at(&model(PROGRAM_REFLECT_MAP, picture, third, normal))
    };
    let facing = [0.0, 0.0, 1.0];

    // Looking straight at it the reflection vector is the normal, the centre of the map.
    let straight = window(BLACK, &[GREEN], facing);
    assert!(
        straight[1] > 24 && straight[0] < 8 && straight[2] < 8,
        "the map shows through a black picture: {straight:?}"
    );
    // The gate is the picture's alpha: a clear picture shows none of it.
    let gated = window(CLEAR_BLACK, &[GREEN], facing);
    assert!(gated[1] < 8, "alpha 0 gates the reflection off: {gated:?}");

    // `R = 2 (N.V) N - V` with `V` along z: a normal leaning to +x reflects to +x, `R.x` 0.57 and
    // `0.5 + 0.5 R.x` three quarters across the map; leaning to -x it reads a quarter across. The
    // map repeats, so the lean is kept off the edge, where a two-texel map would wrap.
    let to_right = window(BLACK, &[GREEN, BLUE], [0.3, 0.0, 0.954]);
    assert!(
        to_right[2] > 200 && to_right[1] < 40,
        "a normal leaning +x reads the map's right texel: {to_right:?}"
    );
    let to_left = window(BLACK, &[GREEN, BLUE], [-0.3, 0.0, 0.954]);
    assert!(
        to_left[1] > 200 && to_left[2] < 40,
        "a normal leaning -x reads the map's left texel: {to_left:?}"
    );

    // Without the mode the entry is inert.
    let plain = at(&model(0.0, BLACK, &[GREEN], facing));
    assert!(
        plain[1] < 8,
        "an entry that is not the mode reflects nothing: {plain:?}"
    );
}

#[test]
fn the_awning_adds_its_pictures_square_after_the_light() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) = pollster::block_on(
        adapter.request_device(&mesh_render::device_descriptor("awning", &adapter)),
    )
    .expect("requesting the device");
    let at = |mode: f32| {
        let mut scene = Scene::off();
        scene.light.enabled = 1.0;
        scene.light.direction = [0.0, 0.0, 1.0];
        scene.light.sun = [0.6, 0.6, 0.6];
        scene.fog.camera = [0.0, 0.0, 100.5];
        draw(
            &device,
            &queue,
            &model(mode, GREEN, &[GREEN], [0.0, 0.0, 1.0]),
            scene,
        )
    };
    let lit = at(0.0);
    let awning = at(PROGRAM_PICTURE_ADD);
    assert!(
        lit[1] > 24 && lit[1] < 230,
        "the plain family is lit only: {lit:?}"
    );
    assert!(
        awning[1] >= 250 && awning[0] < 8 && awning[2] < 8,
        "the awning adds the picture's square on top of the light: {awning:?} against {lit:?}"
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
            texture_binding_view_dimension: None,
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
        pass.set_vertex_buffer(0, built.geometry.sole().slice(..));
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
