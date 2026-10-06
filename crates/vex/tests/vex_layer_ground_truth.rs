//! Which **render layer** each `Mesh` node lands in, checked against the disc.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What a layer is
//!
//! Wipeout Pulse does not draw in file order and it does not depth-sort its
//! scene geometry either. Every drawable submits itself to one queue with a
//! 32-bit key, and `Gfx_FlushRenderManager` `qsort`s that queue ascending before
//! dispatching it. The key's top twelve bits are a **layer**; the low twenty are
//! a back-to-front depth that only the exhaust flare computes.
//!
//! A mesh's layer is derived at load from its own payload header by
//! `Mesh_InitFromPayload` - see
//! [`vex::mesh_layer`](../../src/vex.rs) and
//! `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`.
//!
//! # What this file is for
//!
//! **Deciding whether the derivation separates anything.** A rule that put every
//! mesh on one layer would be a correct reading of a field the game does not use
//! to distinguish anything, and a renderer that implemented it would reorder
//! nothing. The census is what tells those two apart, and it is the gate the
//! renderer work was put behind rather than assumed past.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::vex;

/// The archive holding every circuit and every ship.
const PSP_DATA: &str = "PSP_GAME/USRDIR/Data.wad";

/// Smallest number of `Mesh` nodes the sweep must reach before its shares mean
/// anything. Well under the real figure; a count that collapses is a broken
/// walk, not a disc with fewer models on it.
const MIN_MESHES: usize = 5_000;

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

/// The layer census over every `Mesh` node of every version-6 `.vex` on the
/// disc, as `(layer, count)` ascending - which is draw order.
fn census(blobs: &[(usize, Vec<u8>)]) -> std::collections::BTreeMap<u32, usize> {
    let mut out = std::collections::BTreeMap::new();
    for (_, bytes) in blobs {
        let Ok(nodes) = vex::nodes(bytes) else {
            continue;
        };
        for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
            let range = node.payload();
            if range.end > bytes.len() {
                continue;
            }
            let payload = &bytes[range];
            let Some(layer) = vex::mesh_layer(payload, vex::LAYER_SCENE) else {
                continue;
            };
            *out.entry(layer).or_default() += 1;
        }
    }
    out
}

/// **The second gate, and the one the renderer turns on: are the disc's
/// *transparent* batches spread across more than one layer?**
///
/// Only the transparent list is order-dependent - opaque and cutout draws are
/// depth-tested with depth write on and land the same picture in any order. So
/// a layer split that is real over all meshes but lands every transparent batch
/// in one layer would still reorder nothing that shows.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_discs_transparent_batches_are_spread_across_the_layers() {
    let Some(path) = image() else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let blobs = psp_vex_blobs(&mut disc);

    let mut counts: std::collections::BTreeMap<u32, usize> = Default::default();
    for (_, bytes) in &blobs {
        let Ok(nodes) = vex::nodes(bytes) else {
            continue;
        };
        for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
            let range = node.payload();
            if range.end > bytes.len() {
                continue;
            }
            let payload = &bytes[range];
            let Some(layer) = vex::mesh_layer(payload, vex::LAYER_SCENE) else {
                continue;
            };
            for list in [0u8, 1] {
                let Ok(batches) = vex::mesh_batches(payload, list) else {
                    continue;
                };
                *counts.entry(layer).or_default() +=
                    batches.iter().filter(|b| b.is_transparent()).count();
            }
        }
    }
    let total: usize = counts.values().sum();
    println!("{total} transparent batch(es) by layer:");
    for (layer, count) in &counts {
        println!(
            "  layer {layer:#010x}: {count:>6} ({:.1}%)",
            100.0 * *count as f64 / total as f64
        );
    }

    let populated = counts.values().filter(|c| **c > 0).count();
    assert!(
        populated >= 2,
        "every transparent batch is on one layer, so ordering by layer shows nothing: {counts:?}"
    );
    let smallest = counts
        .values()
        .filter(|c| **c > 0)
        .min()
        .copied()
        .unwrap_or(0);
    assert!(
        smallest * 20 >= total,
        "the smaller layer holds {smallest} of {total} transparent batches - under 5%, which is \
         too thin for layer ordering to be worth the machinery"
    );
}

/// **The gate: the derivation separates the disc's meshes into more than one
/// layer, and the split is not a rounding error.**
///
/// `0x4a` is the rule's default branch and `0x45` its narrow one, so the
/// question this answers is whether `0x45` and `0x31` are populated at all. If
/// they were not, faithful layer ordering would reorder nothing and there would
/// be no renderer change worth making.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_layer_derivation_separates_the_discs_meshes() {
    let Some(path) = image() else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let blobs = psp_vex_blobs(&mut disc);
    let counts = census(&blobs);
    let total: usize = counts.values().sum();

    println!(
        "{total} mesh node(s) across {} version-6 .vex file(s)",
        blobs.len()
    );
    for (layer, count) in &counts {
        println!(
            "  layer {layer:#010x}: {count:>6} ({:.1}%)",
            100.0 * *count as f64 / total as f64
        );
    }

    assert!(
        total >= MIN_MESHES,
        "{total} mesh nodes is too few to census; the walk is broken"
    );
    assert!(
        counts.len() >= 2,
        "the derivation puts every mesh on one layer, so it distinguishes nothing: {counts:?}"
    );
    // Named individually rather than by count, so a regression says which
    // branch of the derivation stopped firing.
    for layer in [vex::LAYER_SCENE, vex::LAYER_DEFAULT] {
        assert!(
            counts.get(&layer).copied().unwrap_or(0) > 0,
            "layer {layer:#010x} is empty; the derivation's branches are not all reachable"
        );
    }
}
