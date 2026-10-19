//! Which of 2048's material families read a sampler for lighting alone, over
//! every `.rcsmaterial` in `data.psarc`.
//!
//! **`#[ignore]`d and never run in CI.** It needs the Vita package; run it
//! with `just test-data`. See `oag_rcs::gxp::sampler_use` for the rule and
//! `docs/formats/gxp.md`, "Which sampler carries the colour".
//!
//! Two claims are pinned over the one walk:
//!
//! - [`COLOURLESS`], the families whose ambient-only permutation declares no
//!   material sampler at all while a lit one does: their colour is a constant
//!   uniform and the texture is a lighting lookup. Drop the reader and the
//!   set is empty; widen it (a zone permutation counted as ambient-only) and a
//!   family joins that no one has read.
//! - [`LIGHTING_ONLY_NAMES`], every sampler name the rule calls lighting-only
//!   over the whole package. **The rule never looks at a name**, and the names
//!   it finds are the author's own words for a lookup (`NormalMap`,
//!   `Specular_Texture`, ...): the independent check that it separates what it
//!   claims to. The exceptions are `DiffuseTexture`, `DiffuseTexture1` (the seawater
//!   shaders, whose file is `dc_waternormalmap`: the programs outvote the
//!   name), the numbered `Texture1/2/4` and the refraction capture.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use oag_rcs::gxp::sampler_use::sampler_use;

/// Families, file stem lower-cased, with the colour uniform the ambient-only
/// permutation names.
const COLOURLESS: [(&str, Option<&str>); 8] = [
    ("constantdiffuse_specular_normal", Some("Constant1")),
    ("constantdiffuse_specular_normalnew", Some("Constant1")),
    ("dc_seawater", Some("ColourConstant")),
    ("dc_seawater_cheap", Some("ColourConstant")),
    ("ds_booth_glass", Some("Constant1")),
    ("fc15_effects_vscroll_water", Some("ColourConstant")),
    ("fc17_effects_vscroll_water", Some("Watercolour")),
    ("reflectplane_dc_seawater", Some("ColourConstant")),
];

/// Every sampler name the rule calls lighting-only over the package.
const LIGHTING_ONLY_NAMES: [&str; 18] = [
    "DiffuseTexture",
    "DiffuseTexture1",
    "Norm",
    "Normal",
    "NormalAlphaMap",
    "NormalAndSpecular",
    "NormalMap",
    "NormalMapTexture",
    "NormalTexture",
    "Normal_Map",
    "Spec",
    "SpecularTexture",
    "Specular_Texture",
    "Texture1",
    "Texture2",
    "Texture4",
    "norm",
    "screenSpaceRefractionTex",
];

/// Families with both permutations, the denominator of the claim.
const COMPARABLE_FAMILIES: usize = 204;

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_samplers_read_for_lighting_alone_are_the_lookups() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return;
    }
    let mut archive =
        oag_assets::psarc::Archive::open(path.to_str().expect("utf-8")).expect("opens");
    let mut entries: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmaterial"))
        .cloned()
        .collect();
    entries.sort();
    let mut comparable: BTreeSet<String> = BTreeSet::new();
    let mut colourless: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut names: BTreeSet<String> = BTreeSet::new();
    for entry in entries {
        let blob = archive.read_path(&entry).expect("reads");
        let Some(uses) = sampler_use(&blob) else {
            continue;
        };
        let stem = entry
            .rsplit('/')
            .next()
            .unwrap_or(&entry)
            .trim_end_matches(".rcsmaterial")
            .to_ascii_lowercase();
        comparable.insert(stem.clone());
        names.extend(uses.lighting_only.iter().cloned());
        if uses.ambient_samplers.is_empty() && !uses.lighting_only.is_empty() {
            colourless.insert(stem, uses.colour_uniform);
        }
    }
    println!("{} comparable families", comparable.len());
    println!("lighting-only names: {names:?}");
    println!("colourless: {colourless:?}");
    assert_eq!(comparable.len(), COMPARABLE_FAMILIES);
    let want: BTreeMap<String, Option<String>> = COLOURLESS
        .iter()
        .map(|(stem, colour)| ((*stem).to_string(), colour.map(str::to_string)))
        .collect();
    assert_eq!(colourless, want);
    assert_eq!(
        names.iter().map(String::as_str).collect::<Vec<_>>(),
        LIGHTING_ONLY_NAMES
    );
}
