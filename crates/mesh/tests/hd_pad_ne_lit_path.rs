//! HD's pad program under an authored rig: the `_ne` mask is read as a normal
//! and as light bars, the lightmap stays bound beside it, and a flat mask is
//! the identity.
//!
//! `mesh_render::capture_pixels_from` never enables the authored rig, so a
//! picture drawn through it cannot tell a dropped lightmap or a wrong normal
//! from a right one - the pad's own lighting is only reachable under
//! `scene.light.enabled`, which is what this draws with (the technique
//! `hd_emissive_lit_path.rs` documents). Skips when there is no adapter and
//! needs no game content, so it runs in `just test`.
//!
//! One triangle in the `z = 0.5` plane, facing the camera, with the diffuse
//! `u` running along `+x`: the tangent the shader derives is `+x`, which is
//! the authored tangent's own direction on every pad triangle measured
//! (a one-off tangent read). The sun leans toward `+x`, so a normal map that
//! tilts the pad toward `+x` is lit more than a flat one.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, Emissive, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;
/// The glow table's slot 1.
const GLOW_SLOT: u32 = 1 << slots::MATERIAL_SHIFT;
/// A pad's slots word: the lightmap is the second texture, no constant
/// ambient (`NO_AMBIENT`, as on every pad measured), and `PAD_NE`.
const PAD: u32 = slots::DEFAULT | slots::SECOND_IS_LIGHTMAP | slots::NO_AMBIENT | slots::PAD_NE;

fn texture(label: &str, rgba: [u8; 4]) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: label.into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(rgba.to_vec()),
        mip_count: None,
    })
}

fn vertex(position: [f32; 3], slots: u32) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [0.0, 0.0, 0.0, 1.0],
        // `u` along `+x`, `v` along `+y`.
        texcoord: [(position[0] + 0.9) / 1.8, (position[1] + 0.9) / 1.8],
        lightmap_texcoord: [0.5, 0.5],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent: 0,
    }
}

/// What the three bindings hold.
struct Pad {
    lightmap: Option<[u8; 4]>,
    mask: [u8; 4],
    tint: [f32; 3],
    flagged: bool,
}

fn model(pad: &Pad) -> Model {
    let slots = if pad.flagged {
        PAD | GLOW_SLOT
    } else {
        PAD & !slots::PAD_NE
    };
    let mut model = Model::none("hd pad _ne lit-path triangle");
    model.indices = vec![0, 1, 2];
    model.draws = vec![DrawCall {
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
    }];
    model.textures = vec![Some(texture("albedo", [128, 128, 128, 255]))];
    model.lightmaps = vec![pad.lightmap.map(|t| texture("lightmap", t))];
    model.pad_masks = vec![Some(texture("_ne", pad.mask))];
    model.emissive = vec![Emissive {
        tint: pad.tint,
        offset: 0.0,
        scale: 1.0,
        rate: 0.0,
        program: [0.0; 2],
    }];
    model.vertices = vec![
        vertex([-0.9, -0.9, 0.5], slots),
        vertex([0.9, -0.9, 0.5], slots),
        vertex([0.0, 0.9, 0.5], slots),
    ];
    model
}

fn luma(px: [u8; 4]) -> f32 {
    0.299 * f32::from(px[0]) + 0.587 * f32::from(px[1]) + 0.114 * f32::from(px[2])
}

#[test]
fn the_pad_program_reads_its_mask_as_a_normal_and_as_light_bars_and_keeps_the_lightmap() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let mut scene = Scene::off();
    scene.light.enabled = 1.0;
    // Toward the light: `+x` and `+z`, so a normal tilted toward `+x` faces it
    // more squarely than the flat one does.
    scene.light.direction = [0.8, 0.0, 0.6];
    scene.light.sun = [0.6, 0.6, 0.6];
    scene.fog.camera = [0.0, 0.0, 100.5];

    // A flat tangent-space normal, and a mask alpha of zero: no light bar.
    let flat = [128, 128, 255, 0];
    let lit = |pad: Pad| draw(&device, &queue, &model(&pad), scene);
    let base = Pad {
        lightmap: Some([160, 160, 160, 255]),
        mask: flat,
        tint: [1.0, 0.0, 0.0],
        flagged: true,
    };
    let flat_px = lit(Pad {
        ..base_clone(&base)
    });
    println!("flat mask, lightmap bound: {flat_px:?}");
    assert!(
        luma(flat_px) > 5.0,
        "the pad drew nothing under an authored rig"
    );

    // **The lightmap stays bound beside the mask.** Its rgb is the prelit
    // term; take it away and the pad is darker. A binding that gave the
    // lightmap's slot to the mask would draw the same either way or brighter.
    let no_lightmap = lit(Pad {
        lightmap: None,
        ..base_clone(&base)
    });
    println!("flat mask, no lightmap: {no_lightmap:?}");
    assert!(
        luma(flat_px) > luma(no_lightmap) + 8.0,
        "the lightmap no longer reaches a pad that also binds a mask: \
         {flat_px:?} with it, {no_lightmap:?} without"
    );

    // **A flat mask is the identity**: `nx = 0`, `nz = 1` rebuilds the vertex
    // normal, so the flagged pad equals the unflagged one. A wrong decode of
    // the mask (an unsigned read, a swapped lane) breaks this exactly.
    let unflagged = lit(Pad {
        flagged: false,
        ..base_clone(&base)
    });
    assert_eq!(
        flat_px, unflagged,
        "a flat tangent-space mask must leave the pad lit as the vertex normal does"
    );

    // **The mask's RGB is the normal**: tilted toward the tangent (`+x`, the
    // diffuse `u` direction) it faces the light more squarely.
    let tilted = lit(Pad {
        mask: [230, 128, 200, 0],
        ..base_clone(&base)
    });
    println!("mask tilted toward +x: {tilted:?}");
    assert!(
        luma(tilted) > luma(flat_px) + 8.0,
        "a normal map tilted toward the light must light the pad more: {tilted:?} against \
         {flat_px:?}"
    );
    // ...and away from it, less.
    let away = lit(Pad {
        mask: [26, 128, 200, 0],
        ..base_clone(&base)
    });
    assert!(
        luma(away) + 8.0 < luma(flat_px),
        "a normal map tilted away from the light must light the pad less: {away:?}"
    );

    // **The alpha is the light bar, in the pad's own colour**, added after the
    // light: only the tinted channel rises, by about the tint.
    let bar = lit(Pad {
        mask: [128, 128, 255, 255],
        ..base_clone(&base)
    });
    println!("mask alpha 255, red tint: {bar:?}");
    assert!(
        i32::from(bar[0]) > i32::from(flat_px[0]) + 60,
        "the light bar does not glow: {bar:?} against {flat_px:?}"
    );
    assert!(
        (i32::from(bar[1]) - i32::from(flat_px[1])).abs() <= 2
            && (i32::from(bar[2]) - i32::from(flat_px[2])).abs() <= 2,
        "a red light bar must add red only: {bar:?} against {flat_px:?}"
    );
    // ...and a pad whose bars carry no colour does not glow.
    let black_bar = lit(Pad {
        mask: [128, 128, 255, 255],
        tint: [0.0; 3],
        ..base_clone(&base)
    });
    assert_eq!(
        black_bar, flat_px,
        "the glow is the authored colour times the mask alpha"
    );
}

/// `Emissive::program` mode 12 (`mesh::rcs::alpha_emissive`): the picture's own
/// alpha times the entry's colour, added after the light. No third binding and
/// no role bit: the entry's mode alone selects it.
#[test]
fn a_picture_alpha_emissive_entry_adds_the_pictures_alpha_times_its_colour() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let mut scene = Scene::off();
    scene.light.enabled = 1.0;
    scene.light.direction = [0.8, 0.0, 0.6];
    scene.light.sun = [0.6, 0.6, 0.6];
    scene.fog.camera = [0.0, 0.0, 100.5];

    let lit = |alpha: u8, tint: [f32; 3], mode: f32| {
        let word = slots::DEFAULT | slots::NO_AMBIENT | GLOW_SLOT;
        let mut model = model(&Pad {
            lightmap: None,
            mask: [128, 128, 255, 0],
            tint,
            flagged: false,
        });
        model.textures = vec![Some(texture("albedo", [128, 128, 128, alpha]))];
        model.pad_masks = vec![None];
        model.emissive[0].program = [mode, 0.0];
        for vertex in &mut model.vertices {
            vertex.slots = word;
        }
        draw(&device, &queue, &model, scene)
    };
    let mode = oag_mesh::mesh::PROGRAM_PICTURE_ALPHA_EMISSIVE;
    let dark = lit(255, [0.0; 3], mode);
    let red = lit(255, [1.0, 0.0, 0.0], mode);
    println!("alpha 255: no colour {dark:?}, red {red:?}");
    assert!(luma(dark) > 5.0, "the surface drew nothing");
    assert!(
        i32::from(red[0]) > i32::from(dark[0]) + 60,
        "the picture's alpha does not glow in the entry's colour: {red:?} against {dark:?}"
    );
    assert!(
        (i32::from(red[1]) - i32::from(dark[1])).abs() <= 2
            && (i32::from(red[2]) - i32::from(dark[2])).abs() <= 2,
        "a red emissive colour must add red only: {red:?} against {dark:?}"
    );
    // The mask is the picture's alpha: none of it, none of the colour.
    assert_eq!(
        lit(0, [1.0, 0.0, 0.0], mode),
        lit(0, [0.0; 3], mode),
        "a zero alpha must add nothing"
    );
    let half = lit(128, [1.0, 0.0, 0.0], mode);
    assert!(
        half[0] > lit(128, [0.0; 3], mode)[0] && half[0] < red[0],
        "half the alpha must add less than all of it: {half:?} against {red:?}"
    );
    // And the term belongs to the mode: the same entry as a plain glow layer
    // on a slot with no `ADD_SECOND` draws nothing extra.
    assert_eq!(
        lit(255, [1.0, 0.0, 0.0], 0.0),
        dark,
        "an entry of mode 0 must not take the picture-alpha term"
    );
}

/// A mode 12 entry whose second lane is `ALPHA_EMISSIVE_SPECULAR_MAP` (`dn_diffuse_specular_emiss`,
/// `mesh::rcs::psp2::alpha_emissive`): the specular is multiplied by the third binding's colour
/// and the picture's alpha, which is the glow mask there, no longer weights it.
#[test]
fn an_alpha_emissive_entry_with_a_specular_map_weights_the_specular_by_its_colour() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let mut scene = Scene::off();
    scene.light.enabled = 1.0;
    // Straight down the view axis: the half vector is the normal.
    scene.light.direction = [0.0, 0.0, 1.0];
    scene.light.sun = [0.6, 0.6, 0.6];
    scene.light.specular_scale = 1.0;
    scene.fog.camera = [0.0, 0.0, 100.5];

    let lit = |map: [u8; 4], flag: f32| {
        let word = slots::DEFAULT | slots::NO_AMBIENT | GLOW_SLOT;
        let mut model = model(&Pad {
            lightmap: None,
            mask: [128, 128, 255, 0],
            tint: [0.0; 3],
            flagged: false,
        });
        // A picture with no alpha: the generic gloss-in-alpha mask would kill the specular.
        model.textures = vec![Some(texture("albedo", [64, 64, 64, 0]))];
        model.pad_masks = vec![Some(texture("specular", map))];
        model.emissive[0].program = [oag_mesh::mesh::PROGRAM_PICTURE_ALPHA_EMISSIVE, flag];
        for vertex in &mut model.vertices {
            vertex.slots = word;
        }
        draw(&device, &queue, &model, scene)
    };
    let flag = oag_mesh::mesh::ALPHA_EMISSIVE_SPECULAR_MAP;
    let white = lit([255, 255, 255, 255], flag);
    let black = lit([0, 0, 0, 255], flag);
    let red = lit([255, 0, 0, 255], flag);
    println!("specular map white {white:?}, black {black:?}, red {red:?}");
    assert!(
        luma(white) > luma(black) + 20.0,
        "a white specular map must leave the highlight, a black one remove it: {white:?} \
         against {black:?}"
    );
    assert!(
        i32::from(red[0]) > i32::from(black[0]) + 20
            && (i32::from(red[1]) - i32::from(black[1])).abs() <= 2,
        "the map's colour tints the highlight: {red:?} against {black:?}"
    );
    // Without the flag the third binding is not a specular map, and the picture's alpha
    // (zero here) weights the specular as it does everywhere else.
    assert_eq!(
        lit([255, 255, 255, 255], 0.0),
        lit([0, 0, 0, 255], 0.0),
        "an unflagged entry must not read its third binding into the specular"
    );
}

/// A sky-reflecting window whose entry carries `program[1]` (`mesh::rcs::window_glass`,
/// `pb_glass_2nduv_reflect_specular1_glow`): the specular is multiplied by the second
/// binding's colour, and the picture's alpha no longer weights it.
#[test]
fn a_windows_specular_map_tints_its_specular() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let mut scene = Scene::off();
    scene.light.enabled = 1.0;
    // Straight down the view axis: the half vector is the normal.
    scene.light.direction = [0.0, 0.0, 1.0];
    scene.light.sun = [0.6, 0.6, 0.6];
    scene.light.specular_scale = 1.0;
    scene.fog.camera = [0.0, 0.0, 100.5];

    let lit = |map: [u8; 4], flagged: bool| {
        let word = slots::DEFAULT | slots::NO_AMBIENT | GLOW_SLOT;
        let mut model = model(&Pad {
            lightmap: Some(map),
            mask: [128, 128, 255, 0],
            tint: [1.0, 0.0, 1.0],
            flagged: false,
        });
        // A picture with no alpha: the generic gloss-in-alpha mask would kill the specular.
        model.textures = vec![Some(texture("albedo", [64, 64, 64, 0]))];
        model.pad_masks = vec![None];
        model.emissive[0].rate = oag_mesh::mesh::WINDOW_REFLECTION_RATE;
        model.emissive[0].program = [0.0, f32::from(u8::from(flagged))];
        for vertex in &mut model.vertices {
            vertex.slots = word;
        }
        draw(&device, &queue, &model, scene)
    };
    let white = lit([255, 255, 255, 255], true);
    let black = lit([0, 0, 0, 255], true);
    let red = lit([255, 0, 0, 255], true);
    println!("specular map white {white:?}, black {black:?}, red {red:?}");
    assert!(
        luma(white) > luma(black) + 20.0,
        "a white specular map must leave the highlight, a black one remove it: {white:?} \
         against {black:?}"
    );
    assert!(
        i32::from(red[0]) > i32::from(black[0]) + 20
            && (i32::from(red[1]) - i32::from(black[1])).abs() <= 2,
        "the map's colour tints the highlight: {red:?} against {black:?}"
    );
    // Without the flag the second binding is not a specular map.
    assert_eq!(
        lit([255, 255, 255, 255], false),
        lit([0, 0, 0, 255], false),
        "an unflagged window must not read its second binding into the specular"
    );
}

fn base_clone(pad: &Pad) -> Pad {
    Pad {
        lightmap: pad.lightmap,
        mask: pad.mask,
        tint: pad.tint,
        flagged: pad.flagged,
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
