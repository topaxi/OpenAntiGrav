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

/// **A cell's set is spatially coherent with the chunk index, per chunk.**
///
/// The near/far average that first settled the index mapping is a blunt
/// instrument: it cannot tell a correct mapping from one permuted *locally*,
/// because a locally-permuted bit still lands on a chunk in roughly the same
/// place. This is the sharp version, and it looks for the one signature a
/// permutation must leave - **a chunk visible only from far away**.
///
/// For each chunk, take the 20 cells nearest it. If none of them sees it, ask
/// where it *is* seen from. A mis-mapped bit shows up as a chunk drawn only by
/// cells hundreds of units away while every cell beside it hides it. Measured:
/// **26 of Talon's Junction's 983 chunks and 4 of Anulpha Pass's 1,125**, so
/// 2.6% and 0.4%.
///
/// **What this test also records is that the disc genuinely hides large
/// scenery up close.** 71% and 78% of chunks are seen by all twenty of their
/// nearest cells, but the remainder are not, and following them shows an
/// ordinary profile rather than a broken one: a landmark visible from 285
/// cells at a median 957 units and from none of the twenty beside it is a
/// building you are standing under. So **"it was there and then I got closer
/// and it went"** is authored behaviour on this circuit, not evidence of a
/// bug - which is worth pinning, because it is the first thing a reader will
/// suspect.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_chunk_is_never_visible_only_from_far_away() {
    if image().is_none() {
        return;
    }
    // Chunk centres cost a full geometry walk, so this runs on the two
    // circuits the mapping was settled against rather than all 28.
    for want in [
        "/data/environments/talons_junction/track.pvs",
        "/data/environments/15_anulpha_pass/track.pvs",
    ] {
        let file = every_pvs()
            .into_iter()
            .find(|f| f.path == want)
            .unwrap_or_else(|| panic!("{want} is on the disc"));
        let centres = chunk_centres(&file);
        let cells: Vec<[f32; 3]> = (0..file.pvs.cells())
            .filter_map(|c| file.pvs.position(c))
            .collect();
        let mut far_only = 0usize;
        let mut readable = 0usize;
        for (chunk, centre) in centres.iter().enumerate() {
            let Some(centre) = centre else { continue };
            readable += 1;
            let mut order: Vec<usize> = (0..cells.len()).collect();
            order.sort_by(|&a, &b| {
                distance(cells[a], *centre)
                    .partial_cmp(&distance(cells[b], *centre))
                    .unwrap()
            });
            if order.iter().take(20).any(|&c| file.pvs.visible(c, chunk)) {
                continue;
            }
            let nearest_seeing = (0..cells.len())
                .filter(|&c| file.pvs.visible(c, chunk))
                .map(|c| distance(cells[c], *centre))
                .fold(f32::INFINITY, f32::min);
            // Infinite means no cell draws it at all - interior or dead
            // geometry, which is a different thing and not counted here.
            if nearest_seeing.is_finite() && nearest_seeing > 300.0 {
                far_only += 1;
            }
        }
        let share = far_only as f64 / readable as f64;
        let percent = share * 100.0;
        assert!(
            share < 0.05,
            "{want}: {far_only} of {readable} chunk(s) ({percent:.2}%) are drawn only from \
             beyond 300 units while every cell beside them hides them - the signature of a \
             chunk index that does not mean what this parser thinks it means"
        );
    }
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Each chunk's own bounding-box centre, or `None` for one whose vertex stride
/// cannot be recovered.
fn chunk_centres(file: &Shipped) -> Vec<Option<[f32; 3]>> {
    let image = image().expect("checked by the caller");
    let archive = ARCHIVES
        .iter()
        .find(|a| {
            let spec = format!("{}:PS3_GAME/USRDIR/{a}.PSARC", image.display());
            oag_assets::psarc::Archive::open(&spec)
                .map(|p| p.paths().contains(&file.path))
                .unwrap_or(false)
        })
        .expect("the archive holding it");
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    let blob = open
        .read_path(&file.path.replace(".pvs", ".rcsmodel"))
        .expect("the sibling model reads");
    let model = rcsmodel::Model::parse(&blob).expect("it parses");
    model
        .meshes
        .iter()
        .map(|chunk| {
            let stride = chunk
                .declared_stride()
                .or_else(|| chunk.solve_stride_without_a_box(&blob))?;
            let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
            for submesh in &chunk.submeshes {
                if let Ok(points) = chunk.positions(&blob, submesh, stride) {
                    for p in points {
                        for i in 0..3 {
                            min[i] = min[i].min(p[i]);
                            max[i] = max[i].max(p[i]);
                        }
                    }
                }
            }
            (min[0] <= max[0]).then(|| std::array::from_fn(|i| (min[i] + max[i]) / 2.0))
        })
        .collect()
}
