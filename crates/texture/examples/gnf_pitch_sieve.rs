//! Scratch probe: does the `.gnf` descriptor's own `pitch` field ever pad
//! past `width` for a BC7, tile-mode-13 surface - and if so, to what
//! constant? Pixel-free evidence for `macro_tile_pitch` (see
//! `docs/formats/gnf.md`'s tiling-calibration section): AMD's own
//! `ComputeSurfaceAlignmentsMacroTiled`/`PadDimensions` round a macro-tiled
//! surface's pitch **up** to a multiple of `macro_tile_pitch` at allocation
//! time, so if every sample's `pitch.div_ceil(4)` (blocks) is a multiple of
//! one fixed `P`, `P` is `macro_tile_pitch` - `8 * bank_width * num_pipes *
//! macro_tile_aspect` - which narrows or pins those four unknowns without
//! decoding a single pixel.
//!
//! ```sh
//! cargo run -q --release -p oag-texture --example gnf_pitch_sieve
//! ```

use std::collections::BTreeMap;

use oag_assets::psarc::Archive;
use oag_texture::gnf::{SurfaceFormat, Texture};

const OMEGA_BASE: &str = "data/extracted/ps4/omega-eu/uroot";
const OMEGA_PATCH: &str = "data/extracted/ps4/omega-eu-patch/uroot";
const BASE_ARCHIVES: &[&str] = &["data00", "data01", "data02", "data03", "data04"];
const PATCH_ARCHIVES: &[&str] = &["data05", "data07", "data08", "data09"];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut archives: Vec<String> = Vec::new();
    for name in BASE_ARCHIVES {
        archives.push(format!("{OMEGA_BASE}/{name}.psarc"));
    }
    for name in PATCH_ARCHIVES {
        archives.push(format!("{OMEGA_PATCH}/{name}.psarc"));
    }

    let mut widths_wider_than_pitch = 0;
    let mut pitch_wider_than_width = 0;
    let mut equal = 0;
    let mut smallest: Vec<(u32, u32, u32, String)> = Vec::new(); // (width_blocks, pitch_blocks, height_blocks, path)
    let mut divisors_seen: BTreeMap<u32, usize> = BTreeMap::new();

    for spec in &archives {
        let Ok(mut archive) = Archive::open_file(std::path::Path::new(spec)) else {
            continue;
        };
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".gnf"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(bytes) = archive.read_path(&path) else {
                continue;
            };
            let Ok(t) = Texture::parse(&bytes) else {
                continue;
            };
            if t.surface_format != SurfaceFormat::Bc7 || t.tile_mode.0 != 13 {
                continue;
            }
            let width_blocks = t.width.div_ceil(4);
            let pitch_blocks = t.pitch.div_ceil(4);
            let height_blocks = t.height.div_ceil(4);
            match pitch_blocks.cmp(&width_blocks) {
                std::cmp::Ordering::Less => widths_wider_than_pitch += 1,
                std::cmp::Ordering::Greater => pitch_wider_than_width += 1,
                std::cmp::Ordering::Equal => equal += 1,
            }
            // Largest power-of-two divisor of pitch_blocks, capped at 128 -
            // a stand-in for "which macro_tile_pitch would this pitch be
            // consistent with", since macro_tile_pitch is always a power of
            // two (8 * powers-of-two factors).
            let mut d = 1u32;
            while pitch_blocks % (d * 2) == 0 && d * 2 <= 128 {
                d *= 2;
            }
            *divisors_seen.entry(d).or_default() += 1;

            if width_blocks * height_blocks <= 64 {
                smallest.push((width_blocks, pitch_blocks, height_blocks, path.clone()));
            }
            if pitch_blocks != width_blocks {
                println!(
                    "PADDED: width {width_blocks:4} -> pitch {pitch_blocks:4} blocks (height {height_blocks:4})  {path}"
                );
            }
        }
    }

    println!(
        "pitch < width: {widths_wider_than_pitch}, pitch > width: {pitch_wider_than_width}, pitch == width: {equal}"
    );
    println!("largest power-of-two divisor of pitch_blocks, histogram:");
    for (d, n) in &divisors_seen {
        println!("  divides by {d:4}: {n}");
    }
    smallest.sort_by_key(|(w, _, h, _)| w * h);
    println!("\nsmallest (by block area) BC7 tile-13 textures:");
    for (w, p, h, path) in smallest.iter().take(20) {
        println!("  {w:3}x{h:<3} blocks, pitch {p:3} blocks  {path}");
    }

    Ok(())
}
