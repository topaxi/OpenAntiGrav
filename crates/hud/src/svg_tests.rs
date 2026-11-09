//! A vector icon is rasterised at the pixel size its slot has on screen, once per size,
//! and drawn one texel to one pixel; it is never one bitmap scaled.

use super::draw::*;
use super::pickup::mod_icon_draw_for_test;
use super::svg::VectorIcon;
use super::*;

const LAYOUT: &str = r#"
<Screen>
<Image name="PickupBackground">
<Values x="240" y="35" Centred="true" width="66" height="60" U="3" V="111" TxtrWidth="66" TxtrHeight="60" Color="FEConst->HudColour1" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
<Image name="TurboIcon">
<Values x="240" y="35" Centred="true" width="48" height="48" U="70" V="111" TxtrWidth="48" TxtrHeight="48" Color="FEConst->HudColour1" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
</Screen>
"#;

/// 16 units square; an opaque bar from x = 4 to x = 12, and a half-covered column at 12..12.5.
const BAR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16">
<rect x="4" y="0" width="8.5" height="16" fill="#ffffff"/></svg>"##;

const SRC: &str = "mod:test/bar";

/// The Pulse sheet with the Turbo glyph's 20 by 27 texels opaque, as the draw tests have it.
fn base() -> crate::sprite::Sheet {
    let (width, height) = (128_u32, 160_u32);
    let mut hud = vec![0_u8; (width * height * 4) as usize];
    for y in 111 + 5..111 + 5 + 27 {
        for x in 70 + 14..70 + 14 + 20 {
            hud[((y * width + x) * 4 + 3) as usize] = 255;
        }
    }
    crate::sprite::Sheet::build_with(
        &[],
        vec![crate::sprite::DecodedImage {
            src: "Data\\HUD\\Textures\\PulseHUD.mip".into(),
            width,
            height,
            rgba: hud,
            quad_extent: None,
            quad_centre: None,
            blend: None,
        }],
        &mut Vec::new(),
    )
}

fn alpha(sheet: &crate::sprite::Sheet, x: u32, y: u32) -> u8 {
    let at = sheet.get(SRC).unwrap();
    sheet.rgba[(((at.y + y) * sheet.width + at.x + x) * 4 + 3) as usize]
}

#[test]
fn an_svg_is_rasterised_at_each_size_not_scaled_from_one_bitmap() {
    let layout = Layout::from_xml(LAYOUT);
    let base = base();
    let icon = VectorIcon::parse(SRC.into(), BAR.as_bytes()).unwrap();
    let mut seen = Vec::new();
    for pixels_per_unit in [2.0_f32, 3.0] {
        let mut report = Vec::new();
        let sheet = svg::place(
            &base,
            &layout,
            oag_pulse::hud::ART,
            std::slice::from_ref(&icon),
            pixels_per_unit,
            &mut report,
        );
        let at = sheet
            .get(SRC)
            .unwrap_or_else(|| panic!("placed: {report:?}"));
        // 16 units into the 20 by 27 glyph box: 20 units a side.
        let side = (20.0 * pixels_per_unit) as u32;
        assert_eq!(
            (at.width, at.height),
            (side, side),
            "at {pixels_per_unit} px/unit"
        );
        // The bar's left edge is on a unit boundary of the drawing: 4 of 16 is a quarter of the
        // way across, whatever the size, and it is one texel wide there.
        let edge = side / 4;
        assert_eq!(alpha(&sheet, edge - 1, 3), 0);
        assert_eq!(alpha(&sheet, edge, 3), 255);
        // The right edge falls a quarter into texel 31 at 2 px/unit: a partial texel there,
        // where a 16-texel bitmap scaled up would give a block of equal texels.
        if pixels_per_unit == 2.0 {
            let partial = alpha(&sheet, 31, 3);
            assert!(partial > 0 && partial < 255, "{partial}");
            assert_eq!(alpha(&sheet, 32, 3), 0);
        }
        seen.push(side);
        assert_eq!(sheet.texel_density(SRC), Some(pixels_per_unit));
    }
    assert_eq!(seen, [40, 60]);
}

#[test]
fn a_vector_icon_is_drawn_one_texel_to_one_pixel_on_the_glyphs_centre() {
    let layout = Layout::from_xml(LAYOUT);
    let strings = oag_ui::language::StringTable::default();
    let icon = VectorIcon::parse(SRC.into(), BAR.as_bytes()).unwrap();
    for pixels_per_unit in [1.0_f32, 2.5, 4.0] {
        let sheet = svg::place(
            &base(),
            &layout,
            oag_pulse::hud::ART,
            std::slice::from_ref(&icon),
            pixels_per_unit,
            &mut Vec::new(),
        );
        let cx = Context {
            default_border: layout.default_border(),
            layout: &layout,
            strings: &strings,
            sheet: &sheet,
            art: oag_pulse::hud::ART,
            hud_line_height: 25.0,
            small_line_height: 10.0,
            default_line_height: 10.0,
            skin_line_height: 10.0,
            tag_label: None,
        };
        let Some(Draw::Sprite { rect, uv, .. }) = mod_icon_draw_for_test(&cx, SRC) else {
            panic!("draws at {pixels_per_unit}");
        };
        for v in rect {
            let pixels = v * pixels_per_unit;
            assert!(
                (pixels - pixels.round()).abs() < 1e-3,
                "{rect:?} is not on whole pixels at {pixels_per_unit}"
            );
        }
        // Each texel of the raster covers exactly one screen pixel.
        assert!(
            (rect[2] * pixels_per_unit - uv[2]).abs() < 1e-3,
            "{rect:?} {uv:?}"
        );
        assert!(
            (rect[3] * pixels_per_unit - uv[3]).abs() < 1e-3,
            "{rect:?} {uv:?}"
        );
    }
}

#[test]
fn a_broken_svg_is_refused_with_the_parsers_words() {
    let error = VectorIcon::parse(SRC.into(), b"<svg").unwrap_err();
    assert!(!error.is_empty());
    assert!(VectorIcon::parse(SRC.into(), b"not xml at all").is_err());
}

#[test]
fn text_and_images_in_an_svg_draw_nothing() {
    let text = r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8">
<text x="0" y="8" font-size="8" fill="#fff">A</text>
<image width="8" height="8" href="file:///etc/hostname"/></svg>"##;
    let icon = VectorIcon::parse(SRC.into(), text.as_bytes()).unwrap();
    let pixels = icon.rasterise(8, 8).unwrap();
    assert!(pixels.iter().all(|&b| b == 0), "no glyph, no file");
}

#[test]
fn a_layout_without_a_pickup_slot_still_gets_the_icon_at_its_own_size() {
    let layout = Layout::from_xml("<Screen/>");
    let icon = VectorIcon::parse(SRC.into(), BAR.as_bytes()).unwrap();
    let sheet = svg::place(
        &crate::sprite::Sheet::default(),
        &layout,
        oag_pulse::hud::ART,
        std::slice::from_ref(&icon),
        2.0,
        &mut Vec::new(),
    );
    let at = sheet.get(SRC).expect("drawn for hud-icon");
    assert_eq!((at.width, at.height), (32, 32), "16 units at 2 px/unit");
}
