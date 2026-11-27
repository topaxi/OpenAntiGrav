//! Omega's reflective sea (`reflectplane_mid` and its siblings) through the mesh pipeline:
//! `shade.wesl` reads the two opposite taps, scrolls them by `Speed * time`, and weights the
//! reflection by the program's Fresnel (`docs/rendering/water.md`, "Omega (PS4)"), on a fixture
//! needing no game content.
//!
//! ```text
//! NormalMap  at (u + K Speed t, v)        NormalMap1 at (K Speed t - u, v)
//! reflection weight = Strength * (Min + Scale * sat(1 - N.V)^Power)
//! ```
//!
//! The entry is the one `mesh::rcs::reflective_block` lays. Drop the second tap, give it the
//! first's sign, stop the scroll, or change a term of the weight, and a reading below moves.
//!
//! Skips when there is no adapter. Runs in `just test`.

use std::sync::Arc;

use oag_mesh::mesh::rcs::{Fresnel, OMEGA_EDGE_KIND, OMEGA_SEA_KIND, Tap, reflective_block};
use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;
const ENTRY: u32 = 1 << slots::MATERIAL_SHIFT;
/// Tangent `+x`, bitangent sign `+1`.
const TANGENT: u32 = u32::from_le_bytes([255, 128, 128, 255]);
/// The literal of the pixel programs (`oag_rcs::gcn::SeaTaps::rate`) and the signs of `u`.
const RATE: f32 = 0.05;
const SIGNS: [f32; 2] = [1.0, -1.0];
/// The first texel's centre of the four-texel wave map.
const U: f32 = 0.125;

fn vertex(position: [f32; 3], chain: [f32; 2]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [U, 0.5],
        lightmap_texcoord: [0.5, 0.5],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT | ENTRY,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: chain,
        tangent: TANGENT,
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
    }))
}

/// What one sea triangle is drawn with.
#[derive(Clone, Copy)]
struct Sea {
    kind: f32,
    colour: f32,
    opacity: f32,
    speed: f32,
    /// The second tap's weight: 1 as the program sums it, 0 to drop it.
    second: f32,
    signs: [f32; 2],
    fresnel: Fresnel,
    /// `VertexColour1.xy`.
    chain: [f32; 2],
}

const MATTE: Fresnel = Fresnel {
    min: 0.0,
    scale: 0.0,
    power: 1.0,
    strength: 0.0,
};

/// A lit sea with no reflection: the diffuse term shows which way the summed normal leans.
const LIT: Sea = Sea {
    kind: OMEGA_SEA_KIND,
    colour: 0.5,
    opacity: 1.0,
    speed: 0.5,
    second: 1.0,
    signs: SIGNS,
    fresnel: MATTE,
    chain: [0.0, 1.0],
};

/// The wave map: its first texel leans the normal along `+x` (`x` in alpha, `y` in green, as
/// the program decodes it), the other three are flat.
fn model(sea: Sea) -> Model {
    let mut model = Model::none("omega sea lit-path triangle");
    model.indices = vec![0, 1, 2];
    model.transparent_draws = vec![DrawCall {
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
    model.textures = vec![texture("unread", &[[128, 128, 128, 255]])];
    model.lightmaps = vec![None];
    let flat = [128, 128, 128, 128];
    model.pad_masks = vec![texture(
        "wave map",
        &[[128, 128, 128, 218], flat, flat, flat],
    )];
    model.vertices = vec![
        vertex([-0.9, -0.9, 0.5], sea.chain),
        vertex([0.9, -0.9, 0.5], sea.chain),
        vertex([0.0, 0.9, 0.5], sea.chain),
    ];
    let mut taps = Tap::reflective(RATE, sea.signs, sea.speed);
    taps[1].weight = sea.second;
    model.emissive = reflective_block(
        sea.kind,
        [sea.colour; 3],
        sea.opacity,
        250.0,
        [0.0; 3],
        [&taps[0], &taps[1]],
        sea.fresnel,
    );
    model
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    sky: wgpu::TextureView,
}

fn gpu(label: &str) -> Option<Gpu> {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    let (device, queue) = pollster::block_on(
        adapter.request_device(&mesh_render::device_descriptor(label, &adapter)),
    )
    .expect("requesting the device");
    let (width, height) = oag_mesh::mesh::sky_paraboloid::SIZE;
    let grey = vec![200u8; (width * height * 4) as usize];
    let sky = mesh_render::upload_sky_probe(&device, &queue, &grey);
    Some(Gpu { device, queue, sky })
}

impl Gpu {
    /// The red of the triangle's centre, lit along `+x` (so the diffuse term is the mapped
    /// normal's lean), seen from `eye` at `time` seconds.
    fn read(&self, sea: Sea, time: f32, eye: [f32; 3]) -> [u8; 4] {
        let mut scene = Scene::off();
        scene.light = mesh_render::Light::authored(
            [1.0, 0.0, 0.0],
            [1.0; 3],
            [0.0; 3],
            [1.0; 3],
            [1.0; 3],
            0.0,
        );
        scene.fog.camera = eye;
        scene.time[0] = time;
        draw(&self.device, &self.queue, &model(sea), scene, &self.sky)
    }
}

/// How long the taps take to cross a quarter of the map at [`LIT`]'s speed.
const QUARTER: f32 = 0.25 / (RATE * 0.5);
const ABOVE: [f32; 3] = [0.0, 0.0, 100.5];
/// Sixty degrees off the normal: `N.V = 0.5`.
const OBLIQUE: [f32; 3] = [86.602_54, 0.0, 50.5];

fn near(got: u8, want: u8, what: &str) {
    assert!(got.abs_diff(want) <= 3, "{what}: {got}, want {want}");
}

/// At `t = 0` the first tap reads the leaning texel at `u` and the second the flat one at
/// `-u`. A quarter of the map later (`K Speed t = 0.25`) they have swapped: the first reads a
/// flat texel at `u + 0.25`, the second the leaning one at `0.25 - u`, so the sum is the same.
/// At half the map both read flat texels and the lean is gone.
#[test]
fn the_two_taps_run_opposite_ways_at_speed_times_time() {
    let Some(gpu) = gpu("omega sea taps") else {
        return;
    };
    let lit = |sea: Sea, time: f32| gpu.read(sea, time, ABOVE)[0];
    let quarter = QUARTER;
    let start = lit(LIT, 0.0);
    let flat = lit(LIT, 2.0 * quarter);
    assert!(start > 40, "the sea draws nothing: {start}");
    assert!(
        start > flat + 24,
        "the scroll moves the lean off the taps: {start} at 0, {flat} half a map later"
    );
    near(
        lit(LIT, quarter),
        start,
        "a quarter map later the taps have swapped",
    );
    // `Speed` scales the clock: twice the speed reaches the same place in half the time.
    let fast = Sea { speed: 1.0, ..LIT };
    near(lit(fast, quarter), flat, "Speed times time");
    near(
        lit(Sea { speed: 0.0, ..LIT }, 2.0 * quarter),
        start,
        "no Speed, no scroll",
    );
    // Two taps, not one: without the second the normal leans further.
    let one = lit(Sea { second: 0.0, ..LIT }, 0.0);
    assert!(
        one > start + 8,
        "the second tap is summed: one tap {one}, two {start}"
    );
    // The second tap's `u` is negated: with the first's sign it would read the leaning texel
    // too and the pair would lean as far as one tap alone.
    let same = lit(
        Sea {
            signs: [1.0, 1.0],
            ..LIT
        },
        0.0,
    );
    near(same, one, "both taps on the leaning texel lean as one");
    assert!(
        same > start + 8,
        "the taps' signs differ: same {same}, opposite {start}"
    );
}

/// The reflection of a flat grey sky, with no light on the sea: every route to the same weight
/// reads the same, and the weight is zero where the program's is.
#[test]
fn the_reflection_weight_is_strength_times_min_plus_scale_times_facing_to_the_power() {
    let Some(gpu) = gpu("omega sea fresnel") else {
        return;
    };
    let reflect = |fresnel: Fresnel, eye: [f32; 3]| {
        let sea = Sea {
            colour: 0.0,
            fresnel,
            ..LIT
        };
        // Half a map along both taps read flat texels, so the normal is the vertex normal.
        gpu.read(sea, 2.0 * QUARTER, eye)[0]
    };
    let fresnel = |min, scale, power, strength| Fresnel {
        min,
        scale,
        power,
        strength,
    };
    // Each of these is 0.3 by `Fresnel::weight`; `facing` is 0.5 from `OBLIQUE`, 0 from above.
    let want = reflect(fresnel(0.3, 0.0, 1.0, 1.0), ABOVE);
    assert!(want > 40, "the sky is not reflected: {want}");
    for (what, f, eye) in [
        ("Min alone, oblique", fresnel(0.3, 0.0, 1.0, 1.0), OBLIQUE),
        (
            "Scale times facing squared",
            fresnel(0.0, 1.2, 2.0, 1.0),
            OBLIQUE,
        ),
        ("the power", fresnel(0.0, 0.6, 1.0, 1.0), OBLIQUE),
        (
            "Min plus the facing term",
            fresnel(0.15, 0.6, 2.0, 1.0),
            OBLIQUE,
        ),
        (
            "Strength times the sum",
            fresnel(0.3, 1.2, 2.0, 0.5),
            OBLIQUE,
        ),
        ("Strength times Min", fresnel(0.6, 9.0, 2.0, 0.5), ABOVE),
    ] {
        assert!((f.weight(if eye == ABOVE { 0.0 } else { 0.5 }) - 0.3).abs() < 1e-6);
        near(reflect(f, eye), want, what);
    }
    // Face on, the facing term is zero; with no strength nothing is reflected.
    near(
        reflect(fresnel(0.0, 5.0, 2.0, 1.0), ABOVE),
        0,
        "face on, Scale adds nothing",
    );
    near(
        reflect(fresnel(0.3, 1.0, 2.0, 0.0), OBLIQUE),
        0,
        "no strength",
    );
}

/// The edge's alpha is `VertexColour1.y * Opacity` (`reflectplane_edge`), the map's alpha and
/// `VertexColour1.x` unread: drawn blended over a clear target, a pixel is the opaque sea's
/// times it.
#[test]
fn the_edge_alpha_is_vertex_colour_y_times_opacity() {
    let Some(gpu) = gpu("omega sea edge") else {
        return;
    };
    let reference = gpu.read(LIT, 0.0, ABOVE)[0];
    let alpha = |opacity: f32, chain: [f32; 2]| {
        let edge = Sea {
            kind: OMEGA_EDGE_KIND,
            opacity,
            chain,
            ..LIT
        };
        f32::from(gpu.read(edge, 0.0, ABOVE)[0]) / f32::from(reference)
    };
    for (opacity, chain, want) in [
        (1.0, [0.0, 1.0], 1.0),
        (1.0, [1.0, 0.5], 0.5),
        (0.5, [0.0, 0.5], 0.25),
        (1.0, [1.0, 0.0], 0.0),
    ] {
        let got = alpha(opacity, chain);
        assert!(
            (got - want).abs() < 0.04,
            "Opacity {opacity}, VertexColour1 {chain:?}: alpha {got}, want {want}"
        );
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
fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    scene: Scene,
    sky: &wgpu::TextureView,
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
            sky_probe: Some(sky),
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
