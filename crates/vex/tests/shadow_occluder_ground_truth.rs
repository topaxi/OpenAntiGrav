//! Pins what the Pulse and Pure PSP discs author for shadow, and establishes
//! that a `Dynamic Shadow Occluder` `0x3c3` payload **closes** at
//! `0x50 + 32n + 16m`.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! `docs/rendering/shadows.md` plans four shadow techniques behind one setting,
//! chosen by what each disc authors. Every number that page states is asserted
//! here, so a survey that covers less shows up as a count that moved.
//!
//! # Why closure, and not "the decoder returned something"
//!
//! **`vex::mesh_batches(payload, 0).is_ok()` is `true` on all 129 of these
//! payloads and means nothing.** The `Mesh` closure test (the material count at
//! `+0x02` placing the material array's end at `+0x04`'s geometry offset)
//! **fails on 129 of 129**: here `+0x02` is not a material count and `+0x04` is
//! a stale PSP main-RAM pointer (`skycube_ground_truth.rs` warns of this and it
//! still caught someone). The layout claim rests on its own closure:
//!
//! So the layout claim here rests on its own closure instead:
//!
//! > `n` at `+0x00` and `m` at `+0x02` satisfy
//! > `0x50 + 32n + 16m == payload.len()`.
//!
//! Checked over twelve distinct `(n, m)` pairs and payload lengths from 272 to
//! 4432 bytes; a wrong stride does not fit twelve independent pairs.
//!
//! # What the sweep found
//!
//! - **`shadow` `0x3cb`, `blob` `0x3e0` and `textureBlob` `0x3df` are authored
//!   zero times**: three of the four roadmap shadow classes are inert.
//! - **`Dynamic Shadow Occluder` `0x3c3` is the real mechanism**: 129 nodes.
//! - **Two populations.** 119 sit at their own origin and are named
//!   (`shadowShape`, `shadow_lodShape`, `shadow_agsShape`, `shadow_mineShape`);
//!   10 sit at track coordinates, unnamed: a craft's shadow hull and a track-side
//!   occluder are different features.
//! - **Wipeout Pure authors none**, so its shadow tier is honest absence.
//!
//! # The records are a convex hull: `n` planes and `m` vertices
//!
//! - A **32-byte face record** opens with a unit plane normal (3 x `f32`,
//!   129/129), a `u32` at `+0x0c` for the face's vertex count (3 or more, never
//!   more than the hull owns), then indices.
//! - A **16-byte vertex record** is `(w, x, y, z)`, **`w` first** and `1.0` on
//!   every record on both discs (a homogeneous point).
//! - **`m` counts slots, not vertices**: 150 slots across the Pulse disc sit at
//!   the origin, spare.
//!
//! `pulse_mine` decodes to a tetrahedron (one normal straight down, three
//! up-and-outward at 120 degrees, over four vertices whose extent is *exactly*
//! the declared box); `pulse_bomb` is a pentagonal frustum, five vertices at
//! `y = 2.6341` over five at `y = 2.1059`, two spare slots.
//!
//! **The box is authored, not derived**, so containment is asserted disc-wide:
//! `BEData.wad#20` is a flat hull with all eight vertices at `y = -0.06195458`
//! declaring its `y` maximum as the denormal `0x00800000`. See
//! `docs/rendering/shadows.md`.

use std::path::{Path, PathBuf};

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::vex;

/// Class id of `Dynamic Shadow Occluder`, from the class-ID table at `0x08ab2370`.
const CLASS_OCCLUDER: u32 = 0x3c3;

/// The three effect classes that sound like a shadow and hold nothing (`shadow`,
/// `textureBlob`, `blob`): asserted absent so authoring one on an unseen disc
/// fails loudly.
const INERT_CLASSES: [(u32, &str); 3] =
    [(0x3cb, "shadow"), (0x3df, "textureBlob"), (0x3e0, "blob")];

/// Length of the occluder payload's header, before its two record arrays.
const HEADER_LEN: usize = 0x50;

/// Stride of a record in the first array, whose count is at `+0x00`.
const RECORD_A_STRIDE: usize = 32;

/// Stride of a record in the second array, whose count is at `+0x02`.
const RECORD_B_STRIDE: usize = 16;

/// `Dynamic Shadow Occluder` nodes across the Pulse PSP disc.
const PSP_OCCLUDERS: usize = 129;

/// How many `.vex` files author at least one.
const PSP_OCCLUDER_FILES: usize = 83;

/// Of [`PSP_OCCLUDERS`], how many carry a node name.
const PSP_OCCLUDERS_NAMED: usize = 119;

/// Of [`PSP_OCCLUDERS`], how many have a bounding box centred at their own
/// origin - a craft's or a pickup's own hull, rather than a track-side one.
const PSP_OCCLUDERS_LOCAL_SPACE: usize = 119;

/// How many repeat their bounding box as two padded `vec4`s at `+0x30`, within
/// this test's `1e-6` tolerance.
///
/// **The tolerance hides a mixed population**: the denormal sentinel
/// `0x00800000` is `1.1754944e-38`, so a node whose packed `max.y` is the
/// sentinel and whose padded `max.y` is a hard `0.0` differs by ~`1e-38` and
/// counts as a "match" though it is a hard-zero substitution, not a copy. How
/// many of the 97 is computable but not computed.
///
/// The padded box is doing something shadow-relevant (most divergence pulls it
/// toward the ground plane) but follows no clean exceptionless rule, and an
/// earlier overclaim was corrected (2026-09-03/04, `examples/shadow_padding_probe.rs`):
/// 12 of 14 nodes with a positive header `min.y` get `+0x30`'s floored to `0.0`;
/// of 54 with the sentinel `max.y`, `+0x40`'s keeps the true value on 16 and is a
/// hard `0.0` on 38. Not asserted here (circular on `vertex_extent`). See
/// `docs/ghidra/functions/psp-pulse-usa/shadow-occluder.md` for the readout and
/// the correction; `Shadow_RenderOccluderVolume` reads this field, not the packed
/// one at `+0x0c`/`+0x18`, for its support-point step.
const PSP_OCCLUDERS_WITH_PADDED_BBOX: usize = 97;

/// Every distinct node name across the named population, sorted. `shadow_agsShape`
/// names a team and `shadow_mineShape` a weapon: the named population is
/// per-object, not per-track.
const PSP_OCCLUDER_NAMES: [&str; 6] = [
    "shadow1Shape",
    "shadowShape",
    "shadow_agsShape",
    "shadow_lodShape",
    "shadow_mineShape",
    "shadowlodShape",
];

/// A bounding-box centre further than this from the node's origin makes it
/// world-space. The populations are far apart (local within a couple of units,
/// world hundreds out), so the value is not load-bearing.
const WORLD_SPACE_CENTRE: f32 = 50.0;

/// The `.vex` version Pulse ships throughout.
const PULSE_VERSIONS: [u32; 1] = [6];

/// The two Pure ships: version 4, with 15 version-3 files among them.
///
/// **Both are searched for Pulse's own class ids, soundly**: the exporter's 22
/// class names are one contiguous run in `BOOT.BIN`, identical on both titles,
/// ids consecutive along it ([`pure-status.md`](../../../docs/formats/pure-status.md));
/// Pure's version-3 files share version 4's class-ID space.
const PURE_VERSIONS: [u32; 2] = [3, 4];

/// Version-6 `.vex` files on the Pulse PSP disc.
///
/// Asserted in every sweep, so a walk that covers less fails on the count rather
/// than on a census that reads zero.
const PULSE_VEX_FILES: usize = 382;

/// Vertex slots across the Pulse disc at the origin, in no hull (`m` is an
/// allocation, not a vertex count); pinned so a decode explaining them shows up.
const PSP_PADDING_VERTICES: usize = 150;

/// Version-3-and-4 `.vex` files on the Pure PSP disc, for the same reason.
const PURE_VEX_FILES: usize = 222;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

fn u16_at(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

fn f32_at(data: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

/// One decoded `.vex` file's worth of raw bytes.
struct VexFile {
    label: String,
    bytes: Vec<u8>,
    tree: Vec<vex::Node>,
}

/// Every `.vex` file of an accepted version in every `.wad` on the disc,
/// decompressed and walked.
///
/// **Every archive, not just `Data.wad`**: the named occluders live in both it
/// and `BEData.wad`, and one alone silently halves the population (reading as
/// "the front end has no shadows").
///
/// **The version filter is a parameter**: Pulse is version 6, Pure version 4 with
/// 15 version-3 files ([`pure-status.md`]). Hard-coding 6 walks **zero** Pure
/// files, indistinguishable from Pure authoring none, the false negative the
/// file-count assertion catches.
fn vex_files(disc: &mut DiscImage, versions: &[u32]) -> Vec<VexFile> {
    let archives: Vec<_> = disc
        .entries()
        .expect("entries")
        .iter()
        .filter(|entry| entry.path.to_ascii_lowercase().ends_with(".wad"))
        .cloned()
        .collect();

    let mut out = Vec::new();
    for archive in archives {
        let header = disc
            .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
            .expect("header");
        let Ok(count) = Directory::peek_entry_count(&header) else {
            continue;
        };
        let Ok(dir_bytes) = disc.read_entry_range(&archive, 0, Directory::directory_len(count))
        else {
            continue;
        };
        let Ok(dir) = Directory::parse(&dir_bytes, Some(archive.size)) else {
            continue;
        };

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
                    match oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize) {
                        Ok(bytes) => bytes,
                        Err(_) => continue,
                    }
                }
                Compression::Zlib => continue,
            };
            if !vex::has_magic(&bytes) {
                continue;
            }
            match vex::version(&bytes) {
                Ok(version) if versions.contains(&version) => {}
                _ => continue,
            }
            let Ok(tree) = vex::nodes(&bytes) else {
                continue;
            };
            out.push(VexFile {
                label: format!("{}#{index}", archive.path),
                bytes,
                tree,
            });
        }
    }
    out
}

/// The vertex array's own component-wise extent, and its declared face count.
///
/// A vertex record is `(w, x, y, z)` with **`w` first** (`1.0` on every record,
/// position at `+0x04`), as the closure below proves. **`m` counts slots, not
/// vertices**: a hull may end in `(1, 0, 0, 0)` records (Pulse's bomb declares
/// twelve and uses ten), skipped here because they drag the extent to zero on
/// every axis the hull does not straddle (how this check first failed).
fn vertex_extent(payload: &[u8], n: usize, m: usize) -> ([f32; 3], [f32; 3], usize) {
    let base = HEADER_LEN + n * RECORD_A_STRIDE;
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let mut used = 0usize;
    for i in 0..m {
        let at = base + i * RECORD_B_STRIDE;
        let p = [
            f32_at(payload, at + 4),
            f32_at(payload, at + 8),
            f32_at(payload, at + 12),
        ];
        if p == [0.0, 0.0, 0.0] {
            continue;
        }
        used += 1;
        for axis in 0..3 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    (min, max, used)
}

/// The packed bounding box at `+0x0c` (min) and `+0x18` (max).
fn bounds(payload: &[u8]) -> ([f32; 3], [f32; 3]) {
    let mut min = [0.0; 3];
    let mut max = [0.0; 3];
    for axis in 0..3 {
        min[axis] = f32_at(payload, 0x0c + axis * 4);
        max[axis] = f32_at(payload, 0x18 + axis * 4);
    }
    (min, max)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn an_occluder_payload_closes_at_a_header_and_two_record_arrays() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let files = vex_files(&mut disc, &PULSE_VERSIONS);
    assert_eq!(files.len(), PULSE_VEX_FILES, "Pulse .vex files walked");

    let mut occluders = 0usize;
    let mut files_with = 0usize;
    let mut padded_bbox = 0usize;
    let mut padded = 0usize;

    for file in &files {
        let mut here = 0usize;
        for node in vex::nodes_by_class(&file.tree, CLASS_OCCLUDER) {
            here += 1;
            occluders += 1;
            let label = format!("{} {:?}", file.label, node.name);
            let payload = &file.bytes[node.payload()];

            assert!(
                payload.len() >= HEADER_LEN,
                "{label}: {} bytes, shorter than the header",
                payload.len()
            );

            // The closure: `n` and `m` are the only counts in the header and
            // together account for the payload exactly.
            let n = usize::from(u16_at(payload, 0));
            let m = usize::from(u16_at(payload, 2));
            assert_eq!(
                HEADER_LEN + RECORD_A_STRIDE * n + RECORD_B_STRIDE * m,
                payload.len(),
                "{label}: n={n} m={m} does not close on {} bytes",
                payload.len()
            );

            // Two constants, the same on every occluder; what they select is unread,
            // pinned so a differing disc says so.
            assert_eq!(u32_at(payload, 0x24), 5, "{label}: +0x24");
            assert_eq!(f32_at(payload, 0x28), 1.0, "{label}: +0x28");

            // A real box, not padding that happens to parse.
            let (min, max) = bounds(payload);
            for axis in 0..3 {
                assert!(
                    min[axis].is_finite() && max[axis].is_finite() && min[axis] <= max[axis],
                    "{label}: bounding box axis {axis} is {}..{}",
                    min[axis],
                    max[axis]
                );
            }

            // **Every vertex lies inside the declared box**, the disc-wide form (the
            // stronger "box *is* the extent" holds on hand-checked hulls, not every
            // node: `BEData.wad#20` is flat and declares `y` max as the denormal
            // `0x00800000`, so the box is *authored*). Containment still confirms
            // the stride, array offset and position placement at once.
            let (vmin, vmax, used) = vertex_extent(payload, n, m);
            assert!(used >= 3, "{label}: only {used} non-origin vertices of {m}");
            padded += m - used;
            for axis in 0..3 {
                let tol = 1e-3 * min[axis].abs().max(max[axis].abs()).max(1.0);
                assert!(
                    vmin[axis] >= min[axis] - tol && vmax[axis] <= max[axis].max(0.0) + tol,
                    "{label}: axis {axis} vertices span {}..{} outside the declared {}..{}",
                    vmin[axis],
                    vmax[axis],
                    min[axis],
                    max[axis]
                );
            }

            // Every vertex record's `w` is exactly 1.0 - a homogeneous point.
            for i in 0..m {
                let at = HEADER_LEN + n * RECORD_A_STRIDE + i * RECORD_B_STRIDE;
                assert_eq!(f32_at(payload, at), 1.0, "{label}: vertex {i} w");
            }

            // A face's vertex count is a `u32` at `+0x0c`: three or more, never more
            // than the hull has vertices.
            for i in 0..n {
                let count = u32_at(payload, HEADER_LEN + i * RECORD_A_STRIDE + 0x0c) as usize;
                assert!(
                    (3..=m).contains(&count),
                    "{label}: face {i} claims {count} vertices of {m}"
                );
            }

            // The first three floats of every face record are a unit vector -
            // the plane normal.
            for i in 0..n {
                let at = HEADER_LEN + i * RECORD_A_STRIDE;
                let (x, y, z) = (
                    f32_at(payload, at),
                    f32_at(payload, at + 4),
                    f32_at(payload, at + 8),
                );
                let length_squared = x * x + y * y + z * z;
                assert!(
                    (0.99..1.01).contains(&length_squared),
                    "{label}: record {i} opens on {x},{y},{z}, length squared {length_squared}"
                );
            }

            // The box repeated as two padded `vec4`s, on most but not all.
            let repeats = (0..3).all(|axis| {
                (f32_at(payload, 0x30 + axis * 4) - min[axis]).abs() < 1e-6
                    && (f32_at(payload, 0x40 + axis * 4) - max[axis]).abs() < 1e-6
            });
            if repeats {
                padded_bbox += 1;
            }
        }
        if here > 0 {
            files_with += 1;
        }
    }

    assert_eq!(occluders, PSP_OCCLUDERS, "occluder nodes");
    assert_eq!(files_with, PSP_OCCLUDER_FILES, "files authoring one");
    assert_eq!(
        padded_bbox, PSP_OCCLUDERS_WITH_PADDED_BBOX,
        "occluders repeating their box at +0x30"
    );
    assert_eq!(
        padded, PSP_PADDING_VERTICES,
        "unused vertex slots disc-wide"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn the_occluders_are_two_populations_and_only_one_is_per_craft() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let files = vex_files(&mut disc, &PULSE_VERSIONS);
    assert_eq!(files.len(), PULSE_VEX_FILES, "Pulse .vex files walked");

    let mut named = 0usize;
    let mut local_space = 0usize;
    let mut names = Vec::new();

    for file in &files {
        for node in vex::nodes_by_class(&file.tree, CLASS_OCCLUDER) {
            if let Some(name) = &node.name {
                named += 1;
                names.push(name.clone());
            }
            let (min, max) = bounds(&file.bytes[node.payload()]);
            let centre = (0..3)
                .map(|axis| ((min[axis] + max[axis]) * 0.5).abs())
                .fold(0.0f32, f32::max);
            if centre <= WORLD_SPACE_CENTRE {
                local_space += 1;
            }
        }
    }

    names.sort();
    names.dedup();

    assert_eq!(named, PSP_OCCLUDERS_NAMED, "named occluders");
    assert_eq!(
        local_space, PSP_OCCLUDERS_LOCAL_SPACE,
        "occluders centred at their own origin"
    );
    assert_eq!(names, PSP_OCCLUDER_NAMES, "distinct occluder node names");
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn the_three_classes_that_sound_like_shadow_are_authored_zero_times() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let files = vex_files(&mut disc, &PULSE_VERSIONS);
    assert_eq!(files.len(), PULSE_VEX_FILES, "Pulse .vex files walked");

    for (class_id, name) in INERT_CLASSES {
        let count: usize = files
            .iter()
            .map(|file| vex::nodes_by_class(&file.tree, class_id).count())
            .sum();
        assert_eq!(count, 0, "{name} {class_id:#05x} is authored {count} times");
    }
}

#[test]
#[ignore = "needs data/images/pure-psp-eu.chd; run with `just test-data`"]
fn wipeout_pure_authors_no_shadow_class_at_all() {
    let Some(path) = image("pure-psp-eu.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let files = vex_files(&mut disc, &PURE_VERSIONS);
    assert_eq!(files.len(), PURE_VEX_FILES, "Pure .vex files walked");

    for (class_id, name) in INERT_CLASSES
        .into_iter()
        .chain([(CLASS_OCCLUDER, "Dynamic Shadow Occluder")])
    {
        let count: usize = files
            .iter()
            .map(|file| vex::nodes_by_class(&file.tree, class_id).count())
            .sum();
        assert_eq!(count, 0, "{name} {class_id:#05x} is authored {count} times");
    }
}

/// The three PSARC packages an EU 2048 install carries, base first (under
/// `data/extracted/`: 2048 arrives as an encrypted `.pkg`, decrypted in four
/// steps, see `data/README.md`).
const VITA_PACKAGES: [&str; 3] = [
    "vita/PCSF00007/base/PSP2/data.psarc",
    "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn package(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted")
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

/// `.vex` files across 2048's three EU packages.
///
/// Pinned as [`PULSE_VEX_FILES`] is: the answer below is a **zero**, and a walk
/// covering nothing gives the same zero.
const VITA_VEX_FILES: usize = 1158;

/// `Dynamic Shadow Occluder` nodes 2048 authors.
const VITA_OCCLUDERS: usize = 6;

/// The six, by archive path and node name, with the `(n, m)` each declares.
///
/// **Three weapons, shipped twice** (under `data/Weapons/` and
/// `data/Weapons2048/`), their `(n, m, len)` triples the same as Pulse's
/// `BEData.wad` carries for them: the format did not change between a 2007 PSP
/// title and a 2012 Vita one.
const VITA_OCCLUDER_ROWS: [(&str, &str, usize, usize, usize); 6] = [
    ("data/Weapons/pulse_bomb.vex", "shadowShape", 11, 12, 624),
    ("data/Weapons/pulse_mine.vex", "shadow_mineShape", 4, 4, 272),
    ("data/Weapons/pulse_shuriken.vex", "shadowShape", 8, 8, 464),
    (
        "data/Weapons2048/pulse_bomb.vex",
        "shadowShape",
        11,
        12,
        624,
    ),
    (
        "data/Weapons2048/pulse_mine.vex",
        "shadow_mineShape",
        4,
        4,
        272,
    ),
    (
        "data/Weapons2048/pulse_shuriken.vex",
        "shadowShape",
        8,
        8,
        464,
    ),
];

/// 2048 authors the occluder for **three weapons and no craft**, and the payload
/// closes by Pulse's own formula.
///
/// Its `.vex` is version 6 little-endian with HD's class ids
/// ([`2048-status.md`](../../../docs/formats/2048-status.md)), so the ids searched
/// are Pulse's and the search is meaningful, not vacuous (the trap of Pure's first
/// sweep).
///
/// **The expected answer was zero and the disc said six.** The
/// `0x50 + 32n + 16m` closure is a two-title, two-platform result, and 2048's
/// *ship* shadows are not this class: they are its `track_proximity_shadow` pair
/// and precomputed ship environment shadows (`docs/rendering/shadows.md`).
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn wipeout_2048_carries_three_weapon_occluders_that_close_the_same_way() {
    let classes = [
        (CLASS_OCCLUDER, "Dynamic Shadow Occluder"),
        INERT_CLASSES[0],
        INERT_CLASSES[1],
        INERT_CLASSES[2],
    ];
    let mut walked = 0usize;
    let mut counts = [0usize; 4];
    let mut any = false;
    let mut found: Vec<(String, String, usize, usize, usize)> = Vec::new();

    for name in VITA_PACKAGES {
        let Some(path) = package(name) else { continue };
        any = true;
        let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
            .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
        let entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|entry| entry.to_ascii_lowercase().ends_with(".vex"))
            .cloned()
            .collect();
        for entry in entries {
            let Ok(bytes) = archive.read_path(&entry) else {
                continue;
            };
            if !vex::has_magic(&bytes) {
                continue;
            }
            let Ok(tree) = vex::nodes(&bytes) else {
                continue;
            };
            walked += 1;
            for (slot, (class_id, _)) in classes.iter().enumerate() {
                counts[slot] += vex::nodes_by_class(&tree, *class_id).count();
            }
            for node in vex::nodes_by_class(&tree, CLASS_OCCLUDER) {
                let payload = &bytes[node.payload()];
                let n = usize::from(u16_at(payload, 0));
                let m = usize::from(u16_at(payload, 2));
                let label = format!("{entry} {:?}", node.name);

                // Pulse's own closure on another console and endianness: the
                // assertion making the layout a lineage fact, not a one-title reading.
                assert_eq!(
                    HEADER_LEN + RECORD_A_STRIDE * n + RECORD_B_STRIDE * m,
                    payload.len(),
                    "{label}: n={n} m={m} does not close on {} bytes",
                    payload.len()
                );
                assert_eq!(u32_at(payload, 0x24), 5, "{label}: +0x24");
                assert_eq!(f32_at(payload, 0x28), 1.0, "{label}: +0x28");
                for i in 0..n {
                    let at = HEADER_LEN + i * RECORD_A_STRIDE;
                    let length_squared = f32_at(payload, at).powi(2)
                        + f32_at(payload, at + 4).powi(2)
                        + f32_at(payload, at + 8).powi(2);
                    assert!(
                        (0.99..1.01).contains(&length_squared),
                        "{label}: record {i} does not open on a unit vector"
                    );
                }

                found.push((
                    entry.clone(),
                    node.name.clone().unwrap_or_default(),
                    n,
                    m,
                    payload.len(),
                ));
            }
        }
    }
    if !any {
        return;
    }

    assert_eq!(walked, VITA_VEX_FILES, "2048 .vex files walked");
    assert_eq!(
        counts[0], VITA_OCCLUDERS,
        "Dynamic Shadow Occluder nodes on 2048"
    );
    for (slot, (class_id, name)) in classes.iter().enumerate().skip(1) {
        assert_eq!(
            counts[slot], 0,
            "{name} {class_id:#05x} is authored {} times on 2048",
            counts[slot]
        );
    }

    found.sort();
    let expected: Vec<(String, String, usize, usize, usize)> = VITA_OCCLUDER_ROWS
        .iter()
        .map(|(path, name, n, m, len)| ((*path).to_string(), (*name).to_string(), *n, *m, *len))
        .collect();
    assert_eq!(
        found, expected,
        "which files carry an occluder, and its shape"
    );
}
