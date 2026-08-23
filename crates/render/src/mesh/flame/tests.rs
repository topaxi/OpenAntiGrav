use super::*;
use oag_formats::rcsmodel::material::Parameter;

/// Every named constant is the hash of the name it claims, so a typo in a
/// literal is a failing test rather than a parameter that is silently never
/// found.
#[test]
fn the_named_hashes_are_the_hashes_of_their_names() {
    assert_eq!(oag_formats::rcsmaterial::name_hash("power1"), POWER1);
    assert_eq!(oag_formats::rcsmaterial::name_hash("scale1"), SCALE1);
    assert_eq!(oag_formats::rcsmaterial::name_hash("min1"), MIN1);
}

fn material(parameters: Vec<Parameter>) -> rcsmodel::Material {
    rcsmodel::Material {
        name: "engines/flame_test.rcsmaterial".to_string(),
        state: 0x29,
        src_factor: 0x0302,
        dst_factor: 0x0001,
        texture: "flame_01.gtf".to_string(),
        second_texture: None,
        parameters,
    }
}

fn parameter(hash: u32, value: f32) -> Parameter {
    Parameter {
        hash,
        value: [value, 0.0, 0.0, 0.0],
        quads: 1,
    }
}

/// The five the disc authors, read as the disc authors them.
#[test]
fn the_shipped_set_reads_back_as_shipped() {
    let flame = Flame::from_material(&material(vec![
        parameter(POWER1, 10.0),
        parameter(SCALE1, 0.3),
        parameter(MIN1, 0.45),
        parameter(ALPHA_SCALE, 2.0),
        parameter(COLOUR_SCALE, 1.0),
    ]))
    .expect("the full set reads");
    assert_eq!(flame.rim_power, 10.0);
    assert_eq!(flame.alpha_scale, 2.0);
    // The range the shipped numbers imply, which is what the shader computes:
    // 1.1 face-on and 0.5 edge-on, before the vertex ramp.
    let term = |rim: f32| {
        flame.alpha_scale * (1.0 - (rim.powf(flame.rim_power) * flame.rim_scale + flame.rim_min))
    };
    assert!((term(0.0) - 1.1).abs() < 1e-5, "{}", term(0.0));
    assert!((term(1.0) - 0.5).abs() < 1e-5, "{}", term(1.0));
}

/// **One missing parameter is no flame**, rather than a flame with a filled-in
/// number - the rule the whole path hangs on.
#[test]
fn a_material_short_one_parameter_is_not_a_flame() {
    assert!(
        Flame::from_material(&material(vec![
            parameter(POWER1, 10.0),
            parameter(SCALE1, 0.3),
            parameter(MIN1, 0.45),
            parameter(ALPHA_SCALE, 2.0),
        ]))
        .is_none()
    );
}

/// A material that declares none - which is most of them - is not a flame
/// either, and asking costs nothing.
#[test]
fn an_ordinary_material_is_not_a_flame() {
    assert!(Flame::from_material(&material(Vec::new())).is_none());
}
