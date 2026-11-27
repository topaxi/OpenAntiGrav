//! A PSP `.vex` model samples the mip level the GE's slope rule names, off view
//! depth, and a model without the disc's own chains does not.
//!
//! # Why a pixel
//!
//! `mesh_render::build` decides from the model's textures alone
//! ([`Texels::Chain`]) whether to push the `texlod_slope` and `texlod_bias`
//! pipeline constants, and `mesh.wgsl` has to read them by those names and
//! take `textureSampleLevel` rather than `textureSample`. Any of those dropped
//! leaves the sampler on screen-space derivatives, which for a quad with one
//! constant texture coordinate is level 0 at every depth - so this draws one
//! quad against a three-level chain whose levels are red, green and blue, at
//! four view depths, and reads which level arrived.
//!
//! The flat law under test is `level = log2(w * 1/256) + 1`, clamped at 0 and at
//! the last level: depth 64 is level 0, 256 is level 1, 512 is level 2 and a
//! depth far past the chain stays on level 2. A texture with its own bias takes
//! `log2(2 w / 256) + b` instead, with the camera's term and the sixteenths cut
//! (`docs/rendering/frame-audit.md`, "The level law, measured").
//!
//! # TEXTURE DETAIL
//!
//! The player's preset is the scene uniform's `fog.texlod_shift`, added to that
//! level: `high` is one level less at every depth (the step at double the
//! distance) and `maximum` clamps everything to level 0. One test per preset,
//! each at a depth where it must differ from `original`, so a preset that stops
//! reaching the shader fails its own test.
//!
//! # The hardware path
//!
//! Every other model is sampled at the sampler's own derivative level, where the
//! same uniform is a bias (`texture_lod.wesl`, `detail_shift`). Those tests draw
//! an authored block chain (red, green, blue) minified onto level 2, at no title
//! bias (Omega) and at -1.5 (HD, 2048), through the material fetch and through an
//! effect program's own. A PS2 texture has one level, so a minified one reads the
//! base (`docs/rendering/texture-level-per-title.md`, "Pulse PS2 in a race").
//!
//! Skips when there is no adapter.

use std::sync::Arc;

use oag_mesh::mesh::{
    BlockFormat, Bounds, DrawCall, GpuVertex, Model, ModelTexture, Ps2TextureSet, Texels, slots,
};
use oag_mesh::mesh_render::texlod::{PSP_TEXLOD_FRAME, TEXLOD_CODE_OFFSET};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, TextureDetail, UNIFORMS_SIZE};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

fn vertex(position: [f32; 3], texcoord: [f32; 2]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord,
        lightmap_texcoord: [0.0, 0.0],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        // Emissive, so the light rig is the identity on the albedo and the
        // pixel measures the discard alone - the same reason
        // `zone_recolour.rs` uses it.
        slots: slots::DEFAULT | slots::EMISSIVE,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent: 0,
        depth_bias: 0.0,
    }
}

/// How the test quad maps the texture onto itself.
#[derive(Clone, Copy)]
enum Texcoords {
    /// One coordinate everywhere: no screen-space derivative at all.
    Constant,
    /// Strongly anisotropic: `dpdx` many times `dpdy`.
    Stretched,
    /// The same shape at a fraction of a texel per pixel, so the footprint is
    /// narrower than one texel of level 0 and there is nothing to widen.
    Gentle,
}

/// Everything a draw varies besides the depth.
#[derive(Clone, Copy)]
struct Setup {
    detail: TextureDetail,
    anisotropy: Anisotropy,
    texcoords: Texcoords,
    /// The vertex's per-texture level code (`ModelTexture::lod_code`), 0 for the flat law.
    code: f32,
    /// `Fog::texlod_frame`, the camera's shift of that code's bias.
    frame: f32,
}

impl Setup {
    /// The two things a level is asserted under: no derivative at all, and a
    /// stretched footprint at every anisotropy setting. The level is the slope
    /// law's in every one, so any of them drifting fails the same assertion.
    fn all(detail: TextureDetail) -> Vec<Self> {
        let mut setups = vec![Self {
            detail,
            anisotropy: Anisotropy::Off,
            texcoords: Texcoords::Constant,
            code: 0.0,
            frame: 0.0,
        }];
        setups.extend(
            [
                Anisotropy::Off,
                Anisotropy::X2,
                Anisotropy::X4,
                Anisotropy::X16,
            ]
            .map(|anisotropy| Self {
                detail,
                anisotropy,
                texcoords: Texcoords::Stretched,
                code: 0.0,
                frame: 0.0,
            }),
        );
        setups
    }
}

/// One quad in [`Model::draws`], scaled by `depth` so it still covers the
/// centre pixel once the projection divides by it.
fn model(albedo: Arc<ModelTexture>, depth: f32, texcoords: Texcoords) -> Model {
    let [a, b, c] = match texcoords {
        Texcoords::Constant => [[0.0, 0.0]; 3],
        // Sixteen repeats across the base against a fraction of one down it: a
        // long thin footprint at every depth, so the sampler has an axis to widen.
        Texcoords::Stretched => [[0.0, 0.0], [64.0, 0.0], [32.0, 1.0]],
        Texcoords::Gentle => [[0.0, 0.0], [4.0, 0.0], [2.0, 0.25]],
    };
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "slope lod quad".into(),
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
                radius: 1.5 * depth,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: vec![Some(albedo)],
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
        shield_lit: false,
        own_shield: false,
        material_lod_bias: 0.0,
        material_state: Vec::new(),
        per_batch_cull: false,
        sky_reflection: None,
        alpha_test_ref: None,
        centre: [0.0, 0.0, 0.5],
        radius: 2.0 * depth,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
        vertices: vec![
            vertex([-0.9 * depth, -0.9 * depth, 0.5], a),
            vertex([0.9 * depth, -0.9 * depth, 0.5], b),
            vertex([0.0, 0.9 * depth, 0.5], c),
        ],
    }
}

/// Levels of `4x4`, `2x2` and `1x1`: red, green, blue.
fn levels() -> Vec<Vec<u8>> {
    [
        (16, [255, 0, 0, 255]),
        (4, [0, 255, 0, 255]),
        (1, [0, 0, 255, 255]),
    ]
    .into_iter()
    .map(|(texels, rgba)| rgba.repeat(texels))
    .collect()
}

fn chain() -> Arc<ModelTexture> {
    Arc::new(ModelTexture::chain(
        "red, green, blue".into(),
        4,
        4,
        levels(),
    ))
}

/// The same base level with no chain of the disc's - the other title's shape.
fn plain() -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: "red, base only".into(),
        width: 4,
        height: 4,
        texels: Texels::Rgba8(levels().remove(0)),
        mip_count: None,
        lod_code: 0.0,
    })
}

/// Which level answered at each depth, as the dominant channel of the centre
/// texel: 0 red, 1 green, 2 blue.
fn level_at(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: Arc<ModelTexture>,
    depth: f32,
    setup: Setup,
) -> usize {
    let mut quad = model(texture, depth, setup.texcoords);
    for vertex in &mut quad.vertices {
        vertex.texcoord2[1] = setup.code;
    }
    let image = draw(device, queue, &quad, depth, setup);
    // Dead centre of the triangle.
    let at = (((SIZE / 2) * SIZE + SIZE / 2) * 4) as usize;
    let texel = &image[at..at + 4];
    (0..3).max_by_key(|&channel| texel[channel]).unwrap()
}

fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    Some(
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device"),
    )
}

/// Asserts each `(depth, level)` under `detail`.
fn assert_levels(detail: TextureDetail, expected: &[(f32, usize)]) {
    let Some((device, queue)) = gpu() else { return };
    for setup in Setup::all(detail) {
        for &(depth, level) in expected {
            assert_eq!(
                level_at(&device, &queue, chain(), depth, setup),
                level,
                "{detail} at view depth {depth}, anisotropy {}, {:?} texcoords",
                setup.anisotropy,
                matches!(setup.texcoords, Texcoords::Stretched)
            );
        }
    }
}

#[test]
fn a_chain_is_sampled_at_the_level_the_slope_law_names() {
    assert_levels(
        TextureDetail::Original,
        &[
            (64.0, 0),
            (128.0, 0),
            (256.0, 1),
            (512.0, 2),
            (100_000.0, 2),
        ],
    );
    let Some((device, queue)) = gpu() else { return };
    // The discriminating half: the same quad with no chain of the disc's stays
    // on its one level at every depth, so the levels above were the rule's.
    for depth in [64.0, 512.0, 100_000.0] {
        assert_eq!(
            level_at(
                &device,
                &queue,
                plain(),
                depth,
                Setup::all(TextureDetail::Original)[0]
            ),
            0,
            "a model without Texels::Chain keeps the sampler's own selection"
        );
    }
}

/// The vertex code whose bias at the race camera is `bias` levels.
fn code_for_bias(bias: f32) -> f32 {
    TEXLOD_CODE_OFFSET + bias - PSP_TEXLOD_FRAME
}

/// [`assert_levels`] for a texture with its own bias: each `(depth, level)` at that
/// vertex code and camera shift.
fn assert_own_bias_levels(code: f32, frame: f32, expected: &[(f32, usize)]) {
    let Some((device, queue)) = gpu() else { return };
    for setup in Setup::all(TextureDetail::Original) {
        let setup = Setup {
            code,
            frame,
            ..setup
        };
        for &(depth, level) in expected {
            assert_eq!(
                level_at(&device, &queue, chain(), depth, setup),
                level,
                "code {code} frame {frame} at view depth {depth}, anisotropy {}",
                setup.anisotropy
            );
        }
    }
}

/// `log2(2 w / 256) + b` (`texture_lod.wesl`, `ge_level`). `code` -1 is a bias of exactly 0.
#[test]
fn a_texture_with_its_own_bias_is_sampled_on_the_ges_doubled_law() {
    assert_own_bias_levels(-1.0, 0.0, &[(64.0, 0), (256.0, 1), (512.0, 2)]);
}

#[test]
fn a_textures_bias_moves_the_level_by_that_many_levels() {
    assert_own_bias_levels(
        code_for_bias(1.0),
        0.0,
        &[(64.0, 0), (128.0, 1), (256.0, 2)],
    );
    assert_own_bias_levels(
        code_for_bias(-1.0),
        0.0,
        &[(256.0, 0), (512.0, 1), (1024.0, 2)],
    );
}

#[test]
fn a_camera_shift_moves_the_per_texture_bias() {
    assert_own_bias_levels(
        code_for_bias(0.0),
        1.0,
        &[(64.0, 0), (128.0, 1), (256.0, 2)],
    );
}

/// A bias of -0.9 sixteenths is 0 in the GE's register. At `128 * 2^0.52` the level is
/// then 0.52 (level 1); uncut it would be 0.464 (level 0).
#[test]
fn the_ge_cuts_a_bias_to_sixteenths_toward_zero() {
    let depth = 128.0 * 0.52f32.exp2();
    assert_own_bias_levels(code_for_bias(-0.9 / 16.0), 0.0, &[(depth, 1)]);
}

#[test]
fn high_texture_detail_doubles_the_depth_of_each_level_step() {
    // Original steps at 256 and 512; high at 512 and 1024. Depth 256 and 512
    // are where it differs from `original`, 1024 where it reaches the last level.
    assert_levels(
        TextureDetail::High,
        &[
            (64.0, 0),
            (256.0, 0),
            (512.0, 1),
            (1024.0, 2),
            (100_000.0, 2),
        ],
    );
}

#[test]
fn maximum_texture_detail_is_level_zero_at_every_depth() {
    assert_levels(
        TextureDetail::Maximum,
        &[(64.0, 0), (512.0, 0), (100_000.0, 0)],
    );
}

/// With one coordinate everywhere there is no derivative, so the sampler's own level is the
/// base whatever the preset shifts it by.
#[test]
fn a_constant_coordinate_without_a_chain_reads_the_base_at_every_preset() {
    let Some((device, queue)) = gpu() else { return };
    for detail in TextureDetail::ALL {
        assert_eq!(
            level_at(&device, &queue, plain(), 512.0, Setup::all(detail)[0]),
            0
        );
    }
}

#[test]
fn every_preset_round_trips_its_name_and_original_is_the_default() {
    for detail in TextureDetail::ALL {
        assert_eq!(detail.name().parse::<TextureDetail>(), Ok(detail));
    }
    assert_eq!(TextureDetail::default(), TextureDetail::Original);
    assert_eq!(TextureDetail::Original.level_shift(), 0.0);
    assert_eq!(TextureDetail::High.level_shift(), -1.0);
    assert!(TextureDetail::Maximum.level_shift() <= -32.0);
}

/// Identity camera and model, so clip space is model space.
fn uniforms(depth: f32) -> Vec<u8> {
    // Column 3 is (0, 0, 0, depth), so `clip.w` - the view depth the shader reads - is `depth`.
    let projection: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, depth],
    ];
    let identity: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::cast_slice(&projection)); // view_projection
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // model
    bytes.extend_from_slice(&[0u8; 32]); // the four pad floats, then the environment-probe layer and its padding
    bytes.extend_from_slice(&[0u8; 32]); // hull point lights
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // prev_mvp
    assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);
    bytes
}

/// Draws `model`'s one opaque call into a `SIZE`x`SIZE` target cleared to
/// transparent black, and answers every texel - so a discarded fragment
/// reads as the clear and a kept one does not.
fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    depth: f32,
    setup: Setup,
) -> Vec<u8> {
    let built = mesh_render::build(
        device,
        queue,
        model,
        FORMAT,
        setup.anisotropy,
        1,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        // `Written`, so a kept fragment's alpha of 1.0 reaches the frame and
        // "drawn" and "discarded" differ in every channel rather than in three.
        mesh_render::GlowMask::Written,
        mesh_render::Velocity::None,
        &mesh_render::zone::StageArt::NONE,
        // No shadow map, no depth map and no receiver: this test draws one
        // model against nothing.
        mesh_render::ShadowMaps::NONE,
        mesh_render::ShadowReceiver::Never,
    )
    .expect("building the mesh pipeline");
    let mut scene = Scene::off();
    scene.fog.texlod_shift = setup.detail.level_shift();
    scene.fog.texlod_frame = setup.frame;
    queue.write_buffer(&built.fog_buffer, 0, bytemuck::bytes_of(&scene));

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("slope uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buffer, 0, &uniforms(depth));
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("slope uniforms"),
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
            label: Some("slope"),
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
        // Slot 1: the model's own first texture. Slot 0 is `build`'s white 1x1.
        pass.set_bind_group(1, &built.texture_binds[1], &[]);
        pass.set_bind_group(2, &built.fog_bind, &[0]);
        pass.set_bind_group(3, &built.anim_bind, &[]);
        pass.set_vertex_buffer(0, built.geometry.sole().slice(..));
        pass.set_index_buffer(built.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..3, 0, 0..1);
    }

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("slope readback"),
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
    mapped[..].to_vec()
}

/// Level 0 is alternating black and white columns; the levels under it are the
/// mid grey a box filter makes of them.
fn striped() -> Arc<ModelTexture> {
    let column = |x: usize| if x.is_multiple_of(2) { 0 } else { 255 };
    let base: Vec<u8> = (0..16)
        .flat_map(|texel| [column(texel % 4); 3].into_iter().chain([255]))
        .collect();
    Arc::new(ModelTexture::chain(
        "stripes".into(),
        4,
        4,
        vec![
            base,
            [128, 128, 128, 255].repeat(4),
            vec![128, 128, 128, 255],
        ],
    ))
}

/// How many pixels of the drawn triangle differ between anisotropy off and 16x.
/// `None` with no GPU adapter, so a caller skips like every other test here
/// rather than reading a sentinel as a pixel count.
fn pixels_moved_by_anisotropy(texcoords: Texcoords, depth: f32) -> Option<usize> {
    let (device, queue) = gpu()?;
    let with = |anisotropy| {
        draw(
            &device,
            &queue,
            &model(striped(), depth, texcoords),
            depth,
            Setup {
                detail: TextureDetail::Original,
                anisotropy,
                texcoords,
                code: 0.0,
                frame: 0.0,
            },
        )
    };
    let (off, sixteen) = (with(Anisotropy::Off), with(Anisotropy::X16));
    Some(
        off.chunks(4)
            .zip(sixteen.chunks(4))
            .filter(|(a, b)| a.iter().zip(*b).any(|(a, b)| a.abs_diff(*b) > 2))
            .count(),
    )
}

#[test]
fn anisotropy_widens_the_footprint_where_the_surface_is_foreshortened() {
    // Level 0 at depth 64, four texels a pixel along the long axis: sixteen
    // stripes' worth of aliasing that only the probes along it can average.
    let Some(moved) = pixels_moved_by_anisotropy(Texcoords::Stretched, 64.0) else {
        return;
    };
    assert!(
        moved > 100,
        "16x must change a foreshortened surface at the slope law's own level"
    );
}

#[test]
fn anisotropy_leaves_a_footprint_narrower_than_a_texel_alone() {
    // The same shape at a tenth of a texel a pixel: nothing to widen, and the
    // sampler must not blur a magnified surface for the sake of a ratio.
    let Some(moved) = pixels_moved_by_anisotropy(Texcoords::Gentle, 64.0) else {
        return;
    };
    assert_eq!(moved, 0);
}

/// A 16x16 base of one-texel red and blue stripes across `u`, with no chain of
/// the disc's: the renderer generates the levels, and level 1 onwards is the
/// purple average of the pair, so the stripes' contrast is how fine a level
/// arrived.
fn stripes() -> Arc<ModelTexture> {
    let texels = (0..16 * 16)
        .flat_map(|i| {
            if i % 2 == 0 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            }
        })
        .collect();
    Arc::new(ModelTexture {
        label: "red and blue stripes".into(),
        width: 16,
        height: 16,
        texels: Texels::Rgba8(texels),
        mip_count: None,
        lod_code: 0.0,
    })
}

/// The strongest red-against-blue contrast along the centre row of a draw of
/// [`stripes`] minified to about two texels a pixel (level 1 of the sampler's
/// own derivative selection), at `bias` levels of `Model::material_lod_bias`.
fn stripe_contrast(bias: f32) -> i32 {
    let Some((device, queue)) = gpu() else {
        return -1;
    };
    let depth = 64.0;
    let mut quad = model(stripes(), depth, Texcoords::Constant);
    for (vertex, uv) in quad
        .vertices
        .iter_mut()
        .zip([[0.0, 0.0], [8.0, 0.0], [4.0, 8.0]])
    {
        vertex.texcoord = uv;
    }
    quad.material_lod_bias = bias;
    let setup = Setup::all(TextureDetail::Original)[0];
    let image = draw(&device, &queue, &quad, depth, setup);
    let row = (SIZE / 2) * SIZE;
    ((SIZE / 2 - 8)..(SIZE / 2 + 8))
        .map(|x| {
            let texel = &image[((row + x) * 4) as usize..][..4];
            (i32::from(texel[0]) - i32::from(texel[2])).abs()
        })
        .max()
        .unwrap()
}

#[test]
fn a_material_lod_bias_reads_a_finer_level_than_the_samplers_own() {
    let plain = stripe_contrast(0.0);
    if plain < 0 {
        return;
    }
    let biased = stripe_contrast(-1.5);
    assert!(
        plain < 20,
        "with no bias the minified stripes average to purple, contrast {plain}"
    );
    assert!(
        biased > 80,
        "a bias of -1.5 reaches the stripes themselves, contrast {biased} against {plain}"
    );
}

/// A device that samples the block-compressed chain below, or `None` with no
/// adapter that can: without the feature the renderer decodes level 0 alone.
fn gpu_with_blocks() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    if !adapter
        .features()
        .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
    {
        eprintln!("no BC sampling: skipping");
        return None;
    }
    Some(
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::TEXTURE_COMPRESSION_BC,
            ..Default::default()
        }))
        .expect("requesting the device"),
    )
}

/// A 16x16 DXT1 texture authoring red, green and blue levels, as a `.gtf` carries its chain.
fn authored_chain() -> Arc<ModelTexture> {
    let level = |rgb565: u16, blocks: usize| {
        let [low, high] = rgb565.to_le_bytes();
        [low, high, low, high, 0, 0, 0, 0].repeat(blocks)
    };
    Arc::new(ModelTexture {
        label: "red, green, blue blocks".into(),
        width: 16,
        height: 16,
        texels: Texels::Blocks {
            format: BlockFormat::Bc1,
            levels: vec![level(0xf800, 16), level(0x07e0, 4), level(0x001f, 1)],
        },
        mip_count: None,
        lod_code: 0.0,
    })
}

/// Maps `quad` so the sampler's own selection is `level`: the texture is 16 texels
/// across and the target [`SIZE`] pixels across clip space, so a coordinate of
/// `k` per clip unit is `k / 2` texels a pixel. `offset` moves the coordinate.
fn minify(quad: &mut Model, depth: f32, level: f32, offset: [f32; 2]) {
    let k = 2.0 * level.exp2();
    for vertex in &mut quad.vertices {
        vertex.texcoord = [
            k * vertex.position[0] / depth + offset[0],
            k * vertex.position[1] / depth + offset[1],
        ];
    }
}

/// The level each preset reads where the sampler alone would read `level`, with
/// `change` made to the model.
fn hardware_levels(level: f32, change: impl Fn(&mut Model)) -> Option<[usize; 3]> {
    let (device, queue) = gpu_with_blocks()?;
    let depth = 64.0;
    Some(TextureDetail::ALL.map(|detail| {
        let mut quad = model(authored_chain(), depth, Texcoords::Constant);
        minify(&mut quad, depth, level, [0.0; 2]);
        change(&mut quad);
        let setup = Setup::all(detail)[0];
        let image = draw(&device, &queue, &quad, depth, setup);
        let at = (((SIZE / 2) * SIZE + SIZE / 2) * 4) as usize;
        (0..3)
            .max_by_key(|&channel| image[at + channel])
            .expect("three channels")
    }))
}

/// `original`, `high`, `maximum`: level 2, one finer, the base. No title bias,
/// which is 2048 and Omega.
#[test]
fn texture_detail_shifts_the_samplers_own_level() {
    let Some(levels) = hardware_levels(2.0, |_| {}) else {
        return;
    };
    assert_eq!(levels, [2, 1, 0]);
}

/// HD's -1.5 and the preset add: a footprint 1.5 levels coarser reads the same three.
#[test]
fn texture_detail_adds_to_a_titles_own_material_bias() {
    let Some(levels) = hardware_levels(3.5, |quad| quad.material_lod_bias = -1.5) else {
        return;
    };
    assert_eq!(levels, [2, 1, 0]);
}

/// An effect program's fetch (the absorb shell's) takes the preset and not the
/// title's bias.
#[test]
fn texture_detail_reaches_an_effect_programs_own_fetch() {
    let Some(levels) = hardware_levels(2.0, |quad| {
        quad.absorb_shell = true;
        quad.material_lod_bias = -1.5;
    }) else {
        return;
    };
    assert_eq!(levels, [2, 1, 0]);
}

/// A scope whose title biases every texture (2048) reaches a material fetch and an effect
/// program's own fetch of a model that asks for no bias itself: a footprint 1.5 levels
/// coarser reads the same three presets as an unbiased one. HD's case is the test above,
/// where the model's bias stays off the effect fetch.
#[test]
fn an_every_texture_bias_reaches_material_and_effect_fetches() {
    for effect in [false, true] {
        // One scope a device: `hardware_levels` makes its own.
        let scope = mesh_render::BuildCacheScope::open();
        scope.every_texture_lod_bias(-1.5);
        let Some(levels) = hardware_levels(3.5, |quad| quad.absorb_shell = effect) else {
            return;
        };
        assert_eq!(levels, [2, 1, 0], "absorb shell: {effect}");
    }
}

/// One PS2 texture upload of a 16x16 one-texel checkerboard, black and white, in
/// the shape `oag_texture::ps2_texture` reads (raster-order `PSMT8`), alone in a
/// nested set under `name`.
fn ps2_set(name: &str) -> Vec<u8> {
    let mut blob = vec![0x44, 8, 0x00, 0x20];
    blob.extend_from_slice(&16u16.to_le_bytes());
    blob.extend_from_slice(&16u16.to_le_bytes());
    blob.resize(13 + 8 * 16, 0);
    let quad = |low: u64, high: u64, out: &mut Vec<u8>| {
        out.extend_from_slice(&low.to_le_bytes());
        out.extend_from_slice(&high.to_le_bytes());
    };
    let image_tag = |bytes: u64, last: u64| (bytes / 16) | (last << 15) | (2 << 58);
    quad(0, 0x51, &mut blob);
    quad(16 | (16 << 32), 0x52, &mut blob);
    quad(0, 0x53, &mut blob);
    quad(image_tag(256, 0), 0, &mut blob);
    blob.extend((0..256).map(|texel: usize| ((texel % 16 + texel / 16) % 2) as u8));
    quad(0, 0x51, &mut blob);
    quad(16 | (16 << 32), 0x52, &mut blob);
    quad(0, 0x53, &mut blob);
    quad(image_tag(1024, 1), 0, &mut blob);
    // Entry 0 black, entry 1 white, both at the GS's full alpha.
    let mut palette = vec![0u8; 1024];
    palette[..8].copy_from_slice(&[0, 0, 0, 128, 255, 255, 255, 128]);
    blob.extend(palette);

    let mut set = Vec::new();
    for word in [
        1,
        1,
        oag_formats::wad::hash_name(name),
        24,
        blob.len() as u32,
        blob.len() as u32,
    ] {
        set.extend_from_slice(&word.to_le_bytes());
    }
    set.extend(blob);
    set
}

/// A PS2 texture's header authors no level beyond its base, so a distant texel is
/// a texel of the base: here the black one or the white one the sample lands on,
/// never the average a built chain holds.
#[test]
fn a_distant_ps2_texel_reads_the_base_level() {
    let Some((device, queue)) = gpu() else { return };
    let set = Ps2TextureSet::parse(&ps2_set("check.pct")).expect("the set parses");
    let texture = set.resolve("check.pct").expect("the texture resolves");
    let depth = 64.0;
    // Red at the centre of a draw four texels a pixel, the coordinate shifted onto a
    // texel's centre and then `step` texels across.
    let centre = |texture: Arc<ModelTexture>, step: f32| {
        let mut quad = model(texture, depth, Texcoords::Constant);
        minify(&mut quad, depth, 2.0, [(0.5 + step) / 16.0, 0.5 / 16.0]);
        let setup = Setup::all(TextureDetail::Original)[0];
        let image = draw(&device, &queue, &quad, depth, setup);
        image[(((SIZE / 2) * SIZE + SIZE / 2) * 4) as usize]
    };
    let base = [0.0, 1.0].map(|step| centre(Arc::clone(&texture), step));
    let (dark, light) = (base[0].min(base[1]), base[0].max(base[1]));
    assert!(
        dark < 8 && light > 100,
        "one texel apart reads the two colours of the base: {base:?}"
    );
    // The same picture with no level count gets a built chain, which reads neither.
    let chained = Arc::new(ModelTexture {
        mip_count: None,
        ..(*texture).clone()
    });
    let averaged = [0.0, 1.0].map(|step| centre(Arc::clone(&chained), step));
    assert!(
        averaged
            .iter()
            .all(|&red| red > dark + 30 && red + 30 < light),
        "the control must average: {averaged:?} against {base:?}"
    );
}
