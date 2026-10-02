//! Wipeout: Omega Collection's two additions to `.envsettings`: the Nova prelit
//! triple and the `Tonemap` block.
//!
//! The text is written out here in the file's own shape; the real files are
//! checked in `oag-game`'s `omega_lightmap_ground_truth`.

use oag_tables::envsettings::{EnvSettings, NOVA_PRELIT, NOVA_PRELIT_DEFAULT};

const TONEMAP: &str = r#"
"Tonemap.Exposure minimum"=0.1
"Tonemap.Exposure maximum"=2.0
"Tonemap.Exposure response"=3.0
"Tonemap.Exposure time"=4.0
"Tonemap.Luminance a-coefficient"=5.0
"Tonemap.Luminance b-coefficient"=6.0
"Tonemap.Source color end a-coefficient"=7.0
"Tonemap.Source color end b-coefficient"=8.0
"Tonemap.Start angle"=9.0
"Tonemap.End angle"=10.0
"#;

#[test]
fn the_nova_triple_is_scale_bias_power_in_file_order() {
    let env = EnvSettings::parse("\"Lighting.Nova prelit scale bias power\"=2.5 0.2 2\n").unwrap();
    assert_eq!(env.vec3(NOVA_PRELIT), Some([2.5, 0.2, 2.0]));
}

#[test]
fn a_file_without_the_key_has_no_triple_and_the_default_is_the_executables() {
    let env = EnvSettings::parse("\"Fog.Fog Density\"=0.001\n").unwrap();
    assert_eq!(env.vec3(NOVA_PRELIT), None);
    assert_eq!(NOVA_PRELIT_DEFAULT, [1.4, 0.2, 1.5]);
}

#[test]
fn the_tonemap_block_reads_all_ten_keys() {
    let env = EnvSettings::parse(TONEMAP).unwrap();
    let t = env.tonemap("Tonemap").expect("ten keys present");
    assert_eq!(
        [
            t.exposure_minimum,
            t.exposure_maximum,
            t.exposure_response,
            t.exposure_time,
            t.luminance_a,
            t.luminance_b,
            t.source_colour_end_a,
            t.source_colour_end_b,
            t.start_angle,
            t.end_angle,
        ],
        [0.1, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
    );
    assert_eq!(
        env.tonemap("TonemapHDR"),
        None,
        "the other prefix is a different block"
    );
}

#[test]
fn a_tonemap_block_missing_one_key_is_not_a_block() {
    let text = TONEMAP.replace("\"Tonemap.End angle\"=10.0\n", "");
    let env = EnvSettings::parse(&text).unwrap();
    assert_eq!(env.tonemap("Tonemap"), None);
}
