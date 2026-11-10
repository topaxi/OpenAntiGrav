//! TRAIL LENGTH's settings: where it is seeded and what a profile reads.

use super::render_profile::RenderProfile;
use super::*;

/// TRAIL LENGTH is seeded where TRAIL ORDER is: only on the PSP ribbon.
#[test]
fn trail_length_is_seeded_only_where_the_ribbon_is_the_psp_preset() {
    let seeded = |title, platform| {
        menu_seeds(&Settings::default(), Anisotropy::default(), title, platform)
            .into_iter()
            .any(|(key, _)| key == "graphics.trail_length")
    };
    assert!(seeded(oag_pulse::TITLE, oag_disc::Platform::Psp));
    assert!(seeded(oag_pure::TITLE, oag_disc::Platform::Psp));
    assert!(!seeded(oag_hd::TITLE, oag_disc::Platform::Ps3));
}

/// `original` is the default and what a profile without the key reads.
#[test]
fn trail_length_defaults_to_the_original_and_round_trips_its_names() {
    assert_eq!(
        RenderProfile::default().trail_length,
        oag_display::display::TrailLength::Original
    );
    let profile: RenderProfile = toml::from_str("trail_length = \"hd\"").expect("parse");
    assert_eq!(profile.trail_length, oag_display::display::TrailLength::Hd);
    for length in oag_display::display::TrailLength::ALL {
        assert_eq!(length.name().parse(), Ok(length));
    }
}
