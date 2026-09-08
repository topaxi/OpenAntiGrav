//! What [`super::apply`] does to a model's texture slots. No game data in any
//! test: the skin and the hull are both built by hand.

use super::*;
use crate::mesh::model_texture::Texels;

/// A skin whose four blocks are distinguishable: block `n`'s palette entry 0
/// is `[n, n, n, 255]`, so `to_rgba()`'s first pixel names which block it
/// came from.
fn skin() -> ship_skin::Skin {
    let sizes = [
        (ship_skin::LARGE_SIDE, ship_skin::LARGE_SIDE),
        (ship_skin::LARGE_SIDE, ship_skin::LARGE_SIDE),
        (ship_skin::LARGE_SIDE, ship_skin::LARGE_SIDE),
        (ship_skin::SMALL_SIDE, ship_skin::SMALL_SIDE),
    ];
    let blocks = std::array::from_fn(|n| {
        let (width, height) = sizes[n];
        let mut palette = [[0u8, 0, 0, 255]; 16];
        palette[0] = [n as u8, n as u8, n as u8, 255];
        ship_skin::Block {
            width,
            height,
            palette,
            indices: vec![0u8; width * height],
        }
    });
    ship_skin::Skin {
        team_name: "Test".to_string(),
        blocks,
    }
}

/// One texture slot, with a label and a tiny distinct picture so a test can
/// tell a replaced slot from an untouched one.
fn texture(label: &str) -> Option<std::sync::Arc<ModelTexture>> {
    Some(std::sync::Arc::new(ModelTexture::rgba8(
        label.to_string(),
        2,
        2,
        vec![9, 9, 9, 255, 9, 9, 9, 255, 9, 9, 9, 255, 9, 9, 9, 255],
        None,
    )))
}

fn rgba(texture: &ModelTexture) -> &[u8] {
    match &texture.texels {
        Texels::Rgba8(bytes) => bytes,
        Texels::Blocks { .. } => panic!("apply never produces a block-compressed texture"),
    }
}

/// The ordinary case: four slots named the way a race hull's own model
/// authors them, all four replaced.
#[test]
fn all_four_named_slots_are_replaced() {
    let mut hull = Model::none("test hull");
    hull.textures = vec![
        texture("texture1.tga"),
        texture("texture2.tga"),
        texture("texture3.tga"),
        texture("texture4.tga"),
    ];
    let skin = skin();

    let applied = apply(&mut hull, &skin);
    assert_eq!(applied, 4);
    for (index, slot) in hull.textures.iter().enumerate() {
        let texture = slot.as_ref().expect("still Some after replacement");
        assert_eq!(texture.width, skin.blocks[index].width as u32);
        assert_eq!(texture.height, skin.blocks[index].height as u32);
        // Block n's first pixel is palette entry 0, [n, n, n, 255].
        assert_eq!(
            &rgba(texture)[..4],
            &[index as u8, index as u8, index as u8, 255]
        );
    }
}

/// The match is on the last path component, case-insensitively - the same
/// rule `ship-skin.md` traces `Skin_ApplyToModel` using, and the one real
/// models exercise: the table spells `\TEXTURE1.TGA` and shipped hulls spell
/// `texture1.tga`.
#[test]
fn the_match_is_case_insensitive_on_the_slot_name() {
    let mut hull = Model::none("test hull");
    hull.textures = vec![texture("TEXTURE1.TGA")];
    let applied = apply(&mut hull, &skin());
    assert_eq!(applied, 1);
}

/// The front-end preview model is the real case this covers: it names only
/// `texture1`..`texture3`, so block 4 is never uploaded anywhere and that is
/// not an error - it is what the disc's own data does.
#[test]
fn an_unmatched_slot_and_a_model_with_no_fourth_slot_are_both_left_alone() {
    let mut hull = Model::none("preview hull");
    hull.textures = vec![
        texture("texture1.tga"),
        texture("texture2.tga"),
        texture("texture3.tga"),
        None,
        texture("normalmap.tga"),
    ];
    let applied = apply(&mut hull, &skin());
    assert_eq!(applied, 3);
    assert!(hull.textures[3].is_none());
    assert_eq!(hull.textures[4].as_ref().unwrap().label, "normalmap.tga");
    assert_eq!(
        rgba(hull.textures[4].as_ref().unwrap()),
        [9, 9, 9, 255].repeat(4).as_slice()
    );
}

/// A hull with no `\TEXTUREn.TGA`-named slots at all - a skin handed to the
/// wrong model - replaces nothing rather than guessing at a slot.
#[test]
fn a_hull_with_no_matching_slots_is_left_untouched() {
    let mut hull = Model::none("unrelated hull");
    hull.textures = vec![texture("something_else.tga")];
    let applied = apply(&mut hull, &skin());
    assert_eq!(applied, 0);
    assert_eq!(
        hull.textures[0].as_ref().unwrap().label,
        "something_else.tga"
    );
}
