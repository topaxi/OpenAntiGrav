//! `lambert_spec_mult` multiplies its picture by parameter `0x6d0178af`.
//!
//! **`#[ignore]`d and never run in CI**; needs the decrypted PS3 image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_picture_tint_ground_truth::/)'
//! ```
//!
//! Tech de Ra's hexagon panels behind the stand glass are authored
//! `(0.078, 0.1296, 0.26)` times `whitehexagon_colour_spec`; ignoring the tint drew
//! them `(124, 133, 160)` against the original's `(55, 62, 87)`. Dropping
//! `picture_tint` from `skin::skin` fails this.

use crate::archive_cache;

use oag_mesh::mesh;

const TINT: u32 = 0x6d01_78af;

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_tinted_lambert_spec_mult_binds_the_product_and_a_white_one_its_own_picture() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let path = "/data/environments/tech_de_ra/track.vex";
    let data = archive_cache::read(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("a sibling .rcsmodel");
    let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
    let (model, report) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archive_cache::read(&spec, name)
    })
    .expect("Tech de Ra builds");

    let (mut tinted, mut plain) = (0, 0);
    for (slot, material) in rcs.materials.iter().enumerate() {
        if !material.name.ends_with("/lambert_spec_mult.rcsmaterial") {
            continue;
        }
        let Some(value) = material.parameters.iter().find(|p| p.hash == TINT) else {
            continue;
        };
        let Some(texture) = model.textures[slot].as_ref() else {
            continue;
        };
        let is_baked = texture.label.starts_with("picture tint");
        if value.value[..3] == [1.0; 3] {
            assert!(
                !is_baked,
                "slot {slot} is (1, 1, 1) and binds {}",
                texture.label
            );
            plain += 1;
        } else {
            assert!(
                is_baked,
                "slot {slot} is tinted {:?} and binds {}",
                value.value, texture.label
            );
            tinted += 1;
        }
    }
    assert!(tinted > 0 && plain > 0, "tinted {tinted}, plain {plain}");
    assert!(report.picture_tinted >= tinted, "{}", report.describe());
}
