use super::*;

fn texture(label: &str, texels: Texels) -> ModelTexture {
    ModelTexture {
        label: label.to_string(),
        width: 2,
        height: 1,
        texels,
        mip_count: None,
        lod_code: 0.0,
    }
}

/// A listed name matches the embedded file name without its extension, in any case, and not a
/// longer name that merely starts with it (`Seagull` is not `sea`).
#[test]
fn a_label_matches_by_stem() {
    assert!(names_match("NewSeaTest.tga", "NewSeaTest"));
    assert!(names_match("data\\x\\newseatest.TGA", "NewSeaTest.tga"));
    assert!(names_match("sea.tga", "sea"));
    assert!(!names_match("rich_Seagull.tga", "sea"));
    assert!(!names_match("SeaRock.tga", "sea"));
    assert!(!names_match("AAdc_waterFoam.tga", "AAdc_Water"));
}

/// The colour is the base level's mean, from either texel form; a compressed form is unread.
#[test]
fn the_colour_is_the_texture_s_mean() {
    let rgba = texture("a", Texels::Rgba8(vec![255, 0, 0, 255, 0, 255, 0, 255]));
    assert_eq!(mean_colour(&rgba), Some([0.5, 0.5, 0.0]));
    let chain = texture(
        "b",
        Texels::Chain(vec![vec![0, 0, 255, 255, 0, 0, 255, 255], vec![9, 9, 9, 9]]),
    );
    assert_eq!(mean_colour(&chain), Some([0.0, 0.0, 1.0]));
    assert_eq!(mean_colour(&texture("c", Texels::Rgba8(Vec::new()))), None);
}

/// The taps are HD's sea law in the world's units: the first scrolls `u` only, the second runs
/// the opposite way, 1.15 times the scale, and both scales are per world unit.
#[test]
fn the_taps_are_the_sea_s_two_in_world_units() {
    let [first, second] = taps();
    let k = 1.0 / WAVE_SIZE;
    assert_eq!(first.scale, [k, k]);
    assert_eq!(first.scroll, [0.05, 0.0]);
    assert!((second.scale[0] + 1.15 * k).abs() < 1e-9 && (second.scale[1] - 1.15 * k).abs() < 1e-9);
    // One tile of world apart is one tile of the map: the world's `x` is the coordinate.
    let at = |t: &Tap, x: f32| t.at([x, 0.0], 0.0)[0];
    assert!((at(&first, WAVE_SIZE) - at(&first, 0.0) - 1.0).abs() < 1e-5);
}

/// An upward normal gets `+x` as its tangent with the bitangent unflipped; a normal along `x`
/// falls back to `+z`. A dropped `w` or an unnormalised tangent would change these bytes.
#[test]
fn the_tangent_is_world_x_made_perpendicular() {
    let bytes = |n| packed_tangent(n).to_le_bytes();
    assert_eq!(bytes([0.0, 1.0, 0.0]), [255, 128, 128, 255]);
    assert_eq!(bytes([1.0, 0.0, 0.0]), [128, 128, 255, 255]);
    let leaning = bytes([0.6, 0.8, 0.0]);
    assert_eq!(leaning[3], 255);
    assert!(leaning[0] < 255 && leaning[1] < 128, "{leaning:?}");
}
