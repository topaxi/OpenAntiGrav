use super::*;
use oag_core::math::Vec3;
use oag_mesh::mesh::shine_pass::build;
use oag_mesh::mesh::{Bounds, DrawCall, slots};

fn vertex(normal: [f32; 3]) -> GpuVertex {
    GpuVertex {
        position: [0.0; 3],
        normal,
        colour: [0.3, 0.4, 0.5, 0.6],
        texcoord: [0.25, 0.75],
        lightmap_texcoord: [0.0, 0.0],
        lit: 1.0,
        anim: 2,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 1.0,
        texcoord2: [0.0, 0.0],
    }
}

fn draw(range: std::ops::Range<u32>, texture: usize) -> DrawCall {
    DrawCall {
        range,
        texture: Some(texture),
        bounds: Bounds {
            centre: [0.0; 3],
            radius: 1.0,
        },
        moving: false,
        culled: true,
        blend: None,
        blend_state: None,
        alpha_test_ref: None,
        layer: oag_vex::vex::LAYER_DEFAULT,
        node: Some(3),
        chunk: None,
    }
}

fn hull() -> Model {
    let mut hull = Model::none("hull");
    hull.vertices = vec![vertex([0.0, 0.0, 1.0]); 6];
    hull.indices = vec![0, 1, 2, 3, 4, 5];
    hull.draws = vec![draw(0..3, 0), draw(3..6, 1)];
    hull.transparent_draws = vec![draw(3..6, 5)];
    hull.shine_draws = vec![draw(3..6, 2)];
    hull
}

/// The equation over the fixed basis: a normal along light 0 is the far
/// end of `u` and a normal along light 1 is the far end of `v`.
#[test]
fn a_normal_along_each_basis_vector_reaches_the_far_end_of_its_axis() {
    let mut model = build(&hull()).unwrap();
    model.vertices[0].normal = texgen::ENV_BASIS_0.to_array();
    model.vertices[1].normal = texgen::ENV_BASIS_1.to_array();
    let mut out = Vec::new();
    write(&model, &mut out, Mat4::IDENTITY, [0.0; 2]);
    assert!((out[0].texcoord[0] - 1.0).abs() < 1.0e-5, "{:?}", out[0]);
    assert!((out[1].texcoord[1] - 1.0).abs() < 1.0e-5, "{:?}", out[1]);
    assert_eq!(out.len(), model.vertices.len());
}

/// The coordinates follow the ship: a turned ship moves the same vertex's
/// coordinate, and nothing the camera does can reach `write`.
#[test]
fn turning_the_ship_moves_the_coordinates() {
    let model = build(&hull()).unwrap();
    let (mut still, mut turned) = (Vec::new(), Vec::new());
    write(&model, &mut still, Mat4::IDENTITY, [0.0; 2]);
    write(&model, &mut turned, Mat4::from_rotation_x(1.2), [0.0; 2]);
    assert_ne!(still[0].texcoord, turned[0].texcoord);
    // The translation half of the matrix is not a rotation of the normal.
    let mut moved = Vec::new();
    write(
        &model,
        &mut moved,
        Mat4::from_translation(Vec3::new(5.0, 6.0, 7.0)),
        [0.0; 2],
    );
    assert_eq!(still[0].texcoord, moved[0].texcoord);
}

fn circuit() -> Model {
    let mut track = Model::none("circuit");
    track.vertices = (0..9).map(|i| vertex([i as f32, 0.0, 1.0])).collect();
    track.indices = (0..9).collect();
    track.textures = vec![None, None, None];
    track.draws = vec![draw(0..3, 0), draw(3..6, 1), draw(6..9, 0)];
    // The second names a range no opaque draw has: a cutout or blended one.
    track.shine_draws = vec![draw(3..6, 2), draw(9..12, 2)];
    track
}

#[test]
fn a_circuit_pass_keeps_every_opaque_shine_batch_and_says_which_it_left() {
    let pass = build_track(&circuit()).expect("a circuit with a shine batch");
    assert_eq!(pass.model.draws.len(), 1);
    assert_eq!(pass.sources, vec![1], "the circuit draw it redraws");
    assert_eq!(pass.skipped, 1, "the one outside the opaque list");
    assert_eq!(pass.model.draws[0].texture, Some(2), "the second texture");
    assert_eq!(
        pass.model.draws[0].range,
        0..3,
        "re-indexed to its own buffer"
    );
    assert_eq!(pass.model.vertices.len(), 3, "only the batch's vertices");
    assert!(
        pass.model
            .vertices
            .iter()
            .all(|v| v.colour == [1.0; 4] && v.lit == 0.0)
    );
}

#[test]
fn a_circuit_with_only_non_opaque_shine_batches_builds_no_pass() {
    let mut track = circuit();
    track.shine_draws.remove(0);
    assert!(build_track(&track).is_none());
}

/// Rows, not columns: a view that turns the camera away from the world
/// axes has different rows and columns, and the coordinate follows the row.
#[test]
fn the_circuit_coordinate_follows_the_cameras_right_and_up_in_world_space() {
    let pass = build_track(&circuit()).unwrap();
    // The camera yawed 90 degrees about y: its right axis is world +z (forward x up).
    // World to view for a camera whose right is +z, up +y and back -x: the
    // rows are those three axes, so the columns are their components.
    let view = Mat4::from_cols_array(&[
        0.0, 0.0, -1.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        1.0, 0.0, 0.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]);
    let mut out = Vec::new();
    let mut model = pass.model;
    model.vertices[0].normal = [0.0, 0.0, 1.0];
    model.vertices[1].normal = [0.0, 1.0, 0.0];
    model.vertices[2].normal = [1.0, 0.0, 0.0];
    write_view(&model, &mut out, view, 0.0);
    assert!(
        (out[0][0] - 1.0).abs() < 1.0e-5,
        "toward the camera's right: {:?}",
        out[0]
    );
    assert!(
        (out[1][1] - 1.0).abs() < 1.0e-5,
        "toward the camera's up: {:?}",
        out[1]
    );
    assert!(
        (out[2][0] - 0.5).abs() < 1.0e-5,
        "along its line of sight: {:?}",
        out[2]
    );
}

/// A circuit's shine vertices carry authored colours (`vtype 0x139`), and
/// the pass modulates the chrome map by them; only a vertex that authors
/// none is white.
#[test]
fn a_circuit_vertex_keeps_the_colour_it_authors() {
    let mut track = circuit();
    track.vertices[3].lit = 0.0;
    track.vertices[3].colour = [0.2, 0.3, 0.4, 1.0];
    let pass = build_track(&track).unwrap();
    assert_eq!(pass.model.vertices[0].colour, [0.2, 0.3, 0.4, 1.0]);
    assert_eq!(pass.model.vertices[1].colour, [1.0; 4]);
}

/// An animated node's normal is turned by the node's matrix before the
/// coordinate is generated, so the highlight rides the moving mesh.
#[test]
fn an_animated_nodes_normal_is_turned_by_its_matrix() {
    use oag_mesh::mesh::{AnimNode, Motion};
    let mut track = circuit();
    track.vertices[3].xform = 1;
    track.vertices[3].normal = [1.0, 0.0, 0.0];
    // A quarter turn about z: x becomes y.
    let mut quarter = oag_vex::vex::IDENTITY;
    quarter[0..2].copy_from_slice(&[0.0, 1.0]);
    quarter[4..6].copy_from_slice(&[-1.0, 0.0]);
    track.anim_nodes = vec![AnimNode {
        transform: Motion::Vex(Box::default()),
        static_above: quarter,
        parent: None,
    }];
    let pass = build_track(&track).unwrap();
    let (mut still, mut turned) = (Vec::new(), Vec::new());
    // Camera right is +x, up is +y.
    let view = Mat4::IDENTITY;
    let mut flat = pass.model.clone();
    flat.anim_nodes.clear();
    write_view(&flat, &mut still, view, 0.0);
    write_view(&pass.model, &mut turned, view, 0.0);
    assert!(
        (still[0][0] - 1.0).abs() < 1.0e-5,
        "faces right: {:?}",
        still[0]
    );
    assert!(
        (turned[0][1] - 1.0).abs() < 1.0e-5,
        "turned to face up: {:?}",
        turned[0]
    );
}

/// A shine model whose last three vertices are the left airbrake flap: off the
/// hinge, with a normal that turns when the flap does.
fn flapped() -> Model {
    let mut model = build(&hull()).unwrap();
    for (i, v) in model.vertices.iter_mut().enumerate() {
        v.position = [i as f32, 1.0, -2.0];
        v.normal = [0.0, 1.0, 0.0];
    }
    model.airbrakes[0] = Some(oag_mesh::mesh::Flap {
        vertices: 3..6,
        hinge: Mat4::from_translation(Vec3::new(1.5, -0.5, -6.0)),
    });
    model
}

/// The pass rides its flap: its vertices are exactly the ones the hull's base
/// draw writes for the same angle (`Flap::swung`, one call for both), and the
/// glint's coordinates are generated from the *swung* normal. Drop the swing
/// from `write` and the positions stay where the file put them.
#[test]
fn the_extra_pass_follows_the_airbrake_flap_the_hull_deflects() {
    let model = flapped();
    let flap = model.airbrakes[0].clone().unwrap();
    let angle = 0.6;

    let (mut stowed, mut braked) = (Vec::new(), Vec::new());
    write(&model, &mut stowed, Mat4::IDENTITY, [0.0; 2]);
    write(&model, &mut braked, Mat4::IDENTITY, [angle, 0.0]);

    let mut hull_side = Vec::new();
    let span = flap.swung(&model.vertices, angle, &mut hull_side).unwrap();
    assert_eq!(span, 3..6);
    for (shine, hull) in braked[span.clone()].iter().zip(&hull_side) {
        assert_eq!(shine.position, hull.position, "the same swing, bit for bit");
        assert_eq!(shine.normal, hull.normal);
    }
    assert_ne!(braked[3].position, stowed[3].position, "the flap moved");
    assert_ne!(
        braked[3].texcoord, stowed[3].texcoord,
        "the glint reads the swung normal"
    );
    // The rest of the hull, and a flap held at zero, are untouched.
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&braked[..3]),
        bytemuck::cast_slice::<_, u8>(&stowed[..3])
    );
    assert_eq!(stowed[3].position, model.vertices[3].position);
}

/// The right flap's angle moves only the right flap: a model with one flap
/// ignores the other's angle.
#[test]
fn an_angle_for_a_flap_the_model_lacks_changes_nothing() {
    let model = flapped();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    write(&model, &mut a, Mat4::IDENTITY, [0.0; 2]);
    write(&model, &mut b, Mat4::IDENTITY, [0.0, 0.9]);
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&a),
        bytemuck::cast_slice::<_, u8>(&b)
    );
}
