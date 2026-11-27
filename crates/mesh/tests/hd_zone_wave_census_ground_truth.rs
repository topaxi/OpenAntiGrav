//! Wipeout HD's Zone variants of the magstrip wave family carry no wave.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content. Run with
//! `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-mesh --run-ignored all \
//!     -E 'test(/^hd_zone_wave_census_ground_truth::/)'
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

/// The glass floors' Zone variants run no facing-ramp sheen: across all four PS3 archives, every
/// material with a variant that samples the ramp (`0x94b2b285`) and a Zone variant
/// (`zoneColourTint`) has Zone variants that sample none, so a Zone race draws it as the generic
/// Zone surface (`shade.wesl`'s `ramp_sheen` is off with `zone_program`). The census must cover
/// Talon's Junction's and Tech de Ra's `etched_glass_tech`, Modesto Heights' `etched_glass` and
/// the Amphiseum's `etched_glass_amph`.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn no_zone_variant_of_a_facing_ramp_material_samples_the_ramp() {
    const FACING_RAMP: u32 = 0x94b2_b285;
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let tint = rcsmaterial::name_hash("zoneColourTint");
    let mut covered: Vec<String> = Vec::new();
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
            let ramp = |d: &Declared| d.samplers.iter().any(|&(s, _)| s == FACING_RAMP);
            let zone = |d: &&Declared| d.parameters.contains(&tint);
            if !declared.iter().any(|d| ramp(d) && !zone(&d)) || !declared.iter().any(|d| zone(&d))
            {
                continue;
            }
            covered.push(path.clone());
            assert!(
                declared.iter().filter(zone).all(|d| !ramp(d)),
                "{archive} {path}: a Zone variant samples the facing ramp"
            );
        }
    }
    for stem in ["etched_glass_tech", "etched_glass_amph", "etched_glass"] {
        let file = format!("/{stem}.rcsmaterial");
        assert!(
            covered.iter().any(|p| p.ends_with(&file)),
            "the census missed {stem}: {covered:#?}"
        );
    }
}

/// The glass floors' Zone variants: every one that is not a `ShadowToAlpha` redraw (those declare
/// `shadowMapTex`, `0x730df9ee`, and write `1 - shadow` as alpha) takes its output alpha from the
/// `Texture1` grid's own unit, so a Zone race blends the glass by its grid as the sheen does off
/// Zone (`shade.wesl`). Every Zone variant also samples `paraboloidReflectionTex` (`0x9edd3243`)
/// under the weight `0x5e5b1937` and the engine tint `iblScalePower` (`0x7480de6d`): the
/// reflection term, whose Zone-frame scenery tint is unread and which is not drawn
/// (`docs/formats/rcsmaterial.md`, "The Zone glass program").
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_glass_floors_zone_variants_take_the_grid_as_their_alpha() {
    use oag_rcs::rcsmaterial::fragment::{Program, Texel};
    const TEXTURE1: u32 = 0x3bdc_0403;
    const SHADOW_MAP: u32 = 0x730d_f9ee;
    const REFLECTION: u32 = 0x9edd_3243;
    const WEIGHT: u32 = 0x5e5b_1937;
    const IBL_SCALE_POWER: u32 = 0x7480_de6d;
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("opening DATA00");
    let tint = rcsmaterial::name_hash("zoneColourTint");
    for (path, covered) in [
        (
            "/data/environments/talons_junction/materials/etched_glass_tech.rcsmaterial",
            6,
        ),
        (
            "/data/environments/modesto_heights/materials/etched_glass.rcsmaterial",
            6,
        ),
        (
            "/data/environments/amphiseum/materials/etched_glass_amph.rcsmaterial",
            6,
        ),
    ] {
        let blob = open.read_path(path).expect("reading the material");
        let material = RcsMaterial::parse(&blob).expect("parsing the material");
        let mut offsets: Vec<usize> = material
            .variants
            .iter()
            .map(|v| v.fragment.offset)
            .collect();
        offsets.sort_unstable();
        offsets.dedup();
        let mut grid_alpha = 0;
        for at in offsets {
            let Some(declared) = Declared::parse(&blob, at) else {
                continue;
            };
            if !declared.parameters.contains(&tint) {
                continue;
            }
            let has = |h: u32| declared.samplers.iter().any(|&(s, _)| s == h);
            assert!(
                has(REFLECTION)
                    && declared.parameters.contains(&WEIGHT)
                    && declared.parameters.contains(&IBL_SCALE_POWER),
                "{path} @{at:#x}: a Zone variant without the reflection term"
            );
            if has(SHADOW_MAP) {
                continue;
            }
            let grid_unit = declared
                .samplers
                .iter()
                .find(|&&(s, _)| s == TEXTURE1)
                .map(|&(_, unit)| unit);
            let program = Program::parse(&blob, at).expect("decoding the block");
            let alpha = program.output_texels()[3];
            assert!(
                grid_unit.is_some()
                    && matches!(alpha, Texel::Unit { unit, .. } if Some(u32::from(unit)) == grid_unit),
                "{path} @{at:#x}: alpha {alpha:?} is not the grid's (unit {grid_unit:?})"
            );
            grid_alpha += 1;
        }
        assert_eq!(
            grid_alpha, covered,
            "{path}: Zone variants with the grid's alpha"
        );
    }
}

/// The holographic billboards' Zone variants draw the Zone surface, not the advert: across all four PS3
/// archives, every material with a variant declaring the advert, scanline and second-layer samplers
/// (`mesh::rcs::holo_billboard`'s family) has Zone variants (`zoneColourTint`) that declare neither the
/// advert (`0x6f469b89`) nor the second layer (`0xdd7ec609`) and do declare the Zone texture
/// (`0xd5e000d1`), so `shade.wesl` sends a holo chunk to the generic Zone surface in a Zone frame. The
/// census must cover `scanlinebillboard` and its `_alphaboost` and `_desaturate` siblings and at least
/// one holographic shape (`docs/rendering/hd-zone-recolour.md`, "Holographic billboards in a Zone frame").
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn no_zone_variant_of_a_holo_billboard_samples_the_advert() {
    const ADVERT: u32 = 0x6f46_9b89;
    const SCANLINES: u32 = 0xfad8_b460;
    const SECOND: u32 = 0xdd7e_c609;
    const ZONE_TEX: u32 = 0xd5e0_00d1;
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let tint = rcsmaterial::name_hash("zoneColourTint");
    let mut covered: Vec<String> = Vec::new();
    let mut zone_variants = 0usize;
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
            let zone = |d: &&Declared| d.parameters.contains(&tint);
            if !declared
                .iter()
                .any(|d| has(d, ADVERT) && has(d, SCANLINES) && has(d, SECOND) && !zone(&d))
            {
                continue;
            }
            covered.push(path.clone());
            for d in declared.iter().filter(zone) {
                zone_variants += 1;
                assert!(
                    !has(d, ADVERT) && !has(d, SECOND) && has(d, ZONE_TEX),
                    "{archive} {path}: a Zone variant keeps the advert or lacks the Zone texture"
                );
            }
        }
    }
    eprintln!(
        "{} holo materials, {zone_variants} Zone variants: {covered:#?}",
        covered.len()
    );
    for stem in [
        "scanlinebillboard",
        "scanlinebillboard_alphaboost",
        "scanlinebillboard_desaturate",
    ] {
        let file = format!("/{stem}.rcsmaterial");
        assert!(
            covered.iter().any(|p| p.ends_with(&file)),
            "the census missed {stem}: {covered:#?}"
        );
    }
    assert!(
        covered.iter().any(|p| p.contains("holo")) && zone_variants > 0,
        "the census found no holographic shape: {covered:#?}"
    );
}
