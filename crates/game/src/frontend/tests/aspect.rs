//! The grid the front end authors in, and how a picture of a different shape
//! is boxed inside it.
//!
//! Split out of `frontend/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;

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
        (as_grid - Space::PS2.display_aspect).abs() > 0.09,
        "640/448 is {as_grid}, and taking it for the display aspect is the 7% stretch"
    );
}

/// A picture already the screen's shape fills it, on either disc.
///
/// The PS2 half is the case that was wrong and stayed wrong after the
/// renderer was fixed: `INTRO512.PSS` declares 4:3, the PS2 screen *is* 4:3,
/// and the old square-pixel comparison boxed it inside itself anyway.
#[test]
fn a_picture_of_the_screens_own_shape_is_not_boxed() {
    assert_eq!(
        pillarbox_in(Space::PSP, (480, 272)),
        [0.0, 0.0, 480.0, 272.0]
    );
    let ps2 = pillarbox_in(Space::PS2, (4, 3));
    assert!(
        (ps2[2] - 640.0).abs() < 0.01 && (ps2[3] - 448.0).abs() < 0.01,
        "a 4:3 cut fills a 4:3 screen: {ps2:?}"
    );
    assert!(ps2[0].abs() < 0.01 && ps2[1].abs() < 0.01, "{ps2:?}");
}

/// A wider picture is letterboxed, and the bars are in grid units.
#[test]
fn a_wider_picture_is_letterboxed_within_the_grid() {
    let ps2 = pillarbox_in(Space::PS2, (16, 9));
    assert!((ps2[2] - 640.0).abs() < 0.01, "full width: {ps2:?}");
    // 4:3 shown, 16:9 wanted, so the height gives up 3/4 of itself.
    let wanted = (4.0 / 3.0) * 448.0 / (16.0 / 9.0);
    assert!((ps2[3] - wanted).abs() < 0.01, "{ps2:?} against {wanted}");
    assert!((ps2[1] - (448.0 - wanted) / 2.0).abs() < 0.01, "centred");
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
