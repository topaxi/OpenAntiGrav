//! Validates the [`lighting`](oag_vex::lighting) decoder against real
//! tracks, and surveys the full disc for how far the two light classes that
//! can appear off-track (`AmbientLight`, `DirectionalLight`) actually spread.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What this is for
//!
//! `docs/formats/lighting.md` claims three fixed-length payloads decode
//! cleanly against real data - 16 bytes for `AmbientLight`/`DirectionalLight`,
//! 32 for `PointLight` - and that placement is the transform chain, not the
//! payload. Two sweeps check that:
//!
//! - **Primary**: every one of `Data.wad`'s 1142 entries, unfiltered by name,
//!   because `AmbientLight` and `DirectionalLight` are generic scene classes
//!   that may be authored off-track. It does find more than the 40-track-file
//!   figures below - 106 `AmbientLight` and 114 `DirectionalLight` against 74
//!   and 86, mostly single instances on non-track `.vex` files - while
//!   `PointLight` and `Dynamic Point Light` come out unchanged (10 and 0). See
//!   `docs/formats/lighting.md`'s Open section for what that excess is.
//! - **Secondary, track-scoped**: exactly the 40 known track/zone `.vex`
//!   files, resolved by name hash the way `pads_ground_truth.rs` already does,
//!   where a live `oag-view --nodes --class` census this session already
//!   established 74 `AmbientLight`, 86 `DirectionalLight` and 10 `PointLight`
//!   instances - asserted here as literal known values, not discovered and
//!   then pinned.

use std::collections::HashMap;
use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::lighting;
use oag_vex::vex;

/// Track directories present on the PSP disc, from the plugin definitions.
///
/// Duplicated from `pads_ground_truth.rs` rather than shared - in-pattern for
/// this test file, which each ground-truth test already does for its own
/// `image()`/`archive_path()`/`vex_files()` trio.
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

/// Fewest track files that must be checked for the track-scoped test to mean
/// anything.
const MIN_TRACK_FILES: usize = 20;

/// `AmbientLight` instances across the full unfiltered `Data.wad` sweep.
///
/// Pinned from a real run against `pulse-psp-usa.chd`. **Higher than
/// [`TRACK_SCOPED_AMBIENT_LIGHTS`]** by 32: `AmbientLight` is a generic scene
/// class and is genuinely authored off the 40 track/zone files this sweep also
/// covers, mostly as single instances on files outside `Data\Environments\`.
/// See `docs/formats/lighting.md`'s Open section.
const PSP_AMBIENT_LIGHTS: usize = 106;

/// `DirectionalLight` instances across the full unfiltered sweep.
///
/// **Higher than [`TRACK_SCOPED_DIRECTIONAL_LIGHTS`]** by 28, the same
/// off-track pattern as [`PSP_AMBIENT_LIGHTS`].
const PSP_DIRECTIONAL_LIGHTS: usize = 114;

/// `PointLight` instances across the full unfiltered sweep.
///
/// Equal to [`TRACK_SCOPED_POINT_LIGHTS`] - no off-track instance found; the
/// full sweep's own clustering (density 5..5 per authoring file) lands on the
/// same two files the track-scoped sweep does.
const PSP_POINT_LIGHTS: usize = 10;

/// `Dynamic Point Light` (`0x3c2`) instances across the full unfiltered sweep.
///
/// Zero, confirming the track-only census this session's plan already
/// recorded: the class is authored nowhere on the disc, not just nowhere on a
/// track, so it stays out of render scope entirely rather than merely out of
/// the track-scoped count.
const PSP_DYNAMIC_POINT_LIGHTS: usize = 0;

/// `AmbientLight` instances across exactly the 40 track/zone files.
const TRACK_SCOPED_AMBIENT_LIGHTS: usize = 74;

/// `DirectionalLight` instances across exactly the 40 track/zone files.
const TRACK_SCOPED_DIRECTIONAL_LIGHTS: usize = 86;

/// `PointLight` instances across exactly the 40 track/zone files.
const TRACK_SCOPED_POINT_LIGHTS: usize = 10;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// One decoded `.vex` file's worth of raw bytes.
struct VexFile {
    label: String,
    bytes: Vec<u8>,
    tree: Vec<vex::Node>,
}

/// Finds an archive by its full path, not by suffix.
///
/// A suffix match is a trap here: `ends_with("Data.wad")` also matches
/// `BEData.wad` and `FEData.wad`, and the first of those in disc order holds
/// no tracks at all.
fn archive_path(disc: &mut DiscImage, name: &str) -> String {
    disc.entries()
        .expect("entries")
        .iter()
        .map(|e| e.path.clone())
        .find(|p| p.as_str() == name || p.ends_with(&format!("/{name}")))
        .unwrap_or_else(|| panic!("{name} not on the disc"))
}

/// Every `.vex` file in one archive, decompressed and walked. No name filter:
/// this is what makes the primary sweep cover all 1142 entries rather than
/// just the ones under `Data\Environments\`.
fn vex_files(disc: &mut DiscImage, archive_path: &str) -> Vec<VexFile> {
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
            Compression::Zlib => panic!("{archive_path} entry {index}: unexpected zlib entry"),
        };
        if !vex::has_magic(&bytes) {
            continue;
        }
        // Only the current format version. `Data\Defaults\Skycube.vex` is a
        // version-4 file whose class ids are from an older numbering, so
        // walking it here would look for `0x12c`/`0x131`/`0x132` in a file
        // that predates that numbering entirely.
        if vex::version(&bytes) != Ok(6) {
            continue;
        }
        let Ok(tree) = vex::nodes(&bytes) else {
            continue;
        };
        out.push(VexFile {
            label: format!("{archive_path} entry {index}"),
            bytes,
            tree,
        });
    }
    out
}

/// Every track/zone `.vex` model on the PSP disc, name-hash resolved the same
/// way `pads_ground_truth.rs::track_models` does, with its archive-relative
/// name.
fn track_vex_files(disc: &mut DiscImage) -> Vec<(String, Vec<u8>, Vec<vex::Node>)> {
    let archive_path = archive_path(disc, "Data.wad");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == archive_path)
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
            let bytes = match entry.compression {
                Compression::None => raw,
                Compression::Lzss => {
                    oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize)
                        .expect("lzss")
                }
                Compression::Zlib => panic!("{name}: unexpected zlib entry"),
            };
            assert!(vex::has_magic(&bytes), "{name} is not a .vex model");
            let tree = vex::nodes(&bytes).expect("nodes");
            out.push((name, bytes, tree));
        }
    }
    assert!(
        out.len() >= MIN_TRACK_FILES,
        "only {} track files found, expected at least {MIN_TRACK_FILES}",
        out.len()
    );
    out
}

/// The rotation part of a `to_world` matrix (rows 0-2, columns 0-2), checked
/// against [`vex::IDENTITY`]'s own 3x3 block within `epsilon` per element.
fn is_identity_rotation(m: &[f32; 16], epsilon: f32) -> bool {
    let identity_3x3 = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    let rows = [[m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]]];
    for (i, row) in rows.iter().enumerate() {
        for (j, &c) in row.iter().enumerate() {
            if (c - identity_3x3[i * 3 + j]).abs() > epsilon {
                return false;
            }
        }
    }
    true
}

fn row2_length(m: &[f32; 16]) -> f32 {
    (m[8] * m[8] + m[9] * m[9] + m[10] * m[10]).sqrt()
}

/// Invariants that must hold for every decoded light, regardless of which
/// sweep found it. Returns nothing; panics on the first violation, the same
/// shape `skycube_ground_truth.rs::assert_mesh_shaped` uses.
fn assert_ambient_or_directional_sane(label: &str, colour: [f32; 3], intensity: f32) {
    for (axis, c) in colour.iter().enumerate() {
        assert!(
            c.is_finite() && *c >= 0.0,
            "{label}: colour channel {axis} = {c}, not finite and non-negative"
        );
    }
    assert!(
        intensity.is_finite() && intensity > 0.0,
        "{label}: intensity {intensity} is not finite and positive"
    );
}

/// Every light node's payload is **exactly** the length its class expects,
/// not merely long enough to parse.
///
/// Run on both sweeps: `AmbientLight::parse`/`DirectionalLight::parse`/
/// `PointLight::parse` all accept a payload that is merely `>=` their
/// minimum, so a longer-than-expected payload decodes without error and
/// would otherwise only surface here. This matters most on the full sweep,
/// not the track-scoped one - the 32 off-track `AmbientLight`/
/// `DirectionalLight` instances come mostly from a different authoring
/// pipeline (`ship_FE.vex` front-end models), which is exactly the
/// population where a different payload length would plausibly turn up.
fn assert_payload_lengths(label: &str, tree: &[vex::Node]) {
    for node in vex::nodes_by_class(tree, vex::CLASS_AMBIENT_LIGHT) {
        assert_eq!(
            node.payload().len(),
            lighting::AMBIENT_OR_DIRECTIONAL_PAYLOAD_LEN,
            "{label}: AmbientLight payload is {} bytes",
            node.payload().len()
        );
    }
    for node in vex::nodes_by_class(tree, vex::CLASS_DIRECTIONAL_LIGHT) {
        assert_eq!(
            node.payload().len(),
            lighting::AMBIENT_OR_DIRECTIONAL_PAYLOAD_LEN,
            "{label}: DirectionalLight payload is {} bytes",
            node.payload().len()
        );
    }
    for node in vex::nodes_by_class(tree, vex::CLASS_POINT_LIGHT) {
        assert_eq!(
            node.payload().len(),
            lighting::POINT_PAYLOAD_LEN,
            "{label}: PointLight payload is {} bytes",
            node.payload().len()
        );
    }
}

fn assert_point_sane(label: &str, colour: [f32; 3], range: f32, trailer: [u32; 4]) {
    for (axis, c) in colour.iter().enumerate() {
        assert!(
            c.is_finite() && *c >= 0.0,
            "{label}: colour channel {axis} = {c}, not finite and non-negative"
        );
    }
    assert!(
        range.is_finite() && range > 0.0,
        "{label}: range {range} is not finite and positive"
    );
    assert_eq!(
        trailer,
        [1, 0, 0, 0],
        "{label}: trailer is {trailer:?}, not the {{1, 0, 0, 0}} every prior sample carries"
    );
}

/// Density across files: the count of one class in each file that authors at
/// least one, so a min/max/mean can be printed without every empty file
/// dragging the mean toward zero.
fn density(counts: &[usize]) -> Option<(usize, usize, f64)> {
    if counts.is_empty() {
        return None;
    }
    let min = *counts.iter().min().unwrap();
    let max = *counts.iter().max().unwrap();
    let mean = counts.iter().sum::<usize>() as f64 / counts.len() as f64;
    Some((min, max, mean))
}

/// The full, unfiltered sweep: every `.vex` in `Data.wad`, every light class,
/// no name filter. Also settles [`PSP_DYNAMIC_POINT_LIGHTS`], which the
/// track-only census could not: `Dynamic Point Light` was never checked
/// off-track before this test existed.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn the_full_disc_sweep_matches_the_pinned_counts() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open disc");
    let data_wad = archive_path(&mut disc, "Data.wad");

    let mut ambient_total = 0usize;
    let mut directional_total = 0usize;
    let mut point_total = 0usize;
    let mut dynamic_point_total = 0usize;

    let mut ambient_files = Vec::new();
    let mut directional_files = Vec::new();
    let mut point_files = Vec::new();

    // Intensity/range extremes across every light of every class: the check
    // that the "not clamped to 0..=1, HDR-ish" reading in the module docs is
    // measured on this run rather than carried forward from an earlier one.
    let mut max_intensity = 0.0f32;
    let mut max_range = 0.0f32;
    let mut max_colour_channel = 0.0f32;

    for file in vex_files(&mut disc, &data_wad) {
        let ambient = lighting::ambient_lights(&file.bytes, &file.tree);
        let directional = lighting::directional_lights(&file.bytes, &file.tree);
        let point = lighting::point_lights(&file.bytes, &file.tree);
        let dynamic_point = vex::nodes_by_class(&file.tree, vex::CLASS_DYNAMIC_POINT_LIGHT).count();

        // Decoded count must equal authored count: `filter_map` drops parse
        // failures silently in the production path, which is right for a
        // loader and wrong for a check. A dropped light would otherwise read
        // as "this file has fewer lights", not as an error.
        let ambient_authored = vex::nodes_by_class(&file.tree, vex::CLASS_AMBIENT_LIGHT).count();
        let directional_authored =
            vex::nodes_by_class(&file.tree, vex::CLASS_DIRECTIONAL_LIGHT).count();
        let point_authored = vex::nodes_by_class(&file.tree, vex::CLASS_POINT_LIGHT).count();
        assert_eq!(
            ambient.len(),
            ambient_authored,
            "{}: {ambient_authored} AmbientLight node(s) authored, {} decoded",
            file.label,
            ambient.len()
        );
        assert_eq!(
            directional.len(),
            directional_authored,
            "{}: {directional_authored} DirectionalLight node(s) authored, {} decoded",
            file.label,
            directional.len()
        );
        assert_eq!(
            point.len(),
            point_authored,
            "{}: {point_authored} PointLight node(s) authored, {} decoded",
            file.label,
            point.len()
        );

        for light in &ambient {
            assert_ambient_or_directional_sane(&file.label, light.colour, light.intensity);
            max_intensity = max_intensity.max(light.intensity);
            max_colour_channel =
                max_colour_channel.max(light.colour.iter().cloned().fold(0.0, f32::max));
        }
        for light in &directional {
            assert_ambient_or_directional_sane(&file.label, light.colour, light.intensity);
            max_intensity = max_intensity.max(light.intensity);
            max_colour_channel =
                max_colour_channel.max(light.colour.iter().cloned().fold(0.0, f32::max));
        }
        for light in &point {
            assert_point_sane(&file.label, light.colour, light.range, light.trailer);
            max_range = max_range.max(light.range);
            max_colour_channel =
                max_colour_channel.max(light.colour.iter().cloned().fold(0.0, f32::max));
        }
        assert_payload_lengths(&file.label, &file.tree);

        if !ambient.is_empty() {
            println!("{}: {} AmbientLight", file.label, ambient.len());
            ambient_files.push(ambient.len());
        }
        if !directional.is_empty() {
            println!("{}: {} DirectionalLight", file.label, directional.len());
            directional_files.push(directional.len());
        }
        if !point.is_empty() {
            println!("{}: {} PointLight", file.label, point.len());
            point_files.push(point.len());
        }

        ambient_total += ambient.len();
        directional_total += directional.len();
        point_total += point.len();
        dynamic_point_total += dynamic_point;
    }

    println!(
        "full sweep: {ambient_total} AmbientLight, {directional_total} DirectionalLight, \
         {point_total} PointLight, {dynamic_point_total} Dynamic Point Light"
    );
    println!(
        "extremes: max colour channel {max_colour_channel}, max AmbientLight/DirectionalLight \
         intensity {max_intensity}, max PointLight range {max_range}"
    );
    if let Some((min, max, mean)) = density(&ambient_files) {
        println!("AmbientLight density per authoring file: {min}..{max}, mean {mean:.2}");
    }
    if let Some((min, max, mean)) = density(&directional_files) {
        println!("DirectionalLight density per authoring file: {min}..{max}, mean {mean:.2}");
    }
    if let Some((min, max, mean)) = density(&point_files) {
        println!("PointLight density per authoring file: {min}..{max}, mean {mean:.2}");
    }

    // Deliberately not `assert_eq!(count, 74)` here: this sweep can
    // legitimately exceed the track-only figures below, since AmbientLight and
    // DirectionalLight are generic scene classes. What is asserted is the
    // number this run actually observed against the real disc, pinned so a
    // later regression shows up as a count that moved.
    assert_eq!(
        ambient_total, PSP_AMBIENT_LIGHTS,
        "the full-sweep AmbientLight count moved from the pinned {PSP_AMBIENT_LIGHTS}"
    );
    assert_eq!(
        directional_total, PSP_DIRECTIONAL_LIGHTS,
        "the full-sweep DirectionalLight count moved from the pinned {PSP_DIRECTIONAL_LIGHTS}"
    );
    assert_eq!(
        point_total, PSP_POINT_LIGHTS,
        "the full-sweep PointLight count moved from the pinned {PSP_POINT_LIGHTS}"
    );
    assert_eq!(
        dynamic_point_total, PSP_DYNAMIC_POINT_LIGHTS,
        "Dynamic Point Light was authored {dynamic_point_total} time(s) somewhere on the disc; \
         the prior track-only census only ever checked tracks, not the whole disc"
    );
}

/// The track-scoped assertion: exactly the 40 known track/zone files, where
/// this session's own live `oag-view --nodes --class` census already
/// established 74/86/10. Asserted as literal known values.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn every_track_file_authors_the_light_counts_the_census_found() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open disc");

    let mut ambient_total = 0usize;
    let mut directional_total = 0usize;
    let mut point_total = 0usize;

    // Point-light clustering: which source file each instance came from, so
    // the "exactly 2 files, 5 each" structural claim can be checked without
    // hardcoding which two files they are.
    let mut point_files: HashMap<String, usize> = HashMap::new();

    // Direction-basis survey: exploratory data collection for the Ghidra
    // stage that decides whether direction reads from the rotation basis or
    // the translation row. Not a strict assertion either way.
    let mut identity_rotation_directional = 0usize;
    let mut total_directional = 0usize;
    let mut row2_lengths = Vec::new();

    for (name, bytes, tree) in track_vex_files(&mut disc) {
        let ambient = lighting::ambient_lights(&bytes, &tree);
        let directional = lighting::directional_lights(&bytes, &tree);
        let point = lighting::point_lights(&bytes, &tree);

        for light in &ambient {
            assert_ambient_or_directional_sane(&name, light.colour, light.intensity);
        }
        for light in &directional {
            assert_ambient_or_directional_sane(&name, light.colour, light.intensity);
            total_directional += 1;
            // 1e-4 per element: loose enough that authoring-time float noise
            // does not flip the classification, tight enough that a real
            // rotation does not read as identity.
            if is_identity_rotation(&light.to_world, 1e-4) {
                identity_rotation_directional += 1;
            }
            row2_lengths.push(row2_length(&light.to_world));
        }
        for light in &point {
            assert_point_sane(&name, light.colour, light.range, light.trailer);
            *point_files.entry(name.clone()).or_insert(0) += 1;
        }

        assert_payload_lengths(&name, &tree);

        ambient_total += ambient.len();
        directional_total += directional.len();
        point_total += point.len();
    }

    println!(
        "track-scoped: {ambient_total} AmbientLight, {directional_total} DirectionalLight, \
         {point_total} PointLight"
    );
    println!(
        "directional lights under identity-rotation parent: {identity_rotation_directional} / \
         {total_directional}"
    );
    if let Some(max) = row2_lengths
        .iter()
        .cloned()
        .fold(None, |m: Option<f32>, x| Some(m.map_or(x, |m| m.max(x))))
    {
        let all_unit_length = row2_lengths.iter().all(|l| (l - 1.0).abs() < 1e-3);
        println!(
            "directional light row-2 length: max deviation from 1.0 is {}, all unit-length: {}",
            (max - 1.0).abs(),
            all_unit_length
        );
    }

    assert_eq!(
        ambient_total, TRACK_SCOPED_AMBIENT_LIGHTS,
        "docs/formats/lighting.md records {TRACK_SCOPED_AMBIENT_LIGHTS} AmbientLight instances \
         across the 40 track/zone files"
    );
    assert_eq!(
        directional_total, TRACK_SCOPED_DIRECTIONAL_LIGHTS,
        "docs/formats/lighting.md records {TRACK_SCOPED_DIRECTIONAL_LIGHTS} DirectionalLight \
         instances across the 40 track/zone files"
    );
    assert_eq!(
        point_total, TRACK_SCOPED_POINT_LIGHTS,
        "docs/formats/lighting.md records {TRACK_SCOPED_POINT_LIGHTS} PointLight instances \
         across the 40 track/zone files"
    );

    // Structural clustering, not a hardcoded file name: exactly two files
    // carry point lights, five each.
    assert_eq!(
        point_files.len(),
        2,
        "point lights are on {} distinct file(s), expected exactly 2: {point_files:?}",
        point_files.len()
    );
    for (name, count) in &point_files {
        assert_eq!(
            *count, 5,
            "{name}: {count} point light(s), expected exactly 5"
        );
    }
}
