//! Which of Omega's ship materials take `VertexColour1` as a factor on the light, over every
//! `.rcsmaterial` in a `ships` directory in the base and patch archives.
//!
//! **`#[ignore]`d and never run in CI.** It needs `data/extracted/ps4/`; run with `just test-data`.
//!
//! The reading is [`oag_rcs::gcn::input_is_factor`]: the vertex program fetches the colour and
//! exports it unchanged, and the pixel program after it reads that parameter only as a multiplicand
//! (`docs/ghidra/functions/ps4-omega-eu/hull-tone.md`, "Recipe"), and one of them reads
//! `liveLighting0diffuse`, so the colour is a factor on a light. Drop the reader and `FACTOR`
//! is empty; widen it (an additive read counted) and an effect or shield joins that no one has
//! read. `EFFECTS` are the materials that fetch the colour and do not multiply it, among them the
//! shields, trails and flames that read it as an alpha or a tint.

use std::collections::BTreeSet;
use std::path::Path;

use oag_rcs::gcn::input_is_factor;

const ARCHIVES: [&str; 6] = [
    "ps4/omega-eu-patch/uroot/data09.psarc",
    "ps4/omega-eu-patch/uroot/data08.psarc",
    "ps4/omega-eu-patch/uroot/data05.psarc",
    "ps4/omega-eu/uroot/data00.psarc",
    "ps4/omega-eu/uroot/data01.psarc",
    "ps4/omega-eu/uroot/data02.psarc",
];

/// Ship materials whose every colour-fetching program multiplies it, by file stem with the
/// `_1`/`_2` variant suffix dropped.
const FACTOR: [&str; 24] = [
    "2048_nitro_ships",
    "2048_ship_carbon_fibre_shiny",
    "2048_ship_carbon_fibre_shiny_lod",
    "2048_ship_paint_matt",
    "2048_ship_paint_shiny",
    "2048_ship_paint_shiny_final",
    "2048_ship_paint_shiny_lod",
    "2048_ship_plastic",
    "2048_ship_tech",
    "2048_ship_zonevertexcolour",
    "carbonfibre",
    "debris",
    "detonator_basicalpha_vcol",
    "detonator_diffuse_vcol",
    "detonator_diffuse_with_specular_from_alpha_n_vcol",
    "detonator_ship",
    "detonator_zoneship_basic",
    "diffuse_vcol",
    "diffuse_with_specular_from_alpha_n_vcol",
    "diffuse_with_specular_from_alpha_vcol",
    "fighter_version",
    "nitro_perspex_new",
    "ship_lod_vcol",
    "ship_main_livery",
];

/// Ship materials that fetch the colour and do not multiply it in every program.
const EFFECTS: [&str; 32] = [
    "2048_engine_additive",
    "2048_engine_additive_unique",
    "2048_engineflare_old",
    "2048_magnetic_shield",
    "2048_ship_zonefacing",
    "2048_ship_zonevertexcolour_emissive",
    "2048_ship_zonevertexcolour_emissive_trans",
    "2048_shipshield_old",
    "colourtest",
    "detonator_flame_test",
    "detonator_hd_absorbinternal",
    "detonator_hexagonalshield",
    "detonator_leacheffectmat",
    "emissive_bloom_test",
    "emissive_bloom_test_2",
    "flame_test",
    "hd_absorbinternal",
    "hd_shipshield_old",
    "hexagonalshield",
    "hexagonalshield_alpha",
    "hexagonalshield_rich",
    "holographic_projector",
    "holographic_projector2",
    "jet_test",
    "leacheffectmat",
    "nitro_body_new",
    "shield",
    "trail_test",
    "trail_test_02",
    "water_test_2_castnoshipshadows",
    "zone_hd_absorbinternal",
    "zonebattle_shield",
];

fn stem(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    let file = file.trim_suffix(".rcsmaterial");
    for suffix in ["_1", "_2"] {
        if let Some(base) = file.strip_suffix(suffix) {
            return base.to_string();
        }
    }
    file.to_string()
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and omega-eu-patch"]
fn the_hull_materials_multiply_the_vertex_colour_and_the_effects_do_not() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    let mut archives: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    if archives.len() != ARCHIVES.len() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but Omega is not extracted"
        );
        println!("skipping: Omega not extracted");
        return;
    }
    let mut seen = BTreeSet::new();
    let (mut factor, mut effects) = (BTreeSet::new(), BTreeSet::new());
    for index in 0..archives.len() {
        let paths: Vec<String> = archives[index]
            .paths()
            .iter()
            .filter(|p| {
                let p = p.to_ascii_lowercase();
                p.ends_with(".rcsmaterial") && p.contains("ships")
            })
            .cloned()
            .collect();
        for path in paths {
            if !seen.insert(path.to_ascii_lowercase()) {
                continue;
            }
            let blob = archives[index].read_path(&path).expect("reads");
            let found = input_is_factor(&blob, "VertexColour1");
            if found.fetching == 0 {
                continue;
            }
            if found.is_factor() {
                factor.insert(stem(&path));
            } else {
                effects.insert(stem(&path));
            }
        }
    }
    let mixed: Vec<_> = factor.intersection(&effects).collect();
    assert!(
        mixed.is_empty(),
        "a stem is a factor in one file and not in another: {mixed:?}"
    );
    assert_eq!(
        factor.iter().map(String::as_str).collect::<Vec<_>>(),
        FACTOR
    );
    assert_eq!(
        effects.iter().map(String::as_str).collect::<Vec<_>>(),
        EFFECTS
    );
}
