//! Every PSP `.vex` texture reaches the renderer with the mip levels the disc
//! authors, not levels this project derived.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(psp_texture_levels_ground_truth)'
//! ```
//!
//! The rule that picks among the levels is `psp_slope_lod.rs`'s; this pins the
//! other half - that there are authored levels to pick from. A loader that went
//! back to the base level and a box filter would leave every texture
//! `Texels::Rgba8`, and the count below would be zero.

use oag_mesh::mesh::{self, Texels};

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn talons_junction_textures_carry_the_levels_the_disc_authors() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("the disc opens");
    let name = r"Data\Environments\16_Track\track.vex";
    let blob = archives.read_name(name).expect("the track is on the disc");
    let model = mesh::build_with_textures(name, &blob, None).expect("the track builds");
    let authored = oag_vex::vex::textures(&blob).expect("the texture block reads");

    let (mut chained, mut deeper, mut unlike_box_filter) = (0, 0, 0);
    for (slot, embedded) in model.textures.iter().zip(&authored) {
        let (Some(texture), Some(embedded)) = (slot, embedded) else {
            continue;
        };
        let Texels::Chain(levels) = &texture.texels else {
            panic!("{}: not the disc's own chain", texture.label);
        };
        assert_eq!(
            levels.len(),
            usize::from(embedded.mip_count.max(1)),
            "{}: one level per declared mip",
            texture.label
        );
        for (level, texels) in levels.iter().enumerate() {
            let (w, h) = (
                (texture.width >> level).max(1),
                (texture.height >> level).max(1),
            );
            assert_eq!(
                texels.len() as u32,
                w * h * 4,
                "{} level {level}: {w}x{h} RGBA8",
                texture.label
            );
        }
        chained += 1;
        if levels.len() > 1 {
            deeper += 1;
            // Authored, not derived: a texture whose level 1 is not the box
            // filter of its base is one only the disc could have supplied.
            let (w, h) = (texture.width as usize, texture.height as usize);
            let box_level: Vec<u8> = (0..(h / 2).max(1))
                .flat_map(|y| (0..(w / 2).max(1)).map(move |x| (x, y)))
                .flat_map(|(x, y)| {
                    (0..4).map(move |c| {
                        let at = |dx: usize, dy: usize| {
                            let (sx, sy) = ((2 * x + dx).min(w - 1), (2 * y + dy).min(h - 1));
                            u32::from(levels[0][(sy * w + sx) * 4 + c])
                        };
                        ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1)) / 4) as u8
                    })
                })
                .collect();
            if box_level != levels[1] {
                unlike_box_filter += 1;
            }
        }
    }
    assert!(chained > 100, "{chained} textures checked");
    assert!(deeper > 100, "{deeper} of them declare more than one level");
    assert!(
        unlike_box_filter > deeper / 2,
        "{unlike_box_filter} of {deeper} authored level 1s differ from a box filter of the base"
    );
}
