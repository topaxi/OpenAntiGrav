//! Every HD weapon pad carries its own glow-table entry, holding its
//! material's authored colour, so a pad's light bars can be recoloured alone.
//!
//! **`#[ignore]`d and never run in CI**: it needs a decrypted PS3 disc image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_weapon_pad_glow_ground_truth)'
//! ```
//!
//! The original recolours each pad's `_ne`-gated constant from the pad's own
//! cycle (`WeaponPad_UpdateRefreshTimer`, `docs/rendering/pads.md`), measured
//! on `01_vineta_k` where two pads of one frame read different values. The
//! glow table deduplicates by value, so without this every pad of one material
//! would share an entry and a recolour could not tell two pads apart.

mod archive_cache;

use std::collections::HashSet;

use oag_mesh::mesh::{self, slots};

const ARCHIVE: &str = "PS3_GAME/USRDIR/DATA02.PSARC";
const TRACK: &str = "/data/environments/01_vineta_k/track.vex";

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn each_vineta_weapon_pad_has_an_entry_of_its_own_in_the_authored_colour() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:{ARCHIVE}", image.display());
    let vex_data = archive_cache::read(&spec, TRACK).expect("reading the .vex");
    let model_name = mesh::rcs::sibling_name(TRACK).expect("a .vex name to rewrite");
    let model_blob = archive_cache::read(&spec, &model_name).expect("reading the .rcsmodel");
    let (model, _) = mesh::rcs::build_weapon_pads(TRACK, &vex_data, &model_blob, &mut |path| {
        archive_cache::read(&spec, path)
    })
    .expect("build_weapon_pads decodes 01_vineta_k");

    assert_eq!(model.node_vertex_ranges.len(), 7, "seven Weapon Pad nodes");
    let mut entries = HashSet::new();
    for range in &model.node_vertex_ranges {
        let vertices = &model.vertices[range.start as usize..range.end as usize];
        let glow: Vec<_> = vertices
            .iter()
            .filter(|v| v.slots & slots::PAD_NE != 0)
            .map(|v| slots::material_index(v.slots))
            .collect();
        assert!(!glow.is_empty(), "a pad node with no light-bar vertices");
        assert!(glow.iter().all(|&i| i == glow[0] && i != 0), "{glow:?}");
        assert!(
            entries.insert(glow[0]),
            "two pads share glow entry {}",
            glow[0]
        );
        let layer = model.emissive[glow[0] as usize - 1];
        // The authored `W_Cycle` of this circuit's weapon pad material.
        assert_eq!(layer.tint, [0.0, 0.768_628, 0.992_157], "entry {}", glow[0]);
    }
}
