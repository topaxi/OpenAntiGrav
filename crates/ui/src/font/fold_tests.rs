use super::*;

/// The bug the user saw: a letter, not just its accent, went missing.
#[test]
fn accented_letters_are_never_dropped() {
    let atlas = Atlas::build();
    for name in ["Français", "Español", "Português", "Türkçe", "Íslenska"] {
        for ch in name.chars() {
            assert!(
                atlas.cell(ch).is_some(),
                "{name:?}: {ch:?} has no cell, so it would vanish from the screen"
            );
        }
    }
}

#[test]
fn the_language_names_measure_their_full_length() {
    let atlas = Atlas::build();
    for name in ["Français", "Español", "Italiano", "Deutsch", "English"] {
        let expected = name.chars().count() as u32 * (GLYPH_WIDTH + 1);
        assert_eq!(
            measure(&atlas, name),
            expected as f32,
            "{name:?} must measure every character, or it draws off-centre"
        );
    }
}

#[test]
fn case_folds_beyond_ascii() {
    let atlas = Atlas::build();
    // The accented pair share one glyph, and both resolve.
    assert_eq!(atlas.cell('ç'), atlas.cell('Ç'));
    assert_eq!(atlas.cell('ñ'), atlas.cell('Ñ'));
    assert_eq!(atlas.cell('é'), atlas.cell('É'));
}

/// An accent we do not draw must cost the accent, not the letter.
#[test]
fn an_unknown_accent_falls_back_to_its_base_letter() {
    let atlas = Atlas::build();
    // A-ring is not in the set; it must land on 'A' rather than vanish.
    assert_eq!(atlas.cell('Å'), atlas.cell('A'));
    assert_eq!(atlas.cell('å'), atlas.cell('A'));
    assert_eq!(atlas.cell('Ø'), atlas.cell('O'));
    assert_eq!(base_letter('Ç'), 'C');
    assert_eq!(base_letter('Z'), 'Z');
}

/// Every accented glyph must still leave its base letter recognisable.
#[test]
fn accented_glyphs_keep_ink_below_their_diacritic() {
    for (ch, rows) in GLYPHS {
        if (*ch as u32) < 0xc0 {
            continue;
        }
        let ink: usize = rows[2..]
            .iter()
            .map(|r| r.bytes().filter(|&b| b == b'#').count())
            .sum();
        assert!(
            ink >= 6,
            "glyph {ch:?} has only {ink} ink below its diacritic, which will not read as a letter"
        );
    }
}

/// Builds a `.fnt` by hand so the real-font path is tested without game
/// data, the same way `oag-formats` tests its own decoder.
fn synthetic_font(glyphs: &[(u16, u8, u8, u16, u16)]) -> Vec<u8> {
    const HEADER: usize = 0x30;
    const RECORD: usize = 18;
    let count = glyphs.len();
    let codepoints_at = HEADER;
    let offsets_at = codepoints_at + count * 2;
    let records_at = offsets_at + count * 4;
    let atlas_at = records_at + count * RECORD;
    let (width, height): (u16, u16) = (64, 16);

    let mut out = vec![0u8; HEADER];
    out[0] = 1;
    out[1..4].copy_from_slice(b"FNT");
    out[4..8].copy_from_slice(&(count as u32).to_le_bytes());
    out[8..12].copy_from_slice(&(codepoints_at as u32).to_le_bytes());
    out[12..16].copy_from_slice(&(offsets_at as u32).to_le_bytes());
    out[16..20].copy_from_slice(&13u32.to_le_bytes());
    out[24..28].copy_from_slice(&(atlas_at as u32).to_le_bytes());

    for g in glyphs {
        out.extend_from_slice(&g.0.to_le_bytes());
    }
    for i in 0..count {
        out.extend_from_slice(&((records_at + i * RECORD) as u32).to_le_bytes());
    }
    for &(codepoint, w, h, u0, v0) in glyphs {
        out.extend_from_slice(&codepoint.to_le_bytes());
        out.push(w);
        out.push(h);
        out.extend_from_slice(&u0.to_le_bytes());
        out.extend_from_slice(&(u0 + u16::from(w)).to_le_bytes());
        out.extend_from_slice(&v0.to_le_bytes());
        out.extend_from_slice(&(v0 + u16::from(h)).to_le_bytes());
        out.push(w + 1);
        out.extend_from_slice(&[0xff; 5]);
    }

    let texels = usize::from(width) * usize::from(height) / 2;
    let mut atlas = vec![0u8; 0x40];
    atlas[0..2].copy_from_slice(&width.to_le_bytes());
    atlas[2..4].copy_from_slice(&height.to_le_bytes());
    atlas[4] = 4;
    atlas[5] = 1;
    atlas[6] = 0; // stored linear, so the fixture needs no swizzler
    atlas[8..12].copy_from_slice(&64u32.to_le_bytes());
    atlas[12..16].copy_from_slice(&(texels as u32).to_le_bytes());
    for i in 0..16u8 {
        atlas.extend_from_slice(&[255, 255, 255, i * 17]);
    }
    // Index 15 everywhere: fully opaque, so coverage is easy to assert on.
    atlas.extend(std::iter::repeat_n(0xffu8, texels));
    out.extend_from_slice(&atlas);
    out
}

#[test]
fn a_real_font_supplies_its_own_boxes_and_advances() {
    let blob = synthetic_font(&[(b'A' as u16, 6, 9, 0, 0), (b'i' as u16, 2, 9, 8, 0)]);
    let font = fnt::Font::parse(&blob).expect("parse");
    let atlas = Atlas::from_font(&font);
    assert!(atlas.is_real());
    assert!((atlas.line_height - 13.0).abs() < f32::EPSILON);

    let wide = atlas.cell('A').expect("A");
    let narrow = atlas.cell('i').expect("i");
    assert_eq!((wide.width, wide.height), (6, 9));
    assert_eq!((narrow.x, narrow.width), (8, 2));
    // Proportional, unlike the built-in set where every advance is 6.
    assert!(narrow.advance < wide.advance);
    assert!((measure(&atlas, "Ai") - (wide.advance + narrow.advance)).abs() < f32::EPSILON);
}

#[test]
fn a_texel_scale_moves_line_height_and_measure_and_one_moves_nothing() {
    let blob = synthetic_font(&[(b'A' as u16, 6, 9, 0, 0)]);
    let font = fnt::Font::parse(&blob).expect("parse");
    let plain = Atlas::from_font(&font);
    let same = Atlas::from_font(&font).with_texel_scale(1.0);
    assert_eq!(same.line_height, plain.line_height);
    assert_eq!(measure(&same, "AA"), measure(&plain, "AA"));

    let half = Atlas::from_font(&font).with_texel_scale(0.5);
    assert_eq!(half.line_height, plain.line_height * 0.5);
    assert_eq!(measure(&half, "AA"), measure(&plain, "AA") * 0.5);
    assert_eq!(half.cell('A'), plain.cell('A'), "texel boxes stay texels");
}

#[test]
fn a_real_atlas_keeps_room_for_the_solid_patch() {
    let blob = synthetic_font(&[(b'A' as u16, 6, 9, 0, 0)]);
    let font = fnt::Font::parse(&blob).expect("parse");
    let atlas = Atlas::from_font(&font);
    // The `.fnt` block is exactly its own pixels, so the patch needs rows
    // that the font itself does not provide.
    assert_eq!(atlas.height, u32::from(font.height) + 2);
    for dy in 0..2 {
        for dx in 0..2 {
            let at = ((atlas.solid.y + dy) * atlas.width + atlas.solid.x) as usize + dx as usize;
            assert_eq!(atlas.coverage[at], 0xff, "solid patch at +{dx},+{dy}");
        }
    }
}

#[test]
fn a_real_font_prefers_its_own_lower_case_over_folding() {
    // The built-in set has no lower case and folds; a real font does, and
    // folding first would throw away glyphs the disc actually carries.
    let blob = synthetic_font(&[(b'A' as u16, 6, 9, 0, 0), (b'a' as u16, 5, 9, 16, 0)]);
    let font = fnt::Font::parse(&blob).expect("parse");
    let atlas = Atlas::from_font(&font);
    assert_eq!(atlas.cell('a').expect("a").x, 16);
    assert_eq!(atlas.cell('A').expect("A").x, 0);
}

#[test]
fn a_real_font_still_folds_an_accent_it_does_not_carry() {
    // The fold that stopped `Français` becoming `FRANAIS` has to survive
    // the switch to real glyphs, because a real font can be missing a
    // character too.
    let blob = synthetic_font(&[(b'C' as u16, 6, 9, 0, 0)]);
    let font = fnt::Font::parse(&blob).expect("parse");
    let atlas = Atlas::from_font(&font);
    assert!(atlas.cell('\u{e7}').is_some(), "c-cedilla lost its letter");
    assert_eq!(atlas.cell('\u{e7}'), atlas.cell('C'));
    assert!(
        atlas.cell('\u{4e2d}').is_none(),
        "unrelated scripts still skip"
    );
}

#[test]
fn the_fallback_is_still_the_fallback() {
    let atlas = Atlas::build();
    assert!(!atlas.is_real());
    let cell = atlas.cell('A').expect("A");
    assert_eq!((cell.width, cell.height), (GLYPH_WIDTH, GLYPH_HEIGHT));
}

#[test]
fn a_border_grows_the_quad_on_every_side_and_leaves_the_advance_alone() {
    let blob = synthetic_font(&[(b'A' as u16, 6, 9, 8, 4)]);
    let font = fnt::Font::parse(&blob).expect("parse");
    let plain = Atlas::from_font(&font);
    let bordered = Atlas::from_font(&font).with_border_extend(3);
    let cell = bordered.cell('A').expect("A");

    let (rect, uv) = plain.glyph_quad(&cell, 10.0, 20.0, 2.0);
    assert_eq!(rect, [10.0, 20.0, 12.0, 18.0], "no border: the metric box");
    assert_eq!(uv, [8.0, 4.0, 6.0, 9.0]);

    let (rect, uv) = bordered.glyph_quad(&cell, 10.0, 20.0, 2.0);
    assert_eq!(
        uv,
        [5.0, 1.0, 12.0, 15.0],
        "3 texels past the box all round"
    );
    assert_eq!(rect, [4.0, 14.0, 24.0, 30.0], "3 texels at scale 2 past it");
    assert_eq!(
        measure(&bordered, "AA"),
        measure(&plain, "AA"),
        "pen unmoved"
    );
}

#[test]
fn an_extended_quad_never_samples_past_the_glyph_rows_or_the_atlas() {
    let blob = synthetic_font(&[(b'A' as u16, 6, 9, 0, 7)]);
    let font = fnt::Font::parse(&blob).expect("parse");
    let atlas = Atlas::from_font(&font).with_border_extend(5);
    let cell = atlas.cell('A').expect("A");
    let (rect, uv) = atlas.glyph_quad(&cell, 0.0, 0.0, 1.0);
    let glyph_rows = u32::from(font.height) as f32;
    assert_eq!(uv[0], 0.0, "left edge stops at the atlas edge");
    assert!(
        uv[1] + uv[3] <= glyph_rows,
        "bottom stops above the solid patch"
    );
    assert_eq!(
        rect[2], uv[2],
        "texel size on screen is unchanged by the clamp"
    );
    assert_eq!(rect[3], uv[3]);
}
