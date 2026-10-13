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

#[test]
fn a_views_scissor_is_that_views_own_pixels() {
    for viewport in [SCREEN, (96.0, 0.0, 1248.0, 816.0)] {
        for space in [Space::PSP, Space::PS2, Space::of(oag_disc::Platform::Ps3)] {
            for count in 1..=8 {
                for view in oag_render::viewport::layout(count, viewport) {
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
    };
    let heights = oag_hud::split_hud::LineHeights {
        hud: 25.0,
        small: 10.0,
        default: 13.0,
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
        Some(&assets),
        one_view_each(4),
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
    let rects = oag_render::viewport::layout(4, SCREEN);
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
