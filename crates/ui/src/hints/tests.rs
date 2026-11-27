use super::*;
use oag_title::prompts::{Prompts, Scope};

const TABLE: Prompts = Prompts {
    scope: Scope::AnyFace,
    glyphs: &[
        ('X', Prompt::Cross),
        ('Y', Prompt::Circle),
        ('Z', Prompt::Triangle),
        ('Q', Prompt::Up),
    ],
};

const BUTTONS_ONLY: Prompts = Prompts {
    scope: Scope::ButtonsFace,
    glyphs: &[('X', Prompt::Cross)],
};

fn text(role: Option<&'static str>, x: f32, y: f32, align: Align, words: &str) -> Draw {
    match role {
        None => Draw::Text {
            x,
            y,
            scale: 1.0,
            color: [1.0; 4],
            border: None,
            align,
            text: words.to_string(),
            wrap_width: None,
        },
        Some(role) => Draw::FacedText {
            role,
            x,
            y,
            scale: 1.0,
            color: [1.0; 4],
            border: None,
            align,
            text: words.to_string(),
            wrap_width: None,
        },
    }
}

fn found(list: &[Draw], clip: Option<usize>, substitution: &Substitution) -> Vec<Hint> {
    let atlas = Atlas::build();
    scan(list, clip, substitution, &|role| {
        Some(Face {
            atlas: &atlas,
            buttons: role == Some("Buttons"),
            pens: crate::font::Pens::Advance,
        })
    })
}

fn width(words: &str) -> f32 {
    crate::font::measure(&Atlas::build(), words)
}

#[test]
fn a_glyph_that_opens_a_string_is_a_hint_spanning_its_word() {
    let list = [text(None, 40.0, 200.0, Align::Left, "Y BACK")];
    let hints = found(&list, None, &Substitution::reading(&TABLE));
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].press, Press::Button(Button::Circle));
    assert_eq!(hints[0].rect[0], 40.0);
    assert_eq!(hints[0].rect[1], 200.0);
    assert!((hints[0].rect[2] - width("Y BACK")).abs() < 0.01);
}

#[test]
fn a_string_of_pairs_is_one_hint_each() {
    let list = [text(None, 10.0, 0.0, Align::Left, "X OK  Y BACK")];
    let hints = found(&list, None, &Substitution::reading(&TABLE));
    let presses: Vec<_> = hints.iter().map(|hint| hint.press).collect();
    assert_eq!(
        presses,
        [Press::Button(Button::Cross), Press::Button(Button::Circle)]
    );
    assert!(hints[0].rect[0] + hints[0].rect[2] <= hints[1].rect[0]);
}

#[test]
fn a_glyph_alone_takes_the_word_beside_it_on_its_row() {
    let list = [
        text(None, 40.0, 200.0, Align::Left, "X"),
        text(None, 60.0, 200.0, Align::Left, "OK"),
        text(None, 60.0, 100.0, Align::Left, "ABOVE"),
        text(None, 300.0, 200.0, Align::Left, "FAR"),
    ];
    let hints = found(&list, None, &Substitution::reading(&TABLE));
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].press, Press::Button(Button::Cross));
    assert_eq!(hints[0].rect[0], 40.0);
    assert!((hints[0].rect[0] + hints[0].rect[2] - (60.0 + width("OK"))).abs() < 0.01);
}

#[test]
fn a_glyph_in_a_sentence_is_not_a_hint() {
    let list = [text(None, 40.0, 200.0, Align::Left, "PRESS X TO GO")];
    assert!(found(&list, None, &Substitution::reading(&TABLE)).is_empty());
}

#[test]
fn a_d_pad_glyph_is_not_a_hint() {
    let list = [text(None, 40.0, 200.0, Align::Left, "Q MOVE")];
    assert!(found(&list, None, &Substitution::reading(&TABLE)).is_empty());
}

#[test]
fn a_buttons_face_table_reads_glyphs_in_that_face_only() {
    let substitution = Substitution::reading(&BUTTONS_ONLY);
    let letters = [text(Some("Default"), 40.0, 0.0, Align::Left, "X OK")];
    assert!(found(&letters, None, &substitution).is_empty());
    let glyphs = [text(Some("Buttons"), 40.0, 0.0, Align::Left, "X OK")];
    assert_eq!(found(&glyphs, None, &substitution).len(), 1);
}

#[test]
fn the_glyph_as_drawn_is_the_button_under_an_exchange() {
    let list = [
        text(None, 40.0, 200.0, Align::Left, "Y BACK"),
        text(None, 140.0, 200.0, Align::Left, "Z HELP"),
    ];
    let exchanged = Substitution::exchanging(&TABLE, Prompt::Circle, Prompt::Triangle);
    let hints = found(&list, None, &exchanged);
    assert_eq!(
        hints.iter().map(|hint| hint.press).collect::<Vec<_>>(),
        [
            Press::Button(Button::Triangle),
            Press::Button(Button::Circle)
        ],
        "the circle's stand-in is drawn as the triangle, so it taps the triangle"
    );
    let plain = found(&list, None, &Substitution::reading(&TABLE));
    assert_eq!(plain[0].press, Press::Button(Button::Circle));
}

#[test]
fn the_clipped_line_and_wrapped_text_are_never_hints() {
    let mut wrapped = text(None, 40.0, 0.0, Align::Left, "X OK");
    if let Draw::Text { wrap_width, .. } = &mut wrapped {
        *wrap_width = Some(100.0);
    }
    let list = [text(None, 40.0, 200.0, Align::Left, "X OK"), wrapped];
    assert!(found(&list, Some(0), &Substitution::reading(&TABLE)).is_empty());
}

#[test]
fn alignment_moves_the_rect_with_the_string() {
    let list = [text(None, 200.0, 0.0, Align::Right, "X OK")];
    let hints = found(&list, None, &Substitution::reading(&TABLE));
    assert!((hints[0].rect[0] - (200.0 - width("X OK"))).abs() < 0.01);
}

#[test]
fn hit_answers_the_hint_under_the_point() {
    let list = [text(None, 40.0, 200.0, Align::Left, "Y BACK")];
    let hints = found(&list, None, &Substitution::reading(&TABLE));
    assert_eq!(
        hit(&hints, (45.0, 203.0)),
        Some(Press::Button(Button::Circle))
    );
    assert_eq!(hit(&hints, (39.0, 203.0)), None);
    assert_eq!(hit(&hints, (45.0, 150.0)), None);
}

#[test]
fn a_legend_line_puts_a_rect_over_each_entry_that_names_a_press() {
    let atlas = Atlas::build();
    let entries = [
        Entry::button(Button::Cross, "X OK"),
        Entry::new(None, "STICK MOVE").after("   "),
        Entry::new(Some(Press::Escape), "ESC QUIT").after("   "),
    ];
    let line = Line {
        entries: &entries,
        x: 16.0,
        y: 100.0,
        align: Align::Left,
        height: 10.0,
    };
    assert_eq!(line_text(&entries), "X OK   STICK MOVE   ESC QUIT");
    let hints = legend(&atlas, 1.0, &[line]);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0].press, Press::Button(Button::Cross));
    assert_eq!(hints[0].rect, [16.0, 100.0, width("X OK"), 10.0]);
    assert_eq!(hints[1].press, Press::Escape);
    let start = 16.0 + width("X OK   STICK MOVE   ");
    assert!((hints[1].rect[0] - start).abs() < 0.01);
    assert!((hints[1].rect[2] - width("ESC QUIT")).abs() < 0.01);
}

#[test]
fn a_centred_legend_line_is_centred_as_a_whole() {
    let atlas = Atlas::build();
    let entries = [Entry::button(Button::Square, "SQ HUD")];
    let line = Line {
        entries: &entries,
        x: 200.0,
        y: 0.0,
        align: Align::Centre,
        height: 9.0,
    };
    let hints = legend(&atlas, 0.5, &[line]);
    let half = width("SQ HUD") * 0.5;
    assert!((hints[0].rect[0] - (200.0 - half / 2.0)).abs() < 0.01);
    assert!((hints[0].rect[2] - half).abs() < 0.01);
}

#[test]
fn a_wrapped_legend_puts_a_rect_on_each_line_an_entry_falls_on() {
    let atlas = Atlas::build();
    let entries = [
        Entry::button(Button::Cross, "X TYPE"),
        Entry::button(Button::Circle, "Y CANCEL").after("   "),
    ];
    let place = |width| Wrapped {
        x: 10.0,
        y: 50.0,
        scale: 1.0,
        width,
    };
    let one = legend_wrapped(
        &atlas,
        place(10_000.0),
        &entries,
        crate::font::Pens::Advance,
    );
    assert_eq!(one.len(), 2);
    // `wrap` joins the words with one space, as the renderer draws them.
    assert_eq!(one[0].rect[0], 10.0);
    assert_eq!(one[0].rect[1], 50.0);
    assert!((one[0].rect[2] - width("X TYPE")).abs() < 0.01);
    assert_eq!(one[1].rect[1], 50.0);
    assert!((one[1].rect[0] - (10.0 + width("X TYPE "))).abs() < 0.01);

    // Narrow enough that the second entry wraps below the first.
    let narrow = legend_wrapped(
        &atlas,
        place(width("X TYPE") + 2.0),
        &entries,
        crate::font::Pens::Advance,
    );
    let cancel: Vec<_> = narrow
        .iter()
        .filter(|hint| hint.press == Press::Button(Button::Circle))
        .collect();
    assert!(!cancel.is_empty());
    assert!(cancel.iter().all(|hint| hint.rect[1] > 50.0), "{narrow:?}");
    assert_eq!(cancel[0].rect[0], 10.0);
}

/// Space, `a` and `b`: 7 texels wide, advance 8, bearing 1, for a snapped face.
fn snapped_atlas() -> Atlas {
    use oag_texture::fnt::{Font, Glyph};
    let glyph = |ch: u8| Glyph {
        codepoint: u16::from(ch),
        width: 7,
        height: 4,
        u0: 0,
        u1: 7,
        v0: 0,
        v1: 4,
        advance: 8,
        bearing: 1,
    };
    let font = Font {
        line_height: 8,
        unknown: 0,
        width: 8,
        height: 8,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 64],
        glyphs: vec![glyph(b' '), glyph(b'a'), glyph(b'b'), glyph(b'X')],
    };
    Atlas::from_font(&font)
}

/// A string that opens with a button glyph is cut on the same stepped pens: three 9-advance
/// glyphs of bearing -1 at 0.8 end at 24 from x, where the fraction ends at 22.4.
#[test]
fn a_bearing_hint_ends_on_the_stepped_pen() {
    use oag_texture::fnt::{Font, Glyph};
    let glyph = |ch: u8| Glyph {
        codepoint: u16::from(ch),
        width: 7,
        height: 4,
        u0: 0,
        u1: 7,
        v0: 0,
        v1: 4,
        advance: 9,
        bearing: -1,
    };
    let font = Font {
        line_height: 8,
        unknown: 0,
        width: 16,
        height: 8,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 128],
        glyphs: vec![glyph(b'X'), glyph(b'a')],
    };
    let atlas = Atlas::from_font(&font);
    let mut list = [text(None, 10.0, 0.0, Align::Left, "Xaa")];
    let Draw::Text { scale, .. } = &mut list[0] else {
        unreachable!("text(None, ..) is a Draw::Text")
    };
    *scale = 0.8;
    let hints = scan(&list, None, &Substitution::reading(&TABLE), &|_| {
        Some(Face {
            atlas: &atlas,
            buttons: false,
            pens: crate::font::Pens::Bearing,
        })
    });
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].rect[0], 10.0);
    assert!(
        (hints[0].rect[2] - 24.0).abs() < 1e-3,
        "{:?}",
        hints[0].rect
    );
}

/// On a snapped face a wrapped legend's rectangle covers the ink as drawn: each glyph of `aaa bbb` steps up to
/// a whole unit, so the line is wider than its fractional advances say and a tap on its last glyph has to land.
#[test]
fn a_snapped_legend_rect_covers_its_drawn_ink() {
    let atlas = snapped_atlas();
    let entries = [Entry::button(Button::Cross, "aaabbb")];
    let place = Wrapped {
        x: 10.0,
        y: 50.0,
        scale: 0.7,
        width: 10_000.0,
    };
    let rect_right = |pens| {
        let hints = legend_wrapped(&atlas, place, &entries, pens);
        hints[0].rect[0] + hints[0].rect[2]
    };
    let ink_right = 10.0 + crate::font::snapped_run(&atlas, "aaabbb", 0.7).ink_right;
    assert!(
        rect_right(crate::font::Pens::Snapped) >= ink_right - 1e-3,
        "the rect ends at {}, the ink at {ink_right}",
        rect_right(crate::font::Pens::Snapped)
    );
    assert!(
        rect_right(crate::font::Pens::Advance) < ink_right - 0.1,
        "the fractional rect would stop short of the drawn ink"
    );
}

/// A snapped legend line is one pen across all its words, as the draw steps it: the second entry's
/// rectangle starts where the text before it puts the pen (rounded up) and the last ends on the
/// whole line's ink, not where each word would end measured alone.
#[test]
fn a_snapped_legend_of_two_entries_follows_one_pen_across_the_line() {
    let atlas = snapped_atlas();
    let entries = [
        Entry::button(Button::Cross, "aaa"),
        Entry::button(Button::Circle, "bbb").after(" "),
    ];
    let place = Wrapped {
        x: 10.0,
        y: 50.0,
        scale: 0.7,
        width: 10_000.0,
    };
    let hints = legend_wrapped(&atlas, place, &entries, crate::font::Pens::Snapped);
    assert_eq!(hints.len(), 2, "{hints:?}");
    let run = |text: &str| crate::font::snapped_run(&atlas, text, 0.7);
    let (first, second) = (hints[0].rect, hints[1].rect);
    assert_eq!(first[0], 10.0);
    assert!((first[0] + first[2] - (10.0 + run("aaa").ink_right)).abs() < 1e-3);
    assert_eq!(second[0], 10.0 + run("aaa ").end.ceil());
    assert!((second[0] + second[2] - (10.0 + run("aaa bbb").ink_right)).abs() < 1e-3);
    let alone = 10.0 + run("aaa").end + run(" ").end;
    assert!(
        second[0] > alone,
        "words measured alone would put it at {alone}"
    );
}

/// On a bearing face (Pulse's front end) a wrapped legend's rectangle runs from the first glyph's ink to the
/// last glyph's ink, which `Renderer::push_text` draws from the text's x: with a bearing of -1 the ink ends
/// `bearing + ink - advance` short of the advance sum, so the advance rectangle overshoots it.
#[test]
fn a_bearing_legend_rect_covers_its_drawn_ink() {
    use oag_texture::fnt::{Font, Glyph};
    let glyph = |ch: u8| Glyph {
        codepoint: u16::from(ch),
        width: 7,
        height: 4,
        u0: 0,
        u1: 7,
        v0: 0,
        v1: 4,
        advance: 9,
        bearing: -1,
    };
    let font = Font {
        line_height: 8,
        unknown: 0,
        width: 16,
        height: 8,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 128],
        glyphs: vec![glyph(b'a'), glyph(b'b')],
    };
    let atlas = Atlas::from_font(&font);
    let entries = [Entry::button(Button::Cross, "aaabbb")];
    let place = Wrapped {
        x: 10.0,
        y: 50.0,
        scale: 1.0,
        width: 10_000.0,
    };
    let right = |pens| {
        let hints = legend_wrapped(&atlas, place, &entries, pens);
        hints[0].rect[0] + hints[0].rect[2]
    };
    let ink_right = 10.0 + crate::font::ink_width(&atlas, "aaabbb", 1.0, true);
    // At 0.8 the ink edges step up to whole units: lefts 0, 8, 16, 24, 32, 40 from x, the last
    // glyph 5.6 wide, where the fraction would end at 41.6.
    let scaled = legend_wrapped(
        &atlas,
        Wrapped {
            scale: 0.8,
            ..place
        },
        &entries,
        crate::font::Pens::Bearing,
    );
    assert!(
        (scaled[0].rect[0] + scaled[0].rect[2] - 55.6).abs() < 1e-3,
        "{:?}",
        scaled[0].rect
    );
    assert!(
        (right(crate::font::Pens::Bearing) - ink_right).abs() < 1e-3,
        "the rect ends at {}, the ink at {ink_right}",
        right(crate::font::Pens::Bearing)
    );
    assert!(
        right(crate::font::Pens::Advance) > ink_right + 1.0,
        "the advance rect would overshoot the drawn ink"
    );
}

/// A right-aligned line on a snapped face ends its last quad on the anchor, as the draw does
/// (`oag_game::render::Renderer::push_text`): the rectangle over `X aa` starts where `ink_right`
/// puts the pen, not where the pen's end would, so a tap on its first glyph lands.
#[test]
fn a_snapped_right_aligned_line_ends_on_its_anchor() {
    let atlas = snapped_atlas();
    let list = [text(Some("Stats"), 200.0, 0.0, Align::Right, "X aa")];
    let hints = scan(&list, None, &Substitution::reading(&TABLE), &|_| {
        Some(Face {
            atlas: &atlas,
            buttons: false,
            pens: crate::font::Pens::Snapped,
        })
    });
    let run = crate::font::snapped_run(&atlas, "X aa", 1.0);
    assert!(
        run.ink_right < run.end,
        "the trailing advance is what differs"
    );
    assert_eq!(hints[0].rect[0], (200.0 - run.ink_right).ceil());
}

/// A centred line on a snapped face starts where the original's measure puts it
/// (`oag_ui::font::snapped_centre_pen`), not at half its pen end: with glyphs 9 units advance and 7 of
/// ink, `X aa` measures 34 (the pen end is 36), so its rectangle starts at 183 and not at 182.
#[test]
fn a_snapped_centred_line_starts_at_half_its_measure() {
    use oag_texture::fnt::{Font, Glyph};
    let glyph = |ch: u8| Glyph {
        codepoint: u16::from(ch),
        width: 7,
        height: 4,
        u0: 0,
        u1: 7,
        v0: 0,
        v1: 4,
        advance: 9,
        bearing: 0,
    };
    let atlas = Atlas::from_font(&Font {
        line_height: 8,
        unknown: 0,
        width: 8,
        height: 8,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 64],
        glyphs: vec![glyph(b' '), glyph(b'a'), glyph(b'X')],
    });
    let list = [text(Some("Stats"), 200.0, 0.0, Align::Centre, "X aa")];
    let hints = scan(&list, None, &Substitution::reading(&TABLE), &|_| {
        Some(Face {
            atlas: &atlas,
            buttons: false,
            pens: crate::font::Pens::Snapped,
        })
    });
    let run = crate::font::snapped_run(&atlas, "X aa", 1.0);
    assert_eq!(run.end, 36.0, "the pen end is not the measure");
    assert_eq!(hints[0].rect[0], 183.0);
}

/// A centred snapped line whose first glyph has a bearing keeps its pen unrounded, as the draw
/// does: `X O` at 0.75 starts at 233.25, the `X` steps to 234 and the `O` quad sits at 243, where
/// a pen rounded first (234, then 235) would put it at 244.
#[test]
fn a_centred_snapped_line_with_a_leading_bearing_follows_the_draw() {
    use oag_texture::fnt::{Font, Glyph};
    let glyph = |ch: u8, advance: u8, width: u8, bearing: i8| Glyph {
        codepoint: u16::from(ch),
        width,
        height: 4,
        u0: 0,
        u1: u16::from(width),
        v0: 0,
        v1: 4,
        advance,
        bearing,
    };
    let atlas = Atlas::from_font(&Font {
        line_height: 8,
        unknown: 0,
        width: 16,
        height: 8,
        palette: vec![[255, 255, 255, 255]; 16],
        indices: vec![1; 128],
        glyphs: vec![
            glyph(b' ', 4, 1, 0),
            glyph(b'O', 8, 4, 0),
            glyph(b'X', 9, 7, 1),
        ],
    });
    let mut list = [text(Some("Stats"), 240.0, 0.0, Align::Centre, "X O")];
    if let Draw::FacedText { scale, .. } = &mut list[0] {
        *scale = 0.75;
    }
    let hints = scan(&list, None, &Substitution::reading(&TABLE), &|_| {
        Some(Face {
            atlas: &atlas,
            buttons: false,
            pens: crate::font::Pens::Snapped,
        })
    });
    assert_eq!(
        crate::font::snapped_centre_pen(&atlas, "X O", 0.75, 240.0),
        233.25
    );
    assert_eq!(hints[0].rect[0], 234.0);
    let right = hints[0].rect[0] + hints[0].rect[2];
    assert_eq!(right, 246.0, "the O's quad ends at 246, not 247");
}
