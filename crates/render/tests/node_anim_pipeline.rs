//! The `Anim Transform` node matrix reaches the GPU and moves geometry.
//!
//! Runs on a real device, needs no game content, and runs in CI. This is the
//! half a data-level test cannot cover: that `mesh.wgsl` reads the table from
//! binding 1 of group 3 and applies it, and that the vertex layout carries
//! `GpuVertex::xform` on **every** pipeline rather than only the opaque one.
//!
//! The failure it exists to catch is silent. A vertex layout missing attribute
//! 6, or a bind group whose second entry never made it into the layout, leaves
//! every scenery animation frozen while every data-level assertion stays green -
//! which is exactly what happened to the texture-transform port's first
//! attempt.

use oag_mesh::mesh::{AnimNode, AnimTrack, Bounds, DrawCall, GpuVertex, Model, Motion};
use oag_vex::vex;

/// A quad, in the space of an `Anim Transform` that slides it along `z`.
///
/// **In the X=0 plane**, because `mesh_render`'s own capture camera sits at
/// `(distance, 0, 0)` looking at the origin for `yaw = pitch = 0` - a quad in
/// Z=0 is edge-on to it and covers no pixels at all. `z` is therefore the axis
/// that moves the quad *across* the frame rather than toward the lens.
fn moving_quad(xform: u32) -> Model {
    let corner = |y: f32, z: f32| GpuVertex {
        position: [0.0, y, z],
        normal: [1.0, 0.0, 0.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        // Prelit, so the result does not depend on a light rig.
        lit: 0.0,
        anim: 0,
        lightmap_texcoord: [0.0, 0.0],
        xform,
        sun_mask: 1.0,
        slots: oag_mesh::mesh::slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    };
    // Two keys, one second apart: `z` runs 0 to 3 over 60 frames.
    let translation = vex::AnimChannel {
        times: vec![0, 60],
        values: vec![[0.0, 0.0, 0.0], [0.0, 0.0, 3.0]],
        w: Vec::new(),
        wide: false,
    };
    Model {
        label: "moving quad".to_string(),
        vertices: vec![
            corner(-1.0, -1.0),
            corner(1.0, -1.0),
            corner(1.0, 1.0),
            corner(-1.0, 1.0),
        ],
        // Both windings, so the test does not depend on which way the quad
        // faces - `culled` is false, but the two-sided pipeline is the point
        // of the check, not the check itself.
        indices: vec![0, 1, 2, 0, 2, 3, 2, 1, 0, 3, 2, 0],
        draws: vec![DrawCall {
            range: 0..12,
            texture: None,
            bounds: Bounds {
                centre: [0.0; 3],
                radius: 16.0,
            },
            moving: xform != 0,
            culled: false,
            blend: None,
            blend_state: None,
            layer: vex::LAYER_DEFAULT,
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: Vec::new(),
        centre: [0.0; 3],
        // A fixed radius, so the capture's own framing does not move with the
        // quad and turn a translation into no change on screen. Wide enough
        // that three units of travel stays inside the frame.
        radius: 6.0,
        mesh_count: 1,
        airbrakes: [None, None],
        anim_tracks: Vec::new(),
        lightmaps: Vec::new(),
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
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        emissive: Vec::new(),
        anim_nodes: vec![AnimNode {
            transform: Motion::Vex(Box::new(vex::AnimTransform {
                translation,
                translation_quantum: [1.0, 1.0, 1.0],
                translation_base: [0.0, 0.0, 0.0],
                seconds_per_key: 1.0 / 60.0,
                loop_seconds: vex::DEFAULT_LOOP_SECONDS,
                ..vex::AnimTransform::default()
            })),
            static_above: vex::IDENTITY,
            parent: None,
        }],
    }
}

/// The centroid of every pixel brighter than the background, in pixels.
fn bright_centroid(pixels: &[u8], width: u32, height: u32) -> Option<(f32, f32)> {
    let (mut sx, mut sy, mut n) = (0.0f64, 0.0f64, 0usize);
    for y in 0..height {
        for x in 0..width {
            let at = ((y * width + x) * 4) as usize;
            let lum = u32::from(pixels[at]) + u32::from(pixels[at + 1]) + u32::from(pixels[at + 2]);
            if lum > 96 {
                sx += f64::from(x);
                sy += f64::from(y);
                n += 1;
            }
        }
    }
    (n > 0).then(|| ((sx / n as f64) as f32, (sy / n as f64) as f32))
}

fn capture(model: &Model, seconds: f32) -> Option<Vec<u8>> {
    oag_mesh::mesh_render::capture_pixels_from(
        model,
        128,
        128,
        0.0,
        0.0,
        oag_mesh::mesh_render::Anisotropy::Off,
        seconds,
    )
    .ok()
}

/// The quad is somewhere else after one second, and it is the node table that
/// moved it.
///
/// The control is the same geometry with `xform` of `0`: identical vertices,
/// identical clock, and it must not move at all. Without it a test like this
/// passes on any per-frame change at all - a camera drift, a fog write - rather
/// than on the thing it names.
#[test]
fn the_node_table_moves_geometry_and_slot_zero_does_not() {
    let moving = moving_quad(1);
    let Some(first) = capture(&moving, 0.0) else {
        println!("skipping: no GPU adapter");
        return;
    };
    let later = capture(&moving, 1.0).expect("the device worked a moment ago");

    let a = bright_centroid(&first, 128, 128).expect("the quad is drawn at all");
    let b = bright_centroid(&later, 128, 128).expect("the quad is still drawn");
    assert!(
        (b.0 - a.0).abs() > 8.0,
        "the quad did not move: {a:?} then {b:?} - the shader is not reading the node table"
    );

    // Same model, same clock, no anim slot.
    let still = moving_quad(0);
    let first = capture(&still, 0.0).expect("device");
    let later = capture(&still, 1.0).expect("device");
    let a = bright_centroid(&first, 128, 128).expect("drawn");
    let b = bright_centroid(&later, 128, 128).expect("drawn");
    assert!(
        (b.0 - a.0).abs() < 0.5 && (b.1 - a.1).abs() < 0.5,
        "slot 0 is meant to be the identity, but the quad moved: {a:?} then {b:?}"
    );
}

/// The table is applied on the **transparent** pipelines too, not only the
/// opaque one.
///
/// Each pipeline declares its own vertex layout in `mesh_render::build`, so
/// attribute 6 has to be on all of them. `col_arrows1_GLOW_ADD` - the surface
/// that prompted the texture-transform port - is additive, and trackside
/// scenery is full of blended geometry, so a layout that only carries `xform`
/// on the opaque pipeline would freeze exactly the objects most likely to be
/// noticed.
#[test]
fn a_transparent_draw_moves_too() {
    let mut model = moving_quad(1);
    let draw = model.draws.remove(0);
    model.transparent_draws.push(DrawCall {
        blend: Some(vex::BlendClass::Additive),
        ..draw
    });

    let Some(first) = capture(&model, 0.0) else {
        println!("skipping: no GPU adapter");
        return;
    };
    let later = capture(&model, 1.0).expect("the device worked a moment ago");
    let a = bright_centroid(&first, 128, 128).expect("the quad is drawn at all");
    let b = bright_centroid(&later, 128, 128).expect("the quad is still drawn");
    assert!(
        (b.0 - a.0).abs() > 8.0,
        "an additive draw did not move: {a:?} then {b:?}"
    );
}

/// The **texture** transform reaches the shader too, and a scrolled surface
/// changes colour.
///
/// The sibling check to the two above, and the one that was previously
/// established by *reading* `mesh_render::build` rather than by rendering.
/// `TexAnims::sample` moving is asserted by `authored_uv_ground_truth.rs`; that
/// the sampled table then reaches `mesh.wgsl` and moves a texture coordinate is
/// this test's job. A layout or binding fault here freezes every banner and
/// arrow on the disc while both data-level tests stay green - which is exactly
/// the shape of "the banners do not marquee".
#[test]
fn the_texture_table_scrolls_a_surface() {
    // Two texels side by side: black at u < 0.5, white at u >= 0.5.
    let texture = oag_mesh::mesh::ModelTexture::rgba8(
        "half and half".to_string(),
        2,
        1,
        vec![0, 0, 0, 255, 255, 255, 255, 255],
        None,
    );
    // A quad that samples only the black texel, until a `u` offset slides it.
    let corner = |y: f32, z: f32, u: f32| GpuVertex {
        position: [0.0, y, z],
        normal: [1.0, 0.0, 0.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [u, 0.5],
        lit: 0.0,
        anim: 1,
        lightmap_texcoord: [0.0, 0.0],
        xform: 0,
        sun_mask: 1.0,
        slots: oag_mesh::mesh::slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    };
    let mut model = moving_quad(0);
    model.vertices = vec![
        corner(-1.0, -1.0, 0.0),
        corner(1.0, -1.0, 0.0),
        corner(1.0, 1.0, 0.24),
        corner(-1.0, 1.0, 0.24),
    ];
    model.textures = vec![Some(std::sync::Arc::new(texture))];
    model.draws[0].texture = Some(0);
    model.anim_nodes = Vec::new();
    // `u` runs 0 to 0.5 over one second, which walks the quad off the black
    // texel and onto the white one.
    model.anim_tracks = vec![AnimTrack::Psp(vex::TexTransform {
        offset: vex::TexTransformTrack {
            times: vec![0, 60],
            values: vec![(0, 0), (128, 0)],
        },
        scale: vex::TexTransformTrack {
            times: vec![0],
            values: vec![(256, 256)],
        },
        seconds_per_key: 1.0 / 60.0,
        loop_seconds: 100.0,
        step: false,
    })];

    // How many pixels sample the *white* texel. A mean over the whole frame is
    // diluted by background and moves only a few percent; the count of lit
    // pixels is the quantity the scroll actually changes.
    let white = |p: &[u8]| {
        p.as_chunks::<4>()
            .0
            .iter()
            .filter(|c| u32::from(c[0]) + u32::from(c[1]) + u32::from(c[2]) > 400)
            .count()
    };

    let Some(first) = capture(&model, 0.0) else {
        println!("skipping: no GPU adapter");
        return;
    };
    let later = capture(&model, 1.0).expect("the device worked a moment ago");
    let (a, b) = (white(&first), white(&later));
    assert!(
        b > a * 2 + 16,
        "the surface did not scroll: {a} white pixels then {b} - the shader is \
         not applying the texture-transform table"
    );
}
