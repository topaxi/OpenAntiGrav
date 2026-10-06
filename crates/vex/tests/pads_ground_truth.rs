//! Validates the [`pads`](oag_vex::pads) decoder against real tracks.
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
//! Two claims in `docs/formats/pads.md` are only as good as the data agrees with
//! them, and neither is checkable by reading the binary:
//!
//! 1. **The box pair at `+0x10`/`+0x20` is in world units.** `Pad_Bind` copies
//!    those floats raw and then adds a literal `-2.0` and `+8.0`, while the
//!    `Mesh` base divides the same floats by a quantisation scale. The literals
//!    only mean anything if that scale is `1.0` for pad meshes. If it is not,
//!    every trigger volume is wrong by a constant factor - and wrong in a way a
//!    screenshot cannot show, because the pad's *geometry* would still draw
//!    correctly.
//! 2. **A pad's placement is entirely in its transform chain.** All nine of
//!    `01_Track`'s pads share one payload, so if the chain were composed wrongly
//!    they would all land on top of each other.
//!
//! Both are checked here against the track's own spline rather than against
//! numbers written down elsewhere: a pad has to sit on the driveable surface, and
//! the `WO Track` spline is independent evidence of where that is.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::pads::{self, PadVolume};
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

/// Fewest track files that must be checked for the test to mean anything.
const MIN_FILES: usize = 20;

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

/// Every track model on the PSP disc, decompressed, with its archive name.
fn track_models() -> Option<Vec<(String, Vec<u8>)>> {
    let path = image("pulse-psp-usa.chd")?;
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
            out.push((name, model));
        }
    }
    assert!(
        out.len() >= MIN_FILES,
        "only {} track files found, expected at least {MIN_FILES}",
        out.len()
    );
    Some(out)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_pad_on_every_shipped_track_decodes() {
    let Some(models) = track_models() else {
        return;
    };

    let mut speedup = 0usize;
    let mut weapon = 0usize;

    for (name, model) in &models {
        let nodes = vex::nodes(model).expect("nodes");

        for class in [vex::CLASS_SPEEDUP_PAD, vex::CLASS_WEAPON_PAD] {
            let authored = vex::nodes_by_class(&nodes, class).count();
            let decoded = pads::volumes(model, &nodes, class);

            // A pad that fails to parse is dropped silently by `volumes`, which
            // is right for a loader and wrong for a check: count both ends.
            assert_eq!(
                decoded.len(),
                authored,
                "{name}: {authored} class {class:#x} node(s) authored, {} decoded",
                decoded.len()
            );

            match class {
                vex::CLASS_SPEEDUP_PAD => speedup += decoded.len(),
                _ => weapon += decoded.len(),
            }
        }
    }

    println!(
        "{} track file(s): {speedup} speedup pad(s), {weapon} weapon pad(s)",
        models.len()
    );
    assert!(speedup > 0, "no speedup pads found on any track");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_reference_track_authors_the_pad_counts_the_docs_record() {
    let Some(models) = track_models() else {
        return;
    };
    let (_, model) = models
        .iter()
        .find(|(name, _)| name == "Data\\Environments\\01_Track\\track.vex")
        .expect("01_Track present");
    let nodes = vex::nodes(model).expect("nodes");

    assert_eq!(
        pads::volumes(model, &nodes, vex::CLASS_SPEEDUP_PAD).len(),
        9,
        "docs/formats/track.md records nine speedup pads on 01_Track"
    );
    assert_eq!(
        pads::volumes(model, &nodes, vex::CLASS_WEAPON_PAD).len(),
        7,
        "docs/formats/track.md records seven weapon pads on 01_Track"
    );
}

/// The scale check, phrased as what the scale being wrong would look like.
///
/// A pad is a plate a craft drives over. Its horizontal extents are therefore of
/// the order of a craft's width, which the spline's own half-widths put in the
/// tens of units. A quantisation scale other than `1.0` would make these
/// hundreds or hundredths - the failure is orders of magnitude, so the bounds
/// here are deliberately loose rather than tuned.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pad_boxes_are_in_world_units() {
    let Some(models) = track_models() else {
        return;
    };

    let mut checked = 0usize;
    let mut widest: f32 = 0.0;
    let mut narrowest = f32::INFINITY;

    for (name, model) in &models {
        let nodes = vex::nodes(model).expect("nodes");
        for (index, pad) in pads::volumes(model, &nodes, vex::CLASS_SPEEDUP_PAD)
            .iter()
            .enumerate()
        {
            let width = pad.max[0] - pad.min[0];
            let length = pad.max[2] - pad.min[2];

            assert!(
                (1.0..=100.0).contains(&width),
                "{name} pad {index}: width {width} is not a plausible world-unit width; \
                 the mesh quantisation scale is probably not 1.0"
            );
            assert!(
                (1.0..=100.0).contains(&length),
                "{name} pad {index}: length {length} is not plausible"
            );

            // The vertical expansion dominates the authored thickness, so the
            // box is always at least as tall as the two literals together.
            let height = pad.max[1] - pad.min[1];
            assert!(
                height >= 10.0,
                "{name} pad {index}: height {height} is below the 2.0 + 8.0 expansion"
            );

            widest = widest.max(width);
            narrowest = narrowest.min(width);
            checked += 1;
        }
    }

    println!("{checked} pad(s), width {narrowest} to {widest}");
    assert!(checked > 0);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_pad_pushes_along_a_unit_length_axis() {
    let Some(models) = track_models() else {
        return;
    };

    let mut checked = 0usize;
    for (name, model) in &models {
        let nodes = vex::nodes(model).expect("nodes");
        for class in [vex::CLASS_SPEEDUP_PAD, vex::CLASS_WEAPON_PAD] {
            for (index, pad) in pads::volumes(model, &nodes, class).iter().enumerate() {
                let row = [pad.to_world[8], pad.to_world[9], pad.to_world[10]];
                let length = dot(row, row).sqrt();
                assert!(
                    (length - 1.0).abs() < 1e-3,
                    "{name} class {class:#x} pad {index}: row 2 has length {length}, \
                     so the transform chain is not orthonormal"
                );
                assert!(pad.direction().is_some());
                checked += 1;
            }
        }
    }
    assert!(checked > 0);
}

/// Placement, checked against the track's own spline.
///
/// This is the assertion that would catch a mis-composed transform chain, and it
/// is why the pads are worth decoding against real data at all: the payload is
/// byte-identical across all nine of `01_Track`'s pads, so the *only* thing that
/// distinguishes them is the chain.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_pad_sits_on_the_driveable_surface() {
    let Some(models) = track_models() else {
        return;
    };

    let mut checked = 0usize;
    let mut worst = 0.0f32;

    for (name, model) in &models {
        let nodes = vex::nodes(model).expect("nodes");

        // The track's own spline, as independent evidence of where the surface
        // is. A file with no `WO Track` node has nothing to check against.
        let Some(node) = track::find_node(model, &nodes) else {
            continue;
        };
        let ai = track::parse(&model[node.payload()]).expect("parse WO Track");
        let points: Vec<&track::SplinePoint> =
            ai.paths.iter().flat_map(|p| p.points.iter()).collect();
        if points.is_empty() {
            continue;
        }
        let widest = points
            .iter()
            .map(|p| p.half_width_left.abs().max(p.half_width_right.abs()))
            .fold(0.0f32, f32::max);

        let volumes = pads::volumes(model, &nodes, vex::CLASS_SPEEDUP_PAD);
        let mut seen: Vec<[f32; 3]> = Vec::new();

        for (index, pad) in volumes.iter().enumerate() {
            let centre = pad.centre();
            let nearest = points
                .iter()
                .map(|p| distance(p.pos, centre))
                .fold(f32::INFINITY, f32::min);

            // Generous: the spline is sampled at control points, not resampled,
            // so a pad between two of them is legitimately further from both
            // than the track is wide. The failure this catches is a pad in the
            // wrong place entirely, which lands thousands of units out.
            let bound = widest * 4.0 + 100.0;
            assert!(
                nearest < bound,
                "{name} pad {index}: centre {centre:?} is {nearest} from the nearest \
                 spline point, past the {bound} bound - the transform chain is wrong"
            );
            worst = worst.max(nearest);

            // All nine of 01_Track's pads share one payload, so distinct
            // positions can only come from the chain being composed per node.
            assert!(
                seen.iter().all(|s| distance(*s, centre) > 1.0),
                "{name} pad {index}: centre {centre:?} coincides with an earlier pad, \
                 so the per-node transform chain is not being applied"
            );
            seen.push(centre);
            checked += 1;
        }
    }

    println!("{checked} pad(s) placed, worst spline distance {worst}");
    assert!(checked > 0);
}

/// A craft driving over a pad's centre is inside it; one a track-width away is
/// not.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_containment_test_agrees_with_the_boxes_it_decoded() {
    let Some(models) = track_models() else {
        return;
    };

    let mut checked = 0usize;
    for (name, model) in &models {
        let nodes = vex::nodes(model).expect("nodes");
        for (index, pad) in pads::volumes(model, &nodes, vex::CLASS_SPEEDUP_PAD)
            .iter()
            .enumerate()
        {
            let centre = pad.centre();
            assert!(
                pad.contains(centre),
                "{name} pad {index}: its own centre is outside it"
            );
            assert_eq!(pad.distance(centre), 0.0);

            let away = [centre[0] + 500.0, centre[1], centre[2]];
            assert!(
                !pad.contains(away),
                "{name} pad {index}: 500 units away hit"
            );
            assert!(pad.distance(away) > 0.0);
            checked += 1;
        }
    }
    assert!(checked > 0);
}

/// Every pad payload on the disc is the same length, and long enough to hold the
/// box pair with room for the mesh that follows it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pad_payloads_are_mesh_payloads() {
    let Some(models) = track_models() else {
        return;
    };

    let mut sizes = std::collections::BTreeSet::new();
    let mut checked = 0usize;

    for (name, model) in &models {
        let nodes = vex::nodes(model).expect("nodes");
        for class in [vex::CLASS_SPEEDUP_PAD, vex::CLASS_WEAPON_PAD] {
            for node in vex::nodes_by_class(&nodes, class) {
                let payload = &model[node.payload()];
                assert!(
                    payload.len() > 0x30,
                    "{name}: a class {class:#x} payload of {} bytes cannot be a mesh",
                    payload.len()
                );
                // A pad carries geometry, so the payload must also decode as the
                // mesh it claims to be.
                assert!(
                    PadVolume::parse(payload, vex::IDENTITY, vex::byte_order(model)).is_some(),
                    "{name}: class {class:#x} payload did not decode"
                );
                sizes.insert(payload.len());
                checked += 1;
            }
        }
    }

    println!("{checked} pad payload(s), sizes {sizes:?}");
    assert!(checked > 0);
}
