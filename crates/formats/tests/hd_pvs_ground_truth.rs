//! What the disc says about `track.pvs`, over every one of the 28 shipped
//! files rather than the two the parser was written against.
//!
//! The three claims `oag_formats::hd_pvs`' module docs rest on, made
//! executable: the declared chunk count is the sibling `.rcsmodel`'s, the
//! bitmap is `ceil(chunks / 8)` bytes, and a cell's bits fall inside it.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

mod rcsmodel_common;

use oag_formats::{hd_pvs, rcsmodel};
use rcsmodel_common::image;

/// Every archive on the disc that holds a `.pvs`.
const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// How many `.pvs` files the EU disc ships, so a run that silently found none
/// - a renamed archive, a reader change - fails rather than passing empty.
const SHIPPED: usize = 28;

/// One file: its path, its parsed partition, and the sibling model's chunk
/// count.
struct Shipped {
    path: String,
    pvs: hd_pvs::Pvs,
    model_chunks: usize,
    /// The raw bytes, for the padding-bit assertion the parser does not make.
    blob: Vec<u8>,
}

fn every_pvs() -> Vec<Shipped> {
    let image = image().expect("checked by the caller");
    let mut out = Vec::new();
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".pvs"))
            .cloned()
            .collect();
        for path in paths {
            let blob = open.read_path(&path).expect("the .pvs reads");
            let pvs = hd_pvs::Pvs::parse(&blob)
                .unwrap_or_else(|e| panic!("{path}: every shipped .pvs parses, but {e}"));
            let model_path = path.replace(".pvs", ".rcsmodel");
            let model_chunks = open
                .read_path(&model_path)
                .ok()
                .and_then(|b| rcsmodel::Model::parse(&b).ok())
                .unwrap_or_else(|| panic!("{model_path}: the sibling model reads"))
                .meshes
                .len();
            out.push(Shipped {
                path,
                pvs,
                model_chunks,
                blob,
            });
        }
    }
    out
}

/// **The header's second word is the sibling `.rcsmodel`'s chunk count.**
///
/// The claim the whole mapping rests on: without it the bitmap's width is a
/// guess and bit `k` addresses nothing in particular.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn every_declared_chunk_count_is_the_sibling_models() {
    if image().is_none() {
        return;
    }
    let shipped = every_pvs();
    assert_eq!(
        shipped.len(),
        SHIPPED,
        "the EU disc ships {SHIPPED} .pvs files"
    );
    for file in &shipped {
        assert_eq!(
            file.pvs.chunks(),
            file.model_chunks,
            "{}: declares {} chunk(s), the model beside it has {}",
            file.path,
            file.pvs.chunks(),
            file.model_chunks,
        );
        assert!(file.pvs.cells() > 0, "{}: no cells", file.path);
    }
}

/// **A cell's bitmap is `ceil(chunks / 8)` bytes, and the disc proves it by
/// what it leaves out.**
///
/// The last byte of every cell's map carries only the bits the declared chunk
/// count leaves valid. On Talon's Junction that is 621 bytes all below `0x80`,
/// which chance would produce with probability `2^-621`; a reader that had the
/// width wrong would see the next cell's leading bits there instead.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn no_bitmap_sets_a_bit_past_its_own_chunk_count() {
    if image().is_none() {
        return;
    }
    for file in &every_pvs() {
        let spare = file.pvs.chunks() % 8;
        if spare == 0 {
            continue;
        }
        let mask = !((1u8 << spare) - 1);
        for cell in 0..file.pvs.cells() {
            let bits = file.pvs.cell_bits(cell).expect("a declared cell");
            let last = *bits.last().expect("a non-empty map");
            assert_eq!(
                last & mask,
                0,
                "{}: cell {cell}'s last byte is {last:#04x}, which sets a bit past chunk {}",
                file.path,
                file.pvs.chunks(),
            );
        }
    }
}

/// **The file is at least its own declared table, and any surplus is
/// reported.**
///
/// 16 of the 28 are exactly the table; the other 12 carry a further section
/// the retail loader provably never reaches.
/// Asserted so the split is a measured fact in the suite rather than a
/// sentence in a doc comment.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_trailing_section_is_reported_rather_than_parsed() {
    if image().is_none() {
        return;
    }
    let shipped = every_pvs();
    let exact = shipped.iter().filter(|f| f.pvs.trailing() == 0).count();
    assert_eq!(
        exact, 16,
        "16 of the disc's {SHIPPED} .pvs files are exactly their own table"
    );
    let mut trailing: Vec<usize> = shipped
        .iter()
        .map(|f| f.pvs.trailing())
        .filter(|n| *n > 0)
        .collect();
    trailing.sort_unstable();
    println!("trailing byte counts: {trailing:?}");
    for file in &shipped {
        let table = 0x10 + file.pvs.cells() * 16 + file.pvs.cells() * file.pvs.bitmap_bytes();
        assert_eq!(
            file.blob.len(),
            table + file.pvs.trailing(),
            "{}: the reported trailing size accounts for the whole file",
            file.path,
        );
    }
}

/// **A cell's set is a fraction of the circuit, which is the whole point.**
///
/// Around a third on the two circuits measured by hand; asserted loosely
/// across all 28, because a partition that let almost everything through would
/// be a partition this reader had misunderstood rather than one the artists
/// authored.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_cell_sees_a_fraction_of_the_circuit() {
    if image().is_none() {
        return;
    }
    for file in &every_pvs() {
        let total: u32 = (0..file.pvs.cells())
            .map(|cell| file.pvs.visible_count(cell))
            .sum();
        let mean = f64::from(total) / (file.pvs.cells() * file.pvs.chunks()) as f64;
        assert!(
            (0.02..0.85).contains(&mean),
            "{}: cells see {:.1}% of the circuit on average",
            file.path,
            mean * 100.0,
        );
    }
}
