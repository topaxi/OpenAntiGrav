//! A texture pack's picture of a HUD image is drawn at its own size and the layout's rects,
//! which are in the disc's pixels, still land on the same part of it.

use std::collections::HashMap;
use std::sync::Arc;

use oag_texture::png::encode_rgba;
use oag_texture::replacement::Stored;
use oag_texture::replacer::{self, Files, Replacements};
use oag_ui::frontend::Draw;

use crate::Sprite;
use crate::sprite::Sheet;

struct Memory(HashMap<String, Vec<u8>>);

impl Files for Memory {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.0.get(name).cloned()
    }
}

/// A 16x16 8bpp `.mip` with a palette of 256 greys, every texel index 5.
fn mip() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.push(8);
    out.resize(oag_texture::texture::HEADER_LEN, 0);
    for index in 0..=255u8 {
        out.extend_from_slice(&[index, index, index, 255]);
    }
    out.extend(std::iter::repeat_n(5u8, 16 * 16));
    out
}

/// A pack whose one picture, `64x64` (4x the `.mip`), is transparent in its top left quarter
/// and opaque elsewhere, so which texel a lookup lands on shows in the alpha.
fn pack_for(blob: &[u8]) -> Arc<Replacements> {
    let key = Stored::from_mip(blob)
        .unwrap()
        .replacement()
        .unwrap()
        .key
        .unwrap();
    let rgba: Vec<u8> = (0..64u32 * 64)
        .flat_map(|at| {
            let (x, y) = (at % 64, at / 64);
            [200, 100, 50, if x < 32 && y < 32 { 0 } else { 255 }]
        })
        .collect();
    let ini = format!(
        "[options]\nhash = xxh64\nignoreAddress = true\n[games]\nUCES00465 = textures.ini\n[hashes]\n{key} = hud.png\n"
    );
    let files = Memory(HashMap::from([(
        "hud.png".to_string(),
        encode_rgba(64, 64, &rgba),
    )]));
    Arc::new(Replacements::new("test", &ini, Box::new(files)).unwrap())
}

fn sprite(uv: [f32; 4]) -> Sprite {
    Sprite {
        name: "s".to_string(),
        rect: [10.0, 20.0, 8.0, 8.0],
        uv,
        color: [1.0; 4],
        src: "hud".to_string(),
        rotation: 0.0,
        gradient: None,
        centred: false,
    }
}

fn uv_of(draw: Draw) -> [f32; 4] {
    match draw {
        Draw::Sprite { uv, .. } => uv,
        other => panic!("not a plain sprite: {other:?}"),
    }
}

#[test]
fn a_pack_picture_is_drawn_larger_and_the_layouts_rects_follow_it() {
    let blobs = [("hud".to_string(), mip())];
    replacer::install(Some(pack_for(&blobs[0].1)));
    let mut report = Vec::new();
    let sheet = Sheet::build_replaced(&blobs, Vec::new(), &mut report);
    replacer::install(None);

    assert_eq!(
        (sheet.width, sheet.height),
        (64, 65),
        "the pack's picture, and a gutter"
    );
    let placed = sheet.get("hud").unwrap();
    assert_eq!(
        (placed.width, placed.height),
        (16, 16),
        "the disc's own measure"
    );
    assert_eq!(sheet.scale("hud"), [4.0, 4.0]);
    assert!(
        report
            .iter()
            .any(|line| line.contains("from the texture pack")),
        "the report names it: {report:?}"
    );

    // A patch authored at (4, 2) 8x8 of the 16x16 image is (16, 8) 32x32 of the 64x64.
    assert_eq!(
        uv_of(crate::sprite_draw(&sprite([4.0, 2.0, 8.0, 8.0]), &sheet).unwrap()),
        [16.0, 8.0, 32.0, 32.0]
    );
    // A sprite that authors no size samples the whole picture.
    assert_eq!(
        uv_of(crate::sprite_draw(&sprite([0.0; 4]), &sheet).unwrap()),
        [0.0, 0.0, 64.0, 64.0]
    );
    // The fraction addressing alpha_at takes lands on the picture's own texels.
    assert_eq!(
        sheet.alpha_at(placed, 0.25, 0.25),
        Some(0.0),
        "inside the clear quarter"
    );
    assert_eq!(sheet.alpha_at(placed, 0.6, 0.6), Some(1.0));
    // opaque_extent is in the disc's pixels: the opaque part is an L over the clear quarter,
    // so its box is the whole 16x16 image.
    assert_eq!(sheet.opaque_extent("hud"), Some([0.0, 0.0, 16.0, 16.0]));
}

#[test]
fn with_no_pack_on_the_sheet_and_its_draws_are_the_disc_own() {
    let blobs = [("hud".to_string(), mip())];
    let mut report = Vec::new();
    let plain = Sheet::build_with(&blobs, Vec::new(), &mut report);
    let replaced = Sheet::build_replaced(&blobs, Vec::new(), &mut report);

    assert_eq!(
        (replaced.width, replaced.height),
        (plain.width, plain.height)
    );
    assert_eq!(replaced.rgba, plain.rgba);
    assert_eq!(replaced.scale("hud"), [1.0, 1.0]);
    for uv in [[4.0, 2.0, 8.0, 8.0], [0.0; 4], [3.0, 1.0, -5.0, 2.0]] {
        let placed = plain.get("hud").unwrap();
        let expected = if uv[2] == 0.0 && uv[3] == 0.0 {
            [placed.x as f32 + uv[0], placed.y as f32 + uv[1], 16.0, 16.0]
        } else {
            [
                placed.x as f32 + uv[0],
                placed.y as f32 + uv[1],
                uv[2],
                uv[3],
            ]
        };
        assert_eq!(
            uv_of(crate::sprite_draw(&sprite(uv), &replaced).unwrap()),
            expected
        );
    }
}

#[test]
fn the_plain_build_never_asks_the_pack() {
    let blobs = [("hud".to_string(), mip())];
    replacer::install(Some(pack_for(&blobs[0].1)));
    let sheet = Sheet::build_with(&blobs, Vec::new(), &mut Vec::new());
    replacer::install(None);
    assert_eq!((sheet.width, sheet.get("hud").unwrap().width), (16, 16));
    assert_eq!(sheet.scale("hud"), [1.0, 1.0]);
}
