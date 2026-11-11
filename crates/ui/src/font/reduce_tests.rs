use super::*;
use oag_title::prompts::{Prompt, Prompts, Scope};

const PROMPTS: Prompts = Prompts {
    scope: Scope::AnyFace,
    glyphs: &[('A', Prompt::Cross)],
};

fn font() -> fnt::Font {
    fnt::Font {
        line_height: 12,
        unknown: 0,
        width: 16,
        height: 8,
        palette: vec![[0, 0, 0, 0], [200, 200, 200, 255]],
        indices: vec![0; 16 * 8],
        glyphs: vec![fnt::Glyph {
            codepoint: u16::from(b'A'),
            width: 4,
            height: 4,
            u0: 2,
            u1: 6,
            v0: 1,
            v1: 5,
            advance: 5,
        }],
    }
}

/// The face's atlas drawn from a 16x picture whose only ink is the glyph's box.
fn sixteen_times() -> Atlas {
    let (width, height) = (16 * 16, 8 * 16);
    let rgba = (0..width * height)
        .flat_map(|at| {
            let (x, y) = (at % width, at / width);
            let ink = (32..96).contains(&x) && (16..80).contains(&y);
            [255, 255, 255, if ink { 255 } else { 0 }]
        })
        .collect();
    let picture = oag_texture::replacer::Image {
        width,
        height,
        rgba,
    };
    Atlas::from_replaced_font(&font(), &picture).unwrap()
}

fn ink_in(atlas: &Atlas, cell: Cell) -> u32 {
    (0..cell.height)
        .flat_map(|y| (0..cell.width).map(move |x| (x, y)))
        .map(|(x, y)| u32::from(atlas.coverage[((cell.y + y) * atlas.width + cell.x + x) as usize]))
        .sum()
}

#[test]
fn a_reduced_face_keeps_its_glyphs_size_on_screen_and_its_ink() {
    let full = sixteen_times();
    let half = full.reduced(2);
    assert_eq!((half.width, half.height), (128, 66));
    let cell = half.cell('A').unwrap();
    assert_eq!((cell.x, cell.y, cell.width, cell.height), (16, 8, 32, 32));
    assert_eq!(half.texel_scale, full.texel_scale * 2.0);
    assert_eq!(measure(&half, "A"), measure(&full, "A"));
    assert_eq!(ink_in(&half, cell), 32 * 32 * 255, "all of the box is ink");
    assert_eq!(half.coverage[0], 0, "and nothing outside it is");
}

#[test]
fn a_reduced_face_keeps_an_opaque_two_by_two_for_fills() {
    let half = sixteen_times().reduced(4);
    let solid = half.solid;
    assert_eq!((solid.width, solid.height), (2, 2));
    assert_eq!(solid.y + 2, half.height, "the patch is still the last rows");
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let at = ((solid.y + dy) * half.width + solid.x + dx) as usize;
        assert_eq!((half.coverage[at], half.luma[at]), (255, 255));
    }
}

#[test]
fn button_glyph_rows_below_the_patch_survive_a_reduction_to_a_smaller_texture() {
    let full = sixteen_times().with_prompts(&PROMPTS);
    assert!(full.height > 1024, "{} rows", full.height);
    let side = 512;
    let fitted = full.clone().fitting(side);
    assert!(fitted.width.max(fitted.height) <= side);
    let substitute = fitted.cell(crate::prompt::sub_char(0, 0)).unwrap();
    assert!(substitute.y + substitute.height <= fitted.height);
    assert!(ink_in(&fitted, substitute) > 0, "the glyph is still drawn");
    assert_eq!(
        measure(&fitted, "A"),
        measure(&full, "A"),
        "a smaller texture moves no line"
    );
}

#[test]
fn a_face_that_fits_is_returned_untouched() {
    let full = sixteen_times();
    let same = full.clone().fitting(256);
    assert_eq!((same.width, same.height), (full.width, full.height));
    assert_eq!(same.texel_scale, full.texel_scale);
}

#[test]
fn the_built_in_set_is_never_reduced() {
    let built = Atlas::build();
    assert_eq!(built.reduced(4).width, built.width);
}
