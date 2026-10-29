use super::*;
use oag_display::space::Space;
use oag_render::viewport::Cut;

const SCREEN: (f32, f32, f32, f32) = (0.0, 0.0, 1440.0, 816.0);

/// Within what the two-pixel gap between views costs a view's size.
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01 * b.abs().max(1.0)
}

#[test]
fn one_view_is_the_identity() {
    let placed = placements(1, Cut::Horizontal, SCREEN, Space::PSP);
    assert_eq!(placed.len(), 1);
    let p = placed[0];
    assert!(near(p.scale, 1.0), "{p:?}");
    assert!(near(p.offset[0], 0.0) && near(p.offset[1], 0.0), "{p:?}");
    assert!(near(p.clip[2], 480.0) && near(p.clip[3], 272.0), "{p:?}");
}

#[test]
fn four_views_are_four_quarter_size_huds_each_in_its_own_quadrant() {
    let placed = placements(4, Cut::Horizontal, SCREEN, Space::PSP);
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
    let placed = placements(2, Cut::Horizontal, SCREEN, Space::PSP);
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

/// The PS2's 640x448 grid is shown 16:9, so a grid unit is not square: 2.25 pixels wide and
/// 1.82 tall on this screen. A view's rectangle in the grid has to use each axis' own number,
/// or the whole screen comes out 362 rows tall of 448 and every group anchored at the bottom
/// of a view sits a fifth of the way up it.
#[test]
fn a_non_square_grids_views_are_measured_in_each_axis_own_units() {
    let whole = oag_render::viewport::layout(1, SCREEN, Cut::Horizontal)[0];
    let grid = grid_rect(whole, SCREEN, Space::PS2);
    assert!(
        near(grid[2], 640.0) && near(grid[3], 448.0),
        "the whole screen is the whole grid: {grid:?}"
    );
    for cut in [Cut::Horizontal, Cut::Vertical] {
        let views = oag_render::viewport::layout(2, SCREEN, cut);
        for view in &views {
            let grid = grid_rect(*view, SCREEN, Space::PS2);
            let (want_w, want_h) = match cut {
                Cut::Horizontal => (640.0, 224.0),
                Cut::Vertical => (320.0, 448.0),
            };
            assert!(
                near(grid[2], want_w) && near(grid[3], want_h),
                "{cut:?}: {grid:?} is not {want_w} by {want_h}"
            );
        }
    }
    // The whole-screen HUD placed into the whole screen is the identity, and its clip the grid.
    let placed = place_into(whole, [0.0, 0.0, 640.0, 448.0], SCREEN, Space::PS2);
    assert!(near(placed.scale, 1.0), "{placed:?}");
    assert!(
        near(placed.clip[2], 640.0) && near(placed.clip[3], 448.0),
        "{placed:?}"
    );
}

/// A file's bottom-anchored group lands at the bottom of its view on a non-square grid too:
/// the group's lowest grid row, once placed, is the view's last row less the margin.
#[test]
fn a_piece_pinned_to_the_bottom_sits_at_the_bottom_of_a_ps2_view() {
    // The PS2's grid is shown at 16:9, so the viewport is that shape: a PSP-shaped one letterboxes it.
    const WIDE: (f32, f32, f32, f32) = (0.0, 0.0, 1440.0, 810.0);
    let view = oag_render::viewport::layout(2, WIDE, Cut::Horizontal)[1];
    let rect = grid_rect(view, WIDE, Space::PS2);
    let piece = oag_hud::split_hud::Piece {
        layout: oag_hud::Layout::default(),
        source: [0.0, 400.0],
        group: oag_hud::split_hud::Group {
            widgets: vec![],
            anchor: oag_hud::split_hud::Anchor::Bottom,
            offset: [0.0, -0.04],
            scale: 1.0,
        },
    };
    let placed = piece.placement(rect, Space::PS2.size);
    // In screen pixels, which is where the player looks: the grid is 1.82 pixels a row here.
    let low_row = placed.scale * 400.0 + placed.offset[1];
    let low = low_row * WIDE.3 / Space::PS2.size.1;
    let want = view.1 + view.3 * 0.96;
    assert!(
        (low - want).abs() < 2.0,
        "{low} px is not {want} px, view {view:?}"
    );
}

#[test]
fn a_views_scissor_is_that_views_own_pixels() {
    for viewport in [SCREEN, (96.0, 0.0, 1248.0, 816.0)] {
        for space in [Space::PSP, Space::PS2, Space::of(oag_disc::Platform::Ps3)] {
            for count in 1..=8 {
                for view in oag_render::viewport::layout(count, viewport, Cut::Horizontal) {
                    let got = scissor_rect(grid_rect(view, viewport, space), viewport, space);
                    let want = [view.0, view.1, view.2, view.3];
                    for (got, want) in got.iter().zip(want) {
                        assert!(
                            (*got as f32 - want).abs() <= 1.0,
                            "{count} views, {space:?}: {got:?} is not {view:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn a_scissor_never_leaves_the_viewport() {
    let viewport = (96.0, 0.0, 1248.0, 816.0);
    // A rectangle hanging off every side of the HUD's grid.
    let [x, y, w, h] = scissor_rect([-50.0, -50.0, 700.0, 500.0], viewport, Space::PSP);
    assert_eq!([x, y], [96, 0]);
    assert!(x + w <= 1344 && y + h <= 816, "{x} {y} {w} {h}");
}

const SAMPLE: &str = r#"<Screen>
<Text name="Lap"><Values scale="1.0" font="HUD" align="left" vertalign="top" x="10" y="10" string="1"/></Text>
<Text name="SpeedBarText"><Values scale="1.0" font="HUD" align="right" vertalign="top" x="470" y="250" string="2"/></Text>
</Screen>"#;

const FILE: &str = r#"
[[layout]]
name = "any"
[[layout.group]]
widgets = ["Lap"]
anchor = "top-left"
[[layout.group]]
widgets = ["SpeedBarText"]
anchor = "bottom-right"
"#;

fn one_view_each(count: usize) -> Vec<Readout> {
    (0..count)
        .map(|_| Readout {
            mode: oag_race::Mode::SingleRace,
            speed_kmh: 120.0,
            lap: 1,
            laps: 3,
            ..Readout::blank()
        })
        .collect()
}

/// A split-HUD file's groups are one draw each and one scissor span each, plus the
/// view's reticle: every group lands in its own view and the spans say so.
#[test]
fn a_split_hud_file_draws_each_group_into_its_own_view_with_a_span() {
    let layout = oag_hud::Layout::from_xml(SAMPLE);
    let strings = oag_ui::language::StringTable::default();
    let sheet = oag_hud::sprite::Sheet::default();
    let cx = oag_hud::Context {
        default_border: layout.default_border(),
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art: oag_pulse::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
        skin_line_height: 10.0,
    };
    let heights = oag_hud::split_hud::LineHeights {
        hud: 25.0,
        small: 10.0,
        default: 13.0,
        skin: 13.0,
    };
    let file = oag_hud::split_hud::SplitHud::parse(FILE).unwrap();
    let assets = oag_hud::split_hud::SplitHudAssets::new(
        "x",
        oag_hud::split_hud::Origin::Fallback,
        file,
        &layout,
        heights,
        oag_race::Mode::SingleRace,
        4,
    );
    let views = view_draws(
        cx,
        None,
        None,
        Some(&assets),
        one_view_each(4),
        Cut::Horizontal,
        SCREEN,
        Space::PSP,
    );
    // Two groups and the reticle, for each of four views.
    assert_eq!(views.len(), 12);
    let frame = crate::hud_overlay::views_frame(&views);
    assert_eq!(frame.spans.len(), 12);
    assert!(
        frame
            .spans
            .windows(2)
            .all(|pair| (0..4).all(|i| pair[0].start[i] <= pair[1].start[i])),
        "{:?}",
        frame.spans
    );
    // Each view's lap is inside that view's rectangle.
    let rects = oag_render::viewport::layout(4, SCREEN, Cut::Horizontal);
    let per = 3.0;
    let texts: Vec<_> = frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { x, y, text, .. } => {
                Some((*x * per, *y * per, text.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(texts.len(), 4, "the lap, once in each view: {texts:?}");
    for (i, rect) in rects.iter().enumerate() {
        for (x, y, text) in &texts[i..=i] {
            assert!(
                *x >= rect.0 - 1.0
                    && *x <= rect.0 + rect.2 + 1.0
                    && *y >= rect.1 - 1.0
                    && *y <= rect.1 + rect.3 + 1.0,
                "view {i}: {text} at ({x}, {y}) is outside {rect:?}"
            );
        }
    }
}
