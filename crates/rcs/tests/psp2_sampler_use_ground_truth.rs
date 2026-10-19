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

fn root(rel: &str) -> Option<std::path::PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    None
}

/// What a family's instances look like across a set of archives: how many
/// match [`INHERITED_FROM_2048`]'s row, and how many do not.
fn tally(
    archives: &mut [oag_assets::psarc::Archive],
    is_model: impl Fn(&str) -> bool,
) -> BTreeMap<&'static str, (usize, usize)> {
    use oag_rcs::rcsmodel::psp2::{self, lighting_only};
    let mut out: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    for archive in archives {
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| is_model(p))
            .cloned()
            .collect();
        for path in paths {
            let blob = archive.read_path(&path).expect("reads");
            let model = psp2::parse(&blob).expect("parses");
            for m in &model.materials {
                let family = lighting_only::family(m);
                let Some(row) = lighting_only::INHERITED_FROM_2048
                    .iter()
                    .find(|r| r.family == family)
                else {
                    continue;
                };
                let slot = out.entry(row.family).or_default();
                if lighting_only::inherited(m).is_some() {
                    slot.0 += 1;
                } else {
                    slot.1 += 1;
                }
            }
        }
    }
    out
}

/// The inherited table is 2048's own answer, and Omega matches it where it
/// says so. Drop `lighting_only::inherited` from the build and Omega's
/// `omega_*_binds_no_more_lookups_than_known` ratchets fail; change a row and
/// this fails.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007 and data/extracted/ps4/omega-eu"]
fn omega_inherits_2048s_answer_only_where_its_record_is_2048s() {
    use oag_rcs::rcsmaterial::name_hash;
    use oag_rcs::rcsmodel::psp2::lighting_only;
    let (Some(vita), Some(omega)) = (
        root("data/extracted/vita/PCSF00007/base/PSP2/data.psarc"),
        root("data/extracted/ps4/omega-eu/uroot"),
    ) else {
        return;
    };
    let mut vita_archive =
        oag_assets::psarc::Archive::open(vita.to_str().expect("utf-8")).expect("opens");
    for row in &lighting_only::INHERITED_FROM_2048 {
        let entry = vita_archive
            .paths()
            .iter()
            .find(|p| {
                p.to_ascii_lowercase()
                    .ends_with(&format!("/{}.rcsmaterial", row.family))
            })
            .cloned()
            .unwrap_or_else(|| panic!("{} is in 2048's package", row.family));
        let uses = sampler_use(&vita_archive.read_path(&entry).expect("reads")).expect("both");
        let mut hashes: Vec<u32> = uses.lighting_only.iter().map(|n| name_hash(n)).collect();
        hashes.sort_unstable();
        assert_eq!(hashes, row.lighting_only, "{}", row.family);
        assert_eq!(
            uses.colour_uniform.as_deref().map(name_hash),
            Some(row.colour),
            "{}",
            row.family
        );
    }
    let on_vita = tally(std::slice::from_mut(&mut vita_archive), |p| {
        p.contains("/environments/")
            && (p.ends_with("/track.rcsmodel") || p.ends_with("/track_reversed.rcsmodel"))
    });
    println!("2048 instances (match, differ): {on_vita:?}");
    let mut omega_archives: Vec<_> = (0..5)
        .map(|i| {
            oag_assets::psarc::Archive::open_file(&omega.join(format!("data0{i}.psarc")))
                .expect("opens")
        })
        .collect();
    let on_omega = tally(&mut omega_archives, |p| {
        p.contains("/environments") && p.ends_with(".final.rcsmodel") && !p.contains(".final.pvs")
    });
    println!("Omega instances (match, differ): {on_omega:?}");
    for row in &lighting_only::INHERITED_FROM_2048 {
        assert_eq!(on_vita[row.family].1, 0, "2048's {} all match", row.family);
        assert!(on_omega[row.family].0 > 0, "Omega's {}", row.family);
    }
    assert_eq!(
        on_omega.values().map(|v| v.1).sum::<usize>(),
        0,
        "no Omega instance of a table family differs from it"
    );
}
