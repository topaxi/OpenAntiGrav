//! Wipeout 2048's and Omega's Fury Nitro hulls paint their own diffuse, not
//! the facing ramp or the one-channel mask their material lists before it.
//!
//! **`#[ignore]`d and never run in CI.** It needs the extracted 2048 package
//! (`data/extracted/vita/PCSF00007`) and, for Omega, the merged extraction
//! (`data/extracted/ps4/omega-eu-merged`).
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-mesh --run-ignored all \
//!     -E 'test(/^psp2_nitro_hull_ground_truth::/)'
//! ```
//!
//! `nitro_body_new` and `nitro_perspex_new` list `iridescence` (a view-angle
//! ramp) and `holographic_02_rich_glow` (a one-channel mask) ahead of the
//! ship's diffuse, and the generic reading took the first texture path it met,
//! so every `_n1` hull wore the ramp in green and purple. The unit-0 sampler
//! `0x1b2a3d71` is the diffuse (`docs/rendering/hd-ship-materials.md`, "The
//! nitro hull's picture was a ramp and a mask"). Dropping the two hashes from
//! `skin::NITRO_LOOKUPS`, or the family keying in `psp2::bind_textures`, fails
//! this.

use std::path::PathBuf;

use oag_assets::psarc::Archive;
use oag_mesh::mesh::rcs::psp2;

const DIFFUSE: u32 = 0x1b2a_3d71;
const RAMP: u32 = 0x2d5f_c6a6;
const MASK: u32 = 0x3a7e_591d;
const FAMILIES: [&str; 3] = ["nitro_body", "nitro_body_new", "nitro_perspex_new"];

fn open(root: &str, names: &[&str]) -> Option<Vec<Archive>> {
    let mut opened = Vec::new();
    for name in names {
        let path: PathBuf = oag_testdata::exact(&format!("{root}/{name}"))?;
        opened.push(Archive::open(&path.display().to_string()).expect("opens"));
    }
    Some(opened)
}

fn read(archives: &mut [Archive], path: &str) -> Option<Vec<u8>> {
    archives.iter_mut().find_map(|a| a.read_path(path).ok())
}

/// Every `_n1` hull of `archives`, built through the layers in order, and the
/// nitro slots checked. Returns how many hulls and nitro materials were checked.
fn check(archives: &mut [Archive]) -> (usize, usize) {
    let mut hulls: Vec<String> = archives
        .iter()
        .flat_map(|a| a.paths().iter().cloned())
        .filter(|p| {
            let p = p.to_ascii_lowercase();
            p.contains("/hdships/") && p.ends_with("_n1/ship.rcsmodel")
        })
        .collect();
    hulls.sort_by_key(|p| p.to_ascii_lowercase());
    hulls.dedup_by_key(|p| p.to_ascii_lowercase());
    let mut materials_checked = 0;
    for hull in &hulls {
        let blob = read(archives, hull).expect("the hull reads");
        let parsed = oag_rcs::rcsmodel::psp2::parse(&blob).expect("parses");
        let mut textures = |p: &str| read(archives, p);
        let (model, _) = psp2::build("hull", &blob, None, &mut textures).expect("builds");
        // `model.textures` holds one slot per material the hull's draws bind,
        // so the labels are the pictures the hull is painted with.
        let labels: Vec<String> = model
            .textures
            .iter()
            .flatten()
            .map(|t| t.label.to_ascii_lowercase())
            .collect();
        for material in &parsed.materials {
            let family = oag_rcs::rcsmodel::psp2::lighting_only::family(material);
            if !FAMILIES.contains(&family.as_str()) {
                continue;
            }
            let sampler = |hash: u32| {
                material
                    .samplers
                    .iter()
                    .find(|(h, _)| *h == hash)
                    .map(|(_, p)| p.to_ascii_lowercase())
            };
            for lookup in [RAMP, MASK] {
                if let Some(path) = sampler(lookup) {
                    assert!(
                        !labels.contains(&path),
                        "{hull} {family} paints a lookup, {path}: {labels:?}"
                    );
                }
            }
            let diffuse =
                sampler(DIFFUSE).expect("the nitro program's unit 0 sampler names a file");
            assert!(
                labels.contains(&diffuse),
                "{hull} {family} does not paint its diffuse {diffuse}: {labels:?}"
            );
            materials_checked += 1;
        }
    }
    (hulls.len(), materials_checked)
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn every_2048_nitro_hull_paints_its_diffuse() {
    // The order the executable mounts them, a later layer shadowing an earlier
    // one (`oag_2048::PATCH_CANDIDATES`): the patch's `data2`, `data1`, the base.
    let Some(mut archives) = open(
        "data/extracted/vita/PCSF00007",
        &[
            "patch-v104/PSP2/data2.psarc",
            "patch-v104/PSP2/data1.psarc",
            "base/PSP2/data.psarc",
        ],
    ) else {
        return;
    };
    let (hulls, materials) = check(&mut archives);
    println!("2048: {hulls} nitro hulls, {materials} nitro materials");
    assert_eq!(hulls, 12, "twelve teams ship an _n1 hull");
    assert!(
        materials >= 12,
        "at least one nitro material per hull, saw {materials}"
    );
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu-merged"]
fn every_omega_nitro_hull_paints_its_diffuse() {
    let Some(mut archives) = open(
        "data/extracted/ps4/omega-eu-merged",
        &[
            "data09.psarc",
            "data08.psarc",
            "data07.psarc",
            "data05.psarc",
            "data04.psarc",
            "data03.psarc",
            "data02.psarc",
            "data01.psarc",
            "data00.psarc",
        ],
    ) else {
        return;
    };
    let (hulls, materials) = check(&mut archives);
    println!("Omega: {hulls} nitro hulls, {materials} nitro materials");
    assert!(hulls >= 1, "no Omega _n1 hull found");
    assert!(
        materials >= hulls,
        "at least one nitro material per hull, saw {materials}"
    );
}
