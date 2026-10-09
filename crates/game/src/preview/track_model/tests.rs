use super::*;
use bytemuck::Zeroable;
use oag_core::math::Vec4;

/// The model's origin lands on the pixel `OriginX`/`OriginY` name: the
/// widget's authored `(1308, 440)` in the 1920 by 1080 grid.
#[test]
fn the_model_origin_lands_on_the_authored_pixel() {
    let widget = TrackModel {
        origin: [1308.0, 440.0],
        z: -180.0,
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
        z: -180.0,
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
