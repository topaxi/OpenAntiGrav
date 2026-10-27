//! Which of Wipeout HD's circuit materials scale the bitangent by `2w - 1` and which by the raw
//! `w`, read off each resolved **vertex** block (`vertex::Program::bitangent_is_signed`).
//!
//! The original's compile of Amphiseum, Modesto Heights, Talon's Junction, Tech de Ra and Zone
//! 1 to 4 is signed; the eight numbered circuits' is raw
//! (`docs/rendering/hd-track-tone.md`, "Tech de Ra's near-left wall"). Reading the circuit's name
//! instead, or reading every block as signed, fails here by name. `#[ignore]`d and never run in
//! CI; `just test-data` runs it.

use std::collections::BTreeSet;

use oag_rcs::rcsmodel::vertex_decl::TANGENT_HASH;
use oag_rcs::{rcsmaterial, rcsmodel};

use crate::rcsmodel_common::image;

/// Every distinct `(material, vertex block offset, signed)` among the circuit's materials whose
/// chunks declare a tangent.
fn tangent_blocks(
    image: &std::path::Path,
    circuit: &str,
) -> Option<BTreeSet<(String, usize, bool)>> {
    for archive in ["DATA00.PSARC", "DATA01.PSARC", "DATA02.PSARC"] {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let Ok(bytes) = open.read_path(&format!("/data/environments/{circuit}/track.rcsmodel"))
        else {
            continue;
        };
        let model = rcsmodel::Model::parse(&bytes).expect("the model parses");
        let mut out = BTreeSet::new();
        for mesh in model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces) {
            let Some(decl) = mesh.decl.as_ref().filter(|d| d.tangent().is_some()) else {
                continue;
            };
            let Some(material) = model.materials.get(mesh.material as usize) else {
                continue;
            };
            let Ok(blob) = open.read_path(&format!("/{}", material.name)) else {
                continue;
            };
            let Ok(parsed) = rcsmaterial::RcsMaterial::parse(&blob) else {
                continue;
            };
            let word = rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, Some(decl));
            let key = rcsmaterial::Features::from_pass_word(word);
            let Some(variant) = parsed.variant(rcsmaterial::Class::Static, key) else {
                continue;
            };
            let Some(program) = rcsmaterial::vertex::Program::of(&blob, variant.vertex) else {
                continue;
            };
            out.insert((
                material.name.clone(),
                variant.vertex.offset,
                program.bitangent_is_signed(TANGENT_HASH),
            ));
        }
        return Some(out);
    }
    None
}

fn all_read_as(circuit: &str, signed: bool) {
    let Some(image) = image() else { return };
    let blocks = tangent_blocks(&image, circuit).expect("the circuit's model is on the disc");
    assert!(
        !blocks.is_empty(),
        "{circuit}: no tangent-reading block found"
    );
    assert!(
        blocks.iter().all(|(_, _, s)| *s == signed),
        "{circuit} must read signed = {signed} on every block: {blocks:?}"
    );
}

#[test]
#[ignore = "needs the PS3 disc image"]
fn tech_de_ra_scales_its_bitangent_by_two_w_minus_one() {
    all_read_as("tech_de_ra", true);
}

#[test]
#[ignore = "needs the PS3 disc image"]
fn modesto_heights_scales_its_bitangent_by_two_w_minus_one() {
    all_read_as("modesto_heights", true);
}

#[test]
#[ignore = "needs the PS3 disc image"]
fn sol_2_and_vineta_k_scale_their_bitangent_by_the_raw_w() {
    all_read_as("12_sol_2", false);
    all_read_as("01_vineta_k", false);
}
