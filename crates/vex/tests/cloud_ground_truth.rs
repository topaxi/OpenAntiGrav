//! Establishes `cloudCube` `0x3d8` and `cloudGroup` `0x3d9`'s decode against
//! every `.vex` file on both Pulse PSP pressings, and that the three `sea*`
//! environment classes (`sea` `0x3d5`, `seareflect` `0x3d7`, `seaweed` `0x3d6`)
//! author no instance anywhere on either disc.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! See `crates/vex/src/cloud.rs` and
//! `docs/ghidra/functions/psp-pulse-usa/clouds.md` for the claim under test and
//! its evidence.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::cloud;
use oag_vex::vex;

/// The three sea-environment classes, none of which any Pulse `.vex` authors -
/// see [`sea_classes_author_no_instance_on_either_pulse_pressing`].
const SEA_CLASSES: [u32; 3] = [0x3d5, 0x3d6, 0x3d7];

/// `cloudCube` instances across both PSP Pulse pressings: `05_Track` forward
/// and reversed, on each of `pulse-psp-usa.chd` and `pulse-psp-eu.chd`.
const TOTAL_CLOUD_CUBES: usize = 20;

/// `cloudGroup` instances over the same four files.
const TOTAL_CLOUD_GROUPS: usize = 12;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

struct VexFile {
    label: String,
    bytes: Vec<u8>,
    tree: Vec<vex::Node>,
}

/// Finds an archive by its full path, not by suffix - see
/// `skycube_ground_truth.rs` for why a suffix match is a trap (`BEData.wad`,
/// `FEData.wad`).
fn archive_path(disc: &mut DiscImage, name: &str) -> String {
    disc.entries()
        .expect("entries")
        .iter()
        .map(|e| e.path.clone())
        .find(|p| p.as_str() == name || p.ends_with(&format!("/{name}")))
        .unwrap_or_else(|| panic!("{name} not on the disc"))
}

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
            Compression::Zlib => continue,
        };
        if !vex::has_magic(&bytes) || vex::version(&bytes) != Ok(6) {
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

/// Every `.vex` file across the four WADs a Pulse PSP disc ships, on both
/// pressings. This is the sweep both tests below run.
fn all_vex_files() -> Vec<(String, Vec<VexFile>)> {
    let mut out = Vec::new();
    for image_name in ["pulse-psp-usa.chd", "pulse-psp-eu.chd"] {
        let Some(path) = image(image_name) else {
            continue;
        };
        let mut disc = DiscImage::open(&path).expect("open disc");
        for wad_name in ["Data.wad", "FEData.wad", "BEData.wad", "FE.wad"] {
            let archive = archive_path(&mut disc, wad_name);
            let files = vex_files(&mut disc, &archive);
            out.push((format!("{image_name}/{wad_name}"), files));
        }
    }
    out
}

#[test]
#[ignore = "needs data/images/pulse-psp-{usa,eu}.chd; run with `just test-data`"]
fn a_cloud_cube_is_a_type_and_a_scale_owned_by_its_cloud_group() {
    let sweep = all_vex_files();
    if sweep.is_empty() {
        return;
    }

    let mut cubes = 0;
    let mut groups = 0;
    for (_, files) in &sweep {
        for file in files {
            let leaves = cloud::clouds(&file.bytes, &file.tree);
            groups += file
                .tree
                .iter()
                .filter(|n| n.class_id == cloud::CLASS_CLOUD_GROUP)
                .count();
            cubes += file
                .tree
                .iter()
                .filter(|n| n.class_id == cloud::CLASS_CLOUD_CUBE)
                .count();

            // Every shipped `cloudCube` closes: it has an owning `cloudGroup`
            // and its payload is exactly the shipped shape, so `clouds` must
            // find one leaf per node rather than silently dropping any.
            let raw_cube_count = file
                .tree
                .iter()
                .filter(|n| n.class_id == cloud::CLASS_CLOUD_CUBE)
                .count();
            assert_eq!(
                leaves.len(),
                raw_cube_count,
                "{}: {} cloudCube node(s) but {} decoded leaf(ves)",
                file.label,
                raw_cube_count,
                leaves.len()
            );

            for (cube, attrs) in &leaves {
                assert_eq!(cube.kind, 2, "{}: cloudCube kind moved off 2", file.label);
                assert_eq!(
                    cube.scale, 1.0,
                    "{}: cloudCube scale moved off 1.0",
                    file.label
                );
                for axis in cube.world_position {
                    assert!(
                        axis.is_finite(),
                        "{}: cloudCube position is not finite",
                        file.label
                    );
                }
                // Every colour channel and alpha this module reads is a
                // normalised value or the "attribute absent" fallback of
                // zero - never negative, never past 1.0.
                for c in attrs
                    .hi_colour
                    .iter()
                    .chain(&attrs.mid_colour)
                    .chain(&attrs.lo_colour)
                {
                    assert!(
                        (0.0..=1.0).contains(c),
                        "{}: colour channel {c}",
                        file.label
                    );
                }
                assert!(
                    (0.0..=1.0).contains(&attrs.hi_alpha),
                    "{}: HiAlpha {}",
                    file.label,
                    attrs.hi_alpha
                );
                assert!(
                    (0.0..=1.0).contains(&attrs.lo_alpha),
                    "{}: LoAlpha {}",
                    file.label,
                    attrs.lo_alpha
                );
                assert!(
                    attrs.sprite_radius > 0.0,
                    "{}: SpriteRadius {}",
                    file.label,
                    attrs.sprite_radius
                );
            }
        }
    }

    println!(
        "cloudCube {cubes}, cloudGroup {groups}, across {} archive(s)",
        sweep.len()
    );
    assert_eq!(cubes, TOTAL_CLOUD_CUBES, "the cloudCube count moved");
    assert_eq!(groups, TOTAL_CLOUD_GROUPS, "the cloudGroup count moved");
}

#[test]
#[ignore = "needs data/images/pulse-psp-{usa,eu}.chd; run with `just test-data`"]
fn sea_classes_author_no_instance_on_either_pulse_pressing() {
    let sweep = all_vex_files();
    if sweep.is_empty() {
        return;
    }

    let mut total = 0;
    for (archive_label, files) in &sweep {
        for file in files {
            for class_id in SEA_CLASSES {
                let count = file.tree.iter().filter(|n| n.class_id == class_id).count();
                total += count;
                assert_eq!(
                    count, 0,
                    "{archive_label} {}: {count} node(s) of sea-class {class_id:#x}, expected none - \
                     the negative result this test pins has changed",
                    file.label
                );
            }
        }
    }
    assert_eq!(total, 0);
}

/// `CloudGroup_Init` (`0x08933048`) loads this path once, shared by every
/// `cloudGroup` instance - see `docs/ghidra/functions/psp-pulse-usa/clouds.md`.
/// Checked here rather than only asserted in a doc comment: a renderer's first
/// question is whether the asset is reachable at all.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn the_shared_cloud_texture_is_in_data_wad() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open disc");
    let data_wad_path = archive_path(&mut disc, "Data.wad");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == data_wad_path)
        .expect("Data.wad")
        .clone();
    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let wanted = wad::hash_name(r"Data\Tex\Cloud\Wipeout_Clouds_D_128x64x4.mip");
    let entry = dir.entries.iter().find(|e| e.name_hash == wanted);
    assert!(
        entry.is_some(),
        "Data\\Tex\\Cloud\\Wipeout_Clouds_D_128x64x4.mip (hash {wanted:#010x}) is not in Data.wad - \
         the path read out of the executable does not match a real entry"
    );
    println!(
        "cloud texture: entry size {} byte(s)",
        entry.expect("checked above").size
    );
}
