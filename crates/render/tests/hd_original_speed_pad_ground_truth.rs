//! The four original HD circuits' speed pads glow off their own `_ne` mask.
//!
//! Their `Speedup Pad` nodes name a hash no chunk carries, so the pads draw
//! through `build_scene`'s unreferenced-chunk pass; the pad materials there
//! must bind `slots::PAD_NE` with the material's authored colour, and no
//! other circuit's scene may bind it at all (rails and start lines sample the
//! same `_ne` hash through a program the pad path does not read).
//!
//! `#[ignore]`d: needs the PS3 disc image.
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all -E 'binary(hd_original_speed_pad_ground_truth)'`

use std::path::Path;

use oag_render::mesh::{self, slots};

const IMAGE: &str = "data/images/hdfury-ps3-eu-dec.iso";
const CYAN: [f32; 3] = [0.0, 0.768_628, 0.992_157];

/// `(archive, track, the scene carries speed-pad glow)`.
const CIRCUITS: &[(&str, &str, bool)] = &[
    ("DATA00", "talons_junction", true),
    ("DATA00", "amphiseum", true),
    ("DATA00", "modesto_heights", true),
    ("DATA00", "tech_de_ra", true),
    ("DATA02", "12_sol_2", false),
    ("DATA02", "15_anulpha_pass", false),
    ("DATA02", "01_vineta_k", false),
];

fn scene(archive: &str, track: &str) -> Option<mesh::Model> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(IMAGE);
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return None;
    }
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", path.display());
    let name = format!("/data/environments/{track}/track.vex");
    let data = mesh::read_blob(&spec, &name).expect("reading the track");
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data).expect("a .rcsmodel beside it");
    let mut read = |p: &str| mesh::read_blob(&spec, p).ok();
    Some(
        mesh::rcs::build_scene(&name, &data, &geometry, &mut read)
            .expect("the scene builds")
            .0,
    )
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_scene_glows_only_where_a_speed_pad_chunk_is_unreferenced() {
    for (archive, track, glows) in CIRCUITS {
        let Some(model) = scene(archive, track) else {
            return;
        };
        let lit: Vec<_> = model
            .vertices
            .iter()
            .filter(|v| v.slots & slots::PAD_NE != 0)
            .collect();
        if !glows {
            assert!(lit.is_empty(), "{track}: {} scene vertices glow", lit.len());
            continue;
        }
        assert!(!lit.is_empty(), "{track}: its speed pads do not glow");
        for v in lit {
            let index = slots::material_index(v.slots) as usize;
            assert_eq!(
                model.emissive[index - 1].tint,
                CYAN,
                "{track}: speed pad glow colour"
            );
        }
        assert!(
            model.pad_masks.iter().any(Option::is_some),
            "{track}: no _ne mask bound in the scene"
        );
    }
}
