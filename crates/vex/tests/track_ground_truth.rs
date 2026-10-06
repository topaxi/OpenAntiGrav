//! Validates the [`track`](oag_vex::track) decoder against real tracks.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
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

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::{track, vex};

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
    oag_testdata::image(name)
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

/// What a track says about where a ship starts, checked on every shipped file.
///
/// Four separate claims, and each one is load-bearing somewhere else:
///
/// - **Exactly one `Start Position` per track.** This is what says a grid is not
///   authored. `oag_gameplay::spawn` places a ship on the one slot that exists
///   and refuses to derive the other seven, and that refusal is only right if
///   there really is one.
/// - **The rows are the craft's own left-up-forward basis**, satisfying
///   `cross(left, up) = forward` - the same identity the recorded craft basis
///   satisfies on 200 of 200 ticks. Read the rows as right-up-forward instead and
///   a ship spawns mirrored.
/// - **The bind's fix-up is small but not empty.** `track::start_position`
///   forces up to world `(0, 1, 0)`, which is a change on some tracks; this
///   bounds how big a change, so that *which* of the other two rows the original
///   preserves - unread - is a bounded uncertainty rather than an open one.
/// - **The slot is off the centreline, on every track.** That is the evidence
///   that it is a grid slot rather than a start-line marker, and it is why
///   `docs/formats/track.md` no longer reads it as the latter.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_shipped_track_authors_exactly_one_start_position() {
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
    let mut worst_tilt = 0.0f32;
    let mut nearest_centreline = f32::INFINITY;
    let mut furthest_centreline = 0.0f32;

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

            let nodes = vex::nodes(&model).expect("nodes");
            let slots: Vec<_> = nodes
                .iter()
                .filter(|n| n.class_id == vex::CLASS_START_POSITION)
                .collect();
            assert_eq!(
                slots.len(),
                1,
                "{name}: {} Start Position node(s); a grid is not authored, so this must be one",
                slots.len()
            );

            let payload = &model[slots[0].payload()];
            assert_eq!(
                payload.len(),
                track::START_POSITION_LEN,
                "{name}: a Start Position payload is a 4x4 matrix"
            );

            let authored = vex::transform(payload, vex::byte_order(&model))
                .expect("a 64-byte payload is a matrix");
            let row = |r: usize| [authored[r * 4], authored[r * 4 + 1], authored[r * 4 + 2]];
            let (left, up, forward) = (row(0), row(1), row(2));
            let expected = cross(unit(left), unit(up));
            assert!(
                distance(expected, unit(forward)) < 1e-5,
                "{name}: cross(row0, row1) is {expected:?} and row2 is {forward:?}, so the rows \
                 are not the craft's left-up-forward basis"
            );

            let tilt = dot(unit(up), [0.0, 1.0, 0.0]).clamp(-1.0, 1.0).acos();
            worst_tilt = worst_tilt.max(tilt.to_degrees());

            let slot = track::start_position(payload, vex::byte_order(&model))
                .expect("a non-degenerate authored frame");
            assert_eq!(slot.up, [0.0, 1.0, 0.0], "{name}: the bind forces up");
            assert!(
                (dot(slot.forward, slot.forward).sqrt() - 1.0).abs() < 1e-5,
                "{name}: forward is not unit length"
            );
            assert!(
                dot(slot.forward, slot.up).abs() < 1e-6,
                "{name}: forward is not perpendicular to the forced up"
            );

            // Off the centreline, and inside the track. The nearest control point
            // rather than a resampled curve: at these spacings the two differ by
            // far less than the margins asserted here.
            let wo = nodes
                .iter()
                .find(|n| n.class_id == CLASS_WO_TRACK)
                .expect("a track has a WO Track node");
            let ai = track::parse(&model[wo.payload()]).expect("parse WO Track");
            let nearest = ai
                .paths
                .iter()
                .flat_map(|path| path.points.iter())
                .min_by(|a, b| {
                    distance(a.pos, slot.position).total_cmp(&distance(b.pos, slot.position))
                })
                .expect("a track has control points");

            let offset = [
                slot.position[0] - nearest.pos[0],
                slot.position[1] - nearest.pos[1],
                slot.position[2] - nearest.pos[2],
            ];
            let lateral = dot(offset, unit(nearest.lateral));
            nearest_centreline = nearest_centreline.min(lateral.abs());
            furthest_centreline = furthest_centreline.max(lateral.abs());

            assert!(
                lateral.abs() > 1.0,
                "{name}: the slot is {lateral:.2} off the centreline, which would make it a \
                 start-line marker rather than a grid slot"
            );
            assert!(
                lateral >= -nearest.half_width_left - 1e-3
                    && lateral <= nearest.half_width_right + 1e-3,
                "{name}: the slot is {lateral:.2} off the centreline, outside the track's own \
                 [-{}, {}]",
                nearest.half_width_left,
                nearest.half_width_right
            );

            checked += 1;
        }
    }

    println!(
        "{checked} tracks: authored up is off world up by at most {worst_tilt:.3} degrees, and \
         the slot sits {nearest_centreline:.2}-{furthest_centreline:.2} off the centreline"
    );
    assert!(
        checked >= MIN_TRACKS,
        "only {checked} tracks checked; the disc should carry at least {MIN_TRACKS}"
    );
    // The bound the parser's own documentation quotes. A file that broke it would
    // make the unread half of the bind's fix-up matter, which is the point of
    // measuring rather than assuming.
    assert!(
        worst_tilt < 2.0,
        "an authored up row is {worst_tilt:.3} degrees off world up, so which row the bind's \
         re-orthonormalisation preserves is no longer a detail"
    );
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    [v[0] / length, v[1] / length, v[2] / length]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

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
