//! Scratch probe: the picture check for `UBC1`/`UBC3`, since no HD twin exists
//! for either (`vita_gxt_ubc13_hd_oracle.rs` finds 0 pairs) - the same
//! situation `UBC2`'s reticle texture and `Argb8888`'s Zone/Detonator art were
//! in, both settled by decoding both ways and looking at the picture.
//!
//! Samples one legible `UBC1` (a scanned manual page, all text) and one
//! legible `UBC3` (a front-end callout with a text label) and writes both the
//! real (twiddled block grid) decode and a raster-block-order control to
//! `data/shots/`.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_gxt_ubc13_picture_check
//! ```

use oag_texture::gxt;

const BASE: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";
const DATA_DIR: &str = "data";

fn twiddle(x: u32, y: u32, across: u32, down: u32) -> u32 {
    let (mut w, mut h) = (across, down);
    let (mut mask_x, mut mask_y, mut shift) = (0u32, 0u32, 0u32);
    while w > 1 || h > 1 {
        if w > 1 && h > 1 {
            mask_y |= 1 << shift;
            mask_x |= 1 << (shift + 1);
            shift += 2;
        } else if w > 1 {
            mask_x |= 1 << shift;
            shift += 1;
        } else {
            mask_y |= 1 << shift;
            shift += 1;
        }
        w >>= 1;
        h >>= 1;
    }
    scatter(x, mask_x) | scatter(y, mask_y)
}

fn scatter(value: u32, mask: u32) -> u32 {
    let (mut out, mut remaining, mut source) = (0u32, mask, value);
    while remaining != 0 {
        let lowest = remaining.isolate_lowest_one();
        if source & 1 != 0 {
            out |= lowest;
        }
        source >>= 1;
        remaining &= !lowest;
    }
    out
}

/// Rewrites the base level's block order from twiddled to raster, so decoding
/// it through the real (twiddled) reader produces what a naive raster read
/// would have shown - the control this module's other probes use.
fn as_if_raster(level: &[u8], width: u32, height: u32, unit: usize) -> Vec<u8> {
    let blocks_x = (width as usize).div_ceil(4) as u32;
    let blocks_y = (height as usize).div_ceil(4) as u32;
    let mut out = vec![0u8; level.len()];
    for y in 0..blocks_y {
        for x in 0..blocks_x {
            let from = ((y * blocks_x + x) as usize) * unit;
            let to = (twiddle(x, y, blocks_x, blocks_y) as usize) * unit;
            if let (Some(src), true) = (level.get(from..from + unit), to + unit <= out.len()) {
                out[to..to + unit].copy_from_slice(src);
            }
        }
    }
    out
}

/// Mean absolute difference between horizontally adjacent texels, over RGB -
/// the same roughness metric `vita_gxt_pvrtc_smoothness.rs` uses, reused here
/// because BC has no bilinear upscale to smooth a wrong answer the way PVRTC
/// does, so this metric should discriminate cleanly where it could not there.
fn roughness(rgba: &[[u8; 4]], width: usize, height: usize) -> f64 {
    let mut total = 0u64;
    let mut n = 0usize;
    for y in 0..height {
        for x in 0..width - 1 {
            let a = rgba[y * width + x];
            let b = rgba[y * width + x + 1];
            for c in 0..3 {
                total += u64::from(a[c].abs_diff(b[c]));
            }
            n += 1;
        }
    }
    total as f64 / (n * 3) as f64
}

fn sample(entry: &str, unit: usize, label: &str) -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(BASE)?;
    let blob = archive.read_path(entry)?;
    let parsed = gxt::Gxt::parse(&blob)?;
    let texture = parsed.only().expect("one texture");
    let (width, height) = (u32::from(texture.width), u32::from(texture.height));

    let real = texture.to_rgba(&blob)?;
    let real_rough = roughness(&real, width as usize, height as usize);

    let level_len = (width as usize).div_ceil(4) * (height as usize).div_ceil(4) * unit;
    let level_len = level_len.max(16);
    let start = texture.data.start;
    let shuffled = as_if_raster(&blob[start..start + level_len], width, height, unit);
    let mut fake = blob.clone();
    fake[start..start + level_len].copy_from_slice(&shuffled);
    let raster = texture.to_rgba(&fake)?;
    let raster_rough = roughness(&raster, width as usize, height as usize);

    println!(
        "{label} ({entry}, {width}x{height}): twiddled roughness {real_rough:.2}, raster roughness {raster_rough:.2} ({:.2}x)",
        raster_rough / real_rough.max(0.001)
    );

    std::fs::create_dir_all("data/shots")?;
    let real_bytes: Vec<u8> = real.iter().flatten().copied().collect();
    let raster_bytes: Vec<u8> = raster.iter().flatten().copied().collect();
    std::fs::write(
        format!("{DATA_DIR}/shots/2048_{label}_twiddled.png"),
        oag_texture::png::encode_rgba(width, height, &real_bytes),
    )?;
    std::fs::write(
        format!("{DATA_DIR}/shots/2048_{label}_raster.png"),
        oag_texture::png::encode_rgba(width, height, &raster_bytes),
    )?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    sample("data/Books/Manual/Pages/02/001.gxt", 8, "ubc1_manual_page")?;
    sample("data/FE/NewImages/TinyCallout_MP.gxt", 16, "ubc3_callout")?;
    Ok(())
}
