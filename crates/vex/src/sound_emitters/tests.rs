use super::*;

/// `01_Track`'s `basilic~OH_CARGO` node, byte for byte off the disc: the first
/// `sound` node on the first circuit, its radius `100.0` encoding to a round
/// `1310`, so a wrong scale by a percent still looks plausible here and must be
/// caught by [`the_radius_encoding_is_the_executables_and_not_a_fit`].
fn oh_cargo() -> Vec<u8> {
    let mut p = vec![0u8; 0x50];
    let put_f32 = |p: &mut Vec<u8>, at: usize, v: f32| {
        p[at..at + 4].copy_from_slice(&v.to_le_bytes());
    };
    let put_u32 = |p: &mut Vec<u8>, at: usize, v: u32| {
        p[at..at + 4].copy_from_slice(&v.to_le_bytes());
    };
    let put_u16 = |p: &mut Vec<u8>, at: usize, v: u16| {
        p[at..at + 2].copy_from_slice(&v.to_le_bytes());
    };
    put_f32(&mut p, CONE_A, -1.0);
    put_f32(&mut p, CONE_B, -1.0);
    put_f32(&mut p, EMITTER_3C, 100.0);
    put_f32(&mut p, RADIUS, 100.0);
    p[BANK..BANK + 7].copy_from_slice(b"basilic");
    p[CUE..CUE + 9].copy_from_slice(b"~OH_CARGO");
    put_u16(&mut p, CURVE_KEYS, 1);
    put_u32(&mut p, CURVE_TIMES, 0x40);
    put_u32(&mut p, CURVE_VALUES, 0x42);
    put_f32(&mut p, CURVE_TICK, 1.0 / 60.0);
    put_u16(&mut p, 0x40, 0);
    put_u16(&mut p, 0x42, 1310);
    // The exporter leaves its own scratch past every terminator; a reader that
    // stops at the NUL never sees it and one that does not reports `~OH_CARGO?`.
    p[CUE + 9] = 0;
    p[CUE + 10] = b'?';
    p
}

#[test]
fn the_bank_and_cue_stop_at_their_terminators() {
    let e = SoundEmitter::parse(&oh_cargo(), vex::IDENTITY, ByteOrder::Little).expect("parse");
    assert_eq!(e.bank, "basilic");
    assert_eq!(e.cue, "~OH_CARGO");
}

/// A field that fills its whole width has no terminator, which is how
/// `07_Track` reversed ends up naming a bank that cannot exist.
#[test]
fn a_full_width_field_is_taken_whole() {
    let mut p = oh_cargo();
    p[BANK..BANK + 8].copy_from_slice(b"outpostf");
    let e = SoundEmitter::parse(&p, vex::IDENTITY, ByteOrder::Little).expect("parse");
    assert_eq!(e.bank, "outpostf");
}

#[test]
fn a_plain_sound_authors_no_cone() {
    let e = SoundEmitter::parse(&oh_cargo(), vex::IDENTITY, ByteOrder::Little).expect("parse");
    assert_eq!(e.cone, None);
}

/// The flag at `+0x08`, not the angles, says a node is a cone (a reader keyed on
/// "is `+0x00` negative" would disagree on a cone whose angles were authored as
/// zero).
///
/// The angles are asserted **by the offset they came from**, so swapping the two
/// reads fails this; sorting them into wide/narrow at parse time (as this module
/// once did) would hide the swap.
#[test]
fn the_cone_comes_from_the_flag_and_carries_both_angles() {
    let mut p = oh_cargo();
    p[CONE_ENABLED] = 1;
    p[CONE_A..CONE_A + 4].copy_from_slice(&120.0f32.to_radians().to_le_bytes());
    p[CONE_B..CONE_B + 4].copy_from_slice(&40.0f32.to_radians().to_le_bytes());
    let cone = SoundEmitter::parse(&p, vex::IDENTITY, ByteOrder::Little)
        .expect("parse")
        .cone
        .expect("a cone");
    assert!((cone.angle_a.to_degrees() - 120.0).abs() < 1e-3, "{cone:?}");
    assert!((cone.angle_b.to_degrees() - 40.0).abs() < 1e-3, "{cone:?}");
    assert!(
        (cone.wide() - cone.angle_a).abs() < f32::EPSILON,
        "{cone:?}"
    );
    assert!(
        (cone.narrow() - cone.angle_b).abs() < f32::EPSILON,
        "{cone:?}"
    );
}

/// `+0x30` and `+0x34` read `64` and `66` on disc: offsets to the two `u16`
/// arrays, not values. A reader taking them as values finds no curve and falls
/// back to the `f32` (the same number on this node), so this asserts the curve is
/// *there*.
#[test]
fn the_curve_offsets_are_relocated_rather_than_read_as_values() {
    let e = SoundEmitter::parse(&oh_cargo(), vex::IDENTITY, ByteOrder::Little).expect("parse");
    assert_eq!(e.radius_curve.times, vec![0]);
    assert_eq!(e.radius_curve.values, vec![1310]);
    assert!((e.radius_curve.seconds_per_tick - 1.0 / 60.0).abs() < 1e-7);
}

/// `1310 * 5000 / 65535` is `99.9924`, not `100.0`: the stored key is a
/// truncation and the decode does not pretend otherwise.
#[test]
fn one_key_makes_the_radius_constant_and_close_to_the_f32() {
    let e = SoundEmitter::parse(&oh_cargo(), vex::IDENTITY, ByteOrder::Little).expect("parse");
    for frame in [0.0, 1.0, 600.0, 1e6] {
        let r = e.sample_radius(frame);
        assert!((r - 100.0).abs() < 0.075, "at {frame}: {r}");
    }
    assert!(
        e.sample_radius(0.0) < 100.0,
        "the key is truncated, not rounded"
    );
}

/// The constants from `VexSound_SampleRadiusCurve`, against four radii separating
/// them from the `13.1` a data fit gives: `107.44` and `150.0` are wrong under
/// `13.1` (`1407`/`1965` against stored `1408`/`1966`), `70.0` and `100.0` right,
/// so the test fails for the right reason.
#[test]
fn the_radius_encoding_is_the_executables_and_not_a_fit() {
    for (radius, stored) in [(70.0, 917), (100.0, 1310), (107.44, 1408), (150.0, 1966)] {
        assert_eq!(encode_radius(radius), stored, "radius {radius}");
    }
}

/// Two keys, so the lerp is exercised rather than assumed - Pulse authors none.
#[test]
fn two_keys_interpolate_linearly_and_clamp_at_both_ends() {
    let mut p = oh_cargo();
    p[CURVE_KEYS..CURVE_KEYS + 2].copy_from_slice(&2u16.to_le_bytes());
    p[CURVE_TIMES..CURVE_TIMES + 4].copy_from_slice(&0x40u32.to_le_bytes());
    p[CURVE_VALUES..CURVE_VALUES + 4].copy_from_slice(&0x44u32.to_le_bytes());
    p[0x40..0x42].copy_from_slice(&0u16.to_le_bytes());
    p[0x42..0x44].copy_from_slice(&60u16.to_le_bytes());
    p[0x44..0x46].copy_from_slice(&encode_radius(100.0).to_le_bytes());
    p[0x46..0x48].copy_from_slice(&encode_radius(300.0).to_le_bytes());
    let e = SoundEmitter::parse(&p, vex::IDENTITY, ByteOrder::Little).expect("parse");
    assert!((e.sample_radius(-5.0) - 100.0).abs() < 0.075, "clamped low");
    assert!(
        (e.sample_radius(30.0) - 200.0).abs() < 0.075,
        "the midpoint"
    );
    assert!(
        (e.sample_radius(9999.0) - 300.0).abs() < 0.075,
        "clamped high"
    );
}

/// No keys is the one case the evaluator leaves the emitter's radius alone, so
/// the `f32` is what a caller has left to use.
#[test]
fn a_curve_with_no_keys_falls_back_to_the_static_radius() {
    let mut p = oh_cargo();
    p[CURVE_KEYS..CURVE_KEYS + 2].copy_from_slice(&0u16.to_le_bytes());
    let e = SoundEmitter::parse(&p, vex::IDENTITY, ByteOrder::Little).expect("parse");
    assert_eq!(e.radius_curve.sample(0.0), None);
    assert_eq!(e.sample_radius(0.0), 100.0);
}

/// A payload short enough to be something else entirely reads as nothing.
#[test]
fn a_truncated_payload_parses_to_nothing() {
    assert_eq!(
        SoundEmitter::parse(&[0u8; 0x20], vex::IDENTITY, ByteOrder::Little),
        None
    );
}

/// A curve whose offsets point past the end takes what is there and stops,
/// rather than panicking or inventing keys.
#[test]
fn a_curve_pointing_past_the_payload_is_truncated_not_fatal() {
    let mut p = oh_cargo();
    p[CURVE_KEYS..CURVE_KEYS + 2].copy_from_slice(&8u16.to_le_bytes());
    let e = SoundEmitter::parse(&p, vex::IDENTITY, ByteOrder::Little).expect("parse");
    assert_eq!(e.radius_curve.times.len(), e.radius_curve.values.len());
    assert!(e.radius_curve.times.len() < 8);
}

#[test]
fn the_placement_is_the_matrix_translation() {
    let mut m = vex::IDENTITY;
    m[12] = -717.0;
    m[13] = 19.0;
    m[14] = -512.0;
    let e = SoundEmitter::parse(&oh_cargo(), m, ByteOrder::Little).expect("parse");
    assert_eq!(e.position(), [-717.0, 19.0, -512.0]);
}
