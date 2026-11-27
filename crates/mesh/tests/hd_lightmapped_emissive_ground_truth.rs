//! Every HD circuit family whose lightmapped program adds an emissive binds it, and the count per
//! family does not drift (`docs/rendering/hd-zone-recolour.md`, "The other lightmapped emissive
//! families").
//!
//! **`#[ignore]`d and never run in CI.** It needs game content. Run with `just test-data`, or only
//! this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-mesh --run-ignored all \
//!     -E 'test(/^hd_lightmapped_emissive_ground_truth::/)'
//! ```
//!
//! One test per archive: DATA00 holds the DLC circuits, DATA02 the base circuits and DATA03
//! their reversed layouts.

use std::collections::BTreeMap;

use oag_mesh::mesh::{self, slots};
use oag_rcs::rcsmaterial::Declared;

use crate::archive_cache;

const LIGHTMAP: u32 = 0x37b5_db58;

/// `(family, the sampler its program adds, whether it rides the fourth binding)`.
const FAMILIES: &[(&str, u32, bool)] = &[
    ("tunnel_fx_noalpha", 0xb1f2_a176, false),
    ("tunnel_fx_noalpha_mult", 0xb1f2_a176, false),
    (
        "diffuse_emissive_with_specular_from_alpha",
        0x3bdc_0403,
        false,
    ),
    (
        "diffuse_with_specular_from_alpha_scalar",
        0x3bdc_0403,
        false,
    ),
    ("diffuseemissivealphaspecular", 0xb1f2_a176, false),
    ("diffuse_spec_fresnel_emissive", 0xa2d5_55b9, false),
    ("and_diffuse_emissive", 0xfa79_b1cd, false),
    ("and_diffuse_emissive_spec", 0xfa79_b1cd, false),
    ("mr_shopwin_reflecttranspemis", 0xd274_75a1, false),
    ("uvanim_diffuse_emissive", 0xb1f2_a176, false),
    ("mt_uvanim_diffuse_emissive", 0xb1f2_a176, false),
    ("mt_uvanim_diffuse_emissive2", 0xb1f2_a176, false),
    // Bound nowhere: its lightmapped compile declares no `time`, and its law is unread.
    ("mt_uvanim_diffuse_emissive3tokey", 0xb1f2_a176, false),
    ("track_surface", 0xb1f2_a176, true),
    ("track_wall", 0xb1f2_a176, true),
];

/// Lightmapped slots left dark, as `((circuit, family), slots over both layouts)`.
type Dark = BTreeMap<(String, &'static str), usize>;

/// Builds every `track.vex` and `track_reversed.vex` of `DATA0{archive}` and returns the bound
/// lightmapped slots of the listed families whose program declares the emissive, split into those
/// that bind it (per family) and those left dark (per circuit and family).
fn census(archive: usize) -> Option<(BTreeMap<&'static str, usize>, Dark)> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let spec = format!("{}:PS3_GAME/USRDIR/DATA0{archive}.PSARC", image.display());
    let paths: Vec<String> = oag_assets::psarc::Archive::open(&spec)
        .expect("the archive opens")
        .paths()
        .iter()
        .filter(|p| {
            p.starts_with("/data/environments/")
                && (p.ends_with("/track.vex") || p.ends_with("/track_reversed.vex"))
        })
        .cloned()
        .collect();
    // The reversed layouts of the base circuits keep their textures in DATA02.
    let base = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let read = |name: &str| {
        archive_cache::read(&spec, name).or_else(|| {
            (archive == 3)
                .then(|| archive_cache::read(&base, name))
                .flatten()
        })
    };
    let mut bound: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut dark = Dark::new();
    for path in paths {
        let data = read(&path).expect("the .vex reads");
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &path, &data) else {
            continue;
        };
        let source = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
        let (model, report) = mesh::rcs::build_scene(
            &path,
            &data,
            &geometry,
            &mut |name| read(name),
            oag_gpu::formats::SceneStore::default(),
        )
        .expect("the circuit builds");
        // The glow table is shared with readers that run later: this pass must leave them room.
        assert_eq!(report.pad_ne_unread, 0, "{path}: a pad mask lost its entry");
        for (slot, material) in source.materials.iter().enumerate() {
            let leaf = material.name.rsplit('/').next().unwrap_or_default();
            let Some(&(family, sampler, fourth)) = FAMILIES
                .iter()
                .find(|(name, ..)| leaf.strip_suffix(".rcsmaterial") == Some(name))
            else {
                continue;
            };
            let Some(&word) = model.material_slots.get(slot) else {
                continue;
            };
            let Some(variant) = model.material_variants.get(slot).copied().flatten() else {
                continue;
            };
            if word & slots::SECOND_IS_LIGHTMAP == 0 {
                continue;
            }
            let blob = read(&format!("/{}", material.name)).expect("the material reads");
            let declared =
                Declared::parse(&blob, variant.fragment.offset).expect("the block parses");
            let declares = |hash: u32| declared.samplers.iter().any(|(h, _)| *h == hash);
            if !(declares(sampler) && declares(LIGHTMAP)) {
                continue;
            }
            let entry = slots::material_index(word)
                .checked_sub(1)
                .and_then(|i| model.emissive.get(i as usize))
                .map(|e| e.program[0]);
            let table = if fourth {
                &model.wave_maps
            } else {
                &model.pad_masks
            };
            let picture = table.get(slot).and_then(Option::as_ref);
            let wanted = material
                .samplers
                .iter()
                .find(|(hash, _)| *hash == sampler)
                .and_then(|(_, p)| p.clone());
            let circuit = path.split('/').nth(3).unwrap_or_default();
            let add = entry == Some(mesh::PROGRAM_LIGHTMAPPED_ADD);
            let scrolled = entry == Some(mesh::PROGRAM_LIGHTMAPPED_SCROLL);
            if (add || scrolled) && picture.map(|t| t.label.as_str()) == wanted.as_deref() {
                *bound.entry(family).or_default() += 1;
            } else {
                println!("dark: {path} slot {slot} {family} entry {entry:?}");
                *dark.entry((circuit.to_string(), family)).or_default() += 1;
            }
        }
    }
    println!("DATA0{archive} lightmapped emissive slots: {bound:?}, dark: {dark:?}");
    Some((bound, dark))
}

/// Asserts both halves of [`census`]. `dark` names every slot known to stay unlit: Vineta K's
/// `mt_uvanim_diffuse_emissive3tokey` runs an unread law.
fn expect(archive: usize, want: &[(&str, usize)], dark: &[(&str, &str, usize)]) {
    let Some((bound, left)) = census(archive) else {
        return;
    };
    let want: BTreeMap<&str, usize> = want.iter().copied().collect();
    assert_eq!(bound, want, "DATA0{archive}'s census moved");
    let left: Vec<(&str, &str, usize)> = left
        .iter()
        .map(|((circuit, family), &n)| (circuit.as_str(), *family, n))
        .collect();
    assert_eq!(left, dark, "DATA0{archive}'s dark slots moved");
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn data00s_circuits_bind_every_lightmapped_emissive() {
    expect(
        0,
        &[
            ("and_diffuse_emissive", 16),
            ("and_diffuse_emissive_spec", 4),
            ("diffuse_emissive_with_specular_from_alpha", 28),
            ("diffuse_spec_fresnel_emissive", 10),
            ("diffuse_with_specular_from_alpha_scalar", 64),
            ("diffuseemissivealphaspecular", 12),
            ("track_surface", 24),
            ("track_wall", 25),
            ("tunnel_fx_noalpha", 46),
            ("tunnel_fx_noalpha_mult", 4),
            ("uvanim_diffuse_emissive", 36),
        ],
        &[],
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn data02s_circuits_bind_every_lightmapped_emissive() {
    expect(
        2,
        &[
            ("mr_shopwin_reflecttranspemis", 8),
            ("mt_uvanim_diffuse_emissive", 24),
            ("mt_uvanim_diffuse_emissive2", 13),
        ],
        &[("01_vineta_k", "mt_uvanim_diffuse_emissive3tokey", 2)],
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn data03s_circuits_bind_every_lightmapped_emissive() {
    expect(
        3,
        &[
            ("mr_shopwin_reflecttranspemis", 8),
            ("mt_uvanim_diffuse_emissive", 25),
            ("mt_uvanim_diffuse_emissive2", 12),
        ],
        &[("01_vineta_k", "mt_uvanim_diffuse_emissive3tokey", 2)],
    );
}
