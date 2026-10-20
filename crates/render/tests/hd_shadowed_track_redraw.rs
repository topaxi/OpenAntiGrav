//! Draws one lit surface through the mesh pipeline under Wipeout HD's shadow
//! map and asserts what a covered texel shows.
//!
//! **The law under test** is the original's own composite, read off
//! `Shadow_CompileShadowedTrackRedraw` (`0x004053e0`,
//! `docs/ghidra/functions/ps3-hdfury-eu/shadow-track-redraw.md`): the redraw
//! asks for the `ShadowToAlpha` permutation with no `Sun` bit - the same road
//! lit by ambient alone - and blends it in by `1 - shadowMapTex` under
//! `SRC_ALPHA / ONE_MINUS_SRC_ALPHA`. So a fully covered texel shows exactly
//! what the surface draws with the sun switched off, a half-covered one the
//! midway colour, and an uncovered one is untouched. **Not** the colour times a
//! constant: the `MAP_STRENGTH` multiply this replaces would put a covered texel
//! at half the sunlit value whatever the sun was, which this test tells apart
//! by lighting the surface with a sun far brighter than its ambient.
//!
//! Skips when there is no adapter, like `hd_spu_vertex_light.rs`. Needs no game
//! content, so it runs in `just test`.

use std::sync::Arc;

use oag_core::math::{Mat4, Vec3};
use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{
    self, Anisotropy, CraftShadow, Light, Scene, ShadowMap, ShadowMaps, ShadowReceiver,
    UNIFORMS_SIZE,
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

/// The sun and the ambient the rig is lit with: a sun well over twice the
/// ambient, so "half the sunlit colour" and "the ambient alone" are far apart.
const SUN: [f32; 3] = [0.6, 0.5, 0.4];
const AMBIENT: [f32; 3] = [0.08, 0.1, 0.14];

fn vertex(position: [f32; 3], slot_bits: u32) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        // Facing the sun, which sits on `+Z` with the camera.
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        // Neither `NO_AMBIENT` nor `NO_SUN`: the track's own ordinary lit path.
        slots: slot_bits,
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

/// One triangle at `z = 0.5`, a white albedo, no second texture.
fn model(slot_bits: u32) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "hd shadowed track redraw".into(),
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
        textures: vec![Some(texture("white albedo", [255, 255, 255, 255]))],
        lightmaps: vec![None],
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_colour_factor: Vec::new(),
        material_quake: Vec::new(),
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
        emissive: Vec::new(),
        vertices: vec![
            vertex([-0.9, -0.9, 0.5], slot_bits),
            vertex([0.9, -0.9, 0.5], slot_bits),
            vertex([0.0, 0.9, 0.5], slot_bits),
        ],
    }
}

/// An authored rig: the sun on `+Z`, an ambient, no specular and no prelit.
fn rig(sun: [f32; 3]) -> Scene {
    let mut scene = Scene::off();
    scene.light = Light::authored([0.0, 0.0, 1.0], sun, AMBIENT, [1.0; 3], [1.0; 3], 0.0);
    scene.fog.camera = [0.0, 0.0, 100.5];
    scene
}

/// The shadow block and one craft's map record: an identity projection over the
/// whole map, so the triangle's own `x`/`y` pick the texel and its `z = 0.5`
/// sits at the craft map's own centre depth, inside the receiver's slab.
fn shadowed(mut scene: Scene, strength: f32) -> Scene {
    scene.shadow.strength = strength;
    scene.shadow.mode = ShadowMap::COVERAGE;
    scene.craft_shadow[0] = CraftShadow {
        matrix: Mat4::IDENTITY.to_cols_array_2d(),
        rect: [0.0, 0.0, 1.0, 1.0],
        centre: [0.0, 0.0, 0.0, 1.0e6],
    };
    scene
}

#[test]
fn a_covered_texel_shows_the_road_lit_without_its_sun() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let model = model(slots::DEFAULT);

    let sunlit = draw(&device, &queue, &model, rig(SUN), None);
    let ambient_only = draw(&device, &queue, &model, rig([0.0; 3]), None);
    assert!(
        sunlit[0] > ambient_only[0] + 40 && sunlit[2] > ambient_only[2] + 40,
        "the sun must matter for this test to mean anything: {sunlit:?} vs {ambient_only:?}"
    );

    // Fully covered: the sun is taken back out, whatever `strength` says.
    for strength in [0.5, 1.0] {
        let covered = draw(
            &device,
            &queue,
            &model,
            shadowed(rig(SUN), strength),
            Some(255),
        );
        for channel in 0..3 {
            assert!(
                covered[channel].abs_diff(ambient_only[channel]) <= 2,
                "channel {channel} at strength {strength}: a covered texel is the ambient-only \
                 colour {ambient_only:?}, got {covered:?} (sunlit {sunlit:?})"
            );
        }
        // And is not the multiply it replaced.
        let halved = sunlit[0] / 2;
        assert!(
            covered[0].abs_diff(halved) > 10,
            "a covered texel is not half the sunlit colour: {covered:?} vs {sunlit:?}"
        );
    }

    // No coverage: untouched.
    let mut scene = rig(SUN);
    scene = shadowed(scene, 0.5);
    let clear = draw(&device, &queue, &model, scene, Some(0));
    for channel in 0..3 {
        assert!(
            clear[channel].abs_diff(sunlit[channel]) <= 1,
            "an uncovered texel is the sunlit colour: {clear:?} vs {sunlit:?}"
        );
    }

    // Half coverage lands midway between them: the blend is by alpha.
    let mut scene = rig(SUN);
    scene = shadowed(scene, 0.5);
    let half = draw(&device, &queue, &model, scene, Some(128));
    for channel in 0..3 {
        let mid = (u16::from(sunlit[channel]) + u16::from(ambient_only[channel])) / 2;
        assert!(
            half[channel].abs_diff(mid as u8) <= 3,
            "channel {channel}: half coverage is midway between {ambient_only:?} and {sunlit:?}, \
             got {half:?}"
        );
    }
}

/// The scene with each of `matrices` bound as one craft's whole-map record, the
/// first in slot 0.
fn with_crafts(mut scene: Scene, matrices: &[Mat4]) -> Scene {
    scene = shadowed(scene, 0.5);
    for (slot, matrix) in matrices.iter().enumerate() {
        scene.craft_shadow[slot] = CraftShadow {
            matrix: matrix.to_cols_array_2d(),
            rect: [0.0, 0.0, 1.0, 1.0],
            centre: [0.0, 0.0, 0.0, 1.0e6],
        };
    }
    scene
}

/// How far from the sunlit colour towards the ambient-only one `got` is, `0.0`
/// to `1.0`, on the red channel.
fn covered_fraction(got: [u8; 4], sunlit: [u8; 4], ambient_only: [u8; 4]) -> f32 {
    (f32::from(sunlit[0]) - f32::from(got[0])) / (f32::from(sunlit[0]) - f32::from(ambient_only[0]))
}

/// Each craft's map is its own redraw: the original blends the shadowed road in
/// once per ship, so two craft covering the same texel at half coverage leave
/// `1 - (1 - 0.5) * (1 - 0.5)` of the way to the shadowed colour - not the
/// maximum (half) and not the sum (all of it). And a craft's box is a slab: a
/// road on the sun's side of the craft, past the 50 units the original redraws
/// within, or outside its square is not its shadow's to take.
#[test]
fn craft_maps_combine_as_the_originals_redraws_do_and_keep_to_their_box() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let model = model(slots::DEFAULT);
    let sunlit = draw(&device, &queue, &model, rig(SUN), None);
    let ambient_only = draw(&device, &queue, &model, rig([0.0; 3]), None);
    let fraction = |matrices: &[Mat4]| {
        let got = draw(
            &device,
            &queue,
            &model,
            with_crafts(rig(SUN), matrices),
            Some(128),
        );
        covered_fraction(got, sunlit, ambient_only)
    };

    let one = fraction(&[Mat4::IDENTITY]);
    assert!(
        (one - 0.5).abs() < 0.04,
        "one craft at half coverage: {one}"
    );
    let two = fraction(&[Mat4::IDENTITY, Mat4::IDENTITY]);
    assert!(
        (two - 0.75).abs() < 0.04,
        "two craft at half coverage each leave a quarter of the sunlit colour: {two}"
    );

    // The triangle sits at depth 0.5 of the identity box. Moving the box along
    // the sun puts the road on either side of the craft: `0.3` is on the sun's
    // side of it, `0.6` just past it, `0.9` past the 50 units (`0.857`).
    let shifted = |depth: f32| Mat4::from_translation(Vec3::new(0.0, 0.0, depth - 0.5));
    assert!(
        fraction(&[shifted(0.6)]) > 0.45,
        "just past the craft is shadowed"
    );
    assert!(
        fraction(&[shifted(0.3)]).abs() < 0.04,
        "a road nearer the sun than the craft is lit"
    );
    assert!(
        fraction(&[shifted(0.9)]).abs() < 0.04,
        "a road past the 50-unit redraw radius is lit"
    );
    // And outside the craft's square on the map.
    assert!(
        fraction(&[Mat4::from_translation(Vec3::new(1.5, 0.0, 0.0))]).abs() < 0.04,
        "outside the box is lit"
    );
    // And outside the craft's cull sphere, which is checked before the box.
    let mut far = with_crafts(rig(SUN), &[Mat4::IDENTITY]);
    far.craft_shadow[0].centre = [100.0, 0.0, 0.0, 50.0 * 50.0];
    let got = draw(&device, &queue, &model, far, Some(255));
    assert!(
        covered_fraction(got, sunlit, ambient_only).abs() < 0.04,
        "a point outside the cull sphere is lit"
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
    coverage: Option<u8>,
) -> [u8; 4] {
    let coverage_view = coverage.map(|value| {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("coverage"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: mesh_render::COVERAGE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &[value; 16],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(4),
            },
            wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        );
        texture.create_view(&Default::default())
    });
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
        ShadowMaps {
            coverage: coverage_view.as_ref(),
            ..ShadowMaps::NONE
        },
        ShadowReceiver::Both,
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

/// A track chunk whose colour is a special combine - here the glass floor's
/// facing-ramp sheen - is not the generic path's, so the generic sun term must
/// not be subtracted from it: it falls back to the chosen multiply. At a
/// strength of 0.25 the multiply leaves 0.75 of the colour (191 of 255) where
/// subtracting the generic sun term (about 130 on this rig) would leave about
/// half. A `MAG_LOOP` or `LIGHT_CONE` chunk draws nothing in this harness, so the
/// sheen stands for the family.
#[test]
fn a_special_combine_keeps_the_multiply_and_is_not_charged_a_sun_it_never_had() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let model = model(slots::DEFAULT | slots::FACING_RAMP_SHEEN);

    let plain = draw(&device, &queue, &model, rig(SUN), None);
    let mut scene = rig(SUN);
    scene = shadowed(scene, 0.25);
    let covered = draw(&device, &queue, &model, scene, Some(255));
    assert!(
        plain[0] > 100,
        "the floor must draw something to be shadowed: {plain:?}"
    );
    for channel in 0..3 {
        let expected = (f32::from(plain[channel]) * 0.75).round() as u8;
        assert!(
            covered[channel].abs_diff(expected) <= 3,
            "channel {channel}: a sheen chunk keeps the multiply, 0.75 of {plain:?}, got {covered:?}"
        );
    }
}
