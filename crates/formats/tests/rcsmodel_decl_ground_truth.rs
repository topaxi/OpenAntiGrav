//! What the disc says about the vertex declaration a `.rcsmodel` chunk points
//! at.
//!
//! The fourth of the `.rcsmodel` ground-truth binaries, beside
//! `rcsmodel_ground_truth.rs` (the container), `rcsmodel_vertex_ground_truth.rs`
//! (what a vertex holds) and `rcsmodel_material_ground_truth.rs` (how it is
//! painted). Split for the same reason they are: one file per claim family
//! keeps each under the size ratchet and keeps a failure legible.
//!
//! **These claims are what retired two inferences**: the stride search over
//! three widths, and the assumption that a texture coordinate is the last four
//! bytes of a vertex. See `docs/formats/rcsmodel.md`.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

mod rcsmodel_common;

use std::collections::{BTreeMap, BTreeSet};

use oag_formats::rcsmodel::{self, Attribute, Layout};
use rcsmodel_common::image;

/// Every archive on the disc that holds `.rcsmodel` files.
const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// Every model on the disc, parsed, with the path it came from.
fn every_model() -> Vec<(String, Vec<u8>, rcsmodel::Model)> {
    let image = image().expect("checked by the caller");
    let mut out = Vec::new();
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(bytes) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&bytes) else {
                continue;
            };
            out.push((path, bytes, model));
        }
    }
    out
}

/// **The claim the whole decode rests on.**
///
/// Every chunk that carries submesh descriptors resolves a declaration, and
/// every attribute record of every one of them repeats the header's stride at
/// its own `+0x04`. That repeat is an internal cross-check the bytes did not
/// have to pass: a wrong pointer, a wrong header size or a wrong record size
/// would each break it on the first record, and the disc has over a hundred
/// thousand of them.
#[test]
#[ignore]
fn every_described_chunk_declares_a_layout_that_cross_checks() {
    if image().is_none() {
        return;
    }
    let (mut chunks, mut attributes, mut without) = (0usize, 0usize, 0usize);
    for (path, _, model) in every_model() {
        for mesh in &model.meshes {
            if mesh.layout != Layout::Described {
                // An inline chunk's `+0x58` is its index count, so it has no
                // declaration to find and is not what this claim is about.
                continue;
            }
            chunks += 1;
            match &mesh.decl {
                Some(decl) => attributes += decl.attributes.len(),
                None => {
                    without += 1;
                    println!("no declaration: {path} chunk {:#010x}", mesh.hash);
                }
            }
        }
    }
    println!("{chunks} described chunks, {attributes} attribute records");
    assert!(chunks > 5_000, "the sweep found almost nothing: {chunks}");
    assert_eq!(
        without, 0,
        "chunks with submesh descriptors and no declaration"
    );
}

/// **The declaration agrees with the search wherever the search settles.**
///
/// This is what says the field is the stride rather than something that
/// correlates with it. The search is an independent measurement - it fits
/// dequantised positions to the chunk's own authored box - and it is a fitting
/// procedure rather than a reading, so where the two agree the declaration is
/// confirmed and where the search says nothing the declaration is the only
/// answer there is.
///
/// The seven disagreements are kept as a number rather than removed: each is a
/// chunk whose geometry the search fits at a width the file does not declare,
/// and nothing here has read which of the two draws correctly.
#[test]
#[ignore]
fn the_declared_stride_is_the_one_the_search_finds() {
    if image().is_none() {
        return;
    }
    let (mut agree, mut disagree, mut only_declared) = (0usize, 0usize, 0usize);
    for (_, bytes, model) in every_model() {
        for mesh in &model.meshes {
            let Some(decl) = &mesh.decl else { continue };
            match mesh.solve_stride_without_a_box(&bytes) {
                Some(found) if found == decl.stride => agree += 1,
                Some(_) => disagree += 1,
                None => only_declared += 1,
            }
        }
    }
    println!("{agree} agree, {disagree} disagree, {only_declared} the search gave up on");
    assert!(
        disagree * 1_000 < agree,
        "the declared stride should agree with the search: {agree} vs {disagree}"
    );
    assert!(
        only_declared > 1_000,
        "the declaration should rescue the chunks the search skips, not {only_declared}"
    );
}

/// **The widths the disc declares, against the three the search knows.**
///
/// `rcsmodel::STRIDES` is `[14, 18, 22]`, measured off authored bounding boxes.
/// The disc declares four more, and a chunk at one of those is one the search
/// cannot fit by construction - which is the mechanism behind the count the
/// test above asserts.
#[test]
#[ignore]
fn the_disc_declares_more_widths_than_the_search_looks_for() {
    if image().is_none() {
        return;
    }
    let mut widths: BTreeMap<usize, usize> = BTreeMap::new();
    for (_, _, model) in every_model() {
        for mesh in &model.meshes {
            if let Some(decl) = &mesh.decl {
                *widths.entry(decl.stride).or_default() += 1;
            }
        }
    }
    println!("declared strides: {widths:?}");
    for width in rcsmodel::STRIDES {
        assert!(
            widths.contains_key(width),
            "the search's width {width} should be one the disc declares"
        );
    }
    let extra: Vec<_> = widths
        .keys()
        .filter(|w| !rcsmodel::STRIDES.contains(w))
        .collect();
    assert!(
        !extra.is_empty(),
        "the point of this test is the widths the search does not know"
    );
}

/// **Which types the disc uses, so a new one is a finding rather than a
/// rounding.**
///
/// The same rule `rcsmodel::Factor::from_rsx` follows for a blend factor: the
/// values are enumerated, and a value outside them is reported. Seven codes,
/// each a component count in the high nibble and an RSX vertex type in the low.
#[test]
#[ignore]
fn the_disc_uses_seven_vertex_types_and_no_others() {
    if image().is_none() {
        return;
    }
    let mut seen: BTreeMap<(u8, u8), usize> = BTreeMap::new();
    for (_, _, model) in every_model() {
        for mesh in &model.meshes {
            let Some(decl) = &mesh.decl else { continue };
            for attribute in &decl.attributes {
                *seen
                    .entry((attribute.components, attribute.rsx_type))
                    .or_default() += 1;
            }
        }
    }
    for ((components, rsx), count) in &seen {
        println!("  {components} x type {rsx}: {count}");
    }
    let known: BTreeSet<(u8, u8)> = [
        (1, rcsmodel::vertex_decl::RSX_COMPRESSED),
        (2, rcsmodel::vertex_decl::RSX_FLOAT),
        (2, rcsmodel::vertex_decl::RSX_HALF),
        (3, rcsmodel::vertex_decl::RSX_SHORT),
        (4, rcsmodel::vertex_decl::RSX_FLOAT),
        (4, rcsmodel::vertex_decl::RSX_HALF),
        (4, rcsmodel::vertex_decl::RSX_UBYTE_NORM),
    ]
    .into_iter()
    .collect();
    let found: BTreeSet<(u8, u8)> = seen.keys().copied().collect();
    assert_eq!(found, known, "an unrecorded vertex type is a finding");
}

/// **Every chunk declares a position and a normal, in the shape already
/// recovered for them.**
///
/// The control on the name recovery. `position` and `normal` were decoded long
/// before this declaration was read - three quantised shorts at offset 0, one
/// compressed word at offset 6 - and the hashes `~crc32("position")` and
/// `~crc32("normal")` land on exactly those attributes on every chunk of the
/// disc. A name that agrees with content nobody chose it to fit is what makes
/// the rest of the wordlist credible.
#[test]
#[ignore]
fn position_and_normal_are_named_and_are_where_they_were_already_read() {
    if image().is_none() {
        return;
    }
    let mut chunks = 0usize;
    for (path, _, model) in every_model() {
        for mesh in &model.meshes {
            let Some(decl) = &mesh.decl else { continue };
            chunks += 1;
            let named = |want: &str| -> Attribute {
                *decl
                    .attributes
                    .iter()
                    .find(|a| a.name() == Some(want))
                    .unwrap_or_else(|| panic!("{path}: no {want} attribute"))
            };
            let position = named("position");
            assert_eq!(
                (position.components, position.rsx_type, position.offset),
                (3, rcsmodel::vertex_decl::RSX_SHORT, 0),
                "{path}: position"
            );
            let normal = named("normal");
            assert_eq!(
                (normal.components, normal.rsx_type, normal.offset),
                (
                    1,
                    rcsmodel::vertex_decl::RSX_COMPRESSED,
                    rcsmodel::NORMAL_OFFSET as u8
                ),
                "{path}: normal"
            );
        }
    }
    assert!(chunks > 5_000, "the sweep found almost nothing: {chunks}");
}

/// **The finding that changes a picture: the last four bytes of a vertex are
/// usually not its diffuse texture coordinate.**
///
/// Reading them as one is what this module's caller did before the declaration
/// was read, and on the commonest layout it painted a diffuse texture through
/// the atlas-packed coordinates of a lightmap.
#[test]
#[ignore]
fn the_diffuse_coordinate_is_rarely_the_last_four_bytes() {
    if image().is_none() {
        return;
    }
    let (mut last, mut elsewhere, mut none) = (0usize, 0usize, 0usize);
    let mut what_is_last: BTreeMap<String, usize> = BTreeMap::new();
    for (_, _, model) in every_model() {
        for mesh in &model.meshes {
            let Some(decl) = &mesh.decl else { continue };
            let Some(uv) = decl.diffuse_texcoord() else {
                none += 1;
                continue;
            };
            if usize::from(uv.offset) + 4 == decl.stride {
                last += 1;
            } else {
                elsewhere += 1;
                let occupant = decl
                    .attributes
                    .iter()
                    .find(|a| usize::from(a.offset) + a.width().unwrap_or(0) == decl.stride)
                    .and_then(Attribute::name)
                    .unwrap_or("unnamed");
                *what_is_last.entry(occupant.to_string()).or_default() += 1;
            }
        }
    }
    println!("diffuse coordinate last: {last}, elsewhere: {elsewhere}, absent: {none}");
    println!("what sits in the last four bytes instead: {what_is_last:?}");
    assert!(
        elsewhere > last / 4,
        "the whole point of reading the declaration is that {elsewhere} chunks put it elsewhere"
    );
    assert!(
        none > 0,
        "chunks with no texture coordinate at all should be counted, not assumed away"
    );
}
