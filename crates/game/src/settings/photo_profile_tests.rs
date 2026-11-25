//! The profile a saved photo is drawn with (`RenderProfile::for_a_photo`).

use super::*;
use oag_display::display::{Msaa, Reconstruction, Scale};

const RECT: (f32, f32) = (1920.0, 1080.0);

fn profile(scale: u32, reconstruction: Reconstruction) -> RenderProfile {
    RenderProfile {
        render_scale: Scale::try_from(scale).expect("a scale in range"),
        render_scale_chosen: true,
        reconstruction,
        msaa: Msaa::default(),
        ..RenderProfile::default()
    }
}

#[test]
fn a_photo_is_drawn_at_the_output_size_with_no_upscaler() {
    for reconstruction in [Reconstruction::Fsr1, Reconstruction::Fsr3] {
        let photo = profile(50, reconstruction).for_a_photo();
        assert_eq!(photo.scale_for(RECT, false), Scale::FULL);
        assert_eq!(photo.reconstruction, Reconstruction::Off);
    }
}

#[test]
fn a_photo_keeps_the_anti_aliasing_the_profile_asks_for() {
    for reconstruction in [Reconstruction::Fxaa, Reconstruction::Smaa] {
        let photo = profile(75, reconstruction).for_a_photo();
        assert_eq!(photo.reconstruction, reconstruction);
    }
    let mut four = profile(50, Reconstruction::Fsr3);
    four.msaa = Msaa::X4;
    assert_eq!(four.for_a_photo().msaa, Msaa::X4);
}

#[test]
fn the_players_supersampling_stays() {
    for percent in [125, 150, 200] {
        let photo = profile(percent, Reconstruction::Off).for_a_photo();
        assert_eq!(
            u32::from(photo.scale_for(RECT, false)),
            percent,
            "{percent} %"
        );
        assert!(!profile(percent, Reconstruction::Off).photo_differs(RECT, false, false));
    }
    let upscaled = profile(150, Reconstruction::Fsr3);
    assert!(upscaled.photo_differs(RECT, false, false));
    let photo = upscaled.for_a_photo();
    assert_eq!(u32::from(photo.scale_for(RECT, false)), 150);
    assert_eq!(photo.reconstruction, Reconstruction::Off);
}

#[test]
fn a_profile_that_already_draws_natively_is_not_redrawn() {
    for reconstruction in [
        Reconstruction::Off,
        Reconstruction::Fxaa,
        Reconstruction::Smaa,
    ] {
        let native = profile(100, reconstruction);
        assert!(
            !native.photo_differs(RECT, false, false),
            "{reconstruction}"
        );
        let photo = native.for_a_photo();
        assert_eq!(photo.scale_for(RECT, false), native.scale_for(RECT, false));
        assert_eq!(photo.reconstruction, native.reconstruction);
        assert_eq!(photo.msaa, native.msaa);
    }
}

#[test]
fn a_profile_that_draws_smaller_or_upscaled_is_redrawn() {
    assert!(profile(50, Reconstruction::Off).photo_differs(RECT, false, false));
    assert!(profile(100, Reconstruction::Fsr3).photo_differs(RECT, false, false));
    assert!(profile(100, Reconstruction::Fsr1).photo_differs(RECT, false, false));
}

#[test]
fn a_gamescope_default_scale_is_redrawn() {
    let mut unchosen = profile(100, Reconstruction::Off);
    unchosen.render_scale_chosen = false;
    assert!(unchosen.photo_differs((3840.0, 2160.0), true, false));
    assert!(!unchosen.photo_differs((3840.0, 2160.0), false, false));
}

#[test]
fn a_photo_draws_the_glass_and_sea_target_at_the_originals_size() {
    use oag_display::display::ReflectionResolution;
    let mut quarter = profile(100, Reconstruction::Off);
    assert!(!quarter.photo_differs(RECT, false, true));
    quarter.reflection_resolution = ReflectionResolution::Quarter;
    assert!(
        quarter.photo_differs(RECT, false, true),
        "a smaller target is redrawn"
    );
    assert!(
        !quarter.photo_differs(RECT, false, false),
        "a circuit with no target has nothing to redraw"
    );
    for resolution in ReflectionResolution::ALL {
        quarter.reflection_resolution = resolution;
        assert_eq!(
            quarter.for_a_photo().reflection_resolution,
            ReflectionResolution::Original
        );
    }
}
