//! The picture controls against a real race: each one moves the saved shot the way it names, and
//! saturation 0 is grey whatever the render scale, upscaler or MSAA. Split out of
//! `photo_mode_ground_truth`, whose helpers and test group it shares.

use super::*;

fn chroma(p: &[u8]) -> f32 {
    f32::from(p[..3].iter().max().copied().unwrap_or(0) - p[..3].iter().min().copied().unwrap_or(0))
}

/// The edge of the picture, where the speed streak is, and the middle, where it is not.
const EDGE: (f32, f32, f32, f32) = (0.0, 0.0, 0.12, 1.0);
const MIDDLE: (f32, f32, f32, f32) = (0.40, 0.30, 0.60, 0.50);
/// The far field up the left of a `close` shot, and the craft's own body.
const FAR_FIELD: (f32, f32, f32, f32) = (0.02, 0.10, 0.38, 0.55);
const CRAFT: (f32, f32, f32, f32) = (0.44, 0.70, 0.56, 0.82);

/// Each picture control, moved in turn, changes the saved file in the direction it names, and put
/// back where it opened gives the file the mode saved before anything was touched, byte for byte.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn each_picture_control_changes_the_saved_shot_and_at_neutral_it_is_the_unprocessed_one() {
    let Some(image) = pulse() else {
        return;
    };
    let shots = oag_testdata::scratch::dir("photo-effects-shots");
    let _ = std::fs::remove_dir_all(&shots);
    // Rows: exposure, saturation, speed blur, depth of field. 72 ticks of a held push is 21 steps
    // (it fires on the first tick, again after 14 and then every 3), 69 is 20, 39 is 10 and 1 is
    // one. A shot after each move (the drive draws a frame only when it is asked for a screenshot, so each
    // shot is asked for one); the race stands still, so every file is one frame's picture.
    let run = drive(
        &image,
        "effects",
        &format!(
            "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
             until photo=free within 200; press select select; ticks 20; press cross; ticks 2; screenshot SHOTS/frame-1.png; ticks 2; \
             press start; hold right 72; ticks 3; press cross; ticks 2; screenshot SHOTS/frame-2.png; ticks 2; \
             hold left 72; ticks 3; press cross; ticks 2; screenshot SHOTS/frame-3.png; ticks 2; \
             hold down 1; ticks 2; hold left 69; ticks 3; press cross; ticks 2; screenshot SHOTS/frame-4.png; ticks 2; \
             hold right 69; ticks 3; press cross; ticks 2; screenshot SHOTS/frame-5.png; ticks 2; \
             hold down 1; ticks 2; hold right 39; ticks 40; press cross; ticks 2; screenshot SHOTS/frame-6.png; ticks 2; \
             hold left 39; ticks 10; press cross; ticks 2; screenshot SHOTS/frame-7.png; ticks 2; \
             hold down 1; ticks 2; hold right 1; ticks 8; press cross; ticks 2; screenshot SHOTS/frame-8.png; ticks 2; \
             hold left 1; ticks 8; press cross; ticks 2; screenshot SHOTS/frame-9.png; ticks 2; \
             press circle; until photo=none within 50"
        ),
        &shots,
    );
    assert!(run.status.success(), "{}", run.stderr);
    let photos = the_photos(&run);
    assert_eq!(photos.len(), 9, "{photos:?}");
    let [
        neutral,
        bright,
        exposure_back,
        grey,
        saturation_back,
        speed,
        speed_back,
        dof,
        dof_back,
    ] = <[_; 9]>::try_from(photos.iter().map(|p| decode(p)).collect::<Vec<_>>())
        .unwrap_or_else(|_| unreachable!());

    // Back where it opened, the file is the unprocessed one.
    for (name, back) in [
        ("exposure", &exposure_back),
        ("saturation", &saturation_back),
        ("speed blur", &speed_back),
        ("depth of field", &dof_back),
    ] {
        assert!(
            back.2 == neutral.2,
            "{name} put back is not the picture the mode saved before it was touched"
        );
    }

    // Exposure up is brighter.
    let (before, after) = (over(&neutral, ALL, luma), over(&bright, ALL, luma));
    assert!(
        after > before + 3.0,
        "exposure up: mean luma {before} -> {after}"
    );
    // Saturation at zero is grey.
    assert!(
        over(&grey, ALL, chroma) < 0.1 * over(&neutral, ALL, chroma).max(1.0),
        "saturation 0 left colour in the picture"
    );
    // And no pixel of it: Pulse's glow is added over the HUD in the original's order, and the
    // control still covers it (`docs/ui/photo-mode.md`, "Picture controls").
    let worst = grey
        .2
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| chroma(pixel))
        .fold(0.0_f32, f32::max);
    assert!(
        worst <= 2.0,
        "saturation 0 left a pixel with a chroma of {worst} (the glow escaped the control)"
    );
    // Speed blur lands at the edge of the picture and leaves the middle alone.
    assert!(
        difference(&neutral, &speed, EDGE) > 2.0,
        "no streak at the edge"
    );
    assert!(
        difference(&neutral, &speed, MIDDLE) < 1.0,
        "the speed blur reached the middle of the picture"
    );
    // Depth of field blurs the far field and leaves the craft sharp.
    let (sharp, soft) = (detail(&neutral, FAR_FIELD), detail(&dof, FAR_FIELD));
    assert!(soft < 0.75 * sharp, "far-field detail {sharp} -> {soft}");
    assert!(
        difference(&neutral, &dof, CRAFT) < 4.0,
        "the craft in focus changed: {}",
        difference(&neutral, &dof, CRAFT)
    );
    shot_fidelity::controls_are_recorded_and_drawn_again(GAME, &image, &run, &photos);
}

/// Saturation 0 leaves no colour in the saved shot under `args`, and the game runs clean: the
/// glow of Pulse is added to the frame ahead of the controls whatever resolves the frame after
/// them. A neutral shot of the same run proves there is colour to lose.
fn grey_under(tag: &str, args: &[&str]) {
    let Some(image) = pulse() else {
        return;
    };
    let shots = oag_testdata::scratch::dir(&format!("photo-grey-{tag}"));
    let _ = std::fs::remove_dir_all(&shots);
    let run = drive_with(
        &image,
        tag,
        &format!(
            "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
             until photo=free within 200; press select select; ticks 20; press cross; ticks 2; screenshot SHOTS/frame-1.png; ticks 2; \
             press start; hold down 1; ticks 2; hold left 69; ticks 3; press cross; ticks 2; screenshot SHOTS/frame-2.png; ticks 2; \
             press circle; until photo=none within 50"
        ),
        &shots,
        args,
    );
    assert!(run.status.success(), "{}", run.stderr);
    for bad in ["wgpu error", "Validation Error", "panicked"] {
        assert!(!run.stderr.contains(bad), "{tag}: {bad} in\n{}", run.stderr);
    }
    let photos = the_photos(&run);
    assert_eq!(photos.len(), 2, "{photos:?}");
    let worst = |picture: &(u32, u32, Vec<u8>)| {
        picture
            .2
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| chroma(pixel))
            .fold(0.0_f32, f32::max)
    };
    let (neutral, grey) = (decode(&photos[0]), decode(&photos[1]));
    assert!(worst(&neutral) > 20.0, "{tag}: no colour to lose");
    assert!(
        worst(&grey) <= 2.0,
        "{tag}: saturation 0 left a pixel with a chroma of {}",
        worst(&grey)
    );
}

/// Saturation 0 is grey with FSR 3 resolving the frame.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn saturation_zero_is_grey_under_fsr3() {
    grey_under("fsr3", &["--reconstruction", "fsr3"]);
}

/// Saturation 0 is grey with the game rendered at half the displayed size.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn saturation_zero_is_grey_at_render_scale_50() {
    grey_under("scale50", &["--render-scale", "50"]);
}

/// Saturation 0 is grey with four samples a pixel.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn saturation_zero_is_grey_under_msaa() {
    grey_under("msaa", &["--msaa", "4x"]);
}
