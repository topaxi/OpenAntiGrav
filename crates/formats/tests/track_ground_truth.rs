//! Validates the [`track`](oag_formats::track) decoder against real tracks.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The test skips with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What this is for
//!
//! The `WO Track` layout was settled by an arithmetic argument: a correct parse
//! accounts for every byte of the payload, and a parse that is one structure out
//! cannot. That argument was made once, by hand, against 40 files. This is the
//! same argument, executable, so it stays true.

use std::path::{Path, PathBuf};

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_formats::{track, vex};

/// Track directories present on the PSP disc, from the plugin definitions.
const TRACK_DIRS: &[&str] = &[
    "01_Track", "02_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track", "08_Track",
    "09_Track", "10_Track", "11_Track", "12_Track", "13_Track", "14_Track", "15_Track", "16_Track",
];

/// The four spline-carrying models a track directory can hold.
const TRACK_FILES: &[&str] = &[
    "track.vex",
    "track_reversed.vex",
    "zone_track.vex",
    "zone_track_reversed.vex",
];

/// Fewest tracks that must be checked for the test to mean anything.
const MIN_TRACKS: usize = 20;

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

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    dot(d, d).sqrt()
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_shipped_track_decodes_with_nothing_left_over() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
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

    let mut checked = 0usize;
    let mut points_total = 0usize;

    for dir_name in TRACK_DIRS {
        for file in TRACK_FILES {
            let name = format!("Data\\Environments\\{dir_name}\\{file}");
            let hash = wad::hash_name(&name);
            let Some(entry) = dir.entries.iter().find(|e| e.name_hash == hash) else {
                continue;
            };
            if entry.size == 0 {
                continue;
            }

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
            assert!(vex::has_magic(&model), "{name} is not a .vex model");

            let nodes = vex::nodes(&model).expect("nodes");
            let mut splines = 0;
            for node in nodes.iter().filter(|n| n.class_id == CLASS_WO_TRACK) {
                let payload = &model[node.payload()];
                assert!(track::has_magic(payload), "{name}: no WOtd magic");

                let ai = track::parse(payload).expect("parse WO Track");
                splines += 1;
                checked += 1;
                points_total += ai.point_count();

                // The check that settled the layout. Being one structure out
                // cannot produce an exact fit.
                assert_eq!(
                    ai.encoded_len(),
                    payload.len(),
                    "{name}: {} paths, {} junctions, {} points do not account for the payload",
                    ai.paths.len(),
                    ai.junctions.len(),
                    ai.point_count(),
                );

                assert_eq!(ai.version, 0x105, "{name}: unexpected version");
                assert!(!ai.paths.is_empty(), "{name}: no paths");

                for (i, spline) in ai.paths.iter().enumerate() {
                    assert!(
                        spline.points.len() >= 4,
                        "{name} path {i}: {} points is too few for a cubic spline",
                        spline.points.len()
                    );

                    // Junction wiring. Slots 0 and 1 are predecessors and 2 and
                    // 3 successors: that ordering was inferred from which slots
                    // each direction reads, and this is what proves it.
                    let exit = spline.exit.expect("every path exits somewhere");
                    let entry_j = spline.entry.expect("every path has an entry");
                    assert!(
                        ai.junctions[exit].prev.contains(&Some(i)),
                        "{name}: path {i} exits junction {exit}, which does not list it as a \
                         predecessor: {:?}",
                        ai.junctions[exit].prev
                    );
                    assert!(
                        ai.junctions[entry_j].next.contains(&Some(i)),
                        "{name}: path {i} enters from junction {entry_j}, which does not list it \
                         as a successor: {:?}",
                        ai.junctions[entry_j].next
                    );

                    let mut longest = 0.0f32;
                    for (k, p) in spline.points.iter().enumerate() {
                        for (label, v) in [
                            ("tangent", p.tangent),
                            ("down", p.down),
                            ("lateral", p.lateral),
                        ] {
                            let length = dot(v, v).sqrt();
                            assert!(
                                (length - 1.0).abs() < 1e-3,
                                "{name} path {i} point {k}: {label} length {length}"
                            );
                        }
                        // An orthonormal frame, so the axes are perpendicular.
                        assert!(
                            dot(p.tangent, p.down).abs() < 1e-2,
                            "{name} path {i} point {k}: tangent and down are not perpendicular"
                        );

                        assert!(
                            p.section_id < 64,
                            "{name} path {i} point {k}: section {} exceeds the 64-section cap",
                            p.section_id
                        );

                        // The load pass clamps the racing line into the track
                        // width. If the shipped data already satisfies it, the
                        // clamp is defensive rather than corrective, which is
                        // what makes the field safe to trust.
                        assert!(
                            p.racing_line >= -p.half_width_left - 1e-3
                                && p.racing_line <= p.half_width_right + 1e-3,
                            "{name} path {i} point {k}: racing line {} outside [-{}, {}]",
                            p.racing_line,
                            p.half_width_left,
                            p.half_width_right
                        );

                        if k > 0 {
                            longest = longest.max(distance(spline.points[k - 1].pos, p.pos));
                        }
                    }

                    assert!(
                        (spline.max_spacing - longest).abs() <= longest * 1e-5,
                        "{name} path {i}: stored spacing {} is not the longest gap {longest}",
                        spline.max_spacing
                    );

                    // Geometry has to agree with topology: a path that leaves a
                    // junction must begin roughly where its predecessor ended,
                    // within one control-point step. This is what turns the
                    // junction slot ordering from a plausible reading into a
                    // fact -- read the slots the other way round and the ends
                    // land hundreds of units apart.
                    let first = spline.points[0].pos;
                    let predecessors = ai.junctions[entry_j].prev.iter().flatten();
                    for &p in predecessors {
                        let previous = ai.paths[p].points.last().expect("a path with points").pos;
                        let gap = distance(previous, first);
                        let budget = spline.max_spacing.max(ai.paths[p].max_spacing) * 1.5;
                        assert!(
                            gap <= budget,
                            "{name}: path {p} ends {gap:.1} from the start of path {i}, \
                             which junction {entry_j} says follows it (budget {budget:.1})"
                        );
                    }
                }
            }
            assert_eq!(splines, 1, "{name}: expected exactly one WO Track node");
        }
    }

    println!("{checked} tracks, {points_total} control points");
    assert!(
        checked >= MIN_TRACKS,
        "only {checked} tracks checked; the disc should carry at least {MIN_TRACKS}"
    );
}

/// Class ID of a `WO Track` node, from `docs/formats/track.md`.
const CLASS_WO_TRACK: u32 = 0x3bb;

/// Every decoded vertex must fall inside its batch's declared bounding box.
///
/// This is the check `docs/formats/vex.md` describes and, until now, only ever
/// ran by hand: the mesh carries its own extent, so a wrong vertex stride or a
/// wrong scale factor puts positions outside it immediately. Track models are the
/// interesting case, because they use a 16-bit colour format the ship models
/// never do, and getting the stride wrong there decodes position bytes out of the
/// middle of a colour.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn decoded_vertices_stay_inside_their_declared_bounds() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
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

    // A track and a ship: different vertex types, different scales.
    let models = [
        "Data\\Environments\\01_Track\\track.vex",
        "Data\\Environments\\03_Track\\track.vex",
        "Data\\Ships\\Feisar\\Ship.vex",
        "Data\\Ships\\Qirex\\Ship.vex",
    ];

    let mut checked_batches = 0usize;
    let mut checked_vertices = 0usize;
    let mut types = std::collections::BTreeSet::new();

    for name in models {
        let hash = wad::hash_name(name);
        let Some(entry) = dir.entries.iter().find(|e| e.name_hash == hash) else {
            continue;
        };
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let model = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => panic!("{name}: unexpected zlib entry"),
        };

        let nodes = vex::nodes(&model).expect("nodes");

        // The tree has exactly one root, which is what pins `child_count` to 16
        // bits: as a `u32` the counts sum to millions.
        let children: usize = nodes.iter().map(|n| n.child_count).sum();
        assert_eq!(
            nodes.len() - children,
            1,
            "{name}: {} nodes with {children} children is not one tree",
            nodes.len()
        );

        for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
            let payload = &model[node.payload()];
            for list in 0..2 {
                let batches = vex::mesh_batches(payload, list)
                    .unwrap_or_else(|e| panic!("{name}: decoding batch list {list}: {e}"));
                for batch in batches {
                    types.insert(batch.vertex_type);
                    let (lo, hi) = batch.bounds;
                    // The box is stored as s16 in the same space as the
                    // positions, so allow one quantisation step of slack.
                    let slack =
                        (0..3).map(|i| (hi[i] - lo[i]).abs()).fold(0.0f32, f32::max) * 1e-3 + 1e-3;
                    for v in &batch.vertices {
                        for i in 0..3 {
                            assert!(
                                v.position[i] >= lo[i] - slack && v.position[i] <= hi[i] + slack,
                                "{name}: vertex type {:#06x} axis {i}: {} outside [{}, {}]",
                                batch.vertex_type,
                                v.position[i],
                                lo[i],
                                hi[i]
                            );
                        }
                        checked_vertices += 1;
                    }
                    checked_batches += 1;
                }
            }
        }
    }

    println!(
        "{checked_batches} batches, {checked_vertices} vertices, vertex types {:#06x?}",
        types
    );
    assert!(checked_batches > 1000, "only {checked_batches} batches");
    assert!(
        types.len() > 1,
        "one vertex type is not enough to exercise the layout table"
    );
}
