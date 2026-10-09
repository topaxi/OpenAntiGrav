use super::*;
use bytemuck::Zeroable;
use oag_core::math::Vec4;

/// The model's origin lands on the pixel `OriginX`/`OriginY` name: the
/// widget's authored `(1308, 440)` in the 1920 by 1080 grid.
#[test]
fn the_model_origin_lands_on_the_authored_pixel() {
    let widget = TrackModel {
        origin: [1308.0, 440.0],
        offset: [0.0, 0.0],
        z: -180.0,
        ortho_scale: [1.0; 3],
    };
    let (view_projection, model) = matrices(&widget, Space::HD, 3.0);
    let clip = view_projection * model * Vec4::new(0.0, 0.0, 0.0, 1.0);
    let ndc = (clip.x / clip.w, clip.y / clip.w);
    let pixel = (
        (ndc.0 + 1.0) * 0.5 * Space::HD.size.0,
        (1.0 - ndc.1) * 0.5 * Space::HD.size.1,
    );
    assert!((pixel.0 - 1308.0).abs() < 0.5, "x {}", pixel.0);
    assert!((pixel.1 - 440.0).abs() < 0.5, "y {}", pixel.1);
}

/// The circuit turns: a later second is a different pose, one revolution
/// later the same one.
#[test]
fn the_circuit_turns_once_per_revolution() {
    let widget = TrackModel {
        origin: [1308.0, 440.0],
        offset: [0.0, 0.0],
        z: -180.0,
        ortho_scale: [1.0; 3],
    };
    let at = |seconds| matrices(&widget, Space::HD, seconds).1;
    assert_ne!(at(0.0), at(1.0));
    assert!(at(0.0).abs_diff_eq(at(TURN_SECONDS), 1e-4));
}

fn ramp() -> Ramp {
    Ramp {
        row: vec![[0.8; 3], [0.6; 3], [0.4; 3]],
    }
}

/// The ramp is read at `N.V`: a surface facing the camera takes its far end
/// and a grazing one its near end, and a back face is clamped to the near end.
#[test]
fn the_ramp_is_read_at_the_facing() {
    let ramp = ramp();
    assert_eq!(ramp.at(0.0), [0.8; 3]);
    assert_eq!(ramp.at(0.5), [0.6; 3]);
    assert_eq!(ramp.at(1.0), [0.4; 3]);
    assert_eq!(ramp.at(-0.7), [0.8; 3]);
}

/// A vertex 180 units in front of the camera, facing it, is dark; the same
/// vertex turned edge-on is bright.
#[test]
fn shading_follows_the_view_vector() {
    let ramp = ramp();
    let mut vertices = [GpuVertex::zeroed(), GpuVertex::zeroed()];
    vertices[0].normal = [0.0, 0.0, 1.0];
    vertices[1].normal = [1.0, 0.0, 0.0];
    ramp.shade(
        &mut vertices,
        Mat4::from_translation(Vec3::new(0.0, 0.0, -180.0)),
    );
    assert_eq!(vertices[0].colour, [0.4, 0.4, 0.4, 1.0]);
    assert_eq!(vertices[1].colour, [0.8, 0.8, 0.8, 1.0]);
}

/// Omega's widget authors `x y z` of `0.525 0.125 -2.05` under an
/// `orthoScale` of `0.012`, so the circuit stands `1 / 0.012` times as far:
/// 43.75 right, 10.4 up and 170.8 in front, close to HD's 180.
#[test]
fn an_ortho_scale_scales_the_placement() {
    let widget = TrackModel {
        origin: [960.0, 540.0],
        offset: [0.525, 0.125],
        z: -2.05,
        ortho_scale: [0.012; 3],
    };
    let (_, model) = matrices(&widget, Space::HD, 0.0);
    let at = model.transform_point3(Vec3::ZERO);
    assert!((at.x - 43.75).abs() < 1e-3, "{at:?}");
    assert!((at.y - 10.417).abs() < 1e-3, "{at:?}");
    assert!((at.z + 170.833).abs() < 1e-3, "{at:?}");
}

/// A material's uniforms make the ramp: `colourDiffAlpha + Constant1 *
/// (min + scale * (1 - N.V)^power)` with the ambient colour taken as 1. Face
/// on it is the base plus the minimum, edge on the base plus min plus scale.
#[test]
fn the_fresnel_ramp_is_the_materials_own_uniforms() {
    use oag_rcs::rcsmaterial::name_hash;
    let uniform = |name: &str, value: &[f32]| oag_rcs::rcsmodel::psp2::material::Param {
        hash: name_hash(name),
        bits: value.iter().map(|v| v.to_bits()).collect(),
    };
    let material = oag_rcs::rcsmodel::psp2::material::Material {
        name: String::new(),
        technique: None,
        textures: Vec::new(),
        lightmap: None,
        params: vec![
            uniform("colourDiffAlpha", &[0.06, 0.048, 0.029, 1.0]),
            uniform("Constant1", &[1.0; 3]),
            uniform("ReflectivityMin", &[0.05]),
            uniform("ReflectivityScale", &[2.0]),
            uniform("ReflectivityPower", &[3.0]),
        ],
        samplers: Vec::new(),
        state: None,
    };
    let mut model = Model::none("test");
    let ramp = Ramp::fresnel(&material, &mut model).expect("every uniform is authored");
    let (face, edge) = (ramp.at(1.0), ramp.at(0.0));
    assert!((face[0] - 0.11).abs() < 1e-5, "{face:?}");
    assert!((edge[0] - 2.11).abs() < 1e-5, "{edge:?}");
    assert!((ramp.at(0.5)[1] - (0.048 + 0.05 + 2.0 * 0.125)).abs() < 5e-3);
    // A material that authors none of them is refused, not guessed.
    let mut bare = material.clone();
    bare.params.clear();
    assert!(Ramp::fresnel(&bare, &mut Model::none("test")).is_err());
}
