//! Establishes that a `Skycube` node's payload **is a `Mesh` payload**, and
//! decodes `fogCube`, against every `.vex` file on the Pulse PSP disc.
//!
//! **`#[ignore]`d and never run in CI**: it needs game content (see
//! `docs/architecture/adr/0006-no-copyrighted-content.md`). Run with `just
//! test-data`; the tests skip with a message when the disc image is absent, and
//! `OAG_REQUIRE_GAME_DATA=1` turns absence into a failure (a skipped
//! ground-truth test is green and proves nothing).
//!
//! # What this is for
//!
//! `docs/formats/vex.md` only recorded that class `0x3c6` is *called* `Skycube`;
//! no payload byte or handler address was documented, hence a black sky. The
//! claim under test:
//!
//! > `Skycube` and `fogCube` are not new formats. A `Skycube` payload has the
//! > same layout as a `Mesh` `0x125` payload, so [`vex::mesh_materials`] and
//! > [`vex::mesh_batches`] already decode it; a `fogCube` payload is a 64-byte
//! > 4x4 followed by fog parameters.
//!
//! # Why closure, and not "the decoder returned something"
//!
//! `mesh_batches` on a payload that merely *resembles* a mesh returns plausible
//! batches, so a non-empty result proves nothing. The discriminating check is that
//! the decode **closes**: the material count at `+0x02` places the material array
//! where `+0x04`'s geometry offset says the geometry begins, and the batches
//! consume the payload to its end without running past it (the closure argument
//! that settled `WO Track`, `section` and the collision soup).
//!
//! Three further constraints a misreading cannot satisfy at once:
//!
//! - **Every material's texture index is in range** for the file's own
//!   `CLASS_TEXTURE` count: a wrong stride gives out-of-range indices at once
//!   (`05_Track`'s sky cites `0x6b..0x6f` against 130 textures).
//! - **The bounding box is a real box**, min strictly below max on every axis.
//! - **Every decoded vertex falls inside its batch's own bounds**, which is the
//!   standing check on the vertex layout and the per-batch scale.
//!
//! # What the sweep found
//!
//! - **Every PSP circuit authors exactly one `Skycube`**, parented to the world
//!   node with its geometry inline (4624 to 8016 bytes), so the sky is authored
//!   per track, not one shared asset.
//! - **`Data\Defaults\Skycube.vex` is a version-4 file** in a version-6 archive
//!   (`0x0ee` world, `0x373` texture, `0x378` shape), holding one
//!   `skycube1_nolightShape` and six `skybx1..6_nomip.tga` textures: a six-faced
//!   unlit skybox nothing in the shipped tracks needs (a legacy fallback).
//! - **Material counts are 1 (12 files), 5 (22) and 6 (6), not face counts**:
//!   geometry is 474-553 triangles whichever. What separates them is open; the 1s
//!   are the Zone variants.
//! - **`fogCube` is 128 bytes on every track that has one.** `06_Track` has none,
//!   so a loader must handle absence; asserted so a name-resolution miss would
//!   show as a moved count.
//! - **The two colour/near/far sets in a `fogCube` are byte-identical on most
//!   circuits but differ on `01_Track`.** What the second set is *for* is left
//!   open here - measured, not guessed.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::fog;
use oag_vex::vex;

/// Class id of `Skycube`, from the class-ID table at `0x08ab2370`.
const CLASS_SKYCUBE: u32 = 0x3c6;

/// Class id of `fogCube`, from the same table.
const CLASS_FOGCUBE: u32 = 0x3d3;

/// A `fogCube` payload is this long on every track that authors one.
const FOGCUBE_LEN: usize = 128;

/// Where a `fogCube`'s parameters start, after the 64-byte 4x4.
const FOG_PARAMS: usize = 0x40;

/// Node payloads are padded to this, which is what makes the material array's
/// end and the geometry offset agree exactly rather than approximately.
const ALIGN: usize = 0x10;

/// Where a mesh-shaped payload's material array starts.
const MATERIALS_AT: usize = 0x30;

/// Stride of one material record.
const MATERIAL_STRIDE: usize = 0x14;

/// Most materials any shipped sky carries. **Not a face count**: a material is a
/// texture run and the geometry is 474-553 triangles whatever the count.
const MAX_MATERIALS: usize = 6;

/// `Skycube` nodes across the 40 PSP Pulse track files.
///
/// Pinned so a change in the tree walk or class id shows as a moved count.
const PSP_SKYCUBES: usize = 40;

/// `fogCube` nodes across the same files. Lower than [`PSP_SKYCUBES`] because
/// some circuits author no fog at all.
const PSP_FOGCUBES: usize = 36;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// One decoded `.vex` file's worth of raw bytes.
struct VexFile {
    label: String,
    bytes: Vec<u8>,
    tree: Vec<vex::Node>,
}

/// Finds an archive by its full path, not by suffix.
///
/// Finds an archive by its full path, not by suffix: `ends_with("Data.wad")` also
/// matches `BEData.wad` and `FEData.wad`, the first of which in disc order holds
/// no tracks (reading as "this disc has no sky").
fn archive_path(disc: &mut DiscImage, name: &str) -> String {
    disc.entries()
        .expect("entries")
        .iter()
        .map(|e| e.path.clone())
        .find(|p| p.as_str() == name || p.ends_with(&format!("/{name}")))
        .unwrap_or_else(|| panic!("{name} not on the disc"))
}

/// Every `.vex` file in one archive, decompressed and walked.
fn vex_files(disc: &mut DiscImage, archive_path: &str) -> Vec<VexFile> {
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == archive_path)
        .unwrap_or_else(|| panic!("{archive_path} present"))
        .clone();

    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let mut out = Vec::new();
    for (index, entry) in dir.entries.iter().enumerate() {
        if entry.size == 0 {
            continue;
        }
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let bytes = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => panic!("{archive_path} entry {index}: unexpected zlib entry"),
        };
        if !vex::has_magic(&bytes) {
            continue;
        }
        // Only the current format version: `Data\Defaults\Skycube.vex` is version 4
        // with older class ids, so `0x3c6` cannot appear in it (it has its own test).
        if vex::version(&bytes) != Ok(6) {
            continue;
        }
        let Ok(tree) = vex::nodes(&bytes) else {
            continue;
        };
        out.push(VexFile {
            label: format!("{archive_path} entry {index}"),
            bytes,
            tree,
        });
    }
    out
}

/// World position of a volume's centre, from its world-to-local matrix.
///
/// Only the translation is inverted, which suffices: the local origin maps to
/// `-t * R^-1`, and the rows are orthogonal up to their own scale, so dividing
/// each dot product by that row's squared length avoids a general inverse.
fn invert_translation(m: &[f32; 16]) -> [f32; 3] {
    let t = [m[12], m[13], m[14]];
    let mut out = [0.0f32; 3];
    for (axis, o) in out.iter_mut().enumerate() {
        let row = [m[axis * 4], m[axis * 4 + 1], m[axis * 4 + 2]];
        let len2 = row[0] * row[0] + row[1] * row[1] + row[2] * row[2];
        if len2 > 0.0 {
            *o = -(t[0] * row[0] + t[1] * row[1] + t[2] * row[2]) / len2;
        }
    }
    out
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn f32_at(bytes: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Rounds up to the node payload alignment.
fn align_up(value: usize) -> usize {
    value.div_ceil(ALIGN) * ALIGN
}

/// The claim, stated as a check: a `Skycube` payload closes as a mesh payload.
///
/// Returns the material count, the per-file figure worth surveying (not the same
/// on every circuit).
fn assert_mesh_shaped(label: &str, payload: &[u8], texture_count: usize) -> usize {
    assert!(
        payload.len() >= MATERIALS_AT,
        "{label}: payload too short to be mesh-shaped"
    );

    // The material count comes from `+0x02`, as a `Mesh` payload's does; bounded
    // and surveyed by the caller (1, 5 or 6, the distinction unestablished). One
    // material per cube face is **not supported by anything in the payload**:
    // triangle counts are 474-553 regardless.
    let material_count = usize::from(u16_at(payload, 2));
    assert!(
        (1..=MAX_MATERIALS).contains(&material_count),
        "{label}: {material_count} materials, outside the surveyed 1..={MAX_MATERIALS}"
    );

    // The geometry offset at `+0x04` sits at or after the material array's end,
    // **not exactly at it**: `06_Track`'s sky puts a 0x1a8-byte block between,
    // holding six 0x40-byte records with stale PSP main-RAM pointers (`0x080db6c0`).
    // Nothing decodes it; it is bounded, stepped over, and left open.
    let materials_end = align_up(MATERIALS_AT + material_count * MATERIAL_STRIDE);
    let geometry_at = u32_at(payload, 4) as usize;
    assert!(
        geometry_at >= materials_end && geometry_at < payload.len(),
        "{label}: geometry offset {geometry_at:#x} is not in \
         {materials_end:#x}..{:#x}",
        payload.len()
    );

    // Every material must name a texture the file has (a wrong stride walks into
    // the geometry and gives far out-of-range indices).
    let materials = vex::mesh_materials(payload);
    assert_eq!(
        materials.len(),
        material_count,
        "{label}: material array length disagrees with the header count"
    );
    for (i, material) in materials.iter().enumerate() {
        let material =
            material.unwrap_or_else(|| panic!("{label}: material {i} runs past the payload"));
        assert!(
            (material.texture as usize) < texture_count,
            "{label}: material {i} cites texture {} of {texture_count}",
            material.texture
        );
    }

    // The bounding box pair at `+0x10` and `+0x20`: a real box, not padding that
    // parses.
    for axis in 0..3 {
        let lo = f32_at(payload, 0x10 + axis * 4);
        let hi = f32_at(payload, 0x20 + axis * 4);
        assert!(
            lo.is_finite() && hi.is_finite() && lo < hi,
            "{label}: bounding box axis {axis} is {lo}..{hi}"
        );
    }

    // The batches decode, and every vertex lands inside the bounds its own
    // batch declares - the standing check on the vertex layout and the scale.
    let batches = vex::mesh_batches(payload, 0).unwrap_or_else(|e| panic!("{label}: batches: {e}"));
    for (i, batch) in batches.iter().enumerate() {
        let (lo, hi) = batch.bounds;
        for vertex in &batch.vertices {
            for axis in 0..3 {
                let p = vertex.position[axis];
                assert!(
                    p >= lo[axis] - 1.0 && p <= hi[axis] + 1.0,
                    "{label}: batch {i} vertex axis {axis} = {p}, outside {}..{}",
                    lo[axis],
                    hi[axis]
                );
            }
        }
        assert!(
            (batch.material_index as usize) < material_count,
            "{label}: batch {i} selects material {} of {material_count}",
            batch.material_index
        );
    }
    assert!(
        !batches.is_empty(),
        "{label}: a sky that decodes to no geometry is a sky that does not draw"
    );
    material_count
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn a_skycube_payload_is_a_mesh_payload() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open disc");
    let data_wad = archive_path(&mut disc, "Data.wad");

    let mut skycubes = 0;
    let mut faces = [0usize; MAX_MATERIALS + 1];
    let mut sizes = Vec::new();

    for file in vex_files(&mut disc, &data_wad) {
        let texture_count = file
            .tree
            .iter()
            .filter(|n| n.class_id == vex::CLASS_TEXTURE)
            .count();

        for node in file.tree.iter().filter(|n| n.class_id == CLASS_SKYCUBE) {
            let label = format!("{} skycube", file.label);
            let payload = &file.bytes[node.payload()];
            faces[assert_mesh_shaped(&label, payload, texture_count)] += 1;
            sizes.push(payload.len());
            skycubes += 1;
        }
    }

    println!(
        "skycubes {skycubes}, payloads {}..{}",
        sizes.iter().min().copied().unwrap_or(0),
        sizes.iter().max().copied().unwrap_or(0),
    );
    for (count, files) in faces.iter().enumerate() {
        if *files > 0 {
            println!("  {count} material(s): {files} file(s)");
        }
    }
    assert_eq!(
        skycubes, PSP_SKYCUBES,
        "the Skycube count moved; the sweep no longer covers what it did"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn every_track_authors_exactly_one_sky() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open disc");
    let data_wad = archive_path(&mut disc, "Data.wad");

    for file in vex_files(&mut disc, &data_wad) {
        let skies: Vec<&vex::Node> = file
            .tree
            .iter()
            .filter(|n| n.class_id == CLASS_SKYCUBE)
            .collect();
        // Non-track files have no sky, hence "never more than one" rather than
        // "always exactly one".
        assert!(
            skies.len() <= 1,
            "{}: {} Skycube nodes; the sky was assumed unique",
            file.label,
            skies.len()
        );
        // The sky hangs off the world node, so a renderer draws it without
        // composing a hierarchy.
        for sky in skies {
            assert_eq!(
                sky.parent,
                Some(0),
                "{}: Skycube is parented to {:?}, not the world node",
                file.label,
                sky.parent
            );
        }
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn a_fogcube_is_a_matrix_and_two_colour_range_sets() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open disc");
    let data_wad = archive_path(&mut disc, "Data.wad");

    let mut fogcubes = 0;
    let mut sets_agree = 0;
    let mut edges = Vec::new();

    for file in vex_files(&mut disc, &data_wad) {
        for node in file.tree.iter().filter(|n| n.class_id == CLASS_FOGCUBE) {
            let payload = &file.bytes[node.payload()];
            assert_eq!(
                payload.len(),
                FOGCUBE_LEN,
                "{}: fogCube payload is not {FOGCUBE_LEN} bytes",
                file.label
            );

            // The 4x4's last row is a position with w = 1.0 (as `Transform`'s),
            // identifying the first 64 bytes as a matrix, not more parameters.
            let w = f32_at(payload, 0x3c);
            assert!(
                (w - 1.0).abs() < 1e-6,
                "{}: fogCube matrix row 3 w = {w}, not 1.0",
                file.label
            );

            // Two sets of {r, g, b, pad, near, far}, six floats each.
            let mut sets = Vec::new();
            for set in 0..2 {
                let at = FOG_PARAMS + set * 0x18;
                let rgb = [
                    f32_at(payload, at),
                    f32_at(payload, at + 4),
                    f32_at(payload, at + 8),
                ];
                let pad = f32_at(payload, at + 12);
                let near = f32_at(payload, at + 16);
                let far = f32_at(payload, at + 20);

                for (axis, c) in rgb.iter().enumerate() {
                    assert!(
                        (0.0..=1.0).contains(c),
                        "{}: fog set {set} channel {axis} = {c}, not a normalised colour",
                        file.label
                    );
                }
                assert_eq!(
                    pad, 0.0,
                    "{}: fog set {set} alpha slot is not zero",
                    file.label
                );
                assert!(
                    near.is_finite() && far.is_finite() && near >= 0.0 && near < far,
                    "{}: fog set {set} range is {near}..{far}",
                    file.label
                );
                sets.push((rgb, near, far));
            }
            if sets[0] == sets[1] {
                sets_agree += 1;
            }

            // The typed decoder must agree with the hand-read offsets and place the
            // two sets at the ends of its gradient. Sampling the box centre has
            // teeth: it exercises the matrix, edge length and interpolation at once,
            // and a volume whose own centre is outside itself has the box wrong.
            let volume = fog::FogVolume::parse(payload)
                .unwrap_or_else(|| panic!("{}: fogCube did not decode", file.label));
            assert_eq!(volume.near_end.colour, sets[0].0, "{}", file.label);
            assert_eq!(volume.near_end.near, sets[0].1, "{}", file.label);
            assert_eq!(volume.far_end.far, sets[1].2, "{}", file.label);
            assert!(
                volume.edge > 0.0 && volume.edge.is_finite(),
                "{}: edge length {}",
                file.label,
                volume.edge
            );
            edges.push(volume.edge);

            // The volume's own centre in world space (what the local origin maps
            // back to); inside by construction.
            let centre = invert_translation(&volume.to_local);
            let at_centre = volume.sample(centre).unwrap_or_else(|| {
                panic!("{}: the volume does not contain its own centre", file.label)
            });
            for (axis, c) in at_centre.colour.iter().enumerate() {
                assert!(
                    (0.0..=1.0).contains(c),
                    "{}: sampled channel {axis} = {c}",
                    file.label
                );
            }
            assert!(
                at_centre.near < at_centre.far,
                "{}: sampled range {}..{}",
                file.label,
                at_centre.near,
                at_centre.far
            );
            fogcubes += 1;
        }
    }

    edges.sort_by(f32::total_cmp);
    println!(
        "fogcubes {fogcubes}, both sets identical on {sets_agree}, edges {:?}..{:?}",
        edges.first(),
        edges.last()
    );
    assert_eq!(
        fogcubes, PSP_FOGCUBES,
        "the fogCube count moved; some circuit gained or lost its fog"
    );
    assert!(
        fogcubes < PSP_SKYCUBES,
        "at least one circuit authors a sky and no fog, which a loader must handle"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn the_defaults_skycube_is_a_legacy_version_four_asset() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open disc");
    let data_wad = archive_path(&mut disc, "Data.wad");

    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == data_wad)
        .expect("Data.wad")
        .clone();
    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let wanted = wad::hash_name(r"Data\Defaults\Skycube.vex");
    let entry = dir
        .entries
        .iter()
        .find(|e| e.name_hash == wanted)
        .expect(r"Data\Defaults\Skycube.vex is in Data.wad");
    let bytes = disc
        .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
        .expect("blob");

    assert!(vex::has_magic(&bytes), "the default sky is a .vex file");
    // Version 4 in a version-6 archive: the whole reason the class ids look wrong
    // (older numbering, texture `0x373`, no `CLASS_SKYCUBE`).
    assert_eq!(
        vex::version(&bytes),
        Ok(4),
        "the default sky is not the legacy version it was recorded as"
    );

    let tree = vex::nodes(&bytes).expect("walk the default sky");
    assert!(
        !tree.iter().any(|n| n.class_id == CLASS_SKYCUBE),
        "a version-4 file cannot hold a version-6 class id"
    );

    // The claim `vex::classes` rests on, checked on a *Pulse* disc file: a
    // version-4 file uses version-4 numbering whichever release shipped it. `0x373`
    // is the `Texture` id `docs/formats/pure-status.md` measured across 156 Pure
    // files, so the two corpora corroborate each other.
    let table = vex::classes::for_version(4).expect("version 4 is a known table");
    assert_eq!(table, vex::classes::V4);
    let textures = tree
        .iter()
        .filter(|n| Some(n.class_id) == table.texture)
        .count();
    assert_eq!(
        textures, 6,
        "the version-4 table's Texture id does not find the six skybox faces"
    );
    for node in &tree {
        assert_ne!(
            Some(node.class_id),
            vex::classes::V6.texture,
            "a version-6 Texture id turned up in a version-4 file"
        );
    }
    // Six faces named for what they are: the class name means a literal cube, not
    // a Maya-export artefact.
    let faces = tree
        .iter()
        .filter_map(|n| n.name.as_deref())
        .filter(|n| n.contains("skybx"))
        .count();
    assert_eq!(faces, 6, "the default sky is not six-faced");
}
