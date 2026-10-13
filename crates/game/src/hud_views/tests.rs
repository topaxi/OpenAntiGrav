use super::*;
use oag_display::space::Space;

const SCREEN: (f32, f32, f32, f32) = (0.0, 0.0, 1440.0, 816.0);

/// Within what the two-pixel gap between views costs a view's size.
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01 * b.abs().max(1.0)
}

#[test]
fn one_view_is_the_identity() {
    let placed = placements(1, SCREEN, Space::PSP);
    assert_eq!(placed.len(), 1);
    let p = placed[0];
    assert!(near(p.scale, 1.0), "{p:?}");
    assert!(near(p.offset[0], 0.0) && near(p.offset[1], 0.0), "{p:?}");
    assert!(near(p.clip[2], 480.0) && near(p.clip[3], 272.0), "{p:?}");
}

#[test]
fn four_views_are_four_quarter_size_huds_each_in_its_own_quadrant() {
    let placed = placements(4, SCREEN, Space::PSP);
    assert_eq!(placed.len(), 4);
    for (i, p) in placed.iter().enumerate() {
        assert!(near(p.scale, 0.5), "view {i}: {p:?}");
        // The layout's origin lands at the quadrant's corner, give or take the
        // gap between views.
        let (column, row) = (i % 2, i / 2);
        assert!(
            (p.offset[0] - 240.0 * column as f32).abs() < 1.0,
            "{i}: {p:?}"
        );
        assert!((p.offset[1] - 136.0 * row as f32).abs() < 1.0, "{i}: {p:?}");
    }
    // The clips tile the screen: no two overlap.
    for i in 0..4 {
        for j in i + 1..4 {
            let (a, b) = (placed[i].clip, placed[j].clip);
            let apart = a[0] + a[2] <= b[0]
                || b[0] + b[2] <= a[0]
                || a[1] + a[3] <= b[1]
                || b[1] + b[3] <= a[1];
            assert!(apart, "views {i} and {j} overlap: {a:?} {b:?}");
        }
    }
}

#[test]
fn two_views_fit_a_wide_strip_by_its_height_and_centre_the_hud() {
    let placed = placements(2, SCREEN, Space::PSP);
    // A strip is half the screen tall and as wide: the HUD is height-limited.
    for p in &placed {
        assert!(near(p.scale, 0.5), "{p:?}");
    }
    // Centred: the fitted HUD is half the width of the strip, so it starts a
    // quarter of the strip in.
    assert!((placed[0].offset[0] - 120.0).abs() < 1.0, "{:?}", placed[0]);
}

#[test]
fn an_authored_regions_origin_lands_at_the_views_corner() {
    // HD's second player: whole-screen layout whose own half starts at y = 540.
    let view = (0.0, 410.0, 1440.0, 406.0);
    let region = [0.0, 540.0, 1920.0, 540.0];
    let p = place_into(view, region, SCREEN, Space::of(oag_disc::Platform::Ps3));
    // Scale 1 at the identity of the whole-screen HUD: the strip is the authored half.
    assert!((p.scale - 1.0).abs() < 0.01, "{p:?}");
    // A point at the region's top-left (0, 540) maps to the view's top-left in
    // the screen grid, i.e. (0, 540 + a hair for the gap).
    let mapped_y = p.scale * 540.0 + p.offset[1];
    assert!((mapped_y - 540.0).abs() < 4.0, "{mapped_y} {p:?}");
}
