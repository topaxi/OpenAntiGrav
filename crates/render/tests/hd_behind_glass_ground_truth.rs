//! Render flag `0x10` (`rcsmodel::RENDER_BEHIND_GLASS`): the chunks the
//! original draws only into its behind-the-glass target, never in the main
//! view.
//!
//! **`#[ignore]`d and never run in CI**; needs the decrypted PS3 image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_behind_glass_ground_truth)'
//! ```
//!
//! The measurement (`docs/ghidra/functions/ps3-hdfury-eu/visibility.md`,
//! 2026-10-07): on three RPCS3 frames of Vineta K, with every draw tied to its
//! chunk by vertex offset, 93 of 93 draws into the 640x360 target the tunnel
//! glass reads are chunks with this bit, and none of the 357 main-view draws
//! is. The maintainer's report was meshes on the tunnel ceiling the original
//! does not show; those are these chunks.

use oag_mesh::mesh;
use oag_rcs::rcsmodel;

const IMAGE: &str = "hdfury-ps3-eu-dec.iso";
const VINETA_K: &str = "/data/environments/01_vineta_k/track.vex";

fn archive(image: &std::path::Path, n: u32) -> String {
    format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display())
}

fn read(image: &std::path::Path, path: &str) -> Option<(String, Vec<u8>)> {
    (0..4).find_map(|n| {
        let spec = archive(image, n);
        mesh::read_blob(&spec, path).ok().map(|d| (spec, d))
    })
}

/// The chunks the capture's main view draws at the tunnel pose (a sample of
/// its 51), so the build cannot pass by drawing nothing.
const MAIN_VIEW: [u32; 7] = [2, 26, 1040, 1050, 1597, 1616, 1621];

/// The ones on the maintainer's ceiling: `and_metal_struts` (1574, the dark
/// band across it, and 55), `and_girder3` (1320), `and_metalshine2` (1315),
/// `and_metalstruts_pt2` (16-18).
const CEILING: [u32; 7] = [1574, 1320, 1315, 55, 16, 17, 18];

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn vineta_k_leaves_its_behind_the_glass_chunks_out_of_the_main_view() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let (spec, vex) = read(&image, VINETA_K).expect("the circuit's .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, VINETA_K, &vex).expect("its .rcsmodel");
    let model = rcsmodel::Model::parse(&geometry).expect("the circuit parses");
    let behind: Vec<u32> = (0..model.meshes.len() as u32)
        .filter(|&i| model.meshes[i as usize].is_behind_glass())
        .collect();
    for chunk in CEILING {
        assert!(behind.contains(&chunk), "chunk {chunk} carries the bit");
    }
    for chunk in MAIN_VIEW {
        assert!(!behind.contains(&chunk), "chunk {chunk} does not");
    }

    let (built, report) = mesh::rcs::build_scene(VINETA_K, &vex, &geometry, &mut |name| {
        read(&image, name).map(|(_, d)| d)
    })
    .expect("the circuit builds");
    let drawn: std::collections::BTreeSet<u32> = built
        .draws
        .iter()
        .chain(&built.alpha_tested_draws)
        .chain(&built.transparent_draws)
        .filter_map(|d| d.chunk)
        .collect();
    let leaked: Vec<_> = behind.iter().filter(|c| drawn.contains(c)).collect();
    assert!(
        leaked.is_empty(),
        "behind-the-glass chunks drawn: {leaked:?}"
    );
    for chunk in MAIN_VIEW {
        assert!(drawn.contains(&chunk), "main-view chunk {chunk} is drawn");
    }
    assert!(
        report.behind_glass > 0,
        "the load report says what it left out"
    );
}

/// The bit is Vineta K's alone, in both directions, so leaving it out changes
/// no other circuit: 420 chunks over the disc's every `.rcsmodel`.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn only_vineta_k_carries_the_behind_the_glass_bit() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let mut per_file = std::collections::BTreeMap::new();
    for n in 0..4 {
        let spec = archive(&image, n);
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let names: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for name in names {
            let Ok(bytes) = open.read_path(&name) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&bytes) else {
                continue;
            };
            let count = model.meshes.iter().filter(|m| m.is_behind_glass()).count();
            if count > 0 {
                per_file.insert(name, count);
            }
        }
    }
    println!("{per_file:#?}");
    assert_eq!(per_file.values().sum::<usize>(), 420);
    assert!(
        per_file.keys().all(|name| name.contains("01_vineta_k")),
        "a file outside Vineta K: {per_file:?}"
    );
}
