//! Pins what the Pulse and Pure PSP discs author for shadow, and establishes
//! that a `Dynamic Shadow Occluder` `0x3c3` payload **closes** at
//! `0x50 + 32n + 16m`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The tests skip with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What this is for
//!
//! `docs/rendering/shadows.md` plans four shadow techniques behind one setting,
//! and the choice between them turns entirely on what each disc actually
//! authors. Every number that page states is asserted here, so a survey that
//! quietly covers less shows up as a count that moved rather than as a design
//! built on a stale measurement.
//!
//! # Why closure, and not "the decoder returned something"
//!
//! **`vex::mesh_batches(payload, 0).is_ok()` is `true` on all 129 of these
//! payloads and means nothing.** The first pass at this class read that as
//! "the payload is mesh-shaped" and was wrong: the `Mesh` closure test - the
//! material count at `+0x02` placing the material array's end exactly at
//! `+0x04`'s geometry offset - **fails on 129 of 129**, because on this class
//! `+0x02` is not a material count and `+0x04` is a stale PSP main-RAM
//! pointer. `skycube_ground_truth.rs` warns about precisely this in its own
//! header and it still caught someone.
//!
//! So the layout claim here rests on its own closure instead:
//!
//! > `n` at `+0x00` and `m` at `+0x02` satisfy
//! > `0x50 + 32n + 16m == payload.len()`.
//!
//! That is checked over twelve distinct `(n, m)` pairs and payload lengths from
//! 272 to 4432 bytes. A wrong stride does not fit twelve independent pairs.
//!
//! # What the sweep found
//!
//! - **`shadow` `0x3cb`, `blob` `0x3e0` and `textureBlob` `0x3df` are authored
//!   zero times**, so three of the four classes the roadmap lists under shadow
//!   are inert - the same kind of converged negative `PointLight` already is.
//! - **`Dynamic Shadow Occluder` `0x3c3` is the real mechanism**: 129 nodes.
//! - **Two populations.** 119 sit at their own origin and are named
//!   (`shadowShape`, `shadow_lodShape`, `shadow_agsShape`, `shadow_mineShape`);
//!   10 sit at track coordinates and are unnamed. A craft's shadow hull and a
//!   track-side occluder are not the same feature.
//! - **Wipeout Pure authors none of them at all**, which is why its shadow tier
//!   is honest absence rather than a substitute.
//!
//! # The records are a convex hull: `n` planes and `m` vertices
//!
//! - A **32-byte face record** opens with a unit plane normal (3 x `f32`,
//!   129/129), then a `u32` at `+0x0c` giving how many vertices that face has
//!   (3 or more, never more than the hull owns), then indices.
//! - A **16-byte vertex record** is `(w, x, y, z)` with **`w` first** and
//!   `w == 1.0` on every record on both discs - a homogeneous point.
//! - **`m` counts slots, not vertices.** 150 slots across the Pulse disc sit
//!   at the origin, spare.
//!
//! `pulse_mine` is the clearest case and decodes to a tetrahedron: four unit
//! normals - one straight down, three up-and-outward at 120 degrees - over
//! four vertices whose extent is *exactly* the declared bounding box. Its
//! sibling `pulse_bomb` is a pentagonal frustum, five vertices at
//! `y = 2.6341` over five at `y = 2.1059`, with two spare slots.
//!
//! **The box is authored, not derived**, so containment is what is asserted
//! disc-wide rather than equality: `BEData.wad#20` is a flat hull with all
//! eight vertices at `y = -0.06195458` that declares its `y` maximum as the
//! denormal `0x00800000`. See `docs/rendering/shadows.md`.

use std::path::{Path, PathBuf};

use oag_disc::DiscImage;
use oag_formats::vex;
use oag_formats::wad::{self, Compression, Directory};

/// Class id of `Dynamic Shadow Occluder`, from the class-ID table at
/// `0x08ab2370`.
const CLASS_OCCLUDER: u32 = 0x3c3;

/// The three effect classes that sound like they hold a shadow and hold
/// nothing: `shadow`, `textureBlob`, `blob`. Asserted absent rather than
/// assumed absent, so authoring one on a disc this project has not seen yet
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

/// How many repeat their bounding box as two padded `vec4`s at `+0x30`,
/// within this test's `1e-6` tolerance.
///
/// **The tolerance hides a mixed population inside this number.** The
/// denormal authoring sentinel `0x00800000` is `1.1754944e-38`, so a node
/// whose packed `max.y` is the sentinel and whose padded `max.y` is a hard
/// `0.0` differs by ~`1e-38` - well inside `1e-6` - and counts as a
/// "match" here even though it is one of the hard-zero substitutions
/// described below, not an identical copy. How many of the 97 are that
/// case rather than a true copy is computable but not computed.
///
/// **Read by hand, 2026-09-03, and an earlier pass at this same comment
/// overclaimed a clean rule from too small a sample - corrected the same
/// day.** Not asserted here; the split would make this test depend on
/// `vertex_extent`'s own correctness circularly. Two real, quantified
/// patterns, neither exceptionless:
///
/// - **14 nodes have a positive header `min.y`. 12 of them get `+0x30`'s
///   `min.y` floored to exactly `0.0`** (extending the box down to the
///   ground plane even though the hull's own geometry sits entirely above
///   it - useful for a shadow volume, useless for a geometry cache). The
///   **2 exceptions keep their real, positive `min.y`**: both
///   `shadow_lodShape`, both `Data.wad#809`/`#812`, both the *smallest*
///   positive `min.y` in the set (`0.7280522`) - a sibling `shadowShape` at
///   the same value in the same files *does* get floored. No discriminator
///   found for the exception.
/// - **Where the packed header's `max.y` is the denormal authoring sentinel
///   `0x00800000`** (the same one `BEData.wad#20`'s flat hull declares): 70
///   nodes carry it, 16 of which have a true vertex-derived `max.y` that is
///   itself exactly `0.0` (where the two possible behaviours are
///   indistinguishable and excluded below). **Of the other 54: `+0x40`'s
///   `max.y` carries the true vertex-derived value on 16, and is a hard
///   `0.0` on the other 38.** Tried and ruled out as a discriminator: node
///   name/type (both behaviours occur on both `shadowShape` and
///   `shadow_lodShape`, including the same numeric `max.y` value split
///   both ways across different files), the specific numeric value (14 of
///   17 repeated values agree on kept-vs-hard-zero, 3 don't), and file
///   identity - there is no per-entry build/version field in the WAD
///   directory to begin with, and the 3 exceptions above sit in adjacent
///   file entries anyway. Checked 2026-09-04 with
///   `examples/shadow_padding_probe.rs`; see
///   `docs/ghidra/functions/psp-pulse-usa/shadow-occluder.md` for the full
///   readout, including that the unnamed/world-space population diverges
///   on `x`/`z` rather than `y`.
///
/// So the padded box is doing *something* shadow-relevant rather than
/// nothing - most divergence pulls the box toward the ground plane rather
/// than away from it - but it is not the clean deterministic rule an
/// earlier pass at this comment claimed. See
/// `docs/ghidra/functions/psp-pulse-usa/shadow-occluder.md` for the full
/// readout and the correction; `Shadow_RenderOccluderVolume` reads this
/// exact field, not the packed one at `+0x0c`/`+0x18`, for its support-point
/// step.
const PSP_OCCLUDERS_WITH_PADDED_BBOX: usize = 97;

/// Every distinct node name across the named population, sorted.
///
/// `shadow_agsShape` names a team (AG Systems) and `shadow_mineShape` a weapon,
/// which is the evidence that the named population is per-object rather than
/// per-track.
const PSP_OCCLUDER_NAMES: [&str; 6] = [
    "shadow1Shape",
    "shadowShape",
    "shadow_agsShape",
    "shadow_lodShape",
    "shadow_mineShape",
    "shadowlodShape",
];

/// A bounding-box centre further than this from the node's own origin makes it
/// world-space. The two populations are not near the boundary - the local ones
/// sit within a couple of units and the world ones hundreds out - so the exact
/// value is not load-bearing.
const WORLD_SPACE_CENTRE: f32 = 50.0;

/// The `.vex` version Pulse ships throughout.
const PULSE_VERSIONS: [u32; 1] = [6];

/// The two Pure ships: version 4, with 15 version-3 files among them.
///
/// **Both are searched for Pulse's own class ids, and that is sound.** The
/// exporter's 22 class names are one contiguous run in `BOOT.BIN` that is
/// identical string for string and index for index on both titles, with the
/// ids running consecutively along that order - see
/// [`pure-status.md`](../../../docs/formats/pure-status.md). Pure's version-3
/// files share version 4's class-ID space, which that page also establishes.
const PURE_VERSIONS: [u32; 2] = [3, 4];

/// Version-6 `.vex` files on the Pulse PSP disc.
///
/// Asserted in every test that sweeps it, so a walk that quietly covers less
/// fails on the count rather than on a shadow census that reads zero.
const PULSE_VEX_FILES: usize = 382;

/// Vertex slots across the Pulse disc that sit at the origin and are not part
/// of any hull - `m` is an allocation, not a vertex count. Pinned so a decode
/// that explains what the spare slots are for shows up here.
const PSP_PADDING_VERTICES: usize = 150;

/// Version-3-and-4 `.vex` files on the Pure PSP disc, for the same reason.
const PURE_VEX_FILES: usize = 222;

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
/// **Every archive, not just `Data.wad`.** The named occluders live in both
/// `Data.wad` and `BEData.wad`, and a sweep of one of them silently halves the
/// population - which reads exactly like "the front end has no shadows."
///
/// **The version filter is a parameter because the two discs disagree.** Pulse
/// is version 6 throughout; Pure is version 4 with 15 version-3 files, per
/// [`pure-status.md`]. Hard-coding 6 here walks **zero** Pure files and reports
/// zero occluders, which is indistinguishable from Pure authoring none - the
/// exact false negative the file-count assertion in each test exists to catch.
///
/// [`pure-status.md`]: ../../../docs/formats/pure-status.md
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
/// A vertex record is `(w, x, y, z)` with **`w` first**: `+0x00` is `1.0` on
/// every record on both discs, and the position follows at `+0x04`. That is
/// the ordering the closure below proves, not one assumed from the shape.
/// **`m` counts slots, not vertices.** A hull may end in records that are
/// exactly `(1, 0, 0, 0)` - Pulse's bomb declares twelve and uses ten, the
/// two spare ones sitting at the origin - so those are skipped here. They have
/// to be: including them drags the extent to zero on every axis the hull does
/// not straddle, which is exactly how this check first failed.
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

            // The closure. `n` and `m` are the only two counts in the header,
            // and together they account for the payload exactly.
            let n = usize::from(u16_at(payload, 0));
            let m = usize::from(u16_at(payload, 2));
            assert_eq!(
                HEADER_LEN + RECORD_A_STRIDE * n + RECORD_B_STRIDE * m,
                payload.len(),
                "{label}: n={n} m={m} does not close on {} bytes",
                payload.len()
            );

            // Two constants, the same on every occluder on the disc. What they
            // select is unread; they are pinned so that a disc which differs
            // says so rather than being decoded on this one's assumptions.
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

            // **Every vertex lies inside the declared box**, which is the
            // disc-wide form of the relationship. The stronger form - that the
            // box *is* the vertices' extent - holds on the hulls checked by
            // hand but not on every node here, and the exception is
            // instructive rather than fatal: a flat hull such as
            // `BEData.wad#20` has all eight vertices at `y = -0.06195458` and
            // declares its `y` maximum as the denormal `0x00800000`, so the
            // box is *authored* rather than derived. Asserting containment
            // still confirms the stride, the array offset and the position's
            // placement inside the record all at once.
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

            // A face's vertex count sits at `+0x0c` of its record, as a `u32`
            // and not a float. Three or more, and never more than the hull has
            // vertices to offer.
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

/// The three PSARC packages an EU 2048 install carries, base first.
///
/// Under `data/extracted/` rather than `data/images/`: 2048 arrives as an
/// encrypted `.pkg` and these are the decrypted result, four tool-steps in -
/// see `data/README.md`, "Vita PKGs decrypt in four steps".
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
/// Pinned for the reason [`PULSE_VEX_FILES`] is: the interesting answer below
/// is a **zero**, and a walk that covers nothing produces the same zero.
const VITA_VEX_FILES: usize = 1158;

/// `Dynamic Shadow Occluder` nodes 2048 authors.
const VITA_OCCLUDERS: usize = 6;

/// The six, by archive path and node name, with the `(n, m)` each declares.
///
/// **Three weapons, shipped twice** - once under `data/Weapons/` and once
/// under `data/Weapons2048/` - and their `(n, m, len)` triples are the same
/// ones Pulse's own `BEData.wad` carries for the same three weapons. The
/// format did not change between a 2007 PSP title and a 2012 Vita one.
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

/// 2048 authors the occluder for **three weapons and no craft**, and the
/// payload closes by Pulse's own formula.
///
/// Its `.vex` is version 6 little-endian carrying HD's own class ids
/// ([`2048-status.md`](../../../docs/formats/2048-status.md)), so the ids
/// searched for here are the same ones Pulse uses and the search is meaningful
/// rather than vacuous - the trap that made Pure's first sweep a false
/// negative.
///
/// **The expected answer here was zero and the disc said six.** That matters
/// twice over: it makes the `0x50 + 32n + 16m` closure a two-title,
/// two-platform result rather than a Pulse quirk, and it says 2048's *ship*
/// shadows are not this class - the craft author none. What draws those is
/// 2048's own `track_proximity_shadow` pair and its precomputed ship
/// environment shadows; see `docs/rendering/shadows.md`.
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

                // Pulse's own closure, on a different console and a different
                // endianness. This is the assertion that makes the layout a
                // lineage fact rather than a one-title reading.
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

/// Faces on the Pulse disc, over all 129 occluder nodes.
const PSP_FACES: usize = 4381;

/// How many of those are triangles, and how many quads. Nothing else occurs.
const PSP_TRIANGLES: usize = 3184;
const PSP_QUADS: usize = 1197;

/// Directed edges whose `+0x10` slot names a face, disc-wide.
///
/// Every one of them is reciprocal - see the test. This is the closure that
/// settles the index layout, the same shape the payload length's own is.
const PSP_ADJACENT_EDGES: usize = 14328;

/// Edges carrying `NO_NEIGHBOUR` that are *not* a triangle's degenerate
/// fourth: the boundary edges of the two open hulls, `Data.wad#242` and
/// `#244`.
const PSP_OPEN_EDGES: usize = 12;

/// The worst angle, in degrees, between a triangle's declared plane normal and
/// the normal of the three vertices it indexes.
///
/// **Stated over triangles only, and that is the honest population.** A quad
/// is not planar - 300 of the 1,197 spread further than `1e-4` of their hull's
/// own scale - so the same measurement on a sliver quad reaches 180 degrees
/// and means nothing about the decode. Three points always describe a plane;
/// four authored ones need not.
const PSP_WORST_TRIANGLE_NORMAL_DEGREES: f32 = 0.03;

/// Faces that wind against their own declared normal, and faces with no area
/// at all.
///
/// One each, both named in `oag_formats::shadow_occluder`'s own docs and both
/// **carried rather than rejected**: refusing them would refuse two whole
/// hulls over two faces, and only a caller building a volume can decide what
/// to do with a reversed face.
const PSP_NEGATIVE_WINDING: usize = 1;
const PSP_DEGENERATE_WINDING: usize = 1;

/// The face record's own two index arrays decode, and the edge graph they
/// describe closes on itself.
///
/// **This is the test that settles the layout**, and it rests on two
/// independent facts rather than one reading:
///
/// 1. Adjacency is reciprocal on every one of [`PSP_ADJACENT_EDGES`] edges -
///    the face named across an edge owns that same edge. A wrong stride or a
///    wrong offset does not produce a consistent edge graph on fourteen
///    thousand edges.
/// 2. A triangle's declared normal agrees with the geometry of the three
///    vertices it indexes, to [`PSP_WORST_TRIANGLE_NORMAL_DEGREES`] - two
///    quantities stored in different parts of the record, agreeing.
///
/// Everything else here is a count that moves if the corpus does.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn the_face_records_index_the_vertex_array_and_each_other() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let files = vex_files(&mut disc, &PULSE_VERSIONS);

    let mut faces = 0usize;
    let mut triangles = 0usize;
    let mut quads = 0usize;
    let mut adjacent_edges = 0usize;
    let mut reciprocal = 0usize;
    let mut open_edges = 0usize;
    let mut unowned_open_edges = 0usize;
    let mut worst_degrees = 0.0f32;
    let mut negative = 0usize;
    let mut degenerate = 0usize;

    for file in &files {
        for node in vex::nodes_by_class(&file.tree, CLASS_OCCLUDER) {
            let payload = &file.bytes[node.payload()];
            let label = format!("{} {}", file.label, node.name.clone().unwrap_or_default());
            let hull = oag_formats::shadow_occluder::Occluder::parse(
                payload,
                oag_formats::ByteOrder::Little,
            )
            .unwrap_or_else(|| panic!("{label}: does not close"));

            for (index, face) in hull.faces.iter().enumerate() {
                faces += 1;
                match face.count {
                    3 => triangles += 1,
                    4 => quads += 1,
                    other => panic!("{label}: face {index} claims {other} vertices"),
                }
                for slot in face.indices() {
                    assert!(
                        usize::from(*slot) < hull.vertices.len(),
                        "{label}: face {index} indexes vertex {slot} of {}",
                        hull.vertices.len()
                    );
                }
                let used = usize::from(face.count);
                for edge in 0..used {
                    let from = face.vertices[edge];
                    let to = face.vertices[(edge + 1) % used];
                    let owns = |other: &oag_formats::shadow_occluder::Face| {
                        let count = usize::from(other.count);
                        (0..count).any(|k| {
                            let have = (other.vertices[k], other.vertices[(k + 1) % count]);
                            have == (to, from) || have == (from, to)
                        })
                    };
                    match face.neighbour(edge) {
                        Some(across) => {
                            adjacent_edges += 1;
                            if hull.faces.get(usize::from(across)).is_some_and(owns) {
                                reciprocal += 1;
                            }
                        }
                        // A triangle's fourth edge is `v0 -> v0` and no face
                        // can own it; anything else is a real boundary.
                        None if from == to => {}
                        None => {
                            open_edges += 1;
                            let owned = hull
                                .faces
                                .iter()
                                .enumerate()
                                .any(|(other, candidate)| other != index && owns(candidate));
                            if !owned {
                                unowned_open_edges += 1;
                            }
                        }
                    }
                }

                let winding = face.winding(&hull.vertices);
                if winding < 0.0 {
                    negative += 1;
                } else if winding == 0.0 {
                    degenerate += 1;
                }
                if face.count == 3 {
                    let point = |slot: usize| hull.vertices[usize::from(face.vertices[slot])];
                    let (a, b, c) = (point(0), point(1), point(2));
                    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                    let cross = [
                        ab[1] * ac[2] - ab[2] * ac[1],
                        ab[2] * ac[0] - ab[0] * ac[2],
                        ab[0] * ac[1] - ab[1] * ac[0],
                    ];
                    let length =
                        (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
                    if length > 1e-12 {
                        let cosine = (winding / length).clamp(-1.0, 1.0);
                        worst_degrees = worst_degrees.max(cosine.acos().to_degrees());
                    }
                }
            }
        }
    }

    assert_eq!(faces, PSP_FACES, "faces disc-wide");
    assert_eq!(
        (triangles, quads),
        (PSP_TRIANGLES, PSP_QUADS),
        "face shapes"
    );
    assert_eq!(adjacent_edges, PSP_ADJACENT_EDGES, "edges naming a face");
    assert_eq!(
        reciprocal, PSP_ADJACENT_EDGES,
        "every edge's named face owns that edge"
    );
    assert_eq!(open_edges, PSP_OPEN_EDGES, "real boundary edges");
    assert_eq!(
        unowned_open_edges, PSP_OPEN_EDGES,
        "an edge marked NO_NEIGHBOUR is owned by no other face"
    );
    assert!(
        worst_degrees < PSP_WORST_TRIANGLE_NORMAL_DEGREES,
        "a triangle's declared normal is {worst_degrees} degrees off its own geometry"
    );
    assert_eq!(negative, PSP_NEGATIVE_WINDING, "faces winding backwards");
    assert_eq!(degenerate, PSP_DEGENERATE_WINDING, "faces with no area");
}
