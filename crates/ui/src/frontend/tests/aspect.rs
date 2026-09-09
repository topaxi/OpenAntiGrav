//! The grid the front end authors in, and how a picture of a different shape
//! is boxed inside it.
//!
//! Split out of `frontend/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;
use oag_display::space::{Space, pillarbox, pillarbox_in};

/// The coordinates that *do* scale land on this grid and no other.
///
/// Not a round-number check: `Skin.xml`'s own extremes are `x=613 y=415` on
/// the PS2 against `x=460 y=252` on the PSP, and both ratios have to land on
/// 640/480 and 448/272 for 640x448 to be the grid it authors in. They do, to
/// under a tenth of a percent.
///
/// **This is four samples, and four samples is what it claims.** Thirteen of
/// the file's coordinates do not follow this ratio at all; whether the ones
/// here are representative is not a question a hardcoded quartet can answer,
/// and `crates/game/tests/frontend_grid_ground_truth.rs` is what answers it
/// against the discs. See [`Space`].
#[test]
fn the_grid_ratio_is_the_one_the_scaling_coordinates_agree_on() {
    let psp = Space::PSP.size;
    let ps2 = Space::PS2.size;
    assert!((613.0 / 460.0 - ps2.0 / psp.0).abs() < 0.002, "x extreme");
    assert!((415.0 / 252.0 - ps2.1 / psp.1).abs() < 0.002, "y extreme");
    // **220, the USA PSP's value, not the EU's 230.** The EU PSP disc moved
    // this widget down 10px on its own; the PS2 EU disc did not follow it,
    // and 362/220 lands on the resolution ratio while 362/230 misses it by
    // 4%. So the PS2 layout was scaled from the USA-era artwork rather than
    // from the EU PSP release - a small dating fact this test happens to
    // pin, and the reason a naive EU-to-EU comparison looks wrong.
    assert!(
        (362.0 / 220.0 - ps2.1 / psp.1).abs() < 0.005,
        "PRESS START y"
    );
    assert!((119.0 / 72.0 - ps2.1 / psp.1).abs() < 0.01, "the logo's y");
}

/// The grid and the display aspect are two numbers, and on the PS2 they
/// disagree.
#[test]
fn a_ps2_grid_is_not_its_own_display_aspect() {
    assert!((Space::PSP.display_aspect - Space::PSP.size.0 / Space::PSP.size.1).abs() < 1e-6);
    let as_grid = Space::PS2.size.0 / Space::PS2.size.1;
    assert!(
        (as_grid - Space::PS2.display_aspect).abs() > 0.3,
        "640/448 is {as_grid}, and taking it for the display aspect squeezes the front end"
    );
}

/// The display aspect is the one that undoes the port's own stretch.
///
/// The PS2 draws the PSP's artwork - same texture, same source rectangle -
/// scaled by `640/480` across and `448/272` down, and the executable applies
/// those same two factors itself in `FUN_001e9370`. A rectangle stretched by
/// two different factors comes out square only at one display aspect, and this
/// is the arithmetic that names it. See `docs/ps2/aspect-ratio.md`.
#[test]
fn the_display_aspect_undoes_the_anamorphic_stretch() {
    let (sx, sy) = Space::PS2.texture_scale();
    // A texel square, drawn `sx` by `sy` grid units, shown as `display_aspect`.
    let on_screen = (sx / sy) * Space::PS2.display_aspect * Space::PS2.size.1 / Space::PS2.size.0;
    assert!(
        (on_screen - 1.0).abs() < 1e-5,
        "a square comes out {on_screen} times as wide as it is tall"
    );
    // And 4:3, which is what this used to carry, is where the squash was.
    let squashed = (sx / sy) * (4.0 / 3.0) * Space::PS2.size.1 / Space::PS2.size.0;
    assert!((squashed - 0.7556).abs() < 0.001, "{squashed}");
}

/// A picture already the screen's shape fills it, on either disc.
///
/// The PS2's own movies are that case: `INTRO512.PSS` decodes to a square
/// 512x512 and declares 4:3, and is neither - it is cut to fill the frame, so
/// [`crate::frontend::PS2_DISPLAY_ASPECT`] is the frame's own numbers and this
/// returns the whole screen.
#[test]
fn a_picture_of_the_screens_own_shape_is_not_boxed() {
    assert_eq!(
        pillarbox_in(Space::PSP, (480, 272)),
        [0.0, 0.0, 480.0, 272.0]
    );
    let ps2 = pillarbox_in(Space::PS2, crate::frontend::PS2_DISPLAY_ASPECT);
    assert!(
        (ps2[2] - 640.0).abs() < 0.01 && (ps2[3] - 448.0).abs() < 0.01,
        "a cut of the frame's own shape fills it: {ps2:?}"
    );
    assert!(ps2[0].abs() < 0.01 && ps2[1].abs() < 0.01, "{ps2:?}");
}

/// A narrower picture is pillarboxed, and the bars are in grid units.
#[test]
fn a_narrower_picture_is_pillarboxed_within_the_grid() {
    let ps2 = pillarbox_in(Space::PS2, (4, 3));
    assert!((ps2[3] - 448.0).abs() < 0.01, "full height: {ps2:?}");
    // ~16:9 shown, 4:3 wanted, so the width gives up what it is narrower by.
    let wanted = (4.0 / 3.0) * 640.0 / Space::PS2.display_aspect;
    assert!((ps2[2] - wanted).abs() < 0.01, "{ps2:?} against {wanted}");
    assert!((ps2[0] - (640.0 - wanted) / 2.0).abs() < 0.01, "centred");
}

/// Only the PS2 rescales a texture's own size, and by its own two factors.
#[test]
fn a_texture_size_is_psp_pixels_until_the_ps2_says_otherwise() {
    assert_eq!(Space::PSP.texture_scale(), (1.0, 1.0));
    assert_eq!(Space::HD.texture_scale(), (1.0, 1.0));
    let (sx, sy) = Space::PS2.texture_scale();
    assert!((sx - 640.0 / 480.0).abs() < 1e-6, "{sx}");
    assert!((sy - 448.0 / 272.0).abs() < 1e-6, "{sy}");
    // `Show Logo`'s 512x128 logo, which is the same file on both discs, ends
    // up the same fraction of the screen it is on.
    let (w, h) = (512.0 * sx, 128.0 * sy);
    assert!((w / Space::PS2.size.0 - 512.0 / SCREEN.0).abs() < 1e-6);
    assert!((h / Space::PS2.size.1 - 128.0 / SCREEN.1).abs() < 1e-6);
}

/// The old two-argument form is the square-pixel case and is unchanged.
#[test]
fn the_square_pixel_form_still_agrees_with_itself() {
    for aspect in [(4, 3), (16, 9), (512, 512), (480, 272)] {
        assert_eq!(
            pillarbox(SCREEN, aspect),
            pillarbox_in(Space::PSP, aspect),
            "{aspect:?}"
        );
    }
}
