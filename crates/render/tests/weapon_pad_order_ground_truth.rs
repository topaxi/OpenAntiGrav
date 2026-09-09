//! `Model::node_vertex_ranges`' `i`-th entry is `oag_vex::pads::volumes`'
//! `i`-th trigger's own geometry - a positional correspondence the render
//! side leans on to recolour a `Weapon Pad` by whether it currently hands
//! out a pickup (`Drawable::tint_weapon_pads`, `oag_game::race::drawable`).
//!
//! `#[ignore]`d because it needs a disc image, not a GPU - this only reads
//! geometry and trigger volumes, no wgpu involved. Run it by hand:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(weapon_pad_order_ground_truth)'
//! ```

use std::path::PathBuf;

use oag_assets::Archive;
use oag_render::mesh;
use oag_vex::vex;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// `01_Track` and `16_Track`: the two circuits every other pad-related
/// ground truth in this project already measures against.
const TRACKS: [&str; 2] = [
    r"Data\Environments\01_Track\track.vex",
    r"Data\Environments\16_Track\track.vex",
];

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_weapon_pad_trigger_sits_inside_its_own_indexed_vertex_range() {
    let Some(image) = image() else {
        return;
    };
    let mut archive = Archive::open(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()))
        .expect("opening the archive");

    let mut checked = 0;
    for name in TRACKS {
        let blob = archive
            .read_name(name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let nodes = vex::nodes(&blob).expect("walking the node tree");
        let classes = vex::classes_of(&blob).expect("class table");
        let class_id = classes.weapon_pad.expect("weapon_pad id recovered for v6");

        let volumes = oag_vex::pads::volumes(&blob, &nodes, class_id);
        let model =
            mesh::build_weapon_pads(name, &blob, None).unwrap_or_else(|e| panic!("{name}: {e}"));

        assert_eq!(
            volumes.len(),
            model.node_vertex_ranges.len(),
            "{name}: {} trigger volume(s) against {} vertex range(s) - the two \
             node walks disagreed on which nodes are Weapon Pads",
            volumes.len(),
            model.node_vertex_ranges.len()
        );
        assert!(!volumes.is_empty(), "{name}: no weapon pads to check");

        for (i, (volume, range)) in volumes.iter().zip(&model.node_vertex_ranges).enumerate() {
            let verts = &model.vertices[range.start as usize..range.end as usize];
            assert!(!verts.is_empty(), "{name}: pad {i}'s vertex range is empty");
            let mut lo = [f32::MAX; 3];
            let mut hi = [f32::MIN; 3];
            for v in verts {
                for k in 0..3 {
                    lo[k] = lo[k].min(v.position[k]);
                    hi[k] = hi[k].max(v.position[k]);
                }
            }
            // The XZ plane only: the trigger's centre sits at the average of
            // a box the bind expands asymmetrically in Y (`-2.0`/`+8.0`,
            // `docs/formats/pads.md`), which shifts its Y off the flat
            // mesh's own Y range by design and is not part of this check.
            let centre = volume.centre();
            for k in [0usize, 2usize] {
                assert!(
                    lo[k] - 1.0 <= centre[k] && centre[k] <= hi[k] + 1.0,
                    "{name}: pad {i} trigger centre {centre:?} axis {k} falls outside \
                     its own indexed vertex range's bounds [{lo:?}..{hi:?}] - \
                     node_vertex_ranges and pads::volumes have drifted out of \
                     positional alignment"
                );
            }
            checked += 1;
        }
    }
    println!(
        "{checked} weapon pad(s) checked across {} track(s)",
        TRACKS.len()
    );
}
