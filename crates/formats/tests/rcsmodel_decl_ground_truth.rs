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

/// **A submesh is judged on the vertices a triangle names, and that recovers
/// 1,053 of the disc's 1,377 apparent strays.**
///
/// A declared vertex count can run past the buffer the file wrote for it:
/// `talons_junction`'s `tanker4c1Shape` declares 1,288 vertices in 23,152
/// bytes, which at stride 18 is 32 bytes short, so its last two vertices are
/// read out of the *next* submesh's buffer and land 100 units from anything.
/// **The index buffer never references them** - RSX fetches a vertex when an
/// index asks for it and never sweeps the array - so the original draws the
/// same picture either way.
///
/// Both readings are counted, because the number that matters is the
/// difference: judging every declared vertex condemns submeshes the file draws
/// perfectly well, and `oag_render::mesh::rcs` dropped 17 of Talon's Junction's
/// 117 on that basis.
///
/// **324 submeshes are still outside after that, and this test pins the number
/// rather than hiding it behind a percentage.** They are not spread evenly:
/// `talons_junction`, `amphiseum` and `tech_de_ra` have none at all, and the
/// residue clusters in `zone_2`, `zone_3`, `01_vineta_k` and
/// `15_anulpha_pass`. What is different about those is **unrecovered** - see
/// `docs/formats/rcsmodel.md`. A drop from 1,377 to 324 is a reading that got
/// better and not one that is finished.
#[test]
#[ignore]
fn a_submesh_is_judged_on_the_vertices_a_triangle_names() {
    let Some(image) = image() else { return };
    let (mut submeshes, mut strays_all, mut strays_drawn, mut files) = (0usize, 0, 0, 0usize);
    let mut worst: Vec<String> = Vec::new();
    // The widths the residue declares, which is what says whether it is a
    // fifth vertex format or an ordinary one that disagrees with its box.
    let mut residue_widths: BTreeMap<usize, usize> = BTreeMap::new();
    // Strays per top directory under `/data/environments/`, so the three
    // circuits this reading was measured on can be asserted at zero rather than
    // absorbed into a disc-wide total. **Everything outside that tree - the
    // craft, the weapons, the HUD models - pools under the empty key**, so
    // `strays[""]` says nothing about any one of them.
    let mut strays: BTreeMap<String, usize> = BTreeMap::new();
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
            let vex = path.replace(".rcsmodel", ".vex");
            let (Ok(model_blob), Ok(blob)) = (open.read_path(&path), open.read_path(&vex)) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&model_blob) else {
                continue;
            };
            files += 1;
            for (name, hash, min, max) in rcsmodel_common::mesh_nodes(&blob) {
                let Some(mesh) = model.mesh(hash) else {
                    continue;
                };
                let Some(stride) = mesh.declared_stride() else {
                    continue;
                };
                for submesh in &mesh.submeshes {
                    if submesh.vertex_count == 0 || submesh.index_count == 0 {
                        continue;
                    }
                    submeshes += 1;
                    let Ok(points) = mesh.positions(&model_blob, submesh, stride) else {
                        continue;
                    };
                    let slack = 1e-2;
                    let inside = |p: &[f32; 3]| {
                        (0..3).all(|i| p[i] >= min[i] - slack && p[i] <= max[i] + slack)
                    };
                    if !points.iter().all(inside) {
                        strays_all += 1;
                    }
                    let circuit = path
                        .strip_prefix("/data/environments/")
                        .and_then(|rest| rest.split('/').next())
                        .unwrap_or("")
                        .to_string();
                    let entry = strays.entry(circuit).or_default();
                    if !mesh.submesh_fits(&model_blob, submesh, stride, (min, max)) {
                        strays_drawn += 1;
                        *entry += 1;
                        *residue_widths.entry(stride).or_default() += 1;
                        if worst.len() < 10 {
                            worst.push(format!("{path} {name} ({hash:#010x}) stride {stride}"));
                        }
                    }
                }
            }
        }
    }
    println!(
        "{files} model(s) with a sibling .vex, {submeshes} node-addressed submeshes: \
         {strays_all} leave the box counting every declared vertex, \
         {strays_drawn} counting only the ones a triangle names"
    );
    for line in &worst {
        println!("  {line}");
    }
    println!("the residue declares {residue_widths:?}");
    assert_eq!(submeshes, 12_624, "the disc's node-addressed submesh count");
    assert_eq!(
        (strays_all, strays_drawn),
        (1_377, 324),
        "the two readings, pinned exactly: a change either way is a change in \
         what the reader believes a vertex buffer holds"
    );
    for circuit in ["talons_junction", "amphiseum", "tech_de_ra"] {
        assert_eq!(
            strays.get(circuit).copied().unwrap_or(usize::MAX),
            0,
            "{circuit} is one of the three measured to have no stray at all, \
             which is the claim the renderer's per-submesh gate rests on"
        );
    }
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

/// **Which of the four-byte attributes are vectors and which are colours**, on
/// a test neither can pass by being the other.
///
/// Every `0x44` attribute is four normalised bytes and they are not one thing:
/// `tangent` is a packed direction, `colorSet1` is painted vertex colour, and
/// `0x1aaf7631` - the commonest of them disc-wide, 13,485 uses, still unnamed -
/// is neither a direction nor an opaque colour.
///
/// The discriminator is the one
/// [`the_four_bytes_after_a_position_are_the_vertex_normal`] uses for the
/// normal, and it is an oracle the file does not state: read the first three
/// lanes as `byte / 127.5 - 1`, and ask whether the result is a unit vector and
/// whether it is perpendicular to the vertex's own normal. A packed tangent
/// must be both. A colour has no reason to be either, and a null result scores
/// `1/sqrt(3)` = 0.577, which is what a random direction gives against any
/// normal.
///
/// **Why this is a test rather than a note:** it is what says the vertex colour
/// must not be wired into a renderer yet. `colorSet1` runs the full `0..=255`
/// on all three lanes with a mean of 128 and an alpha pinned at 255, on a
/// material the disc itself calls `defuse_occulsion_vert_col_tint` - baked
/// occlusion, not a light tint. `mesh::GpuVertex::lit` records what happens
/// when baked lighting is multiplied by a light rig as well, and which of the
/// two a draw wants is per-draw state nothing here has recovered.
#[test]
#[ignore]
fn a_packed_tangent_is_a_unit_vector_and_a_vertex_colour_is_not() {
    if image().is_none() {
        return;
    }
    /// `~crc32("tangent")`, the control: it must pass.
    const TANGENT: u32 = 0xdbe5_f417;
    /// `~crc32("colorSet1")`.
    const COLOUR_SET: u32 = 0xce5c_d9d9;
    /// The commonest four-byte attribute on the disc, still unnamed.
    const UNNAMED: u32 = 0x1aaf_7631;

    let mut seen: BTreeMap<u32, (usize, usize, f64, usize)> = BTreeMap::new();
    let mut alpha: BTreeMap<u32, BTreeMap<u8, usize>> = BTreeMap::new();
    for (_, bytes, model) in every_model() {
        for mesh in &model.meshes {
            let Some(decl) = &mesh.decl else { continue };
            for want in [TANGENT, COLOUR_SET, UNNAMED] {
                let Some(a) = decl.attributes.iter().find(|a| a.name_hash == want) else {
                    continue;
                };
                for sub in &mesh.submeshes {
                    let normals = mesh.normals(&bytes, sub, decl.stride).ok();
                    for k in 0..sub.vertex_count {
                        let at = sub.vertex_offset + k * decl.stride + usize::from(a.offset);
                        let Some(four) = bytes.get(at..at + 4) else {
                            break;
                        };
                        // A vertex the file left blank is neither, and there are
                        // enough of them to move a mean.
                        if four == [0, 0, 0, 0] {
                            continue;
                        }
                        let entry = seen.entry(want).or_insert((0, 0, 0.0, 0));
                        entry.0 += 1;
                        *alpha.entry(want).or_default().entry(four[3]).or_default() += 1;
                        let v: [f64; 3] = std::array::from_fn(|i| f64::from(four[i]) / 127.5 - 1.0);
                        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                        if (0.9..=1.1).contains(&len) {
                            entry.1 += 1;
                        }
                        if len > 0.5
                            && let Some(n) = normals.as_ref().and_then(|n| n.get(k))
                        {
                            let dot = v[0] * f64::from(n[0])
                                + v[1] * f64::from(n[1])
                                + v[2] * f64::from(n[2]);
                            entry.2 += (dot / len).abs();
                            entry.3 += 1;
                        }
                    }
                }
            }
        }
    }
    let report = |want: u32| -> (f64, f64) {
        let (total, unit, dot, dots) = seen[&want];
        let opaque = alpha[&want].get(&255).copied().unwrap_or(0);
        let (unit, dot) = (unit as f64 / total as f64, dot / dots.max(1) as f64);
        println!(
            "  {want:#010x}: {total} vertices, {:.1} % unit, mean |dot| {dot:.3}, \
             {:.0} % with a fourth byte of 255",
            100.0 * unit,
            100.0 * opaque as f64 / total as f64
        );
        (unit, dot)
    };
    let (tangent_unit, tangent_dot) = report(TANGENT);
    let (colour_unit, colour_dot) = report(COLOUR_SET);
    let (unnamed_unit, unnamed_dot) = report(UNNAMED);

    assert!(
        tangent_unit > 0.95 && tangent_dot < 0.05,
        "tangent is the control and must read as a packed direction"
    );
    for (label, unit, dot) in [
        ("colorSet1", colour_unit, colour_dot),
        ("0x1aaf7631", unnamed_unit, unnamed_dot),
    ] {
        assert!(
            unit < 0.2 && dot > 0.4,
            "{label} reads as a direction ({:.1} % unit, |dot| {dot:.3}), which would mean \
             this discriminator does not separate the two",
            100.0 * unit
        );
    }
    // The alpha lane is what says `colorSet1` is a colour and the unnamed one is
    // something else: an opaque colour pins it, and a mask is bimodal.
    let opaque_share =
        |want: u32| alpha[&want].get(&255).copied().unwrap_or(0) as f64 / seen[&want].0 as f64;
    assert!(
        opaque_share(COLOUR_SET) > 0.99,
        "colorSet1's fourth byte is 255 on every vertex measured"
    );
    assert!(
        opaque_share(UNNAMED) < 0.5,
        "the unnamed attribute's fourth byte is not an opaque alpha, which is part of \
         why it is not named"
    );
}
