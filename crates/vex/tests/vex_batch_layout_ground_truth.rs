//! Establishes that [`vex::VertexLayout`] reads a PSP batch's vertex array at
//! the right stride, and that the GU vertex type at `+0x0a` means what it says -
//! against every `.vex` file on the Pulse PSP disc.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! The tests skip with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What this is for
//!
//! The boost plume's two short batches decode to a **single** texture
//! coordinate - every one of their 9 or 10 vertices carries the same `u`,`v`
//! pair - which reads exactly like a stride or vertex-format misread of the kind
//! `texture_stride_ground_truth.rs` caught in [`vex::textures`]. It is not one,
//! and these are the two closure arguments that settle it, both measured over
//! the whole disc rather than over the one model that raised the question. See
//! `docs/formats/vex.md`, "The vertex type is not misdeclared, and `u8`
//! texcoords really are one byte each".
//!
//! 1. **The stride closes.** A batch declares its vertex count at `+0x04` and
//!    its payload size at `+0x0c`. `count * stride`, rounded up to 16, has to
//!    reproduce the declared payload size - on every batch, with the stride
//!    derived only from the vertex type. A wrong stride desynchronises that
//!    immediately, and it cannot come out right by coincidence across tens of
//!    thousands of batches at eleven different strides.
//!
//! 2. **No batch anywhere declares 16-bit texcoords.** Bits 0-1 of the vertex
//!    type are the GU texcoord format, and `2` (`GU_TEXTURE_16BIT`) never
//!    appears on the disc. That matters because the only competing reading of a
//!    `0x13d` vertex is "bytes 0-3 are two `u16` texcoords, not one `u8` pair
//!    plus two bytes of padding" - which needs a format the exporter never
//!    emits, and which [`vex::VertexLayout::from_vertex_type`] refuses outright.
//!
//! The second test kills that competing reading directly, on the bytes rather
//! than on the declaration: in a `0x13d` vertex the two bytes at `+2` carry the
//! **same value as the unambiguous padding** at `+11` and `+18`/`+19`, in every
//! vertex on the disc, and that value is only ever `0x00` or `0xff`. Two bytes
//! that track the padding byte-for-byte across three quarters of a million
//! vertices are padding. A `v` texcoord that is binary on every ship hull and
//! every circuit is not a texture coordinate at all.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::vex::{self, VertexLayout};

/// The archive holding every circuit and every ship.
const PSP_DATA: &str = "PSP_GAME/USRDIR/Data.wad";

/// Smallest number of batches the sweep must reach before its agreement means
/// anything. Well under the real figure (65,279 at the time of writing); a count
/// that collapses is a broken walk, not a disc with fewer models on it.
const MIN_BATCHES: usize = 50_000;

/// Smallest number of `0x13d` vertices the padding sweep must reach. Real figure
/// is 744,686.
const MIN_PADDED_VERTICES: usize = 500_000;

/// The one PSP vertex type this file's padding argument is about: `u8` texcoord,
/// `ABGR8888` colour, `s8` normal, three `s16` positions, stride 20. It is what
/// `shipboost.vex` uses, and the most common type on the disc.
const VTYPE_TEXTURED_LIT: u16 = 0x013d;

/// Byte offsets inside a [`VTYPE_TEXTURED_LIT`] vertex that no field occupies:
/// after the two texcoord bytes, after the three normal bytes, and after the six
/// position bytes.
const PAD_OFFSETS: [usize; 5] = [2, 3, 11, 18, 19];

/// A batch header is 0x40 bytes, or 0x80 when `+0x03` has `0x40` set.
const HEADER_SMALL: usize = 0x40;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// Every version-6 `.vex` blob in `Data.wad`, decompressed, with its entry index.
fn psp_vex_blobs(disc: &mut DiscImage) -> Vec<(usize, Vec<u8>)> {
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == PSP_DATA)
        .unwrap_or_else(|| panic!("{PSP_DATA} present"))
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
            Compression::Zlib => continue,
        };
        if !vex::has_magic(&bytes) || vex::version(&bytes) != Ok(6) {
            continue;
        }
        out.push((index, bytes));
    }
    out
}

/// One batch header, reduced to the numbers this file is about.
struct BatchHeader {
    label: String,
    vertex_type: u16,
    /// Declared vertex count, `+0x04` or `+0x06` for an alternate block.
    count: usize,
    /// Declared vertex-data size, `+0x0c`.
    payload_size: usize,
    /// Where the vertex array starts in the mesh payload.
    data_at: usize,
}

/// Walks both batch lists of every `Mesh` node, the same way
/// [`vex::mesh_batches`] does, and returns what each batch header declares.
fn batch_headers(blobs: &[(usize, Vec<u8>)]) -> Vec<(BatchHeader, Vec<u8>)> {
    let mut out = Vec::new();
    for (index, bytes) in blobs {
        let Ok(nodes) = vex::nodes(bytes) else {
            continue;
        };
        for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
            let range = node.payload();
            if range.end > bytes.len() {
                continue;
            }
            let payload = &bytes[range];
            if payload.len() < 0x30 {
                continue;
            }
            let u16_at = |at: usize| u16::from_le_bytes([payload[at], payload[at + 1]]);
            let u32_at = |at: usize| {
                u32::from_le_bytes([
                    payload[at],
                    payload[at + 1],
                    payload[at + 2],
                    payload[at + 3],
                ])
            };

            for list in 0..2u8 {
                let terminator = if list == 0 { 1u16 } else { 2 };
                let mut at = u32_at(if list == 0 { 4 } else { 8 }) as usize;

                while at + HEADER_SMALL <= payload.len() {
                    if u16_at(at) & terminator == 0 {
                        break;
                    }
                    let header_size = if payload[at + 3] & 0x40 != 0 {
                        0x80
                    } else {
                        HEADER_SMALL
                    };
                    let alternate = u16_at(at + 6) != 0;
                    let count = usize::from(u16_at(at + if alternate { 6 } else { 4 }));
                    let vertex_type = u16_at(at + 0x0a);
                    let payload_size = usize::from(u16_at(at + 0x0c));
                    let alternate_offset = usize::from(u16_at(at + 0x0e));
                    let data_at = at + header_size + if alternate { alternate_offset } else { 0 };

                    let vertices = VertexLayout::from_vertex_type(vertex_type)
                        .ok()
                        .map(|layout| {
                            let end = (data_at + count * layout.stride).min(payload.len());
                            payload[data_at.min(end)..end].to_vec()
                        })
                        .unwrap_or_default();

                    out.push((
                        BatchHeader {
                            label: format!(
                                "entry {index} {} list {list} batch at {at:#x}",
                                node.name.as_deref().unwrap_or("?")
                            ),
                            vertex_type,
                            count,
                            payload_size,
                            data_at,
                        },
                        vertices,
                    ));

                    let step = header_size + payload_size;
                    if step == 0 {
                        break;
                    }
                    at += step;
                }
            }
        }
    }
    out
}

/// The claim: the stride derived from the vertex type reproduces every declared
/// payload size, and no batch declares a texcoord format the layout refuses.
///
/// This is what rules out "our stride is wrong" as the explanation for the boost
/// plume's single-point texture coordinate. It is arithmetic over a field the
/// decoder does not otherwise consult, so a wrong stride cannot satisfy it.
#[test]
#[ignore = "needs a disc image under data/images"]
fn every_batch_payload_closes_on_the_declared_vertex_stride() {
    let Some(path) = image() else { return };
    let mut disc = DiscImage::open(&path).expect("opening the disc image");
    let blobs = psp_vex_blobs(&mut disc);
    let batches = batch_headers(&blobs);

    assert!(
        batches.len() >= MIN_BATCHES,
        "walked only {} batch(es) across {} model(s); the sweep is broken, \
         not the disc",
        batches.len(),
        blobs.len()
    );

    let mut mismatched = Vec::new();
    let mut sixteen_bit = Vec::new();
    let mut unsupported = Vec::new();

    for (batch, _) in &batches {
        if batch.vertex_type & 3 == 2 {
            sixteen_bit.push(batch.label.clone());
        }
        let Ok(layout) = VertexLayout::from_vertex_type(batch.vertex_type) else {
            unsupported.push(format!("{} vtype {:#06x}", batch.label, batch.vertex_type));
            continue;
        };
        let used = batch.count * layout.stride;
        if used.next_multiple_of(16) != batch.payload_size {
            mismatched.push(format!(
                "{}: vtype {:#06x}, {} vertices at stride {} is {used} bytes, \
                 rounded to {}, but the header declares {}",
                batch.label,
                batch.vertex_type,
                batch.count,
                layout.stride,
                used.next_multiple_of(16),
                batch.payload_size,
            ));
        }
    }

    println!(
        "{} batch(es) across {} model(s); {} closed on count * stride rounded to 16",
        batches.len(),
        blobs.len(),
        batches.len() - mismatched.len()
    );

    assert!(
        unsupported.is_empty(),
        "{} batch(es) declare a vertex type VertexLayout refuses, e.g.\n  {}",
        unsupported.len(),
        unsupported[..unsupported.len().min(5)].join("\n  ")
    );
    assert!(
        mismatched.is_empty(),
        "{} of {} batch(es) do not close on the declared stride, e.g.\n  {}",
        mismatched.len(),
        batches.len(),
        mismatched[..mismatched.len().min(5)].join("\n  ")
    );
    assert!(
        sixteen_bit.is_empty(),
        "{} batch(es) declare GU_TEXTURE_16BIT, e.g. {} - the reading in \
         docs/formats/vex.md that no PSP batch does needs revisiting, and so \
         does VertexLayout::from_vertex_type's refusal of that case",
        sixteen_bit.len(),
        sixteen_bit[0],
    );
}

/// The claim: the two bytes at `+2` of a `0x13d` vertex are padding, not a
/// 16-bit `v` texcoord.
///
/// They carry the same value as the padding at `+11` and `+18`/`+19` - which no
/// reading of the format puts a field in - in every vertex on the disc, and that
/// value is only ever `0x00` or `0xff`. This is the exporter filling a vertex
/// before writing its fields, and it is why `u8` texcoords can be read as one
/// byte each with no ambiguity about what the following two bytes are.
#[test]
#[ignore = "needs a disc image under data/images"]
fn a_textured_lit_vertexs_spare_bytes_track_its_padding() {
    let Some(path) = image() else { return };
    let mut disc = DiscImage::open(&path).expect("opening the disc image");
    let blobs = psp_vex_blobs(&mut disc);
    let batches = batch_headers(&blobs);

    let layout = VertexLayout::from_vertex_type(VTYPE_TEXTURED_LIT).expect("layout");
    assert_eq!(layout.stride, 20, "the padding offsets assume stride 20");

    let mut seen = 0usize;
    let mut fills = [0usize; 256];
    let mut unequal = Vec::new();

    for (batch, data) in &batches {
        if batch.vertex_type != VTYPE_TEXTURED_LIT {
            continue;
        }
        for (i, vertex) in data.chunks_exact(layout.stride).enumerate() {
            seen += 1;
            let fill = vertex[PAD_OFFSETS[0]];
            fills[usize::from(fill)] += 1;
            if PAD_OFFSETS.iter().any(|&o| vertex[o] != fill) {
                unequal.push(format!(
                    "{} vertex {i} at {:#x}: {:?}",
                    batch.label,
                    batch.data_at,
                    PAD_OFFSETS.map(|o| vertex[o]),
                ));
            }
        }
    }

    let distinct: Vec<_> = fills
        .iter()
        .enumerate()
        .filter(|&(_, &n)| n > 0)
        .map(|(v, &n)| (v as u8, n))
        .collect();
    println!("{seen} vertices of type {VTYPE_TEXTURED_LIT:#06x}; fills seen: {distinct:?}");

    assert!(
        seen >= MIN_PADDED_VERTICES,
        "only {seen} vertices of type {VTYPE_TEXTURED_LIT:#06x}; the sweep is broken"
    );
    assert!(
        unequal.is_empty(),
        "{} of {seen} vertices have unequal spare bytes at {PAD_OFFSETS:?}, e.g.\n  {}",
        unequal.len(),
        unequal[..unequal.len().min(5)].join("\n  ")
    );
    assert_eq!(
        distinct.iter().map(|&(v, _)| v).collect::<Vec<_>>(),
        vec![0x00, 0xff],
        "the spare bytes take a value other than 0x00 or 0xff, so they are not \
         a uniform pre-write fill and the argument in this test's doc comment \
         no longer holds"
    );
}
