use super::*;
use oag_formats::ByteOrder;

/// Auricom's camera as it ships on Pulse PSP: one key of value `0x5080`,
/// aspect 2.0, and `camera1` at `(0.4854, 0.4779, 31.832)` looking down -z.
fn auricom_camera() -> oag_vex::camera::Camera {
    let mut payload = vec![0u8; oag_vex::camera::PAYLOAD_LEN];
    payload[0..4].copy_from_slice(&1u32.to_le_bytes());
    payload[4..8].copy_from_slice(&0x20u32.to_le_bytes());
    payload[8..12].copy_from_slice(&0x22u32.to_le_bytes());
    payload[0x1c..0x20].copy_from_slice(&2.0f32.to_le_bytes());
    payload[0x22..0x24].copy_from_slice(&0x5080u16.to_le_bytes());
    let mut to_world = oag_vex::vex::matrix::IDENTITY;
    to_world[12] = 0.485_420_2;
    to_world[13] = 0.477_935_8;
    to_world[14] = 31.832_031;
    oag_vex::camera::Camera::parse(None, &payload, to_world, ByteOrder::Little)
        .expect("a full payload")
}

/// The captured matrices of the running original: view = translation by
/// `(-0.4854, -0.4779, -31.832)`, projection x scale 1.857 and y scale 3.714.
/// The model's origin therefore lands at
/// `(1.857 * -0.4854 / 31.832, 3.714 * -0.4779 / 31.832)` in clip space after
/// the divide - which fails if the field of view is read as vertical, if the
/// aspect scales x instead of y, or if the view is not the camera's inverse.
#[test]
fn the_origin_lands_where_the_captured_matrices_put_it() {
    let matrix = view_projection(&auricom_camera()).expect("a one-key perspective camera");
    let clip = matrix * Vec3::ZERO.extend(1.0);
    let (x, y) = (clip.x / clip.w, clip.y / clip.w);
    assert!((x - (-0.028_32)).abs() < 2e-4, "x {x}");
    assert!((y - (-0.055_76)).abs() < 2e-4, "y {y}");
    assert!((clip.w - 31.832).abs() < 1e-3, "w {}", clip.w);
}

/// A panel 34.3 wide and 17.1 tall at the origin fills the card exactly: the
/// camera was framed on it (`visible half-height = distance / (aspect * x
/// scale)`).
#[test]
fn the_frustum_at_the_model_plane_is_two_to_one() {
    let matrix = view_projection(&auricom_camera()).expect("camera");
    let at = |x: f32, y: f32| {
        let clip = matrix * Vec3::new(x, y, 0.0).extend(1.0);
        (clip.x / clip.w, clip.y / clip.w)
    };
    let (right, _) = at(0.4854 + 17.14, 0.0);
    let (_, top) = at(0.0, 0.4779 + 8.57);
    assert!((right - 1.0).abs() < 0.01, "{right}");
    assert!((top - 1.0).abs() < 0.01, "{top}");
}

#[test]
fn an_orthographic_camera_gets_no_card() {
    let mut camera = auricom_camera();
    camera.fov_flags = 1;
    assert!(view_projection(&camera).is_none());
}
