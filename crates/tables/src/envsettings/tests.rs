//! Unit tests for [`super`], on the shapes the disc actually writes.

use super::*;

/// Talon's Junction's own first lines, byte for byte.
const TALONS: &str = concat!(
    "\"Lighting.Constant ambient color\"=0.403922 0.392157 0.509804\n",
    "\"Lighting.Sky colour\"=128 128 128 0\n",
    "\"Lighting.Sky rotation\"=17.000000\n",
    "\"Lighting.Sun color\"=2.000000 1.827451 0.886275\n",
    "\"Lighting.Sun direction\"=-0.777600 0.581520 -0.239110\n",
    "\"Lighting.Use Lens Flare\"=1\n",
    "\"Fog.Fog Density\"=0.001000\n",
);

#[test]
fn reads_every_arity_the_file_uses() {
    let env = EnvSettings::parse(TALONS).expect("it parses");
    assert_eq!(env.entries.len(), 7);
    assert_eq!(env.scalar(SKY_ROTATION), Some(17.0));
    assert_eq!(env.scalar(FOG_DENSITY), Some(0.001));
    assert_eq!(env.vec3(SUN_COLOUR), Some([2.0, 1.827_451, 0.886_275]));
    assert_eq!(env.rgba8(SKY_COLOUR), Some([128, 128, 128, 0]));
    assert_eq!(env.flag("Lighting.Use Lens Flare"), Some(true));
}

/// **The trap the whole `is_integer` field exists for.**
///
/// `Sky colour` is bytes and `ambient color` is normalised floats, and the only
/// thing that says so is the decimal point.
#[test]
fn a_byte_colour_and_a_float_colour_are_not_confused() {
    let env = EnvSettings::parse(TALONS).expect("it parses");
    // The byte quadruple does not come back as floats in 0..=1 ...
    assert_eq!(env.vec3(SKY_COLOUR), None, "it is four numbers, not three");
    // ... and the float triple does not come back as bytes.
    assert_eq!(env.rgba8(AMBIENT_COLOUR), None);
    assert_eq!(
        env.vec3(AMBIENT_COLOUR),
        Some([0.403_922, 0.392_157, 0.509_804])
    );
}

/// A value above 1.0 is carried through rather than clamped: the caller has to
/// know, because this pipeline has no headroom for it.
#[test]
fn a_colour_over_one_is_reported_rather_than_clamped() {
    let env = EnvSettings::parse(TALONS).expect("it parses");
    assert_eq!(env.vec3(SUN_COLOUR).map(|c| c[0]), Some(2.0));
}

#[test]
fn the_sun_direction_comes_back_as_a_unit_vector() {
    let env = EnvSettings::parse(TALONS).expect("it parses");
    let d = env.direction(SUN_DIRECTION).expect("a direction");
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    assert!((length - 1.0).abs() < 1e-5, "length {length}");
}

/// `modesto_heights` authors this, and it is an off switch rather than a
/// direction. Answering `None` is what stops it becoming a NaN in a shader.
#[test]
fn a_degenerate_direction_is_none_rather_than_a_nan() {
    let env =
        EnvSettings::parse("\"Lighting.Physical Sun direction\"=-0.000030 0.000040 -0.000060\n")
            .expect("it parses");
    assert_eq!(env.direction("Lighting.Physical Sun direction"), None);
}

/// A triple that is not unit is normalised rather than used raw: most of the
/// corpus is unit, and several circuits are not.
#[test]
fn a_direction_that_is_not_unit_is_normalised() {
    let env = EnvSettings::parse("\"Lighting.Sun direction\"=20.0 20.0 20.0\n").expect("parses");
    let d = env.direction(SUN_DIRECTION).expect("a direction");
    for component in d {
        assert!((component - 0.577_35).abs() < 1e-4, "{d:?}");
    }
}

#[test]
fn a_line_that_is_not_key_equals_value_is_reported_with_its_number() {
    let err = EnvSettings::parse("\"a\"=1.0\nnonsense\n").expect_err("the second line");
    assert_eq!(err.line, 2);
    assert!(err.to_string().contains("nonsense"), "{err}");
}

#[test]
fn blank_lines_and_trailing_whitespace_are_not_errors() {
    let env = EnvSettings::parse("\n  \n\"Fog.Fog Density\"=0.001000  \n\n").expect("it parses");
    assert_eq!(env.scalar(FOG_DENSITY), Some(0.001));
}

/// A key whose value is not numeric is dropped rather than defaulted: a zero
/// ambient is a picture, and nothing on the disc has one.
#[test]
fn a_non_numeric_value_drops_its_entry_rather_than_defaulting() {
    let env = EnvSettings::parse("\"Fog.Fog Color\"=red green blue\n").expect("it parses");
    assert!(env.entries.is_empty());
    assert_eq!(env.vec3(FOG_COLOUR), None);
}

#[test]
fn an_absent_key_answers_none_on_every_accessor() {
    let env = EnvSettings::parse(TALONS).expect("it parses");
    assert_eq!(env.scalar("nope"), None);
    assert_eq!(env.vec3("nope"), None);
    assert_eq!(env.rgba8("nope"), None);
    assert_eq!(env.flag("nope"), None);
    assert_eq!(env.direction("nope"), None);
}
