//! The mesh box `Mesh_SubmitNode` orders a list-B mesh by, checked against the
//! vertices it bounds (both lists: the box is the whole mesh's).
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! `Mesh_InitFromPayload` (`0x0890e998`) stores the payload's `+0x10`/`+0x20` box
//! divided by the last batch's position scale, and `Mesh_SubmitNode`
//! (`0x0890cc80`) carries its centre through the node's world matrix for the
//! back-to-front key of a list-B mesh. `vex::mesh_box_centre` returns the centre
//! as stored. The claim here is that it is in the units of the decoded batch
//! positions, so a loader that has already scaled its vertices can use it as is.

use oag_disc::DiscImage;
use oag_vex::vex;

use crate::wad_walk;

const PSP_DATA: &str = "PSP_GAME/USRDIR/Data.wad";

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// `(meshes with list-B batches, those whose box centre lies within 5 % of the
/// vertex bounds' diagonal of the vertex bounds' centre)`.
fn census(blobs: &[(usize, Vec<u8>)]) -> (usize, usize, usize) {
    let (mut meshes, mut agree, mut scaled_agree) = (0, 0, 0);
    for (_, bytes) in blobs {
        let Ok(nodes) = vex::nodes(bytes) else {
            continue;
        };
        for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
            let range = node.payload();
            if range.end > bytes.len() {
                continue;
            }
            let payload = &bytes[range];
            let Some(centre) = vex::mesh_box_centre(payload) else {
                continue;
            };
            let (Ok(mut batches), Ok(list_b)) =
                (vex::mesh_batches(payload, 0), vex::mesh_batches(payload, 1))
            else {
                continue;
            };
            if list_b.is_empty() {
                continue;
            }
            batches.extend(list_b);
            if batches.iter().any(|b| vex::is_vif_batch(b.vertex_type)) {
                continue;
            }
            let mut lo = [f32::MAX; 3];
            let mut hi = [f32::MIN; 3];
            for v in batches.iter().flat_map(|b| &b.vertices) {
                for k in 0..3 {
                    lo[k] = lo[k].min(v.position[k]);
                    hi[k] = hi[k].max(v.position[k]);
                }
            }
            if lo[0] > hi[0] {
                continue;
            }
            let mid = [
                (lo[0] + hi[0]) * 0.5,
                (lo[1] + hi[1]) * 0.5,
                (lo[2] + hi[2]) * 0.5,
            ];
            let diag =
                ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2))
                    .sqrt()
                    .max(1e-3);
            let err = |c: [f32; 3]| {
                ((c[0] - mid[0]).powi(2) + (c[1] - mid[1]).powi(2) + (c[2] - mid[2]).powi(2)).sqrt()
                    / diag
            };
            meshes += 1;
            if err(centre) < 0.05 {
                agree += 1;
            }
            let scale = batches.last().map_or(1.0, |b| b.scale);
            if err([centre[0] / scale, centre[1] / scale, centre[2] / scale]) < 0.05 {
                scaled_agree += 1;
            }
        }
    }
    (meshes, agree, scaled_agree)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_list_b_meshs_box_centre_is_in_the_units_of_its_decoded_positions() {
    let Some(path) = image() else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let blobs: Vec<_> = wad_walk::blobs(&mut disc, PSP_DATA, wad_walk::VEX_HEAD, |head| {
        vex::has_magic(head)
    })
    .into_iter()
    .filter(|(_, b)| vex::version(b) == Ok(6))
    .collect();
    let (meshes, agree, scaled) = census(&blobs);
    eprintln!(
        "list-B meshes {meshes}, box centre as stored agrees {agree}, divided by scale {scaled}"
    );
    assert!(meshes > 500, "the census reached {meshes} list-B meshes");
    assert_eq!(
        agree, meshes,
        "agree as stored; divided by the scale: {scaled} of {meshes}"
    );
}
