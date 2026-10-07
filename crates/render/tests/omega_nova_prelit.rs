//! Wipeout: Omega Collection's lightmap combination through the mesh pipeline:
//! `prelit = scale * pow(lightmap, power) + bias` on the raw atlas, no constant
//! ambient on a lightmapped draw.
//!
//! The law is read out of the circuit pixel shaders' GCN microcode
//! (`docs/ghidra/functions/ps4-omega-eu/lightmap-prelit.md`): `v_log_f32`,
//! `v_mul_f32` by the power, `v_exp_f32`, `v_mad_f32` by scale and bias, on a
//! BC7 UNORM atlas, with no `constantAmbientColour` in any of the 4,664 shaders
//! that declare the triple. These tests fail if the bias is dropped, if the
//! ambient is added back, if the atlas is sRGB-decoded in front of the curve,
//! or if the bias lights a draw that has no lightmap.
//!
//! The authored path encodes `pow(lit, 1 / 2.2)` into a gamma target, so the
//! expected bytes are computed through the same encode and compared to within
//! two levels. Skips when there is no adapter; needs no game content.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Light, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

/// The patch's own Tech De Ra triple: `(scale, bias, power)`.
const NOVA: [f32; 3] = [2.5, 0.2, 2.0];

/// A constant ambient well above anything the sum should reach without it.
const AMBIENT: [f32; 3] = [0.9; 3];

fn vertex(position: [f32; 3], lightmapped: bool) -> GpuVertex {
    GpuVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        // The authored path.
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        // What the PS4 loader sets on a submesh whose declaration carries a
        // `lightmapUV`; nothing else, so no `NO_AMBIENT` rides along and the
        // ambient is dropped by the nova path alone.
        slots: if lightmapped {
            slots::DEFAULT | slots::SECOND_IS_LIGHTMAP
        } else {
            slots::DEFAULT
        },
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

/// One triangle at `z = 0.5`, a white albedo, and a one-texel atlas of `texel`
/// when `lightmapped`.
fn model(lightmapped: bool, texel: u8) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "omega nova prelit quad".into(),
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
        textures: vec![Some(texture("white albedo", [255, 255, 255, 255]))],
        lightmaps: vec![lightmapped.then(|| texture("atlas", [texel, texel, texel, 255]))],
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
        centre: [0.0, 0.0, 0.5],
        radius: 2.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
        vertices: vec![
            vertex([-0.9, -0.9, 0.5], lightmapped),
            vertex([0.9, -0.9, 0.5], lightmapped),
            vertex([0.0, 0.9, 0.5], lightmapped),
        ],
    }
}

/// No sun, a constant ambient, the prelit curve at HD's identity: what a surface
/// shows is the ambient plus whatever the title's own prelit term adds.
fn rig(nova: bool) -> Scene {
    let mut scene = Scene::off();
    let light = Light::authored([0.0, 0.0, 1.0], [0.0; 3], AMBIENT, [1.0; 3], [1.0; 3], 0.0);
    scene.light = if nova {
        light.with_nova_prelit(NOVA[0], NOVA[1], NOVA[2])
    } else {
        light
    };
    scene.fog.camera = [0.0, 0.0, 100.5];
    scene
}

/// The byte the authored path encodes a linear sum to: `pow(clamp(x), 1/2.2)`.
fn encoded(linear: f32) -> f32 {
    linear.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0
}

fn close(got: u8, want: f32, why: &str) {
    assert!(
        (f32::from(got) - want).abs() <= 2.0,
        "{why}: got {got}, wanted about {want:.1}"
    );
}

fn adapter() -> Option<(wgpu::Device, wgpu::Queue)> {
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

#[test]
fn a_lightmapped_draw_is_scale_times_the_raw_texel_to_the_power_plus_bias() {
    let Some((device, queue)) = adapter() else {
        return;
    };
    let [scale, bias, power] = NOVA;
    for texel in [0_u8, 64, 128, 200] {
        let raw = f32::from(texel) / 255.0;
        let want = encoded(scale * raw.powf(power) + bias);
        let got = draw(&device, &queue, &model(true, texel), rig(true));
        close(got[0], want, &format!("texel {texel}, red"));
        close(got[1], want, &format!("texel {texel}, green"));
        close(got[2], want, &format!("texel {texel}, blue"));
    }
}

#[test]
fn the_constant_ambient_does_not_join_a_lightmapped_nova_draw() {
    let Some((device, queue)) = adapter() else {
        return;
    };
    // Texel 0: the prelit term is the bias alone, 0.2. With the 0.9 ambient
    // added back the sum would be 1.1 and clamp to 255.
    let got = draw(&device, &queue, &model(true, 0), rig(true));
    close(
        got[0],
        encoded(NOVA[1]),
        "ambient 0.9 against a bias of 0.2",
    );
    assert!(got[0] < 200, "the ambient leaked into the sum: {got:?}");
}

#[test]
fn the_bias_does_not_light_a_draw_that_has_no_lightmap() {
    let Some((device, queue)) = adapter() else {
        return;
    };
    // No lightmap, so the placeholder is black: the draw is the constant
    // ambient alone, with or without the title's triple bound.
    let with = draw(&device, &queue, &model(false, 0), rig(true));
    let without = draw(&device, &queue, &model(false, 0), rig(false));
    assert_eq!(with, without, "the nova rig must not touch an unlit draw");
    close(with[0], encoded(AMBIENT[0]), "the ambient alone");
}

#[test]
fn hds_curve_is_untouched_when_the_nova_flag_is_off() {
    let Some((device, queue)) = adapter() else {
        return;
    };
    // HD's combination on the same draw: ambient plus the raw atlas through the
    // curve. Texel 128: 0.9 + 0.50196 = 1.4 - clamps to white, which is the
    // opposite of the nova result above for the same texel.
    let hd = draw(&device, &queue, &model(true, 128), rig(false));
    let nova = draw(&device, &queue, &model(true, 128), rig(true));
    assert_eq!(hd[0], 255, "HD's rig, ambient plus a decoded atlas");
    assert!(nova[0] < 255 && nova[0] > 200, "nova's own sum: {nova:?}");
}

/// **HD's lightmap is read raw, not sRGB-decoded.** Block #9 of
/// `track_surface.rcsmaterial` is `TEX` then `LG2`/`MUL`/`EX2` then `MAD` by the
/// scale, and the RSX sRGB-decodes no texture on the disc
/// (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "HD's lightmap is read
/// raw"), so `prelit = scale * pow(texel, power)` on the stored byte. With no
/// ambient and the identity curve a texel of 128 must come out as
/// `encoded(128 / 255)`; the old extra `pow(_, 2.2)` gave `128` back instead.
/// Fails if the decode in front of the curve returns.
#[test]
fn hds_lightmap_enters_the_curve_raw() {
    let Some((device, queue)) = adapter() else {
        return;
    };
    let mut scene = rig(false);
    scene.light = Light::authored([0.0, 0.0, 1.0], [0.0; 3], [0.0; 3], [1.0; 3], [1.0; 3], 0.0);
    for texel in [64_u8, 128, 200] {
        let want = encoded(f32::from(texel) / 255.0);
        let got = draw(&device, &queue, &model(true, texel), scene);
        close(got[0], want, &format!("texel {texel}, HD raw atlas"));
    }
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
