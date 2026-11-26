//! What Wipeout HD's Zone variants compute: no light-bar colour, and no less work per pixel
//! than the lit programs they replace.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content. Run with
//! `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-mesh --run-ignored all \
//!     -E 'test(/^hd_zone_ne_census_ground_truth::/)'
//! ```
//!
//! A lit `_ne` program adds `_ne.a * colour`, the colour a material parameter
//! (`docs/rendering/pads.md`). `shade.wesl` drops that term on a chunk drawn
//! through the Zone surface, on the strength of this census: across all four
//! PS3 archives, no Zone variant (one declaring `zoneColourTint`) of a material
//! whose lit program adds the term gates the same parameter on `_ne.a`. Vineta
//! K's `diffuse_normal_specular_emmissive` (the start bays) gates
//! `zoneBaseInner` or nothing there. `docs/rendering/hd-zone-recolour.md`,
//! "The light-bar term in a Zone race".

use oag_rcs::rcsmaterial::{
    self, Declared, Features, LIT_RACE_PASS, RcsMaterial, fragment::Program,
};

const NE: u32 = 0xa2d5_55b9;

/// Per fragment variant: whether it is a Zone variant, and the parameter its
/// `_ne` alpha gates an accumulate by, if any.
fn read(blob: &[u8], material: &RcsMaterial, tint: u32) -> Vec<(bool, Option<u32>)> {
    material
        .variants
        .iter()
        .filter_map(|v| {
            let d = Declared::parse(blob, v.fragment.offset)?;
            let gated = d
                .samplers
                .iter()
                .find(|&&(s, _)| s == NE)
                .and_then(|&(_, u)| u8::try_from(u).ok())
                .and_then(|unit| {
                    Program::parse(blob, v.fragment.offset)
                        .and_then(|p| p.alpha_gated_parameter(unit))
                });
            Some((d.parameters.contains(&tint), gated))
        })
        .collect()
}

/// One program as `(instructions, texture fetches, samples the lightmap)`.
type Cost = (usize, usize, bool);

/// Each class's lit race program of a lightmapped chunk against its Zone one (`ZoneMode`, the
/// `Ambient` field), the lit one first.
fn zone_against_lit(blob: &[u8], material: &RcsMaterial) -> Vec<[Cost; 2]> {
    let lit = Features::from_pass_word(LIT_RACE_PASS | 0b100);
    let zone = Features::from_pass_word(LIT_RACE_PASS | 1 << 8);
    let read = |class: u32, features: Features| {
        let variant = material
            .variants
            .iter()
            .find(|v| v.class_hash == class && v.feature_hash == features.hash())?;
        let program = Program::parse(blob, variant.fragment.offset)?;
        let fetches = program
            .instructions
            .iter()
            .filter(|i| i.is_texture())
            .count();
        Some((
            program.instructions.len(),
            fetches,
            program.declared.samples_lightmap(),
        ))
    };
    let classes: std::collections::BTreeSet<u32> =
        material.variants.iter().map(|v| v.class_hash).collect();
    classes
        .into_iter()
        .filter_map(|class| Some([read(class, lit)?, read(class, zone)?]))
        .filter(|[(_, _, lightmapped), _]| *lightmapped)
        .collect()
}

/// What a Zone race costs the original per pixel (`docs/rendering/hd-zone-recolour.md`, "What a
/// Zone race costs"): on Chenghou Project's lightmapped surfaces no Zone program samples the
/// lightmap or fetches fewer textures than the lit program it replaces, and all but one are
/// longer.
fn check_zone_programs_are_no_cheaper(pairs: &[(String, [Cost; 2])]) {
    assert!(pairs.len() >= 20, "only {} pairs", pairs.len());
    let mut shorter = Vec::new();
    for (
        path,
        [
            (lit_len, lit_fetches, _),
            (zone_len, zone_fetches, zone_map),
        ],
    ) in pairs
    {
        assert!(!zone_map, "{path}: the Zone program samples the lightmap");
        assert!(
            zone_fetches >= lit_fetches,
            "{path}: {zone_fetches} fetches against {lit_fetches}"
        );
        if zone_len <= lit_len {
            shorter.push(path.as_str());
        }
    }
    println!(
        "{} lit/Zone pairs, shorter in Zone: {shorter:?}",
        pairs.len()
    );
    assert!(
        shorter
            .iter()
            .all(|p| p.ends_with("mageffect_modded.rcsmaterial")),
        "{shorter:?}"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn no_zone_variant_adds_the_light_bar_colour() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let tint = rcsmaterial::name_hash("zoneColourTint");
    let zone_base = rcsmaterial::name_hash("zoneBaseInner");
    let (mut drops, mut keeps) = (Vec::new(), Vec::new());
    let mut vineta_zone_gates = Vec::new();
    let mut chenghou = Vec::new();
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
            if path.starts_with("/data/environments/04_chenghou_project/materials/") {
                chenghou.extend(
                    zone_against_lit(&blob, &material)
                        .into_iter()
                        .map(|pair| (path.clone(), pair)),
                );
            }
            let variants = read(&blob, &material, tint);
            let Some(colour) = variants
                .iter()
                .find_map(|&(zone, gated)| if zone { None } else { gated })
            else {
                continue;
            };
            let zone: Vec<Option<u32>> = variants.iter().filter(|v| v.0).map(|v| v.1).collect();
            if zone.is_empty() {
                continue;
            }
            if path
                == "/data/environments/01_vineta_k/materials/diffuse_normal_specular_emmissive.rcsmaterial"
            {
                vineta_zone_gates = zone.clone();
            }
            if zone.contains(&Some(colour)) {
                keeps.push(path);
            } else {
                drops.push(path);
            }
        }
    }
    println!(
        "{} materials drop the light bar in Zone, {} keep it",
        drops.len(),
        keeps.len()
    );
    assert!(
        keeps.is_empty(),
        "a Zone variant adds the lit light-bar colour: {keeps:?}"
    );
    assert!(
        drops.len() >= 50,
        "the census found {} materials",
        drops.len()
    );
    assert!(
        drops
            .iter()
            .any(|p| p
                .ends_with("01_vineta_k/materials/diffuse_normal_specular_emmissive.rcsmaterial")),
        "Vineta K's start-bay material is in the census"
    );
    assert!(
        !vineta_zone_gates.is_empty()
            && vineta_zone_gates
                .iter()
                .all(|g| g.is_none() || *g == Some(zone_base)),
        "its Zone variants gate zoneBaseInner or nothing: {vineta_zone_gates:x?}"
    );
    check_zone_programs_are_no_cheaper(&chenghou);
}
