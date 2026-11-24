//! [`super::snapped_centre_pen`]: the start of Pure's centred text.

use oag_texture::fnt::{Font, Glyph};

use super::*;

/// `a` advance 3, `b` advance 13 with a 12 wide ink, `c` advance 12; every bearing 0.
fn atlas() -> Atlas {
    let glyph = |ch: u8, advance: u8, width: u8| Glyph {
        codepoint: u16::from(ch),
        width,
        height: 4,
        u0: 0,
        u1: u16::from(width),
        v0: 0,
        v1: 4,
        advance,
        bearing: 0,
    };
    Atlas::from_font(&Font {
        line_height: 8,
        unknown: 0,
        width: 16,
        height: 8,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 128],
        glyphs: vec![glyph(b'a', 3, 3), glyph(b'b', 13, 12), glyph(b'c', 12, 12)],
    })
}

/// What the layout did before the original's measure was read: the pen after the last glyph's
/// advance, halved and rounded up.
fn by_pen_end(atlas: &Atlas, text: &str, scale: f32, x: f32) -> f32 {
    (x - snapped_run(atlas, text, scale).end / 2.0).ceil()
}

/// The measure ends on the last glyph's ink, not its advance, so a string whose last glyph has
/// trailing space (`b`: advance 13, ink 12) is one unit narrower than its pen end says. At scale 1
/// `ab` measures 15: 240 less 7.5 is a tie that rounds up to 233, where the advance sum (16) gave
/// 232.
#[test]
fn an_odd_measure_at_scale_one_rounds_the_tie_up_on_the_ink_end() {
    let atlas = atlas();
    let pen = snapped_centre_pen(&atlas, "ab", 1.0, 240.0);
    assert_eq!(pen, 232.5);
    assert_eq!(pen.ceil(), 233.0);
    assert_eq!(by_pen_end(&atlas, "ab", 1.0, 240.0), 232.0);
}

/// At a scale off 1 the measure steps every glyph to a whole unit before it ends on the ink, so
/// the first column is not the raw sum's: `bcb` at 1.2 measures 44.8 (46.6 by advances) and starts
/// at 217.6, so its first quad is 218, not 217. `ccb` starts at 218.1, so 219, not 218.
#[test]
fn a_scaled_measure_steps_each_glyph_up_before_it_ends_on_the_ink() {
    let atlas = atlas();
    let start = |text| snapped_centre_pen(&atlas, text, 1.2, 240.0).ceil();
    assert_eq!(start("bcb"), 218.0);
    assert_eq!(by_pen_end(&atlas, "bcb", 1.2, 240.0), 217.0);
    assert_eq!(start("ccb"), 219.0);
    assert_eq!(by_pen_end(&atlas, "ccb", 1.2, 240.0), 218.0);
}

/// A first glyph's bearing moves the pen it starts from but not where its ink lands: the pen is
/// left unrounded for the glyph's own step.
#[test]
fn the_start_pen_is_one_bearing_left_of_the_ink() {
    let mut font = Font {
        line_height: 8,
        unknown: 0,
        width: 16,
        height: 8,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 128],
        glyphs: Vec::new(),
    };
    font.glyphs.push(Glyph {
        codepoint: u16::from(b'a'),
        width: 4,
        height: 4,
        u0: 0,
        u1: 4,
        v0: 0,
        v1: 4,
        advance: 6,
        bearing: 2,
    });
    let atlas = Atlas::from_font(&font);
    // One glyph: the measure starts at the ink, so it is the ink's own 4 and the ink spans
    // 238 to 242, centred on 240; the pen starts a bearing (2) left of 238.
    let pen = snapped_centre_pen(&atlas, "a", 1.0, 240.0);
    assert_eq!(pen, 236.0);
    assert_eq!(pen_step(&atlas.cell('a').unwrap(), 1.0, pen, true).0, 238.0);
}
