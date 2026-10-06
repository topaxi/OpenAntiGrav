//! Validates the PS2 mesh decoder against every `.vex` file on the PS2 disc.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What this is for
//!
//! PS2 batches do not hold an interleaved vertex array. They hold a **VIF
//! packet**: one `UNPACK` command per attribute, at a fixed VU1 address, ending
//! in an `MSCNT` that runs the microprogram. Nothing about that is guessable from
//! the batch header, so the reading is established the way the rest of `.vex`
//! was - by arithmetic that cannot come out even unless it is right:
//!
//! - The **three framing lengths close exactly**: the region header's declared
//!   size plus its own 16 bytes is the batch's payload size, the DMA tag's
//!   quadword count spans the packet, and the command walk lands exactly on the
//!   end. A wrong command length desynchronises and fails one of the three.
//! - Every decoded position falls **inside the batch's own bounding box**, with
//!   no slack at all: the box is `f32` in the same space as the positions, so a
//!   wrong attribute address or a wrong element width leaves it immediately.
//! - The **vertex counts reconcile** with the batch header: a triangle list
//!   unpacks exactly as many vertices as the header declares, and a strip
//!   unpacks two extra per chunk boundary, which is how the hardware continues a
//!   strip across a draw. The decoder additionally refuses a split strip whose
//!   non-final chunk has an odd vertex count, because concatenating the chunks
//!   is only winding-correct while they are even; this run passing is the
//!   evidence that none exists.
//! - Where normals are present they are **unit length**, which is not something
//!   an accidentally-correct address would produce.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::vex::{self, Batch};

/// The PS2 archives that hold models.
const PS2_ARCHIVES: [&str; 2] = ["54748/WADS2.WAD", "54748/WADSP.WAD"];

/// Fewer batches than this means the walk stopped finding models.
const MIN_BATCHES: usize = 2000;

/// Fewer vertices than this, likewise.
const MIN_VERTICES: usize = 150_000;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

#[derive(Default)]
struct Survey {
    models: usize,
    batches: usize,
    vertices: usize,
    strips: usize,
    lists: usize,
    stitched: usize,
    types: BTreeMap<u16, usize>,
    /// PSP-format batches, which the PS2 disc also carries.
    psp_batches: usize,
    attributes: BTreeSet<&'static str>,
    worst_normal_error: f32,
}

/// Checks one batch and folds it into the survey.
fn check(label: &str, index: usize, batch: &Batch, into: &mut Survey) {
    let header_count = usize::from(batch.declared_vertex_count);
    *into.types.entry(batch.vertex_type).or_default() += 1;
    into.batches += 1;
    into.vertices += batch.vertices.len();

    let (lo, hi) = batch.bounds;
    for vertex in &batch.vertices {
        for axis in 0..3 {
            // No slack. The box and the positions are the same `f32` values in
            // the same space, so anything outside it is a decode error rather
            // than a rounding one.
            assert!(
                vertex.position[axis] >= lo[axis] && vertex.position[axis] <= hi[axis],
                "{label} entry {index}: vertex type {:#06x} axis {axis}: {} outside [{}, {}]",
                batch.vertex_type,
                vertex.position[axis],
                lo[axis],
                hi[axis]
            );
        }
        if let Some(normal) = vertex.normal {
            let length =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            into.worst_normal_error = into.worst_normal_error.max((length - 1.0).abs());
        }
        if vertex.normal.is_some() {
            into.attributes.insert("normal");
        }
        if vertex.texcoord.is_some() {
            into.attributes.insert("texcoord");
        }
        if vertex.colour.is_some() {
            into.attributes.insert("colour");
        }
    }

    // The reconciliation, which is what says the chunking was read correctly.
    let decoded = batch.vertices.len();
    match batch.primitive_type {
        vex::PRIM_TRIANGLES => {
            into.lists += 1;
            assert_eq!(
                decoded, header_count,
                "{label} entry {index}: a list unpacked {decoded} vertices for a header count of \
                 {header_count}"
            );
            assert_eq!(
                decoded % 3,
                0,
                "{label} entry {index}: a triangle list of {decoded} vertices"
            );
        }
        vex::PRIM_TRIANGLE_STRIP => {
            into.strips += 1;
            let extra = decoded
                .checked_sub(header_count)
                .unwrap_or_else(|| panic!("{label} entry {index}: strip lost vertices"));
            assert_eq!(
                extra % 2,
                0,
                "{label} entry {index}: a strip split repeats two vertices per boundary, not \
                 {extra} in total"
            );
            if extra > 0 {
                into.stitched += 1;
            }
        }
        other => panic!("{label} entry {index}: unexpected primitive type {other}"),
    }
}

/// Decodes every `.vex` file in one PS2 archive.
fn survey(disc: &mut DiscImage, archive_path: &str, into: &mut Survey) {
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

    for (index, entry) in dir.entries.iter().enumerate() {
        if entry.size == 0 {
            continue;
        }
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let model = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => panic!("{archive_path} entry {index}: unexpected zlib entry"),
        };
        if !vex::has_magic(&model) {
            continue;
        }
        let Ok(tree) = vex::nodes(&model) else {
            continue;
        };
        let mut had_mesh = false;

        for node in tree.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
            let payload = &model[node.payload()];
            for list in 0..2 {
                let batches = vex::mesh_batches(payload, list).unwrap_or_else(|e| {
                    panic!("{archive_path} entry {index}: decoding batch list {list}: {e}")
                });
                for batch in &batches {
                    // The PS2 disc carries PSP-format models too, so the batch's
                    // own vertex type is what selects the decoder, not the disc.
                    if !vex::is_vif_batch(batch.vertex_type) {
                        into.psp_batches += 1;
                        continue;
                    }
                    had_mesh = true;
                    check(archive_path, index, batch, into);
                }
            }
        }
        if had_mesh {
            into.models += 1;
        }
    }
}

/// Every PS2 mesh in the game decodes, and every invariant the reading rests on
/// holds.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_ps2_mesh_decodes_and_stays_inside_its_bounds() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let mut found = Survey::default();
    for archive in PS2_ARCHIVES {
        survey(&mut disc, archive, &mut found);
    }

    println!(
        "{} models, {} batches ({} strips, {} of them split, {} lists), {} vertices",
        found.models, found.batches, found.strips, found.stitched, found.lists, found.vertices
    );
    println!("  vertex types: {:#06x?}", found.types);
    println!("  PSP-format batches alongside them: {}", found.psp_batches);
    println!("  attributes decoded: {:?}", found.attributes);
    println!("  worst normal length error: {}", found.worst_normal_error);

    assert!(
        found.batches >= MIN_BATCHES,
        "only {} batches; the walk stopped finding models",
        found.batches
    );
    assert!(
        found.vertices >= MIN_VERTICES,
        "only {} vertices",
        found.vertices
    );
    assert!(
        found.stitched > 0,
        "no split strip anywhere means the two-vertex stitch was never exercised"
    );
    assert!(
        found.worst_normal_error < 1e-3,
        "normals are not unit length: worst error {}",
        found.worst_normal_error
    );
    assert!(
        found.types.len() > 1,
        "one vertex type is not enough to exercise the attribute table"
    );
    for &vertex_type in found.types.keys() {
        assert_eq!(
            vertex_type & vex::POSITION_BITS,
            vex::POSITION_F32,
            "vertex type {vertex_type:#06x} does not declare float positions"
        );
    }
}
