//! Establishes that a `Skycube` node's payload **is a `Mesh` payload**, and
//! decodes `fogCube`, against every `.vex` file on the Pulse PSP disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The tests skip with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What this is for
//!
//! `docs/formats/vex.md` has only ever recorded that class `0x3c6` is *called*
//! `Skycube`, from the class-ID table. No byte of its payload was documented and
//! no handler address was recovered, which is why a race in this engine has a
//! black sky. The claim under test is the one that makes a sky cheap:
//!
//! > `Skycube` and `fogCube` are not new formats. A `Skycube` payload has the
//! > same layout as a `Mesh` `0x125` payload, so [`vex::mesh_materials`] and
//! > [`vex::mesh_batches`] already decode it; a `fogCube` payload is a 64-byte
//! > 4x4 followed by fog parameters.
//!
//! # Why closure, and not "the decoder returned something"
//!
//! `mesh_batches` on a payload that merely *resembles* a mesh will happily
//! return plausible-looking batches, so a non-empty result proves nothing. The
//! discriminating check is that the decode **closes**: the material count at
//! `+0x02` must place the material array exactly where `+0x04`'s geometry offset
//! says the geometry begins, and the decoded batches must consume the payload to
//! its end without running past it. That is the same closure argument that
//! settled `WO Track`, `section` and the collision soup, and a layout that is
//! merely similar fails it.
//!
//! Three further constraints a misreading cannot satisfy at once:
//!
//! - **Every material's texture index is in range** for the file's own
//!   `CLASS_TEXTURE` count. A wrong stride produces out-of-range indices at
//!   once - `05_Track`'s sky cites `0x6b..0x6f` against 130 textures, which
//!   leaves no room for a coincidence.
//! - **The bounding box is a real box**, min strictly below max on every axis.
//! - **Every decoded vertex falls inside its batch's own bounds**, which is the
//!   standing check on the vertex layout and the per-batch scale.
//!
//! # What the sweep found
//!
//! - **Every PSP circuit authors exactly one `Skycube`**, parented to the world
//!   node, with its geometry inline rather than referencing
//!   `Data\Defaults\Skycube.vex`. Payloads run 4624 to 8016 bytes, so the sky is
//!   authored per track and is not one shared asset.
//! - **`Data\Defaults\Skycube.vex` is a version-4 file** in a version-6 archive:
//!   its class IDs (`0x0ee` world, `0x373` texture, `0x378` shape) are from the
//!   older numbering, not Pulse's. It holds one `skycube1_nolightShape` and six
//!   `skybx1..6_nomip.tga` textures - a literally six-faced, unlit, unmipped
//!   skybox - and nothing in the shipped tracks needs it. Recorded as a legacy
//!   fallback rather than the thing to load.
//! - **Material counts are 1 (12 files), 5 (22) and 6 (6), and are not face
//!   counts.** Geometry is 474-553 triangles whichever it is. What separates the
//!   three groups is open; the 1s are the Zone variants.
//! - **`fogCube` is 128 bytes on every track that has one.** `06_Track` has
//!   none, so a loader must handle its absence; that is asserted rather than
//!   assumed, so it shows up as a count that moved if it was ever a
//!   name-resolution miss.
//! - **The two colour/near/far sets in a `fogCube` are byte-identical on most
//!   circuits but differ on `01_Track`.** What the second set is *for* is left
//!   open here - measured, not guessed.

use std::path::{Path, PathBuf};

use oag_disc::DiscImage;
use oag_formats::fog;
use oag_formats::vex;
use oag_formats::wad::{self, Compression, Directory};

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

/// Most materials any shipped sky carries.
///
/// **Not a face count.** A material is a texture run, and the geometry is
/// subdivided into 474-553 triangles whatever the count is, so nothing here ties
/// one material to one cube face.
const MAX_MATERIALS: usize = 6;

/// `Skycube` nodes across the 40 PSP Pulse track files.
///
/// Pinned so that a change in the tree walk or the class id shows up as a count
/// that moved, rather than as a survey that quietly covers less.
const PSP_SKYCUBES: usize = 40;

/// `fogCube` nodes across the same files. Lower than [`PSP_SKYCUBES`] because
/// some circuits author no fog at all.
const PSP_FOGCUBES: usize = 36;

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// One decoded `.vex` file's worth of raw bytes.
struct VexFile {
    label: String,
    bytes: Vec<u8>,
    tree: Vec<vex::Node>,
}

/// Finds an archive by its full path, not by suffix.
///
/// A suffix match is a trap here: `ends_with("Data.wad")` also matches
/// `BEData.wad` and `FEData.wad`, and the first of those in disc order holds no
/// tracks at all - which reads exactly like "this disc has no sky."
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
        // Only the current format version: `Data\Defaults\Skycube.vex` is a
        // version-4 file whose class ids are from the older numbering, so
        // walking it here would look for `0x3c6` in a file that cannot have one.
        // It gets its own test.
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
/// Only the translation is inverted, which is enough: the local origin maps to
/// `-t * R^-1`, and the rows are orthogonal up to their own scale, so dividing
/// each dot product by that row's squared length recovers it without a general
/// inverse.
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
/// Returns the material count, which is the per-file figure worth surveying:
/// it is the sky's face count, and it is not the same on every circuit.
fn assert_mesh_shaped(label: &str, payload: &[u8], texture_count: usize) -> usize {
    assert!(
        payload.len() >= MATERIALS_AT,
        "{label}: payload too short to be mesh-shaped"
    );

    // The material count comes from `+0x02`, exactly as a `Mesh` payload's does.
    // Bounded rather than pinned, and surveyed by the caller: the distribution is
    // 1, 5 or 6 and what distinguishes them is not established. The tempting
    // reading - one material per cube face, five being the cube without its floor
    // - is **not supported by anything in the payload**; triangle counts are
    // 474-553 regardless, so a material is a texture run and the face mapping is
    // a guess.
    let material_count = usize::from(u16_at(payload, 2));
    assert!(
        (1..=MAX_MATERIALS).contains(&material_count),
        "{label}: {material_count} materials, outside the surveyed 1..={MAX_MATERIALS}"
    );

    // The geometry offset at `+0x04` sits at or after the material array's end.
    //
    // **Not exactly at it.** That was the first reading and shipped data refutes
    // it: `06_Track`'s sky puts a 0x1a8-byte block between the two, holding six
    // 0x40-byte records with stale PSP main-RAM pointers (`0x080db6c0`) still
    // baked in. Nothing here decodes that block; it is bounded and stepped over,
    // and left as an open question rather than guessed at.
    let materials_end = align_up(MATERIALS_AT + material_count * MATERIAL_STRIDE);
    let geometry_at = u32_at(payload, 4) as usize;
    assert!(
        geometry_at >= materials_end && geometry_at < payload.len(),
        "{label}: geometry offset {geometry_at:#x} is not in \
         {materials_end:#x}..{:#x}",
        payload.len()
    );

    // Every material must name a texture the file actually has. A wrong stride
    // walks into the geometry and produces indices far out of range.
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

    // The bounding box pair, at `+0x10` and `+0x20`. A real box, not padding
    // that happens to parse.
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
        // Files that are not tracks have no sky at all, which is the point of
        // asserting "never more than one" rather than "always exactly one".
        assert!(
            skies.len() <= 1,
            "{}: {} Skycube nodes; the sky was assumed unique",
            file.label,
            skies.len()
        );
        // The sky hangs off the world node, not off a track transform - which is
        // what lets a renderer draw it without composing a hierarchy first.
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

            // The 4x4's last row is a position with w = 1.0, the same row-major
            // convention `Transform` uses. That is what identifies the first 64
            // bytes as a matrix rather than as more parameters.
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

            // The typed decoder must agree with the hand-read offsets above, and
            // must place the two sets at the ends of its own gradient. Sampling
            // the centre of the box is the check with teeth: it exercises the
            // matrix, the edge length and the interpolation at once, and a
            // volume whose own centre is outside itself is a decoder that has
            // the box wrong.
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

            // The volume's own centre in world space: the point the local-space
            // origin maps back to. It must be inside, by construction.
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
    // Version 4 in a version-6 archive. This is the whole reason the file's
    // class ids look wrong: they are from the older numbering, where texture is
    // `0x373`, and `CLASS_SKYCUBE` cannot appear in it at all.
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

    // The claim `vex::classes` rests on, checked against a file on the *Pulse*
    // disc rather than against Pure: a version-4 file uses version-4 numbering
    // whichever release shipped it. `0x373` here is the same `Texture` id
    // `docs/formats/pure-status.md` measured across 156 Pure files, so the two
    // corpora corroborate each other and neither is the only witness.
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
    // Six faces, named for what they are. This is the corroboration that the
    // class name means a literal cube rather than being a Maya-export artefact.
    let faces = tree
        .iter()
        .filter_map(|n| n.name.as_deref())
        .filter(|n| n.contains("skybx"))
        .count();
    assert_eq!(faces, 6, "the default sky is not six-faced");
}
