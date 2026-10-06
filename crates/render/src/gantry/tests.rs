//! Unit tests for the mount's plane fit, on synthetic panels.
//!
//! The real measurement is `crates/render/tests/gantry_mount_ground_truth.rs`,
//! which needs a disc. These pin the arithmetic: a panel whose normal, up and
//! extents are known by construction, so a wrong eigenvector ordering or a
//! flipped basis fails here rather than as a gantry facing the wrong way.

use super::*;

/// A rectangle in the plane `z = z0`, `width` across x and `height` up y.
fn panel(centre: Vec3, width: f32, height: f32) -> Vec<Vec3> {
    let mut points = Vec::new();
    for i in 0..5 {
        for j in 0..5 {
            let u = (i as f32 / 4.0 - 0.5) * width;
            let v = (j as f32 / 4.0 - 0.5) * height;
            points.push(centre + Vec3::new(u, v, 0.0));
        }
    }
    points
}

#[test]
fn a_flat_panel_gives_its_own_normal_and_extents() {
    let mount = plane(Some(7), &panel(Vec3::new(34.0, -36.0, -185.0), 30.0, 12.0))
        .expect("a panel has a plane");
    assert_eq!(mount.node, Some(7));
    assert!((mount.centre - Vec3::new(34.0, -36.0, -185.0)).length() < 1e-3);
    assert!(
        mount.normal.dot(Vec3::Z).abs() > 0.999,
        "{:?}",
        mount.normal
    );
    assert!(mount.up.dot(Vec3::Y) > 0.999, "{:?}", mount.up);
    assert!((mount.width - 30.0).abs() < 1e-3);
    assert!((mount.height - 12.0).abs() < 1e-3);
    assert!(mount.thickness < 1e-3);
}

#[test]
fn a_taller_than_wide_panel_still_calls_the_vertical_axis_up() {
    // The long axis is the vertical one here, which is the case that breaks a
    // fit that assumes a board is always wider than it is tall.
    let mount = plane(None, &panel(Vec3::ZERO, 6.0, 40.0)).expect("a panel has a plane");
    assert!(mount.up.dot(Vec3::Y) > 0.999, "{:?}", mount.up);
    assert!((mount.height - 40.0).abs() < 1e-3);
    assert!((mount.width - 6.0).abs() < 1e-3);
}

#[test]
fn the_normals_sign_comes_from_the_caller_not_the_fit() {
    let mount = plane(None, &panel(Vec3::ZERO, 30.0, 12.0)).expect("a panel has a plane");
    assert!(mount.facing(Vec3::Z).dot(Vec3::Z) > 0.0);
    assert!(mount.facing(-Vec3::Z).dot(Vec3::Z) < 0.0);
}

#[test]
fn the_matrix_stands_the_model_on_the_mount_facing_the_race() {
    let centre = Vec3::new(34.0, -36.0, -185.0);
    let mount = plane(None, &panel(centre, 30.0, 12.0)).expect("a panel has a plane");
    // A model whose board faces its own -Z, placed on a mount whose race
    // direction is +Z: the board must end up facing +Z in world space.
    let matrix = mount.matrix(Vec3::Z, -Vec3::Z, 1.0);
    let placed = matrix.transform_point3(Vec3::ZERO);
    assert!((placed - centre).length() < 1e-3, "{placed:?}");
    let faced = matrix.transform_vector3(-Vec3::Z).normalize();
    assert!(faced.dot(Vec3::Z) > 0.99, "{faced:?}");
    // Up stays up: a gantry standing on its side would pass a facing test.
    let up = matrix.transform_vector3(Vec3::Y).normalize();
    assert!(up.dot(Vec3::Y) > 0.99, "{up:?}");
}

#[test]
fn a_yawed_panel_reports_a_yawed_normal() {
    let angle = 0.7f32;
    let (s, c) = angle.sin_cos();
    let points: Vec<Vec3> = panel(Vec3::ZERO, 30.0, 12.0)
        .into_iter()
        .map(|p| Vec3::new(c * p.x + s * p.z, p.y, -s * p.x + c * p.z))
        .collect();
    let mount = plane(None, &points).expect("a panel has a plane");
    let expected = Vec3::new(s, 0.0, c);
    assert!(
        mount.facing(expected).dot(expected) > 0.999,
        "{:?}",
        mount.normal
    );
    assert!((mount.width - 30.0).abs() < 1e-2);
}

#[test]
fn too_few_points_is_no_plane() {
    assert!(plane(None, &[Vec3::ZERO, Vec3::X]).is_none());
}

/// A model of two one-triangle draws in the blend list, at the given x offsets.
///
/// No `Anim Transform`, so `clip_to_panel` reads the vertex positions as they
/// are - which is the case the real asset reduces to once its node table is
/// sampled.
fn two_draws(a: f32, b: f32) -> Model {
    use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex};

    let vertex = |x: f32| GpuVertex {
        position: [x, 0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        colour: [1.0; 4],
        texcoord: [0.0; 2],
        lit: 0.0,
        anim: 0,
        lightmap_texcoord: [0.0; 2],
        xform: 0,
        sun_mask: 1.0,
        slots: 0,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
    };
    let mut model = Model::none("two draws");
    model.vertices = vec![
        vertex(a),
        vertex(a),
        vertex(a),
        vertex(b),
        vertex(b),
        vertex(b),
    ];
    model.indices = vec![0, 1, 2, 3, 4, 5];
    let draw = |range: std::ops::Range<u32>| DrawCall {
        range,
        texture: None,
        bounds: Bounds {
            centre: [0.0; 3],
            radius: 0.0,
        },
        moving: false,
        culled: false,
        blend: None,
        blend_state: None,
        layer: 0,
        node: None,
        chunk: None,
        alpha_test_ref: None,
    };
    model.transparent_draws = vec![draw(0..3), draw(3..6)];
    model
}

#[test]
fn a_draw_parked_beyond_the_panel_is_clipped_and_the_one_on_it_is_kept() {
    let mut model = two_draws(0.0, 40.0);
    assert_eq!(clip_to_panel(&mut model, 22.75, 0.0), 1);
    assert_eq!(model.transparent_draws.len(), 1);
    // The survivor is the one at the panel's centre, not whichever came first.
    let kept = &model.transparent_draws[0];
    assert_eq!(
        model.vertices[model.indices[kept.range.start as usize] as usize].position[0],
        0.0
    );
}

#[test]
fn clipping_everything_clips_nothing() {
    // The guard: a model whose board plane is not its own XY would put every
    // draw outside, and deleting the whole object reads exactly like a circuit
    // that authors no gantry at all. See `clip_to_panel`.
    let mut model = two_draws(-40.0, 40.0);
    assert_eq!(clip_to_panel(&mut model, 22.75, 0.0), 0);
    assert_eq!(model.transparent_draws.len(), 2);
}

#[test]
fn every_billboard_slot_name_is_a_placeholder_regardless_of_case() {
    for n in 1..=8 {
        assert!(is_slot_placeholder(&format!("billboard{n}.tga")));
        assert!(is_slot_placeholder(&format!("Billboard{n}.TGA")));
        assert!(is_slot_placeholder(&format!("billboard{n}.gtf")));
        assert!(is_slot_placeholder(&format!("Billboard{n}.GTF")));
    }
    assert!(!is_slot_placeholder("321backplate.tga"));
    assert!(!is_slot_placeholder("billboard9.tga"));
    assert!(!is_slot_placeholder("billboard.tga"));
    assert!(!is_slot_placeholder("billboard9.gtf"));
}

#[test]
fn a_full_archive_path_is_a_placeholder_by_its_own_file_name() {
    // HD's own label shape: a material sampler's full path, not a bare file
    // name - see `basename`'s own doc comment.
    assert!(is_slot_placeholder(
        "data/environments/talons_junction/textures/dds/billboard8.gtf"
    ));
    assert!(is_slot_placeholder(
        r"data\environments\talons_junction\textures\dds\Billboard7.GTF"
    ));
    assert!(!is_slot_placeholder(
        "data/environments/talons_junction/textures/dds/track_surface.gtf"
    ));
}

/// A model with two textured draws - one bound to a placeholder slot, one to
/// ordinary track art - built the same way [`two_draws`] is, so
/// `strip_slot_placeholders` is checked against the same shape
/// `clip_to_panel`'s own tests use.
fn textured_draws(placeholder_label: &str) -> Model {
    use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, ModelTexture, Texels};

    let vertex = || GpuVertex {
        position: [0.0; 3],
        normal: [0.0, 0.0, 1.0],
        colour: [1.0; 4],
        texcoord: [0.0; 2],
        lit: 0.0,
        anim: 0,
        lightmap_texcoord: [0.0; 2],
        xform: 0,
        sun_mask: 1.0,
        slots: 0,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
    };
    let mut model = Model::none("placeholder + art");
    model.vertices = vec![vertex(), vertex(), vertex(), vertex(), vertex(), vertex()];
    model.indices = vec![0, 1, 2, 3, 4, 5];
    let texture = |label: &str| {
        Some(std::sync::Arc::new(ModelTexture {
            label: label.to_string(),
            width: 8,
            height: 8,
            texels: Texels::Rgba8(vec![0; 8 * 8 * 4]),
            mip_count: None,
        }))
    };
    model.textures = vec![
        texture(placeholder_label),
        texture("factory_floor_01_rp.tga"),
    ];
    let draw = |range: std::ops::Range<u32>, texture: usize| DrawCall {
        range,
        texture: Some(texture),
        bounds: Bounds {
            centre: [0.0; 3],
            radius: 0.0,
        },
        moving: false,
        culled: false,
        blend: None,
        blend_state: None,
        layer: 0,
        node: Some(74),
        chunk: None,
        alpha_test_ref: None,
    };
    model.draws = vec![draw(0..3, 0), draw(3..6, 1)];
    model
}

#[test]
fn strip_slot_placeholders_drops_only_the_placeholder_draw() {
    let mut model = textured_draws("billboard8.tga");
    assert_eq!(strip_slot_placeholders(&mut model), 1);
    assert_eq!(model.draws.len(), 1);
    assert_eq!(model.draws[0].texture, Some(1));
}

#[test]
fn strip_slot_placeholders_is_case_insensitive_and_leaves_real_art_alone() {
    let mut model = textured_draws("Billboard8.TGA");
    assert_eq!(strip_slot_placeholders(&mut model), 1);
    let mut untouched = textured_draws("321backplate.tga");
    assert_eq!(strip_slot_placeholders(&mut untouched), 0);
    assert_eq!(untouched.draws.len(), 2);
}

/// A slot whose advert is drawn into the quad keeps its draw; every other
/// placeholder is still dropped, and slot 8 can be dropped on its own where the
/// gantry stands in for it.
#[test]
fn a_served_slot_keeps_its_placeholder_draw_and_the_rest_are_dropped() {
    let mut served = textured_draws("billboard7.tga");
    assert_eq!(strip_unserved_slot_placeholders(&mut served, &[7]), 0);
    assert_eq!(served.draws.len(), 2);
    assert_eq!(placeholder_texture_slots(&served), vec![(0, 7)]);

    let mut unserved = textured_draws("billboard7.tga");
    assert_eq!(strip_unserved_slot_placeholders(&mut unserved, &[1, 2]), 1);
    assert_eq!(unserved.draws.len(), 1);

    let mut eight = textured_draws("billboard8.tga");
    assert_eq!(strip_slot_placeholder(&mut eight, 7), 0);
    assert_eq!(strip_slot_placeholder(&mut eight, 8), 1);
}

#[test]
fn strip_fx350_art_drops_only_the_borrowed_slot7_draw() {
    let mut model = textured_draws("data/billboards/hd_adverts/321go/fx350_nomip.gtf");
    assert_eq!(strip_fx350_art(&mut model), 1);
    assert_eq!(model.draws.len(), 1);
    assert_eq!(model.draws[0].texture, Some(1));
}

#[test]
fn strip_fx350_art_is_case_insensitive_and_leaves_the_digit_boards_own_texture_alone() {
    let mut model = textured_draws("Data/Billboards/HD_Adverts/321Go/FX350_NOMIP.GTF");
    assert_eq!(strip_fx350_art(&mut model), 1);
    let mut untouched = textured_draws("data/billboards/hd_adverts/321go/321_go_64.gtf");
    assert_eq!(strip_fx350_art(&mut untouched), 0);
    assert_eq!(untouched.draws.len(), 2);
}

#[test]
fn mount_finds_hds_full_path_billboard8_label() {
    // HD ships no `321backplate`-shaped second surface - see
    // `HD_SLOT8_TEXTURE`'s own doc comment - so its stub alone has to resolve
    // a mount, through the full archive path a PS3 material names it by.
    let model = textured_draws("data/environments/talons_junction/textures/dds/billboard8.gtf");
    let mount = mount(&model).expect("the HD spelling resolves a mount");
    assert_eq!(mount.node, Some(74));
}

#[test]
fn the_placement_matrix_never_mirrors_the_glyphs() {
    // **The question this answers, and why it is an assertion rather than a
    // screenshot.** Pulse's digits are squared-off enough that a `3` renders as
    // three bars on a spine and reads, to some eyes, like a mirrored one - so
    // "are the countdown glyphs flipped?" is a fair thing to ask of a frame and
    // an expensive thing to guess at. It is not a question about the texture:
    // the glyphs are **geometry**, four groups of triangles in one mesh, and
    // `321go_NOMIP.tga` is a 16x32 colour palette with no letterforms in it at
    // all. So there is no U coordinate whose sign could flip a letter.
    //
    // What *could* flip them is this matrix. A basis with a negative
    // determinant mirrors the mesh - and it would mirror the whole board, so
    // the four glyphs would come out in the order `GO`, `1`, `2`, `3` across
    // the panel and each shape reversed with them. A positive determinant
    // rules out both at once, for every mount, which one frame cannot.
    //
    // Confirmed in a race as well: at 5.90 s the board reads `GO`, G left and O
    // right, and at 2.60 s the lit glyph is the rightmost - which is where
    // `321Go_StartFinish.vex` puts its `1` (model x 5.72..16.87) and the
    // opposite of where a mirror would put it.
    for forward in [Vec3::Z, -Vec3::Z, Vec3::X, Vec3::new(0.3, 0.0, -0.95)] {
        let mount = plane(None, &panel(Vec3::new(4.0, -3.0, 9.0), 45.5, 10.4))
            .expect("a panel has a plane");
        let matrix = mount.matrix(forward, Vec3::Z, 1.0);
        let basis = Mat3::from_cols(
            matrix.x_axis.truncate(),
            matrix.y_axis.truncate(),
            matrix.z_axis.truncate(),
        );
        assert!(
            basis.determinant() > 0.9,
            "facing {forward:?}: determinant {} - a negative one mirrors every glyph \
             on the board",
            basis.determinant(),
        );
    }
}

#[test]
fn a_placeholder_label_names_its_slot_on_either_titles_spelling() {
    assert_eq!(slot_number("billboard7.tga"), Some(7));
    assert_eq!(slot_number("BillBoard1.TGA"), Some(1));
    assert_eq!(
        slot_number("data/environments/talons_junction/textures/dds/billboard8.gtf"),
        Some(8)
    );
    assert_eq!(slot_number("billboard9.tga"), None);
    assert_eq!(slot_number("321backplate.tga"), None);
    assert_eq!(slot_number("billboardwall.tga"), None);
}
