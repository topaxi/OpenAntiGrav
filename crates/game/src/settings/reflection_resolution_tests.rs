//! `graphics.reflection_resolution` in the per-title render profile: default and round trip.

use super::tests::{load_text, read};
use super::*;
use oag_display::display::ReflectionResolution;

/// A profile written before REFLECTION RESOLUTION existed keeps the original's fixed target,
/// and `half` and `quarter` survive a round trip through the file.
#[test]
fn a_profile_without_reflection_resolution_keeps_the_originals_and_the_fractions_round_trip() {
    let key = profile_key(oag_hd::TITLE, oag_disc::Platform::Ps3);
    let old = read("[render_profiles.\"Wipeout HD\"]\nrender_scale = 100\n");
    assert_eq!(
        old.render_profiles[&key].reflection_resolution,
        ReflectionResolution::Original
    );
    for resolution in [ReflectionResolution::Half, ReflectionResolution::Quarter] {
        let text = format!(
            "[render_profiles.\"Wipeout HD\"]\nreflection_resolution = \"{}\"\n",
            resolution.name()
        );
        assert_eq!(
            read(&text).render_profiles[&key].reflection_resolution,
            resolution
        );
    }
}

/// `scaled` is gone: a file that still says it falls back to the original's size alone, the
/// rest of the profile is kept and the file is left for the player.
#[test]
fn the_retired_scaled_value_falls_back_to_original_and_keeps_the_rest() {
    let key = profile_key(oag_hd::TITLE, oag_disc::Platform::Ps3);
    let text = "[render_profiles.\"Wipeout HD\"]\nreflection_resolution = \"scaled\"\n\
                shadows = \"off\"\n";
    let (loaded, after) = load_text(text);
    let profile = &loaded.render_profiles[&key];
    assert_eq!(
        profile.reflection_resolution,
        ReflectionResolution::Original
    );
    assert_eq!(
        profile.shadows,
        oag_display::display::Shadows::Off,
        "the rest of the profile loads"
    );
    assert_eq!(after, text, "the file is left as the player wrote it");
}
