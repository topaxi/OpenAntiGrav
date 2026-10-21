//! Which of Wipeout HD's circuit materials write `1 - v` into the coordinate their
//! fragment program samples, against the disc.
//!
//! `vertex::Program::flips` first read only a negation written straight into an
//! interpolator (21 materials), and missed the shape where the negation lands in a
//! temporary that a later `MOV` copies out (20 more, added 2026-10-21). The miss
//! drew Amphiseum's and Tech de Ra's pit boxes upside down, and the left wall at
//! Talon's Junction's start. Dropping the temporary hop fails here by name.
//!
//! `#[ignore]`d and never run in CI; `just test-data` runs it.

use std::collections::BTreeMap;

use oag_rcs::{rcsmaterial, rcsmodel};

use crate::rcsmodel_common::image;

/// `(material path, vertex block offset)` to whether that block flips its `Uv1`.
fn flips_of(
    image: &std::path::Path,
    archive: &str,
    model_path: &str,
) -> BTreeMap<(String, usize), bool> {
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    let bytes = open.read_path(model_path).expect("the model reads");
    let model = rcsmodel::Model::parse(&bytes).expect("the model parses");
    let mut decl_of = BTreeMap::new();
    for mesh in model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces) {
        if let Some(decl) = mesh.decl.as_ref() {
            decl_of.entry(mesh.material).or_insert(decl);
        }
    }
    let mut out = BTreeMap::new();
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(decl) = decl_of.get(&u32::try_from(slot).unwrap()) else {
            continue;
        };
        let Some(hash) = decl.diffuse_texcoord().map(|a| a.name_hash) else {
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
        out.insert(
            (material.name.clone(), variant.vertex.offset),
            program.flips(hash),
        );
    }
    out
}

fn flips(image: &std::path::Path, circuit: &str, material: &str) -> Vec<bool> {
    flips_of(
        image,
        "DATA00.PSARC",
        &format!("/data/environments/{circuit}/track.rcsmodel"),
    )
    .into_iter()
    .filter(|((name, _), _)| name.ends_with(material))
    .map(|(_, flipped)| flipped)
    .collect()
}

#[test]
#[ignore = "needs the PS3 disc image"]
fn amphiseums_pit_box_material_flips_and_talons_does_not() {
    let Some(image) = image() else { return };
    let amphiseum = flips(
        &image,
        "amphiseum",
        "/track_surface_no_emissive.rcsmaterial",
    );
    assert!(
        !amphiseum.is_empty() && amphiseum.iter().all(|&f| f),
        "{amphiseum:?}"
    );
    let talons = flips(&image, "talons_junction", "/track_surface.rcsmaterial");
    assert!(
        !talons.is_empty() && talons.iter().all(|&f| !f),
        "{talons:?}"
    );
}

#[test]
#[ignore = "needs the PS3 disc image"]
fn talons_wall_block_that_flips_through_a_temporary_is_read() {
    let Some(image) = image() else { return };
    let all = flips_of(
        &image,
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.rcsmodel",
    );
    let wall = all
        .iter()
        .find(|((name, offset), _)| name.ends_with("/track_wall.rcsmaterial") && *offset == 0x19630)
        .map(|(_, &flipped)| flipped);
    assert_eq!(wall, Some(true), "{all:?}");
}
