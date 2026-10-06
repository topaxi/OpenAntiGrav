//! Byte coverage inside a `.vex` payload, one level past what
//! `oag_vex::vex::coverage` already claims whole: `Mesh`/`Skycube`/pads (via
//! [`oag_vex::mesh_coverage`]) and `WO Track`/`section`/collision (via
//! [`oag_vex::track_coverage`]).
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # Why one level past the node walk
//!
//! `vex::coverage` claims a node's whole payload once it is reached at all,
//! which is the right scope for "does the tree walk visit every byte" and the
//! wrong scope for "does the *decoder* for this payload class". A batch list
//! reached by an offset at `+0x04`/`+0x08`, a material array sized by a count
//! at `+0x02`, and a texture-transform block array reached by a per-material
//! flag are exactly the "container of tables reached by offsets" shape
//! `docs/formats/README.md#coverage` names as the highest-value place to add
//! this next, after `.rcsmodel` lost a quarter of HD's geometry to one such
//! table going unread.
//!
//! # What each sweep found
//!
//! See `docs/formats/skycube.md#the-extra-block` for `06_Track`'s own gap -
//! the one non-trivial result here - and `docs/formats/README.md#coverage`
//! for the per-format numbers this test's own `println!`s reproduce.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::ByteOrder;
use oag_formats::coverage::Coverage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::vex::{
    self, CLASS_MESH, CLASS_SECTION, CLASS_SKYCUBE, CLASS_SPEEDUP_PAD, CLASS_WEAPON_PAD,
    CLASS_WO_TRACK,
};
use oag_vex::{collision, mesh_coverage, track_coverage};

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Every `.vex` file (version 6 only) in one WAD archive, decompressed.
fn vex_files(disc: &mut DiscImage, archive_path: &str) -> Vec<Vec<u8>> {
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
    for entry in &dir.entries {
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
        out.push(bytes);
    }
    out
}

fn archive_path(disc: &mut DiscImage, name: &str) -> String {
    disc.entries()
        .expect("entries")
        .iter()
        .map(|e| e.path.clone())
        .find(|p| p.as_str() == name || p.ends_with(&format!("/{name}")))
        .unwrap_or_else(|| panic!("{name} not on the disc"))
}

/// One archive's worth of `.vex` files, from a named disc image.
fn corpus(image_name: &str, archive_name: &str) -> Vec<Vec<u8>> {
    let Some(path) = image(image_name) else {
        return Vec::new();
    };
    let mut disc = DiscImage::open(&path).expect("open disc");
    let archive = archive_path(&mut disc, archive_name);
    vex_files(&mut disc, &archive)
}

/// Sums coverage over every node of a given class across a corpus, and
/// returns `(files touched, node count, total bytes, claimed bytes, worst
/// gap description)`.
fn sweep_class(
    files: &[Vec<u8>],
    class_id: u32,
    mut per_payload: impl FnMut(&[u8]) -> Coverage,
) -> (usize, usize, u64, u64, Option<String>) {
    let (mut touched, mut nodes, mut total, mut claimed) = (0usize, 0usize, 0u64, 0u64);
    let mut worst: Option<(usize, String)> = None;
    for bytes in files {
        let Ok(tree) = vex::nodes(bytes) else {
            continue;
        };
        let mut any = false;
        for node in tree.iter().filter(|n| n.class_id == class_id) {
            let range = node.payload();
            let Some(payload) = bytes.get(range) else {
                continue;
            };
            let seen = per_payload(payload);
            if seen.is_empty() {
                continue;
            }
            any = true;
            nodes += 1;
            total += seen.len() as u64;
            claimed += seen.claimed() as u64;
            let gap_len: usize = seen.gaps(1).iter().map(|g| g.len).sum();
            if gap_len > 0 && worst.as_ref().is_none_or(|(n, _)| gap_len > *n) {
                worst = Some((gap_len, seen.describe(1)));
            }
        }
        if any {
            touched += 1;
        }
    }
    (touched, nodes, total, claimed, worst.map(|(_, d)| d))
}

/// **`Mesh` (`0x125`) and the two pad classes, which share its payload
/// layout exactly.**
///
/// Reported rather than asserted to a hard floor for now: batches whose
/// `payload_size` disagrees with what `mesh_batches` itself would decode (a
/// PS2 VIF batch that fails one of its own three framing checks, see
/// `vex.rs`) fall out of this walk the same way they fall out of that one,
/// which is a *decoder* gap and not this instrument's to hide.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn mesh_and_pad_payloads_close_on_the_psp_disc() {
    let files = corpus("pulse-psp-usa.chd", "Data.wad");
    if files.is_empty() {
        return;
    }
    for (label, class) in [
        ("Mesh", CLASS_MESH),
        ("Speedup Pad", CLASS_SPEEDUP_PAD),
        ("Weapon Pad", CLASS_WEAPON_PAD),
    ] {
        let (files_touched, nodes, total, claimed, worst) =
            sweep_class(&files, class, mesh_coverage::coverage);
        let pct = 100.0 * claimed as f64 / total.max(1) as f64;
        println!(
            "{label}: {nodes} node(s) across {files_touched} file(s), {claimed} of {total} byte(s) ({pct:.2}%)"
        );
        if let Some(worst) = &worst {
            println!("  worst: {worst}");
        }
        assert!(nodes > 0, "{label}: no nodes found on the disc");
    }
}

/// **`Skycube` (`0x3c6`) and `fogCube` (`0x3d3`)**, and the one place this
/// sweep expects a named, structurally-closed gap: `06_Track`'s sky.
///
/// See `docs/formats/skycube.md#the-extra-block`.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn skycube_extra_block_closes_on_the_one_circuit_that_has_it() {
    let files = corpus("pulse-psp-usa.chd", "Data.wad");
    if files.is_empty() {
        return;
    }
    let (files_touched, nodes, total, claimed, worst) =
        sweep_class(&files, CLASS_SKYCUBE, mesh_coverage::coverage);
    let pct = 100.0 * claimed as f64 / total.max(1) as f64;
    println!(
        "Skycube: {nodes} node(s) across {files_touched} file(s), {claimed} of {total} byte(s) ({pct:.2}%)"
    );
    if let Some(worst) = &worst {
        println!("  worst: {worst}");
    }
    assert_eq!(nodes, 40, "every PSP circuit authors exactly one Skycube");

    // The named block itself: recognised, sized exactly, on both of
    // `06_Track`'s directions.
    let mut recognised = 0usize;
    for bytes in &files {
        let Ok(tree) = vex::nodes(bytes) else {
            continue;
        };
        for node in tree.iter().filter(|n| n.class_id == CLASS_SKYCUBE) {
            let range = node.payload();
            let Some(payload) = bytes.get(range) else {
                continue;
            };
            let material_count = usize::from(ByteOrder::Little.u16(payload, 2));
            let materials_end = (0x30 + material_count * 0x14).div_ceil(0x10) * 0x10;
            if let Some(block) = mesh_coverage::skycube_extra_block(payload, materials_end) {
                assert_eq!(block.len, 0x1a0, "the block's own measured length");
                assert_eq!(block.record_count, material_count);
                recognised += 1;
            }
        }
    }
    assert_eq!(
        recognised, 2,
        "06_Track's forward and reversed directions, and no other circuit"
    );
}

/// **`WO Track` (`0x3bb`) and `section` (`0x3c9`).**
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn wo_track_closes_and_section_leaves_its_two_named_gaps() {
    let files = corpus("pulse-psp-usa.chd", "Data.wad");
    if files.is_empty() {
        return;
    }
    let (files_touched, nodes, total, claimed, worst) =
        sweep_class(&files, CLASS_WO_TRACK, track_coverage::wo_track_coverage);
    println!(
        "WO Track: {nodes} node(s) across {files_touched} file(s), {claimed} of {total} byte(s)"
    );
    assert_eq!(
        claimed, total,
        "a WO Track payload is expected to close exactly"
    );
    assert_eq!(nodes, 40, "every PSP circuit authors exactly one WO Track");
    assert!(worst.is_none(), "unexpected gap: {worst:?}");

    let (files_touched, nodes, total, claimed, worst) = sweep_class(&files, CLASS_SECTION, |p| {
        track_coverage::section_coverage(p)
    });
    let pct = 100.0 * claimed as f64 / total.max(1) as f64;
    println!(
        "section: {nodes} node(s) across {files_touched} file(s), {claimed} of {total} byte(s) ({pct:.2}%)"
    );
    if let Some(worst) = &worst {
        println!("  worst: {worst}");
    }
    // `pvs_ground_truth.rs`'s own `PSP_PULSE_SECTIONS` is 2272 raw nodes too;
    // `docs/formats/track.md`'s "2,268" is the *unique-id* count after
    // `survey.repeated_ids` (4, two tracks each authoring one id three times
    // over) is folded out, not a different node census.
    assert_eq!(
        nodes, 2272,
        "raw section nodes, matching pvs_ground_truth's PSP_PULSE_SECTIONS"
    );
    // Every section leaves exactly the same 6-byte pad[6] gap and nothing
    // else, so the total unclaimed is exactly 6 bytes per node.
    assert_eq!(
        total - claimed,
        6 * nodes as u64,
        "pad[6] on every section, nothing more"
    );
}

/// **The five collision classes, on both the PSP and PS2 discs.**
///
/// Closed by construction - `collision::from_vex` already refuses a node
/// whose chunk walk does not land on the payload's own alignment boundary -
/// so this exists to make that checkable from the coverage sweep rather than
/// to find anything new.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and data/images/pulse-ps2-eu.chd"]
fn collision_closes_on_both_platforms() {
    let mut total_nodes = 0usize;
    for (image_name, archive_name) in [
        ("pulse-psp-usa.chd", "Data.wad"),
        ("pulse-ps2-eu.chd", "WADS2.WAD"),
    ] {
        let files = corpus(image_name, archive_name);
        if files.is_empty() {
            continue;
        }
        let (mut nodes, mut total, mut claimed) = (0usize, 0u64, 0u64);
        for bytes in &files {
            let order = vex::byte_order(bytes);
            let Ok(tree) = vex::nodes(bytes) else {
                continue;
            };
            let Ok(found) = collision::from_vex(bytes) else {
                continue;
            };
            for node in found {
                let Some(tree_node) = tree.get(node.node_index) else {
                    continue;
                };
                let range = tree_node.payload();
                let Some(payload) = bytes.get(range) else {
                    continue;
                };
                let seen = track_coverage::collision_coverage(payload, order);
                nodes += 1;
                total += seen.len() as u64;
                claimed += seen.claimed() as u64;
            }
        }
        println!("{image_name}: {nodes} collision node(s), {claimed} of {total} byte(s)");
        total_nodes += nodes;
    }
    assert_eq!(total_nodes, 319, "130 on the PSP disc, 189 on the PS2 disc");
}
