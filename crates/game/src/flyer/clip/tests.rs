use super::*;
use bytemuck::Zeroable;

fn at(x: f32, y: f32) -> GpuVertex {
    GpuVertex {
        position: [x, y, 0.0],
        texcoord: [x, y],
        slots: 0xc4,
        ..GpuVertex::zeroed()
    }
}

/// A triangle wholly inside the rectangle comes back as itself.
#[test]
fn a_triangle_inside_is_untouched() {
    let out = clip_polygon(
        vec![at(0.0, 0.0), at(1.0, 0.0), at(0.0, 1.0)],
        [-10.0, -10.0, 10.0, 10.0],
    );
    assert_eq!(out.len(), 3);
}

/// A triangle wholly outside is gone.
#[test]
fn a_triangle_outside_is_dropped() {
    let out = clip_polygon(
        vec![at(20.0, 0.0), at(21.0, 0.0), at(20.0, 1.0)],
        [-10.0, -10.0, 10.0, 10.0],
    );
    assert!(out.is_empty());
}

/// A triangle straddling one edge is cut on it: every corner is inside, the
/// cut corners sit exactly on the bound, and the texture coordinate was carried
/// to the cut rather than left at its source.
#[test]
fn a_straddling_triangle_is_cut_on_the_edge_with_its_uvs() {
    let out = clip_polygon(
        vec![at(0.0, 0.0), at(20.0, 0.0), at(0.0, 10.0)],
        [-10.0, -10.0, 10.0, 10.0],
    );
    assert!(out.len() >= 3);
    for v in &out {
        assert!(v.position[0] <= 10.0 + 1e-5, "{:?}", v.position);
        assert_eq!(v.texcoord, [v.position[0], v.position[1]]);
        assert_eq!(v.slots, 0xc4, "per-draw constants ride through");
    }
    assert!(out.iter().any(|v| v.position[0] == 10.0));
}

/// A model of single vertices at the given positions.
fn model_of(points: &[[f32; 3]]) -> Model {
    let mut model = Model::none("flatten fixture");
    model.vertices = points
        .iter()
        .map(|&position| GpuVertex {
            position,
            ..GpuVertex::zeroed()
        })
        .collect();
    model
}

/// The camera stands 10 units in front of the origin, on the axis.
fn camera_at_ten() -> [f32; 16] {
    let mut camera = oag_vex::vex::IDENTITY;
    camera[14] = 10.0;
    camera
}

/// The top of the camera's window lands on the top of the card, and a point on
/// the axis on its centre - the whole reading of `window_tan` and the card
/// height.
#[test]
fn the_top_of_the_window_lands_on_the_top_of_the_card() {
    // Half the vertical field of view has a tangent of 0.5, so the window's
    // top at 10 units is 5 up; the card is 2 tall.
    let mut model = model_of(&[[0.0, 5.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
    flatten(&mut model, &camera_at_ten(), 0.5, 2.0, 1.0);
    let at = |i: usize| model.vertices[i].position;
    assert!((at(0)[1] - 1.0).abs() < 1e-5, "{:?}", at(0));
    assert!(
        at(1)[0].abs() < 1e-5 && at(1)[1].abs() < 1e-5,
        "{:?}",
        at(1)
    );
    assert!((at(2)[0] - 0.2).abs() < 1e-5, "{:?}", at(2));
}

/// A layer nearer the camera is scaled up by its nearness - the same offset
/// from the axis, 2 units from the camera instead of 10, is five times as far
/// out on the card - and it keeps its place in front of the layer behind it.
#[test]
fn a_nearer_layer_is_larger_and_stays_in_front() {
    let mut model = model_of(&[[1.0, 0.0, 0.0], [1.0, 0.0, 8.0]]);
    flatten(&mut model, &camera_at_ten(), 0.5, 2.0, 1.0);
    let (far, near) = (model.vertices[0].position, model.vertices[1].position);
    assert!((near[0] / far[0] - 5.0).abs() < 1e-4, "{far:?} {near:?}");
    assert!(near[2] > far[2], "{far:?} {near:?}");
}

/// A stretch widens the picture and leaves its height alone.
#[test]
fn a_stretch_widens_the_picture_only() {
    let mut model = model_of(&[[1.0, 1.0, 0.0]]);
    flatten(&mut model, &camera_at_ten(), 0.5, 2.0, 1.5);
    let [u, v, _] = model.vertices[0].position;
    assert!(
        (u - 0.2 * 1.5).abs() < 1e-5 && (v - 0.2).abs() < 1e-5,
        "{u} {v}"
    );
}

/// A triangle of vertices at the given card positions.
fn tri(points: [[f32; 2]; 3]) -> Model {
    let mut model = model_of(&points.map(|[x, y]| [x, y, 0.0]));
    model.indices = vec![0, 1, 2];
    model.draws = vec![DrawCall {
        range: 0..3,
        texture: None,
        bounds: oag_render::mesh::Bounds {
            centre: [0.0; 3],
            radius: 1.0,
        },
        moving: false,
        culled: false,
        blend: None,
        blend_state: None,
        layer: 0,
        alpha_test_ref: None,
        node: None,
        chunk: None,
    }];
    model
}

/// A picture is cut to the shape it is shown on: a triangle wider than the
/// shape keeps only the part inside it, whichever way the shape is wound.
#[test]
fn a_triangle_is_cut_to_the_shape() {
    let mut model = tri([[-10.0, -10.0], [10.0, -10.0], [-10.0, 10.0]]);
    let shape = [[[0.0, 0.0], [-1.0, 4.0], [-4.0, 0.0]]];
    clip_to_shape(&mut model, &shape);
    assert!(!model.indices.is_empty());
    for v in &model.vertices {
        assert!(
            v.position[0] <= 1e-4 && v.position[1] >= -1e-4,
            "{:?}",
            v.position
        );
        assert!(v.position[0] >= -4.0 - 1e-4 && v.position[1] <= 4.0 + 1e-4);
    }
}

/// The reflection is a mirrored, alpha-blended copy of the part of the picture
/// nearest the bottom edge, faded from the top alpha to nothing.
#[test]
fn the_reflection_mirrors_the_bottom_and_fades() {
    let mut model = tri([[-4.0, -10.0], [4.0, -10.0], [0.0, -6.0]]);
    for v in &mut model.vertices {
        v.colour = [1.0; 4];
    }
    let fade = Fade {
        edge_y: -10.0,
        depth: 4.0,
        top_alpha: 0.3,
    };
    reflect(&mut model, fade);
    let added = model.transparent_draws.last().expect("a reflection draw");
    assert!(added.blend == Some(oag_vex::vex::BlendClass::AlphaOver));
    let range = added.range.start as usize..added.range.end as usize;
    for &i in &model.indices[range] {
        let v = &model.vertices[i as usize];
        assert!(
            v.position[1] <= -10.0 + 1e-4 && v.position[1] >= -14.0 - 1e-4,
            "{:?}",
            v.position
        );
        let want = 0.3 * (v.position[1] + 14.0) / 4.0;
        assert!(
            (v.colour[3] - want).abs() < 1e-4,
            "{} against {want}",
            v.colour[3]
        );
    }
    assert_eq!(model.draws.len(), 1, "the picture itself is untouched");
}
