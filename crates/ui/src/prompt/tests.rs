use super::*;
use oag_title::prompts::Prompt;

const PROMPTS: Prompts = Prompts {
    scope: Scope::AnyFace,
    glyphs: &[('A', Prompt::Cross), ('B', Prompt::L)],
};

#[test]
fn every_art_cell_has_ink_and_a_name() {
    assert_eq!(names().len() * ART_CELL * ART_CELL, ART.len());
    for index in 0..names().len() {
        assert!(art(index).iter().any(|&a| a > 128), "{}", names()[index]);
    }
    assert!(names().len() <= 64);
}

#[test]
fn a_substitute_takes_the_disc_glyphs_box_and_advance() {
    let atlas = Atlas::build();
    let original = atlas.cell('A').unwrap();
    let swapped = atlas.with_prompts(&PROMPTS);
    let cross = art_index("xbox-a").unwrap();
    let cell = swapped.cell(sub_char(0, cross)).expect("a cell was added");
    assert_eq!((cell.width, cell.height), (original.width, original.height));
    assert_eq!(cell.advance, original.advance);
    let ink: u32 = (0..cell.height)
        .flat_map(|y| (0..cell.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            u32::from(swapped.coverage[((cell.y + y) * swapped.width + cell.x + x) as usize])
        })
        .sum();
    assert!(ink > 0, "the substitute is drawn into the atlas");
}

#[test]
fn a_stand_in_the_atlas_lacks_gets_no_cells() {
    let swapped = Atlas::build().with_prompts(&Prompts {
        scope: Scope::AnyFace,
        glyphs: &[('\u{3b5}', Prompt::Cross)],
    });
    assert!(swapped.cell(sub_char(0, 0)).is_none());
}

#[test]
fn applying_a_substitution_swaps_only_the_stand_ins() {
    let cross = art_index("sony-a").unwrap();
    let substitution = Substitution::none().with(&PROMPTS, 0, Some(cross));
    let out = substitution.apply("PRESS A TO GO", false);
    assert_eq!(out.chars().filter(|c| *c == 'A').count(), 0);
    assert_eq!(out.chars().count(), "PRESS A TO GO".chars().count());
    assert!(out.contains(sub_char(0, cross)));
    assert!(matches!(
        Substitution::none().apply("PRESS A", false),
        Cow::Borrowed(_)
    ));
}

#[test]
fn a_buttons_face_table_leaves_other_faces_alone() {
    const TABLE: Prompts = Prompts {
        scope: Scope::ButtonsFace,
        glyphs: &[('A', Prompt::Cross)],
    };
    let substitution = Substitution::none().with(&TABLE, 0, art_index("xbox-a"));
    assert_eq!(substitution.apply("A", false), "A");
    assert_ne!(substitution.apply("A", true), "A");
}
