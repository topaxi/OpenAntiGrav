//! Validates the [`collision`](oag_vex::collision) decoder against every
//! collision node on the PSP and PS2 discs.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What this is for
//!
//! The collision layout was settled the way [`WO Track`](oag_vex::track) was:
//! by arithmetic that cannot come out even unless the reading is right. These
//! tests are that argument, executable, over **319 collision nodes** holding
//! 18,745 objects and 594,615 triangles.
//!
//! Rather than resolving track names, they walk **every** entry of an archive and
//! decode every `.vex` file in it. That covers ships and front-end models as well
//! as tracks, and it is what establishes that collision geometry appears *only*
//! in the five collision node classes: no other class ID, out of 132,000 nodes
//! across both discs, carries a payload that satisfies the closure check.
//!
//! What each assertion is for is recorded next to it. The four that carry the
//! layout are the payload closing on its 16-byte boundary, the chunk header's own
//! stride field agreeing with its type, the scalar array being per-vertex, and
//! every triangle index naming a vertex.

use std::collections::BTreeMap;
use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::collision::{self, CollisionNode, PAYLOAD_ALIGN, SurfaceKind};
use oag_vex::vex;
use oag_vex::vex::classes::V6;

/// Collision nodes in the PSP disc's `Data.wad`: 40 floor, 40 wall, 26 reset,
/// 24 mag floor, and no cages.
const PSP_NODES: usize = 130;

/// Collision nodes in the PS2 disc's archives: 59, 59, 32, 33, and **6 cages**.
const PS2_NODES: usize = 189;

/// A coordinate larger than this is a misdecode, not a big track.
///
/// The largest seen is 1,335.8 on PSP and 1,554.1 on PS2, both printed by the
/// tests. Note that those exceed the roughly +/-1024 the sweep-and-prune packing
/// reaches from the origin, and that collision nodes carry an identity world
/// transform, so the coordinates really are world space - while `max_span` below
/// stays under the packing's 2,048-unit window everywhere. Where the packing's
/// origin comes from is an open question in `docs/formats/collision.md`, not
/// something these tests judge.
const COORD_LIMIT: f32 = 1.0e5;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// What a survey of one archive found.
#[derive(Default)]
struct Survey {
    vex_files: usize,
    vex_nodes: usize,
    nodes: usize,
    objects: usize,
    vertices: usize,
    triangles: usize,
    per_kind: BTreeMap<&'static str, usize>,
    chunk_orders: BTreeMap<Vec<u32>, usize>,
    header_words: BTreeMap<u32, usize>,
    padding: BTreeMap<usize, usize>,
    max_coord: f32,
    /// Widest axis of any one file's union-of-collidable bounding box, and which
    /// entry it came from.
    ///
    /// The sweep-and-prune packing covers a 2048-unit **window**; `max_coord`
    /// says whether raw world coordinates fit that window centred on the origin,
    /// and this says whether they would fit it centred anywhere. See
    /// `docs/formats/collision.md`.
    max_span: f32,
    max_span_entry: usize,
    non_neutral_scalars: usize,
    /// Class IDs, other than the five known ones, whose payload nevertheless
    /// satisfies the closure check. Expected to be empty.
    false_positives: BTreeMap<u32, usize>,
}

impl Survey {
    fn report(&self, label: &str) {
        println!("== {label}");
        println!(
            "  {} .vex files, {} nodes; {} collision nodes {:?}",
            self.vex_files, self.vex_nodes, self.nodes, self.per_kind
        );
        println!(
            "  {} objects, {} vertices, {} triangles",
            self.objects, self.vertices, self.triangles
        );
        println!("  chunk orders, by object: {:?}", self.chunk_orders);
        println!("  payload first word, by node: {:x?}", self.header_words);
        println!("  alignment padding, by node: {:?}", self.padding);
        println!("  largest coordinate: {}", self.max_coord);
        println!(
            "  widest per-file collidable span: {} (entry {})",
            self.max_span, self.max_span_entry
        );
        println!("  non-neutral vertex scalars: {}", self.non_neutral_scalars);
        println!(
            "  other class IDs whose payload closes as collision: {:x?}",
            self.false_positives
        );
    }
}

/// Decodes every `.vex` file in one archive and checks every collision node in it.
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
        into.vex_files += 1;
        into.vex_nodes += tree.len();

        // The convenience layer, over every model in the game rather than a
        // hand-built fixture. It enforces the closure check itself, so a single
        // failure anywhere in the archive fails here.
        let found: Vec<CollisionNode> = collision::from_vex(&model)
            .unwrap_or_else(|e| panic!("{archive_path} entry {index}: from_vex: {e}"));

        // Collision geometry appears only in collision nodes. Any other class
        // whose payload closes as collision data would mean the format is not
        // specific to these five IDs, which matters because that specificity is
        // how the same format was located in Pure.
        for node in &tree {
            if SurfaceKind::from_class_id(node.class_id, V6).is_some() {
                continue;
            }
            let range = node.payload();
            if range.end > model.len() || range.len() < 32 {
                continue;
            }
            let payload = &model[range];
            if let Ok(geometry) = collision::parse_chunks(payload, vex::byte_order(&model))
                && geometry.padded_len() == payload.len()
                && geometry.vertex_count() > 2
            {
                *into.false_positives.entry(node.class_id).or_default() += 1;
            }
        }

        // Union box of everything collidable in this one file, for `max_span`.
        // `Cage` is excluded because it is not collidable, matching what the
        // broadphase would actually be asked to hold.
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        let mut any = false;
        for node in &found {
            if node.kind == SurfaceKind::Cage {
                continue;
            }
            for mesh in &node.geometry.meshes {
                for v in &mesh.vertices {
                    any = true;
                    for axis in 0..3 {
                        lo[axis] = lo[axis].min(v[axis]);
                        hi[axis] = hi[axis].max(v[axis]);
                    }
                }
            }
        }
        if any {
            let span = (0..3).fold(0.0f32, |m, axis| m.max(hi[axis] - lo[axis]));
            if span > into.max_span {
                into.max_span = span;
                into.max_span_entry = index;
            }
        }

        let worlds = vex::world_transforms(&model, &tree);
        for node in found {
            let kind = node.kind;
            let payload = &model[tree[node.node_index].payload()];
            let geometry = &node.geometry;

            // Collision vertices are already in world space: every collision node
            // on both discs sits at depth 1 with an identity world matrix, so a
            // consumer must not transform them a second time.
            assert_eq!(
                worlds[node.node_index],
                vex::IDENTITY,
                "{archive_path} entry {index}: {} node {} has a non-identity world \
                 transform, so its vertices are local and every consumer that \
                 treats them as world space is wrong",
                kind.node_name(),
                node.node_index,
            );

            // The check that settles the layout. `from_vex` already enforced it;
            // this is the same arithmetic said out loud, because it is the reason
            // to believe any of the rest.
            assert_eq!(
                geometry.padded_len(),
                payload.len(),
                "{archive_path} entry {index}: {} node {}: {} objects, {} vertices \
                 and {} triangles consume {} of {} bytes.\n\
                 Short by 2 per object would mean the per-object chunk count is a \
                 u16, not the u32 collision::OBJECT_HEADER_LEN assumes.",
                kind.node_name(),
                node.node_index,
                geometry.meshes.len(),
                geometry.vertex_count(),
                geometry.triangle_count(),
                geometry.encoded_len(),
                payload.len(),
            );
            // And the padding really is padding: zero-filled, never data.
            assert!(
                payload[geometry.encoded_len()..].iter().all(|&b| b == 0),
                "{archive_path} entry {index}: {} node {}: the {} bytes after the \
                 chunk walk are not zero, so they are not alignment padding",
                kind.node_name(),
                node.node_index,
                payload.len() - geometry.encoded_len(),
            );

            *into
                .padding
                .entry(payload.len() - geometry.encoded_len())
                .or_default() += 1;
            *into.header_words.entry(geometry.version).or_default() += 1;

            for (i, mesh) in geometry.meshes.iter().enumerate() {
                // Not a format requirement -- the parser locates chunks by type --
                // but true of all 18,745 shipped objects, and worth knowing: the
                // scalars come *before* the indices, which keeps the f32 array
                // 4-byte aligned even when an odd triangle count leaves the index
                // array ending on a half-word.
                let order: Vec<u32> = mesh.chunks.iter().map(|c| c.kind).collect();
                assert_eq!(
                    order,
                    vec![
                        collision::CHUNK_VERTICES,
                        collision::CHUNK_VERTEX_SCALARS,
                        collision::CHUNK_TRIANGLES,
                    ],
                    "{archive_path} entry {index}: {} node {} object {i}: chunk \
                     order {order:?} is new; every shipped object stores 1, 3, 2",
                    kind.node_name(),
                    node.node_index,
                );
                for chunk in &mesh.chunks {
                    // Redundant with the parser, which refuses a mismatch. Stated
                    // anyway because this field was recorded as an unknown word.
                    assert_eq!(
                        usize::from(chunk.stride),
                        collision::chunk_stride(chunk.kind).expect("known chunk type"),
                        "{archive_path} entry {index}: chunk type {} declares \
                         stride {}",
                        chunk.kind,
                        chunk.stride,
                    );
                }

                assert_eq!(
                    mesh.vertex_scalars.len(),
                    mesh.vertices.len(),
                    "{archive_path} entry {index}: {} node {} object {i}: the \
                     scalar array is not one per vertex",
                    kind.node_name(),
                    node.node_index,
                );

                for v in &mesh.vertices {
                    for (axis, c) in v.iter().enumerate() {
                        assert!(
                            c.is_finite() && c.abs() < COORD_LIMIT,
                            "{archive_path} entry {index}: {} node {} object {i}: \
                             axis {axis} is {c}, which is not a coordinate",
                            kind.node_name(),
                            node.node_index,
                        );
                        into.max_coord = into.max_coord.max(c.abs());
                    }
                }
                into.non_neutral_scalars += mesh
                    .vertex_scalars
                    .iter()
                    .filter(|&&s| s != collision::DEFAULT_VERTEX_SCALAR)
                    .count();

                *into.chunk_orders.entry(order).or_default() += 1;
                into.objects += 1;
                into.vertices += mesh.vertices.len();
                into.triangles += mesh.triangles.len();
            }

            *into.per_kind.entry(kind.node_name()).or_default() += 1;
            into.nodes += 1;
        }
    }
}

/// Assertions that hold on both discs, applied after a survey.
fn check_common(label: &str, s: &Survey, expected_nodes: usize) {
    s.report(label);

    assert_eq!(
        s.nodes,
        expected_nodes,
        "{label}: {} collision nodes, expected {expected_nodes}. A count of zero \
         would mean collision geometry is not stored as .vex nodes under class \
         IDs {:#x?} at all.",
        s.nodes,
        SurfaceKind::ALL.map(|kind| kind.class_id(V6)),
    );

    // The four surfaces the engine instantiates are all present in both builds.
    for kind in [
        SurfaceKind::Floor,
        SurfaceKind::Wall,
        SurfaceKind::MagFloor,
        SurfaceKind::Reset,
    ] {
        assert!(
            s.per_kind.contains_key(kind.node_name()),
            "{label}: no {} node, though the class table lists {:#x}",
            kind.node_name(),
            kind.class_id(V6).expect("version 6 has every collision id"),
        );
    }

    assert!(
        s.false_positives.is_empty(),
        "{label}: class IDs {:#x?} also carry payloads that close as collision \
         data, so the format is not specific to the five collision classes",
        s.false_positives,
    );

    // Every object in both builds stores exactly three chunks, in one order.
    assert_eq!(
        s.chunk_orders.len(),
        1,
        "{label}: more than one chunk order: {:?}",
        s.chunk_orders
    );

    // The first word is all ones everywhere, which is why calling it a version is
    // a reading rather than an observation.
    assert_eq!(
        s.header_words.keys().copied().collect::<Vec<_>>(),
        vec![collision::HEADER_WORD],
        "{label}: the payload's first word is not always {:#x}: {:x?}",
        collision::HEADER_WORD,
        s.header_words,
    );

    // Padding is what the alignment rule predicts and nothing else.
    for &pad in s.padding.keys() {
        assert!(
            pad < PAYLOAD_ALIGN,
            "{label}: {pad} bytes of padding is more than the alignment allows"
        );
    }

    // Every vertex scalar in both builds is exactly 1.0. A failure here is a
    // **finding**, not a bug: it would be the first shipped use of the field
    // whose meaning `docs/formats/collision.md` records as unknown.
    assert_eq!(
        s.non_neutral_scalars,
        0,
        "{label}: {} vertex scalars are not {}. That is new: no shipped scalar \
         has ever been anything else, and this is the field nobody has explained.",
        s.non_neutral_scalars,
        collision::DEFAULT_VERTEX_SCALAR,
    );

    assert!(s.vertices > 0 && s.triangles > 0, "{label}: no geometry");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_collision_node_on_the_psp_disc_decodes_with_nothing_left_over() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let mut s = Survey::default();
    survey(&mut disc, "PSP_GAME/USRDIR/Data.wad", &mut s);
    check_common("pulse psp", &s, PSP_NODES);

    // `Cage Collision` is registered, parsed and then dropped by the loader, and
    // the PSP build ships none at all. The PS2 build does; see the test below.
    assert!(
        !s.per_kind.contains_key(SurfaceKind::Cage.node_name()),
        "the PSP disc now has {:?} cage nodes, which it did not before",
        s.per_kind.get(SurfaceKind::Cage.node_name()),
    );
}

/// The second binary, which is worth more than a second reading of the first.
///
/// Same format, same strides, same closure, same all-ones header word - and the
/// cage geometry the PSP build does not carry.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_collision_node_on_the_ps2_disc_decodes_with_nothing_left_over() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let mut s = Survey::default();
    survey(&mut disc, "54748/WADS2.WAD", &mut s);
    survey(&mut disc, "54748/WADSP.WAD", &mut s);
    check_common("pulse ps2", &s, PS2_NODES);

    // The answer to "dead content, or another SKU": the other SKU. Cage geometry
    // ships on PS2 and the loader still drops it.
    assert_eq!(
        s.per_kind.get(SurfaceKind::Cage.node_name()).copied(),
        Some(6),
        "the PS2 disc's cage nodes are what makes Cage Collision a platform \
         difference rather than dead content"
    );
}

/// Class IDs in one archive, and which of them carry collision-shaped payloads.
///
/// The second map is the decoder used as a detector: a payload that parses, ends
/// exactly on its 16-byte boundary and opens with the all-ones header word is not
/// something an unrelated structure produces by chance. On both Pulse discs the
/// detector fires on exactly the five known collision classes and nothing else,
/// which is what licenses using it to find the format in another title.
fn collision_shaped(
    disc: &mut DiscImage,
    archive_path: &str,
) -> (BTreeMap<u32, usize>, BTreeMap<u32, usize>) {
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

    let mut census: BTreeMap<u32, usize> = BTreeMap::new();
    let mut candidates: BTreeMap<u32, usize> = BTreeMap::new();

    for entry in &dir.entries {
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
            Compression::Zlib => continue,
        };
        if !vex::has_magic(&model) {
            continue;
        }
        let Ok(tree) = vex::nodes(&model) else {
            continue;
        };
        for node in &tree {
            *census.entry(node.class_id).or_default() += 1;
            let range = node.payload();
            if range.end > model.len() || range.len() < 32 {
                continue;
            }
            let payload = &model[range];
            if let Ok(geometry) = collision::parse_chunks(payload, vex::byte_order(&model))
                && geometry.padded_len() == payload.len()
                && geometry.version == collision::HEADER_WORD
                && geometry.vertex_count() > 2
            {
                *candidates.entry(node.class_id).or_default() += 1;
            }
        }
    }

    (census, candidates)
}

/// The same collision format is in Wipeout Pure, under different class IDs.
///
/// Pure's whole class-ID space is renumbered - `0x6d` and `0x11e` dominate where
/// Pulse has `Transform` at `0x6e` and `Mesh` at `0x125` - and **none** of
/// Pulse's five collision IDs appears anywhere in its 23,677 nodes. Pointing the
/// decoder at every node instead finds three IDs whose payloads decode exactly:
/// 16 nodes each, one per track, matching Pulse's shape down to two large classes
/// and one small one.
///
/// Which of the three is floor, wall or reset is **not** determined: the object
/// counts suggest it by analogy with Pulse, and analogy is not evidence. That is
/// why nothing here is named and no constants were added.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_same_collision_format_appears_in_wipeout_pure() {
    let Some(path) = image("pure-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let (census, candidates) = collision_shaped(&mut disc, "PSP_GAME/USRDIR/Data.wad");

    println!("pure psp: {} distinct class ids", census.len());
    println!("collision-shaped payloads by class id: {candidates:x?}");

    for kind in SurfaceKind::ALL {
        assert!(
            !census.contains_key(&kind.class_id(V6).expect("version 6 has every collision id")),
            "Pure uses Pulse's {:#x} ({}), so the class table is shared after all",
            kind.class_id(V6).expect("version 6 has every collision id"),
            kind.node_name(),
        );
    }

    assert_eq!(
        candidates,
        BTreeMap::from([(0x36b, 16), (0x36c, 16), (0x37f, 16)]),
        "Pure's collision-shaped class IDs are not the three found before"
    );
}
