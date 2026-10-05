//! Whether a `0.0` [`fragment::Program::specular_exponent`] that is patched
//! from `SpecularPower` is always backed by a non-zero authored value.
//!
//! `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "Ships have no Lambert
//! diffuse either" names a resolved `0.0` the strongest candidate for
//! `SpecularPower` patched at draw time rather than baked in the file -
//! `pow(x, 0) = 1` is not a plausible authored shininess. This pins that as a
//! disc invariant rather than a one-off reading:
//! `crates/render/examples/hd_specular_patch_census.rs` found it holding on
//! 62 materials across 16 circuits; this is the same check, narrower and
//! `#[ignore]`d like every other disc-backed test here, on the three models
//! [`rcsmodel_common::PAIRS`] already shares with the other `.rcsmodel`
//! ground-truth binaries.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run
//! with `just test-data`.

mod rcsmodel_common;

use oag_rcs::rcsmaterial::{self, Class, Features, fragment};
use oag_rcs::rcsmodel;
use rcsmodel_common::{PAIRS, image};

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_zero_specular_exponent_patched_from_specular_power_is_authored_non_zero() {
    let Some(image) = image() else {
        return;
    };

    let mut zero_and_patched = 0usize;
    let mut confirmed = 0usize;
    for &(archive, _vex, rcsmodel_path) in PAIRS {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let bytes = open.read_path(rcsmodel_path).expect("the .rcsmodel reads");
        let model = rcsmodel::Model::parse(&bytes).expect("it parses");

        // The chunk-carried vertex declaration for each material slot that
        // draws at least one chunk - the same lookup
        // `oag_mesh::mesh::rcs::skin::variants` builds, reimplemented here
        // rather than depended on: this crate sits below `oag-render` in the
        // dependency graph.
        let mut decl_of: std::collections::BTreeMap<u32, Option<&rcsmodel::VertexDecl>> =
            Default::default();
        for mesh in &model.meshes {
            decl_of.entry(mesh.material).or_insert(mesh.decl.as_ref());
        }

        let mut cache: std::collections::BTreeMap<String, Option<Vec<u8>>> = Default::default();
        for (slot, material) in model.materials.iter().enumerate() {
            let ordinal = u32::try_from(slot).unwrap_or(u32::MAX);
            let Some(decl) = decl_of.get(&ordinal).copied() else {
                continue;
            };
            let blob = cache
                .entry(material.name.clone())
                .or_insert_with(|| open.read_path(&format!("/{}", material.name)).ok())
                .clone();
            let Some(blob) = blob else { continue };
            let Ok(parsed) = rcsmaterial::RcsMaterial::parse(&blob) else {
                continue;
            };
            let word = Features::chunk_word(rcsmaterial::LIT_RACE_PASS, decl);
            let key = Features::from_pass_word(word);
            let Some(variant) = parsed.variant(Class::Static, key) else {
                continue;
            };
            let Some(program) = fragment::Program::parse(&blob, variant.fragment.offset) else {
                continue;
            };
            if program.specular_exponent() != Some(0.0) {
                continue;
            }
            let Some(exponent_slot) = program.specular_exponent_slot() else {
                continue;
            };
            if !program
                .patches(rcsmaterial::SPECULAR_POWER)
                .any(|slot| slot == exponent_slot)
            {
                continue;
            }
            zero_and_patched += 1;
            let authored = material
                .parameters
                .iter()
                .find(|p| p.hash == rcsmaterial::SPECULAR_POWER)
                .map(|p| p.value[0]);
            assert!(
                authored.is_some_and(|v| v != 0.0),
                "{} slot {slot}: SpecularPower is patched from code slot \
                 {exponent_slot} but the model authors {authored:?}",
                material.name
            );
            confirmed += 1;
        }
    }

    println!("{zero_and_patched} slot(s) resolve to a 0.0 chain patched from SpecularPower");
    // Measured on these three models: 12. A floor rather than an exact
    // match, so a small, unrelated shift in variant resolution does not fail
    // this test for a reason that has nothing to do with the invariant it
    // checks - but a drop to near zero, or a rise by an order of magnitude,
    // both deserve a look rather than a silent floor bump.
    assert!(
        zero_and_patched >= 6,
        "only {zero_and_patched} slot(s) exercised the SpecularPower patch on \
         these three models (measured 12) - a fixture drift, not a passing check"
    );
    assert_eq!(confirmed, zero_and_patched);
}
