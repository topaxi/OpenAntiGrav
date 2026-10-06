//! Validates the [`quake`](oag_vex::quake) span table against every Pulse
//! circuit on the disc.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What this is for
//!
//! The ripple is drawn by moving the vertices of the batch each span names, so
//! three things have to hold for it to move the right road, and none is
//! checkable by reading the executable alone:
//!
//! 1. **A span names a whole batch.** `+0x4c` resolves to a batch header, and
//!    the span's own vertex count is that batch's.
//! 2. **[`quake::batch_offsets`] walks the batches [`vex::mesh_batches`]
//!    decodes**, one for one, since it is a second copy of the same walk.
//! 3. **The neighbour links are distance-exact**, which is what lets the ripple
//!    be drawn as one bump at one distance rather than by porting the
//!    original's arm-and-propagate machine span by span.
//!
//! See `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "the
//! ripple itself", for the numbers these reproduce.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::quake::{self, Span};
use oag_vex::vex;

const TRACK_DIRS: &[&str] = &[
    "01_Track", "02_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track", "08_Track",
    "09_Track", "10_Track", "11_Track", "12_Track", "13_Track", "14_Track", "15_Track", "16_Track",
];

const TRACK_FILES: &[&str] = &["track.vex", "track_reversed.vex"];

/// The twelve circuits, forwards and reversed.
const EXPECTED_FILES: usize = 24;

fn track_models() -> Option<Vec<(String, Vec<u8>)>> {
    let path: PathBuf = oag_testdata::image("pulse-psp-usa.chd")?;
    let mut disc = DiscImage::open(&path).expect("open");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == "PSP_GAME/USRDIR/Data.wad")
        .expect("Data.wad present")
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
    for dir_name in TRACK_DIRS {
        for file in TRACK_FILES {
            let name = format!("Data\\Environments\\{dir_name}\\{file}");
            let hash = wad::hash_name(&name);
            let Some(entry) = dir.entries.iter().find(|e| e.name_hash == hash) else {
                continue;
            };
            let raw = disc
                .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
                .expect("blob");
            let model = match entry.compression {
                Compression::None => raw,
                Compression::Lzss => {
                    oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize)
                        .expect("lzss")
                }
                Compression::Zlib => panic!("{name}: unexpected zlib entry"),
            };
            out.push((name, model));
        }
    }
    assert_eq!(out.len(), EXPECTED_FILES, "circuit files found");
    Some(out)
}

/// The classes whose payloads are meshes: the road, and the two pads.
const MESH_CLASSES: [u32; 3] = [
    vex::CLASS_MESH,
    vex::CLASS_SPEEDUP_PAD,
    vex::CLASS_WEAPON_PAD,
];

fn quake_spans(model: &[u8], nodes: &[vex::Node]) -> Vec<Span> {
    let mut quake_nodes = vex::nodes_by_class(nodes, quake::CLASS_QUAKE);
    let node = quake_nodes.next().expect("a Quake node");
    assert!(quake_nodes.next().is_none(), "one Quake node per circuit");
    quake::spans(model, node).expect("spans parse")
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_span_is_one_whole_mesh_batch() {
    let Some(models) = track_models() else {
        return;
    };
    let mut total = 0;
    for (name, model) in &models {
        let nodes = vex::nodes(model).expect("nodes");
        // Every batch header of every mesh-shaped node, with its vertex count.
        let mut batches = std::collections::BTreeMap::new();
        for node in nodes.iter().filter(|n| MESH_CLASSES.contains(&n.class_id)) {
            let payload = &model[node.payload()];
            for list in [0u8, 1] {
                let decoded = vex::mesh_batches(payload, list).expect("batches");
                for (offset, batch) in quake::batch_offsets(payload, list).into_iter().zip(decoded)
                {
                    batches.insert(node.payload().start + offset, batch.vertices.len());
                }
            }
        }
        let spans = quake_spans(model, &nodes);
        assert!(spans.len() > 300, "{name}: {} spans", spans.len());
        for (i, span) in spans.iter().enumerate() {
            let vertices = batches
                .get(&span.batch)
                .unwrap_or_else(|| panic!("{name} span {i}: {:#x} is no batch header", span.batch));
            assert_eq!(
                *vertices,
                span.parameters.len(),
                "{name} span {i}: vertex count"
            );
            let into = span.first_position - span.batch;
            assert!(
                (0x40..0x40 + 20).contains(&into),
                "{name} span {i}: position at +{into:#x}"
            );
        }
        total += spans.len();
    }
    assert_eq!(total, 9226, "spans across all 24 files");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn batch_offsets_match_mesh_batches() {
    let Some(models) = track_models() else {
        return;
    };
    let mut meshes = 0;
    for (name, model) in &models {
        let nodes = vex::nodes(model).expect("nodes");
        for node in nodes.iter().filter(|n| MESH_CLASSES.contains(&n.class_id)) {
            let payload = &model[node.payload()];
            for list in [0u8, 1] {
                let decoded = vex::mesh_batches(payload, list).expect("batches").len();
                let walked = quake::batch_offsets(payload, list).len();
                assert_eq!(
                    walked, decoded,
                    "{name} node at {:#x}, list {list}",
                    node.offset
                );
            }
            meshes += 1;
        }
    }
    assert!(meshes > 10_000, "only {meshes} meshes walked");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn neighbour_links_are_distance_exact() {
    let Some(models) = track_models() else {
        return;
    };
    for (name, model) in &models {
        let nodes = vex::nodes(model).expect("nodes");
        let spans = quake_spans(model, &nodes);
        // Each path's length, from any span on it: `length / (t_end - t_start)`.
        let path_length = |path: i16| {
            let span = spans
                .iter()
                .find(|s| s.path == path && s.t_end > s.t_start)
                .expect("a span on the path");
            span.length / (span.t_end - span.t_start)
        };
        let mut worst = 0.0f32;
        let mut links = 0;
        for span in &spans {
            let own = path_length(span.path);
            let origin = span.t_start * own;
            let forward = span
                .forward
                .iter()
                .flatten()
                .map(|&(n, at)| (n, origin + at));
            let backward =
                span.backward.iter().flatten().map(|&(n, gap)| {
                    (n, origin + span.length + gap - spans[usize::from(n)].length)
                });
            for (neighbour, predicted) in forward.chain(backward) {
                let other = &spans[usize::from(neighbour)];
                if other.path != span.path {
                    continue;
                }
                let error =
                    (other.t_start * own - predicted + own / 2.0).rem_euclid(own) - own / 2.0;
                worst = worst.max(error.abs());
                links += 1;
            }
        }
        assert!(links > 300, "{name}: {links} links");
        assert!(
            worst < 0.05,
            "{name}: worst neighbour origin off by {worst}"
        );
    }
}
