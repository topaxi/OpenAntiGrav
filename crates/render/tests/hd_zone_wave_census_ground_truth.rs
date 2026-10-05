//! Wipeout HD's Zone variants of the magstrip wave family carry no wave.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content. Run with
//! `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_zone_wave_census_ground_truth)'
//! ```
//!
//! `mesh.wgsl` switches `slots::MAG_WAVE` and `slots::MAG_LOOP` off when
//! `scene.zone.enabled` is set, on the strength of this census: across all four
//! PS3 archives, every fragment variant of a material that has the wave and
//! also declares a Zone input (`zoneColourTint`) declares no wave sampler
//! (`0x85c9fd48`), no emissive sampler (`0x1202d8df`) and no engine `time`
//! (`0x906b67ba`). A material that failed this would need its own gate.
//! `crates/render/examples/hd_zone_variants.rs` prints the per-variant table.

use oag_rcs::rcsmaterial::{self, Declared, RcsMaterial};

const WAVE: u32 = 0x85c9_fd48;
const EMISSIVE: u32 = 0x1202_d8df;
const TIME: u32 = 0x906b_67ba;

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn no_zone_variant_of_a_wave_material_declares_the_wave() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let tint = rcsmaterial::name_hash("zoneColourTint");
    let (mut materials, mut zone_variants) = (0usize, 0usize);
    for archive in ["DATA00", "DATA01", "DATA02", "DATA03"] {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmaterial"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(material) = RcsMaterial::parse(&blob) else {
                continue;
            };
            let declared: Vec<Declared> = material
                .variants
                .iter()
                .filter_map(|v| Declared::parse(&blob, v.fragment.offset))
                .collect();
            let has = |d: &Declared, h: u32| d.samplers.iter().any(|&(s, _)| s == h);
            if !declared
                .iter()
                .any(|d| has(d, WAVE) && has(d, EMISSIVE) && d.parameters.contains(&TIME))
            {
                continue;
            }
            materials += 1;
            for d in declared.iter().filter(|d| d.parameters.contains(&tint)) {
                zone_variants += 1;
                assert!(
                    !has(d, WAVE) && !has(d, EMISSIVE) && !d.parameters.contains(&TIME),
                    "{archive} {path}: a Zone variant declares the wave"
                );
            }
        }
    }
    assert!(
        materials >= 6 && zone_variants > 0,
        "the census found {materials} wave materials and {zone_variants} Zone variants"
    );
}
