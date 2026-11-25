//! `graphics.reflection_resolution` in the per-title render profile: default and round trip.

use super::tests::read;
use super::*;

/// A profile written before REFLECTION RESOLUTION existed keeps the original's fixed target,
/// and `scaled` survives a round trip through the file.
#[test]
fn a_profile_without_reflection_resolution_keeps_the_originals_and_scaled_round_trips() {
    use oag_display::display::ReflectionResolution;
    let key = profile_key(oag_hd::TITLE, oag_disc::Platform::Ps3);
    let old = read("[render_profiles.\"Wipeout HD\"]\nrender_scale = 100\n");
    assert_eq!(
        old.render_profiles[&key].reflection_resolution,
        ReflectionResolution::Original
    );
    let scaled = read("[render_profiles.\"Wipeout HD\"]\nreflection_resolution = \"scaled\"\n");
    assert_eq!(
        scaled.render_profiles[&key].reflection_resolution,
        ReflectionResolution::Scaled
    );
}
