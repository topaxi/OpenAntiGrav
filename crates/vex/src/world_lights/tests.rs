use super::*;

/// Tech de Ra's `wopoint` at `(-58.024, -5.471, 89.360)`, byte for byte.
const WOPOINT: [u8; 32] = [
    0xf0, 0xa7, 0xc6, 0x3e, 0x95, 0x2f, 0x68, 0x3f, 0x00, 0x00, 0x80, 0x3f, 0x00, 0x00, 0x80, 0x3f,
    0x00, 0x00, 0xe0, 0x40, 0x00, 0x00, 0x70, 0x41, 0, 0, 0, 0, 0, 0, 0, 0,
];

fn at(x: f32, y: f32, z: f32) -> [f32; 16] {
    let mut m = [0.0; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    m[12] = x;
    m[13] = y;
    m[14] = z;
    m
}

#[test]
fn a_wopoint_reads_the_runtime_objects_colour_and_radii() {
    let light = WorldLight::parse(
        &WOPOINT,
        at(-58.024, -5.471, 89.36),
        ByteOrder::Little,
        false,
    )
    .expect("32 bytes");
    // The live object's colour was (0.388, 0.907, 1.0) with radii 7 and 15.
    assert!((light.colour[0] - 0.388).abs() < 1e-3);
    assert!((light.colour[1] - 0.907).abs() < 1e-3);
    assert_eq!(light.colour[2], 1.0);
    assert_eq!((light.inner_radius, light.outer_radius), (7.0, 15.0));
    assert_eq!(light.position, [-58.024, -5.471, 89.36]);
    assert!(light.cone.is_none());
}

#[test]
fn a_wospots_axis_is_the_negated_second_row() {
    let mut m = at(0.0, 0.0, 0.0);
    m[4..7].copy_from_slice(&[-0.486, 0.621, 0.615]);
    let light = WorldLight::parse(&WOPOINT, m, ByteOrder::Little, true).expect("32 bytes");
    assert_eq!(light.cone.expect("a spot").axis, [0.486, -0.621, -0.615]);
}

#[test]
fn a_short_payload_is_refused() {
    assert!(
        WorldLight::parse(&WOPOINT[..31], at(0.0, 0.0, 0.0), ByteOrder::Little, false).is_none()
    );
}
