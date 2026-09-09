//! Scratch probe: which occluder nodes get their padded `+0x30`/`+0x40` bbox
//! floored/hard-zeroed rather than kept, and does file identity discriminate.
//!
//! `crates/formats/tests/shadow_occluder_ground_truth.rs`'s
//! `PSP_OCCLUDERS_WITH_PADDED_BBOX` doc comment quantifies two exception
//! populations (2 of 14 positive-`min.y` nodes keep their real value instead
//! of being floored to `0.0`; 38 of 54 denormal-`max.y` nodes get a hard
//! `0.0` instead of the true vertex-derived value) and rules out node name and
//! numeric value as the discriminator, leaving file/build-version as the
//! untried guess. This prints archive path, directory index and node name for
//! every node in both populations so that guess can actually be checked.
//!
//! ```sh
//! cargo run -q -p oag-formats --example shadow_padding_probe
//! ```

use std::path::{Path, PathBuf};

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::vex;

const CLASS_OCCLUDER: u32 = 0x3c3;
const HEADER_LEN: usize = 0x50;
const RECORD_A_STRIDE: usize = 32;
const RECORD_B_STRIDE: usize = 16;
const PULSE_VERSIONS: [u32; 1] = [6];

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);
    path.exists().then_some(path)
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

struct VexFile {
    label: String,
    bytes: Vec<u8>,
    tree: Vec<vex::Node>,
}

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

fn bounds(payload: &[u8]) -> ([f32; 3], [f32; 3]) {
    let mut min = [0.0; 3];
    let mut max = [0.0; 3];
    for axis in 0..3 {
        min[axis] = f32_at(payload, 0x0c + axis * 4);
        max[axis] = f32_at(payload, 0x18 + axis * 4);
    }
    (min, max)
}

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

fn main() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        eprintln!("skipping: data/images/pulse-psp-usa.chd not present");
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let files = vex_files(&mut disc, &PULSE_VERSIONS);

    println!("== positive header min.y: kept vs floored ==");
    for file in &files {
        for node in vex::nodes_by_class(&file.tree, CLASS_OCCLUDER) {
            let payload = &file.bytes[node.payload()];
            let n = usize::from(u16_at(payload, 0));
            let m = usize::from(u16_at(payload, 2));
            let (min, _max) = bounds(payload);
            if min[1] <= 0.0 {
                continue;
            }
            let padded_min_y = f32_at(payload, 0x30 + 4);
            let kept = (padded_min_y - min[1]).abs() < 1e-6;
            println!(
                "{:<28} {:<20} n={n:<3} m={m:<3} header.min.y={:<12} padded.min.y={:<12} {}",
                file.label,
                node.name.clone().unwrap_or_default(),
                min[1],
                padded_min_y,
                if kept { "KEPT" } else { "floored" },
            );
        }
    }

    println!();
    println!("== denormal header max.y (true max.y != 0): kept vs hard-zeroed ==");
    for file in &files {
        for node in vex::nodes_by_class(&file.tree, CLASS_OCCLUDER) {
            let payload = &file.bytes[node.payload()];
            let n = usize::from(u16_at(payload, 0));
            let m = usize::from(u16_at(payload, 2));
            let (_min, max) = bounds(payload);
            if u32_at(payload, 0x18 + 4) != 0x0080_0000 {
                continue;
            }
            let (_vmin, vmax, _used) = vertex_extent(payload, n, m);
            if vmax[1] == 0.0 {
                continue;
            }
            let padded_max_y = f32_at(payload, 0x40 + 4);
            let kept = (padded_max_y - vmax[1]).abs() < 1e-3;
            println!(
                "{:<28} {:<20} n={n:<3} m={m:<3} header.max.y={:<12?} true.max.y={:<12} padded.max.y={:<12} {}",
                file.label,
                node.name.clone().unwrap_or_default(),
                max[1],
                vmax[1],
                padded_max_y,
                if kept { "KEPT" } else { "hard-zero" },
            );
        }
    }

    println!();
    println!("== every one of the 32 mismatchers: which axes differ, local vs world-space ==");
    const WORLD_SPACE_CENTRE: f32 = 50.0;
    for file in &files {
        for node in vex::nodes_by_class(&file.tree, CLASS_OCCLUDER) {
            let payload = &file.bytes[node.payload()];
            let (min, max) = bounds(payload);
            let repeats_all = (0..3).all(|axis| {
                (f32_at(payload, 0x30 + axis * 4) - min[axis]).abs() < 1e-6
                    && (f32_at(payload, 0x40 + axis * 4) - max[axis]).abs() < 1e-6
            });
            if repeats_all {
                continue;
            }
            let centre = (0..3)
                .map(|axis| ((min[axis] + max[axis]) * 0.5).abs())
                .fold(0.0f32, f32::max);
            let space = if centre <= WORLD_SPACE_CENTRE {
                "local"
            } else {
                "world"
            };
            let axis_diff: Vec<&str> = ["x", "y", "z"]
                .iter()
                .enumerate()
                .filter(|(axis, _)| {
                    (f32_at(payload, 0x30 + axis * 4) - min[*axis]).abs() >= 1e-6
                        || (f32_at(payload, 0x40 + axis * 4) - max[*axis]).abs() >= 1e-6
                })
                .map(|(_, name)| *name)
                .collect();
            println!(
                "{:<28} {:<20} {space:<6} differs on {:?}",
                file.label,
                node.name.clone().unwrap_or_default(),
                axis_diff,
            );
        }
    }
}
