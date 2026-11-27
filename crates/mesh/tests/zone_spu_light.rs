//! Draws one Zone surface through the mesh pipeline with Wipeout HD's SPU vertex light bound
//! and asserts where the light lands in the pixel.
//!
//! **The law under test** (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "The blue
//! wash of a volley"): every `HalfBrightZone*` program's `SVC1` twin ends
//! `out = (lerp(fogColour, light * surface + glow, fog) + spu) / 2`. The light is added after the
//! fog and outside the surface multiply, exactly once. Its `ShadowToAlpha` redraw takes the light
//! inside its light term instead, so a texel under a craft's shadow keeps only `surface * spu`.
//! A chunk on its plain program takes the light in its sum, and a chunk the Zone grade is
//! switched off for takes none after the fog.
//!
//! The target is HD's race target (`SCENE_FORMAT_8BIT` storing encoded values), so the add
//! runs in linear light and the byte read back is its sRGB encode. Skips when there is no
//! adapter, like `zone_recolour.rs`.

use std::sync::Arc;

use oag_core::math::Mat4;
use oag_gpu::formats::SceneStore;
use oag_mesh::mesh::{
    Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, Texels, ZONE_EXEMPT_RATE, slots,
};
use oag_mesh::mesh_render::{
    self, Anisotropy, CraftShadow, Scene, ShadowMap, ShadowMaps, ShadowReceiver, SpuLight,
    SpuLights, UNIFORMS_SIZE, Zone, ZoneSet, zone,
};

const FORMAT: wgpu::TextureFormat = oag_gpu::formats::SCENE_FORMAT_8BIT;
const SIZE: u32 = 64;
const OUTPUT_SCALE: f32 = 0.5;
const AMBIENT: f32 = 0.6;
/// The stage texel and the Scene set's effect: a grey Zone surface, so a light inside the
/// multiply shows.
const ZONE_TEXEL: u8 = 128;
const EFFECT: f32 = 2.0;
/// The light: blue only, far along the normal, so `N.L` is 1 and the falloff the same to 1e-3
/// over the quad.
const BLUE: f32 = 0.6;
const RANGE: f32 = 1000.0;
const HEIGHT: f32 = 50.0;
/// The fog: green only, so the fog colour and the light never share a channel.
const FOG_GREEN: f32 = 0.3;
const FOG_DENSITY: f32 = 1.0;
const EXEMPT_SLOT: u32 = 1 << slots::MATERIAL_SHIFT;

fn vertex(position: [f32; 3], slot_bits: u32) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: slot_bits,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent: 0,
        depth_bias: 0.0,
    }
}

fn texture(label: &str, rgba: [u8; 4]) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: label.into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(rgba.to_vec()),
        mip_count: None,
        lod_code: 0.0,
    })
}

fn model(albedo: [u8; 4], slot_bits: u32) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "zone spu light test quad".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
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
        transparent_draws: Vec::new(),
        textures: vec![Some(texture("albedo", albedo))],
        lightmaps: vec![None],
        pad_masks: Vec::new(),
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
        // Entry 1: the Zone grade switched off for the chunk (`ZONE_EXEMPT_RATE`).
        emissive: vec![Emissive {
            tint: [0.0; 3],
            offset: 0.0,
            scale: 1.0,
            rate: ZONE_EXEMPT_RATE,
            program: [0.0; 2],
        }],
        vertices: vec![
            vertex([-0.9, -0.9, 0.5], slot_bits),
            vertex([0.9, -0.9, 0.5], slot_bits),
            vertex([0.0, 0.9, 0.5], slot_bits),
        ],
    }
}

/// Identity camera and model, so clip space is model space and the view depth is 1.
fn uniforms() -> Vec<u8> {
    let identity: [[f32; 4]; 4] = Mat4::IDENTITY.to_cols_array_2d();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // view_projection
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // model
    bytes.extend_from_slice(&[0u8; 32]); // the four pad floats, then the environment-probe layer and its padding
    bytes.extend_from_slice(&[0u8; 32]); // hull point lights
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // prev_mvp
    assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);
    bytes
}

fn stage_art(stage: &Arc<ModelTexture>) -> zone::StageArt {
    zone::StageArt {
        track: Some(stage.clone()),
        track_outer: Some(stage.clone()),
        scene: Some(stage.clone()),
        scene_outer: Some(stage.clone()),
    }
}

/// HD's Zone frame: the stage's ambient alone (no sun), the Scene set a grey `EFFECT`, the
/// authored fog on, the title's Zone frame marker set, and `lights` bound.
fn scene(lights: &[SpuLight]) -> Scene {
    let mut scene = Scene::off();
    scene.light = mesh_render::Light::authored(
        [0.0, 0.0, 1.0],
        [0.0; 3],
        [AMBIENT; 3],
        [1.0; 3],
        [1.0; 3],
        0.0,
    )
    .with_output_scale(OUTPUT_SCALE);
    scene.fog.camera = [0.0, 0.0, 100.5];
    scene.fog.colour = [0.0, FOG_GREEN, 0.0];
    scene.fog.enabled = 1.0;
    scene.fog.curve = 1.0;
    scene.fog.density = FOG_DENSITY;
    let set = ZoneSet {
        effect: [EFFECT, EFFECT, EFFECT, 0.0],
        base: [0.0; 4],
        base_alt: [0.0; 4],
    };
    scene.zone = Zone {
        uv_scale: [1.0, 1.0],
        enabled: 1.0,
        radius: 0.0,
        origin: [0.0; 4],
        track: set,
        scene: set,
        track_outer: set,
        scene_outer: set,
        ..Zone::default()
    };
    scene.zone.shape[3] = 1.0;
    scene.spu_lights = SpuLights::from_slice(lights);
    scene
}

/// One craft's shadow map over the whole quad at full coverage, as
/// `hd_shadowed_track_redraw.rs` binds it.
fn covered(mut scene: Scene) -> Scene {
    scene.shadow.strength = 0.5;
    scene.shadow.mode = ShadowMap::COVERAGE;
    scene.craft_shadow[0] = CraftShadow {
        matrix: Mat4::IDENTITY.to_cols_array_2d(),
        rect: [0.0, 0.0, 1.0, 1.0],
        centre: [0.0, 0.0, 0.0, 1.0e6],
    };
    scene
}

const LIGHT: SpuLight = SpuLight {
    position: [0.0, 0.0, HEIGHT, 1.0],
    colour: [0.0, 0.0, BLUE, RANGE],
};

fn srgb_decode(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb_encode(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let e = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (e * 255.0).round() as u8
}

/// The light's blue at the quad, before the program's `/2`.
fn spu_blue() -> f32 {
    BLUE * (1.0 - (HEIGHT - 0.5) / RANGE) * (256.0 / 255.0)
}

/// The authored fog's factor at the view depth of 1.
fn fog_factor() -> f32 {
    (-(FOG_DENSITY * FOG_DENSITY)).exp()
}

/// The Zone surface's linear value: the stage texel decoded, times the effect.
fn surface() -> f32 {
    srgb_decode(f32::from(ZONE_TEXEL) / 255.0) * EFFECT
}

fn assert_near(got: u8, want: u8, what: &str, pixel: [u8; 4]) {
    assert!(
        got.abs_diff(want) <= 2,
        "{what}: got {pixel:?}, want {want}"
    );
}

#[test]
fn a_zone_chunk_adds_the_spu_light_after_its_fog_once_and_its_redraw_inside() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let stage = texture("zone stage", [ZONE_TEXEL, ZONE_TEXEL, ZONE_TEXEL, 255]);
    let art = stage_art(&stage);
    let zone_chunk = model([200, 200, 200, 255], slots::DEFAULT);

    let f = fog_factor();
    let lit = OUTPUT_SCALE * surface() * AMBIENT;
    let fogged = |colour: f32, fog: f32| f * colour + (1.0 - f) * fog * OUTPUT_SCALE;
    let after = OUTPUT_SCALE * spu_blue();

    // Unlit by the SPU: the fogged surface on every channel, green also the fog's.
    let dark = draw(&device, &queue, &zone_chunk, &art, scene(&[]), None);
    assert_near(
        dark[0],
        srgb_encode(fogged(lit, 0.0)),
        "red without the light",
        dark,
    );
    assert_near(
        dark[2],
        srgb_encode(fogged(lit, 0.0)),
        "blue without the light",
        dark,
    );

    // Lit: blue is the fogged surface plus the light, once, after the fog, in linear light.
    let pixel = draw(&device, &queue, &zone_chunk, &art, scene(&[LIGHT]), None);
    let want = srgb_encode(fogged(lit, 0.0) + after);
    assert_near(pixel[2], want, "blue under the light", pixel);
    assert_near(pixel[0], dark[0], "red under a blue light", pixel);
    // The three wrong placements this tells apart, each more than the tolerance off.
    for (name, wrong) in [
        ("before the fog", fogged(lit + after, 0.0)),
        (
            "added twice",
            fogged(lit + OUTPUT_SCALE * surface() * spu_blue(), 0.0) + after,
        ),
        (
            "added in the encoded domain",
            srgb_decode(f32::from(srgb_encode(fogged(lit, 0.0))) / 255.0 + after),
        ),
    ] {
        let wrong = srgb_encode(wrong);
        assert!(
            want.abs_diff(wrong) > 3,
            "the test must tell the light {name} ({wrong}) from the law ({want})"
        );
    }

    // Under a craft's full shadow the redraw replaces the pass: the light inside its multiply.
    let shadowed = draw(
        &device,
        &queue,
        &zone_chunk,
        &art,
        covered(scene(&[LIGHT])),
        Some(255),
    );
    let inside = srgb_encode(fogged(
        OUTPUT_SCALE * surface() * (AMBIENT + spu_blue()),
        0.0,
    ));
    assert_near(shadowed[2], inside, "blue under the shadow", shadowed);

    // A chunk the Zone grade is switched off for adds nothing after the fog.
    let exempt = model([200, 200, 200, 255], slots::DEFAULT | EXEMPT_SLOT);
    let exempt_dark = draw(&device, &queue, &exempt, &art, scene(&[]), None);
    let exempt_lit = draw(&device, &queue, &exempt, &art, scene(&[LIGHT]), None);
    assert_near(
        exempt_lit[2],
        exempt_dark[2],
        "an exempt chunk under the light",
        exempt_lit,
    );
}

#[test]
fn a_black_zone_surface_turns_the_lights_colour_and_a_plain_chunk_multiplies_it() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    // Detonator's stage 1: the stage colours are black, so the Zone surface is zero.
    let black = texture("black stage", [0, 0, 0, 255]);
    let art = stage_art(&black);
    let mut lit = scene(&[LIGHT]);
    lit.fog.enabled = 0.0;
    let after = srgb_encode(OUTPUT_SCALE * spu_blue());

    let zone_chunk = model([200, 200, 200, 255], slots::DEFAULT);
    let pixel = draw(&device, &queue, &zone_chunk, &art, lit, None);
    assert_near(
        pixel[2],
        after,
        "a black Zone surface under the light",
        pixel,
    );

    // A plain chunk (no ambient here, to read the light alone): the light times its albedo.
    let mut plain_lit = lit;
    plain_lit.light =
        mesh_render::Light::authored([0.0, 0.0, 1.0], [0.0; 3], [0.0; 3], [1.0; 3], [1.0; 3], 0.0)
            .with_output_scale(OUTPUT_SCALE);
    let white = model([255, 255, 255, 255], slots::DEFAULT | slots::ZONE_PLAIN);
    let pixel = draw(&device, &queue, &white, &art, plain_lit, None);
    assert_near(
        pixel[2],
        after,
        "a white plain chunk under the light",
        pixel,
    );
    let dark = model([0, 0, 0, 255], slots::DEFAULT | slots::ZONE_PLAIN);
    let pixel = draw(&device, &queue, &dark, &art, plain_lit, None);
    assert_near(pixel[2], 0, "a black plain chunk under the light", pixel);

    // A title without the Zone frame marker adds nothing after the fog.
    let mut unmarked = lit;
    unmarked.zone.shape[3] = 0.0;
    let pixel = draw(&device, &queue, &zone_chunk, &art, unmarked, None);
    assert_near(pixel[2], 0, "no marker", pixel);
}

/// A circuit built against the stage light (`zone_stage_light`) draws what the uniform-driven Zone
/// branches draw: a chunk on its Zone program, a plain chunk, and an emissive plain chunk, each
/// under a sun, an ambient and an SPU light. The three expected pixels differ from one another, so
/// a build that lights one kind as another fails.
#[test]
fn the_stage_light_pipeline_draws_what_the_zone_branches_draw() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let art = stage_art(&texture("grey stage", [128, 128, 128, 255]));
    let mut lit = scene(&[LIGHT]);
    lit.fog.enabled = 0.0;
    lit.light = mesh_render::Light::authored(
        [0.0, 0.0, 1.0],
        [0.4, 0.2, 0.1],
        [0.2, 0.3, 0.4],
        [1.0; 3],
        [1.0; 3],
        0.0,
    )
    .with_output_scale(OUTPUT_SCALE);

    let kinds = [
        ("a Zone chunk", slots::DEFAULT),
        ("a plain chunk", slots::DEFAULT | slots::ZONE_PLAIN),
        (
            "an emissive plain chunk",
            slots::DEFAULT | slots::ZONE_PLAIN | slots::EMISSIVE,
        ),
    ];
    let mut seen = Vec::new();
    for (what, bits) in kinds {
        let quad = model([200, 200, 200, 255], bits);
        let uniform = draw_built(&device, &queue, &quad, &art, lit, None, true, false);
        let built = draw_built(&device, &queue, &quad, &art, lit, None, true, true);
        assert_eq!(built, uniform, "{what}");
        assert!(
            !seen.contains(&built),
            "{what} reads like another kind: {built:?}"
        );
        seen.push(built);
    }
}

/// The centre texel of the quad drawn into HD's race target, with a craft's coverage map of
/// `coverage` everywhere when one is given.
fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    stage: &zone::StageArt,
    scene: Scene,
    coverage: Option<u8>,
) -> [u8; 4] {
    draw_built(device, queue, model, stage, scene, coverage, false, false)
}

/// [`draw`] with the build told it is a circuit's (`prepass`) and, with `stage_light`, built in a
/// scope whose race lights it by the stage alone (`BuildCacheScope::zone_stage_light`).
#[allow(clippy::too_many_arguments)]
fn draw_built(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    stage: &zone::StageArt,
    scene: Scene,
    coverage: Option<u8>,
    prepass: bool,
    stage_light: bool,
) -> [u8; 4] {
    let scope = stage_light.then(mesh_render::BuildCacheScope::open);
    if let Some(scope) = &scope {
        scope.zone_stage_light(true);
    }
    let coverage_view = coverage.map(|value| {
        let extent = wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("coverage"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: mesh_render::COVERAGE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
            texture_binding_view_dimension: None,
        });
        queue.write_texture(
            texture.as_image_copy(),
            &[value; 16],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(4),
            },
            extent,
        );
        texture.create_view(&Default::default())
    });
    let built = mesh_render::build_with(
        device,
        queue,
        model,
        FORMAT,
        SceneStore::Encoded,
        Anisotropy::Off,
        1,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        mesh_render::GlowMask::Protected,
        mesh_render::Velocity::None,
        stage,
        ShadowMaps {
            coverage: coverage_view.as_ref(),
            ..ShadowMaps::NONE
        },
        ShadowReceiver::Both,
        mesh_render::Texcoords::Interleaved,
        prepass,
        false,
        false,
        false,
    )
    .expect("building the mesh pipeline");
    queue.write_buffer(&built.fog_buffer, 0, bytemuck::bytes_of(&scene));

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("zone spu uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buffer, 0, &uniforms());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("zone spu uniforms"),
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
            label: Some("zone spu"),
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
        pass.set_bind_group(1, &built.texture_binds[1], &[]);
        pass.set_bind_group(2, &built.fog_bind, &[0]);
        pass.set_bind_group(3, &built.anim_bind, &[]);
        pass.set_vertex_buffer(0, built.geometry.sole().slice(..));
        pass.set_index_buffer(built.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..3, 0, 0..1);
    }

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("zone spu readback"),
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
    let frame = readback
        .slice(..)
        .get_mapped_range()
        .expect("mapping")
        .to_vec();
    let at = (((SIZE / 2) * SIZE + SIZE / 2) * 4) as usize;
    [frame[at], frame[at + 1], frame[at + 2], frame[at + 3]]
}
