//! Scratch probe: the cross-title oracle for `PVRTII4BPP`.
//!
//! 2048's DLC re-ships Wipeout HD/Fury's circuits and its whole roster, so the
//! *same authored texture* exists twice: as a `.gtf` on the PS3 disc, in a BC
//! format `oag_texture::gtf` has decoded and rendered since before this pass,
//! and as a `.gxt` in the Vita package, in the `PVRTII4BPP` this pass added.
//! Two lossy compressions of one source image do not agree bit for bit, but
//! they agree *closely* - and a wrong decode does not agree at all.
//!
//! So the measurement is the mean absolute per-channel difference between the
//! two decodes over every same-name, same-dimension pair, against two
//! controls:
//!
//! - **untwiddled**: the same `.gxt` payload decoded with its word grid read in
//!   raster order, which is the single most likely way to get this format
//!   wrong;
//! - **mismatched**: the correct decode compared against a *different* pair's
//!   HD texture of the same size, which is chance level for this metric.
//!
//! This is the same HD-as-ground-truth method that settled the `WO Track`
//! point tail, the 2048 vertex normal and `Uv1` - see
//! `docs/formats/2048-rcsmodel.md`.
//!
//! ```sh
//! cargo run -q --release -p oag-game --example vita_gxt_hd_oracle
//! ```

use std::collections::BTreeMap;

const VITA: [&str; 3] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn basename(path: &str) -> String {
    let lower = path.to_ascii_lowercase().replace('\\', "/");
    let file = lower.rsplit('/').next().unwrap_or(&lower).to_string();
    file.rsplit_once('.')
        .map_or(file.clone(), |(s, _)| s.to_string())
}

/// Mean absolute per-channel difference over RGB, 0 to 255.
fn difference(a: &[[u8; 4]], b: &[[u8; 4]]) -> f64 {
    let mut total = 0u64;
    for (x, y) in a.iter().zip(b) {
        for c in 0..3 {
            total += u64::from(x[c].abs_diff(y[c]));
        }
    }
    total as f64 / (a.len() * 3) as f64
}

fn flipped(rgba: &[[u8; 4]], width: usize, height: usize) -> Vec<[u8; 4]> {
    let mut out = Vec::with_capacity(rgba.len());
    for y in (0..height).rev() {
        out.extend_from_slice(&rgba[y * width..(y + 1) * width]);
    }
    out
}

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

/// The control payload: the level's words permuted so the decoder's twiddled
/// lookup lands where a raster reading would have put each word.
fn as_if_raster(level: &[u8], width: u32, height: u32) -> Vec<u8> {
    let words_x = width.max(8) / 4;
    let words_y = height.max(8) / 4;
    let mut out = vec![0u8; level.len()];
    for y in 0..words_y {
        for x in 0..words_x {
            let from = ((y * words_x + x) * 8) as usize;
            let to = (twiddle(x, y, words_x, words_y) * 8) as usize;
            if let (Some(src), true) = (level.get(from..from + 8), to + 8 <= out.len()) {
                out[to..to + 8].copy_from_slice(src);
            }
        }
    }
    out
}

fn report(name: &str, mut v: Vec<f64>) {
    if v.is_empty() {
        println!("{name}: none");
        return;
    }
    v.sort_by(f64::total_cmp);
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let under = |t: f64| v.iter().filter(|&&x| x < t).count() * 100 / v.len();
    println!(
        "{name}: n={}, mean {:.2}, median {:.2}, p10 {:.2}, p90 {:.2}; under 8: {}%, under 16: {}%",
        v.len(),
        mean,
        v[v.len() / 2],
        v[v.len() / 10],
        v[v.len() * 9 / 10],
        under(8.0),
        under(16.0),
    );
}

fn main() -> anyhow::Result<()> {
    // Every HD `.gtf` that decodes, by basename.
    let mut hd: BTreeMap<String, (u32, u32, Vec<[u8; 4]>)> = BTreeMap::new();
    for index in 0..7 {
        let path = format!("data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/DATA0{index}.PSARC");
        let Ok(mut archive) = oag_assets::psarc::Archive::open(&path) else {
            continue;
        };
        let entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".gtf"))
            .cloned()
            .collect();
        for entry in entries {
            let name = basename(&entry);
            if hd.contains_key(&name) {
                continue;
            }
            let Ok(blob) = archive.read_path(&entry) else {
                continue;
            };
            let Ok(parsed) = oag_texture::gtf::Gtf::parse(&blob) else {
                continue;
            };
            let Some(texture) = parsed.textures.first() else {
                continue;
            };
            let Ok(rgba) = texture.to_rgba(&blob) else {
                continue;
            };
            if rgba.len() != usize::from(texture.width) * usize::from(texture.height) {
                continue;
            }
            hd.insert(
                name,
                (u32::from(texture.width), u32::from(texture.height), rgba),
            );
        }
    }
    println!(
        "{} HD .gtf textures decoded and available as ground truth",
        hd.len()
    );

    let (mut matched, mut control, mut mismatched, mut upside_down) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    // For the mismatched control: one previously-seen HD texture per size.
    let mut previous: BTreeMap<(u32, u32), Vec<[u8; 4]>> = BTreeMap::new();

    for package in VITA {
        let Ok(mut archive) = oag_assets::psarc::Archive::open(package) else {
            continue;
        };
        let entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".gxt"))
            .cloned()
            .collect();
        for entry in entries {
            let Some((hw, hh, truth)) = hd.get(&basename(&entry)) else {
                continue;
            };
            let Ok(blob) = archive.read_path(&entry) else {
                continue;
            };
            let Ok(parsed) = oag_texture::gxt::Gxt::parse(&blob) else {
                continue;
            };
            let Some(texture) = parsed.only() else {
                continue;
            };
            if texture.format_byte >> 24 != 0x83
                || u32::from(texture.width) != *hw
                || u32::from(texture.height) != *hh
            {
                continue;
            }
            let Ok(rgba) = texture.to_rgba(&blob) else {
                continue;
            };
            let (w, h) = (*hw as usize, *hh as usize);

            matched.push(difference(&rgba, truth));
            upside_down.push(difference(&rgba, &flipped(truth, w, h)));
            if let Some(other) = previous.get(&(*hw, *hh)) {
                mismatched.push(difference(&rgba, other));
            }
            previous.insert((*hw, *hh), truth.clone());

            let start = texture.data.start;
            let level = (w.div_ceil(4) * h.div_ceil(4) * 8).max(16);
            let shuffled = as_if_raster(&blob[start..start + level], *hw, *hh);
            let mut fake = blob.clone();
            fake[start..start + level].copy_from_slice(&shuffled);
            if let Ok(other) = texture.to_rgba(&fake) {
                control.push(difference(&other, truth));
            }
        }
    }

    report("PVRTII4BPP vs HD .gtf, same art", matched);
    report("  same art, HD flipped vertically", upside_down);
    report("  CONTROL untwiddled vs HD .gtf", control);
    report("  CONTROL different art, same size", mismatched);
    Ok(())
}
