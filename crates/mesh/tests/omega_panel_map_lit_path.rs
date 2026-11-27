//! Draws one Omega panel-map road triangle (`road_alphaspec_normal_emissive_panelmap1`) through the
//! mesh pipeline and asserts `shade.wesl` marches the panel map and adds the sky sheen, on a
//! fixture needing no game content (`mesh::rcs::psp2::panel_map`, mode
//! [`PROGRAM_PANEL_MAP`]).
//!
//! The view is oblique along the tangent, so a ray through a recessed panel map (a field
//! positive everywhere) descends to the channel's floor and the diffuse is read `Depth / 100`
//! further along `-u` (`0.5` at a 45 degree view and `Depth` 50): a red texel there instead of the green one under the vertex coordinate.
//! The reflection weight (`ReflectivityAmount`) adds the sky map, here a flat grey; at zero it
//! adds nothing.
//!
//! Skips when there is no adapter. Runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::{
    BlockFormat, Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, PROGRAM_PANEL_MAP,
    Texels, slots,
};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, TextureDetail, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;
const ENTRY: u32 = 1 << slots::MATERIAL_SHIFT;
/// Tangent `+X`, bitangent sign `+1`, as `pack_tangent` stores it.
const TANGENT_X: u32 = u32::from_le_bytes([255, 128, 128, 255]);

fn vertex(position: [f32; 3], u: f32) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [u, 0.5],
        lightmap_texcoord: [0.5, 0.5],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT | ENTRY,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent: TANGENT_X,
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

/// A 4x4 panel map holding `0.25` everywhere when `value` is positive, else `-0.25`.
fn field(value: f32) -> Option<Arc<ModelTexture>> {
    // IEEE half bits of the two values used here, `+-0.25`.
    let bits: u16 = if value > 0.0 { 0x3400 } else { 0xb400 };
    Some(Arc::new(ModelTexture {
        label: "panel map".into(),
        width: 4,
        height: 4,
        texels: Texels::Blocks {
            format: BlockFormat::R16Float,
            levels: vec![bits.to_le_bytes().repeat(16)],
        },
        mip_count: None,
        lod_code: 0.0,
    }))
}

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];

/// The road at vertex coordinate `u = 0.75` over a diffuse whose first half is red and second
/// half green (alpha `gloss`), with a flat normal map and the panel map holding `depth_field`.
/// `amount` is `ReflectivityAmount`, `depth` the authored `Depth` (in hundredths of a height).
fn model(mode: f32, depth_field: f32, amount: f32, depth: f32, gloss: u8) -> Model {
    let u = 0.75;
    let vertices = vec![
        vertex([-0.9, -0.9, 0.5], u),
        vertex([0.9, -0.9, 0.5], u),
        vertex([0.0, 0.9, 0.5], u),
    ];
    let red = [RED[0], RED[1], RED[2], gloss];
    let green = [GREEN[0], GREEN[1], GREEN[2], gloss];
    let rows = [
        Emissive {
            tint: [amount, 1.0, 0.0],
            offset: 1.0,
            scale: 0.0,
            rate: 0.0,
            program: [mode, 0.0],
        },
        Emissive {
            tint: [0.0, 1.0, 0.0],
            offset: 0.0,
            scale: 0.0,
            rate: 0.0,
            program: [0.0; 2],
        },
        Emissive {
            tint: [depth, 0.01, 1.0],
            offset: 0.0,
            scale: 0.0,
            rate: 0.0,
            program: [0.0; 2],
        },
    ];
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "omega panel-map road".into(),
        indices: vec![0, 1, 2],
        draws: Vec::new(),
        transparent_draws: vec![DrawCall {
            writes_depth: false,
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
        textures: vec![texture("diffuse", &[red, red, green, green])],
        lightmaps: Vec::new(),
        pad_masks: vec![texture("flat normal", &[[128, 128, 255, 255]])],
        wave_maps: Vec::new(),
        fifth_maps: vec![field(depth_field)],
        sixth_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_glow: Vec::new(),
        material_colour_factor: Vec::new(),
        material_flip_v: Vec::new(),
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
        emissive: rows.to_vec(),
        vertices,
    }
}

#[test]
fn the_march_reads_the_floor_and_the_reflection_adds_the_sky() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) = pollster::block_on(
        adapter.request_device(&mesh_render::device_descriptor("panel-map road", &adapter)),
    )
    .expect("requesting the device");
    let (width, height) = oag_mesh::mesh::sky_paraboloid::SIZE;
    let grey = vec![160u8; (width * height * 4) as usize];
    let sky = mesh_render::upload_sky_probe(&device, &queue, &grey);
    let at = |model: &Model| {
        let mut scene = Scene::off();
        scene.light.enabled = 1.0;
        scene.light.direction = [0.0, 0.0, 1.0];
        scene.light.sun = [0.6, 0.6, 0.6];
        // Up and off along `+X`, so the view ray runs along `-u` into the surface.
        scene.fog.camera = [100.0, 0.0, 100.5];
        draw(&device, &queue, model, scene, Some(&sky))
    };
    // A panel map below `-w` everywhere is solid at the top: no march, the green texel under `u`.
    let flat = at(&model(PROGRAM_PANEL_MAP, -0.25, 0.0, 50.0, 255));
    assert!(
        flat[1] > 24 && flat[0] < 8,
        "a solid panel map reads under u: {flat:?}"
    );
    // Recessed everywhere, the ray reaches the floor `0.5` along `-u`: the red half.
    let marched = at(&model(PROGRAM_PANEL_MAP, 0.25, 0.0, 50.0, 255));
    assert!(
        marched[0] > 24 && marched[1] < 8,
        "the march reads the diffuse at the floor: {marched:?} (unmarched {flat:?})"
    );
    // The sheen: `Amount * Dm.a * (Min + Scale (1 - N.V)^Power) * sky`, added after the light.
    let sheen = at(&model(PROGRAM_PANEL_MAP, -0.25, 1.0, 50.0, 255));
    assert!(
        sheen[0] > flat[0] + 24 && sheen[2] > flat[2] + 24,
        "the reflection adds the grey sky: {sheen:?} against {flat:?}"
    );
    // The diffuse alpha gates it.
    let matte = at(&model(PROGRAM_PANEL_MAP, -0.25, 1.0, 50.0, 0));
    assert!(
        matte[2] <= flat[2] + 2,
        "a zero diffuse alpha reflects nothing: {matte:?} against {flat:?}"
    );
    // The `EmissiveMap` (fourth binding) is added after the light where the second entry's tint
    // `z` says the material has one.
    let mut glowing = model(PROGRAM_PANEL_MAP, -0.25, 0.0, 50.0, 255);
    glowing.wave_maps = vec![texture("emissive", &[[0, 0, 255, 255]])];
    glowing.emissive[1].tint[2] = 1.0;
    let glow = at(&glowing);
    assert!(
        glow[2] > flat[2] + 24 && glow[0] <= flat[0] + 2,
        "the emissive picture is added: {glow:?} against {flat:?}"
    );
    // Without the mode neither happens.
    let plain = at(&model(0.0, 0.25, 1.0, 50.0, 255));
    assert!(
        plain[1] > 24 && plain[0] < 8 && plain[2] < 8,
        "an entry that is not the mode marches nothing and reflects nothing: {plain:?}"
    );
}

/// A PS4 lightmapped material's slot word carries no alpha lane, so `texel.a` is the diffuse
/// map's red. The sheen and the sun lobe are gated by the diffuse **alpha** (`0xe144`
/// `@0xde0-0xde4`, `docs/ghidra/functions/ps4-omega-eu/panel-displacement.md`, "Live"): a
/// texel with a bright red and no alpha reflects nothing, and one with no red and a full alpha
/// reflects the sky.
#[test]
fn the_sheen_is_gated_by_the_diffuse_alpha_not_its_red() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) = pollster::block_on(
        adapter.request_device(&mesh_render::device_descriptor("panel-map gate", &adapter)),
    )
    .expect("requesting the device");
    let (width, height) = oag_mesh::mesh::sky_paraboloid::SIZE;
    let grey = vec![160u8; (width * height * 4) as usize];
    let sky = mesh_render::upload_sky_probe(&device, &queue, &grey);
    let at = |diffuse: [u8; 4], amount: f32| {
        let mut m = model(PROGRAM_PANEL_MAP, -0.25, amount, 50.0, 255);
        m.textures = vec![texture("diffuse", &[diffuse; 4])];
        for v in &mut m.vertices {
            v.slots = ENTRY;
        }
        let mut scene = Scene::off();
        scene.light.enabled = 1.0;
        scene.light.direction = [0.0, 0.0, 1.0];
        scene.light.sun = [0.6, 0.6, 0.6];
        scene.fog.camera = [100.0, 0.0, 100.5];
        draw(&device, &queue, &m, scene, Some(&sky))
    };
    // A faint red with a full alpha: the sheen follows the alpha. A bright red with no alpha:
    // it does not follow the red. Blue carries the sheen; the red lane stays below the
    // diffuse's own blue so the albedo is the same in both.
    let faint_base = at([40, 0, 128, 255], 0.0);
    let faint = at([40, 0, 128, 255], 1.0);
    let bright_base = at([255, 0, 128, 0], 0.0);
    let bright = at([255, 0, 128, 0], 1.0);
    assert!(
        faint[2] > faint_base[2] + 12,
        "a full diffuse alpha reflects the sky: {faint:?} against {faint_base:?}"
    );
    assert!(
        bright[2] <= bright_base[2] + 2,
        "a bright red with no diffuse alpha reflects nothing: {bright:?} against {bright_base:?}"
    );
}

/// The road's diffuse is read under explicit gradients (`material_sample_grad`), which
/// TEXTURE DETAIL scales (`texture_lod.wesl`, `grad_scale`). An authored red, green, blue
/// chain minified two levels: `original` reads level 2, `high` 1, `maximum` the base.
#[test]
fn texture_detail_scales_the_explicit_gradients() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    if !adapter
        .features()
        .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
    {
        eprintln!("no BC sampling: skipping");
        return;
    }
    let (device, queue) = pollster::block_on(adapter.request_device(
        &mesh_render::device_descriptor("panel-map detail", &adapter),
    ))
    .expect("requesting the device");
    let (width, height) = oag_mesh::mesh::sky_paraboloid::SIZE;
    let grey = vec![160u8; (width * height * 4) as usize];
    let sky = mesh_render::upload_sky_probe(&device, &queue, &grey);
    let level = |rgb565: u16, blocks: usize| {
        let [low, high] = rgb565.to_le_bytes();
        [low, high, low, high, 0, 0, 0, 0].repeat(blocks)
    };
    let mut quad = model(PROGRAM_PANEL_MAP, -0.25, 0.0, 50.0, 255);
    quad.textures = vec![Some(Arc::new(ModelTexture {
        label: "red, green, blue blocks".into(),
        width: 16,
        height: 16,
        texels: Texels::Blocks {
            format: BlockFormat::Bc1,
            levels: vec![level(0xf800, 16), level(0x07e0, 4), level(0x001f, 1)],
        },
        mip_count: None,
        lod_code: 0.0,
    }))];
    // 16 texels across, 64 pixels across clip space: 8 per clip unit is 4 texels a pixel.
    for vertex in &mut quad.vertices {
        vertex.texcoord = [8.0 * vertex.position[0], 8.0 * vertex.position[1]];
    }
    let levels = TextureDetail::ALL.map(|detail| {
        let mut scene = Scene::off();
        scene.light.enabled = 1.0;
        scene.light.direction = [0.0, 0.0, 1.0];
        scene.light.sun = [0.6, 0.6, 0.6];
        scene.fog.camera = [100.0, 0.0, 100.5];
        scene.fog.texlod_shift = detail.level_shift();
        let texel = draw(&device, &queue, &quad, scene, Some(&sky));
        (0..3)
            .max_by_key(|&channel| texel[channel])
            .expect("three channels")
    });
    assert_eq!(levels, [2, 1, 0]);
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
    sky: Option<&wgpu::TextureView>,
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
        mesh_render::GlowMask::Protected,
        mesh_render::Velocity::None,
        &mesh_render::zone::StageArt::NONE,
        mesh_render::ShadowMaps {
            sky_probe: sky,
            ..mesh_render::ShadowMaps::NONE
        },
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
        pass.set_pipeline(&built.blend_pipeline[0]);
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
