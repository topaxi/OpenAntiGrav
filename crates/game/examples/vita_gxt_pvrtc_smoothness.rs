//! Scratch probe: does the new `PVRTII4BPP` decode produce *images*, at the
//! scale of the whole corpus, rather than something that merely renders?
//!
//! PVRTC's low-frequency image is bilinearly upscaled from one colour pair per
//! 4x4 word, so a correctly decoded surface is strongly correlated between
//! neighbouring texels. A wrong word order - the single most likely way to get
//! this wrong, since the words are twiddled and nothing in the descriptor says
//! so - destroys exactly that correlation while leaving the histogram intact.
//! So the metric is the mean absolute difference between horizontally adjacent
//! texels, and the argument is a comparison against two references:
//!
//! - the already-trusted `UBC2` textures in the same corpus (`docs/formats/gxt.md`),
//! - and a **control**: the same `PVRTII4BPP` bytes decoded with the word grid
//!   read in raster order instead of twiddled.
//!
//! ```sh
//! cargo run -q --release -p oag-game --example vita_gxt_pvrtc_smoothness
//! ```

use oag_texture::gxt;

const PACKAGE: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";

/// Mean absolute difference between horizontally adjacent texels, over RGB.
fn roughness(rgba: &[[u8; 4]], width: usize, height: usize) -> f64 {
    let mut total = 0u64;
    let mut count = 0u64;
    for y in 0..height {
        for x in 1..width {
            let a = rgba[y * width + x];
            let b = rgba[y * width + x - 1];
            for c in 0..3 {
                total += u64::from(a[c].abs_diff(b[c]));
                count += 1;
            }
        }
    }
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}

/// The Morton word index `(x, y)` reads from - the same bit-scatter
/// `oag_texture::gxt::twiddle` implements, restated here because this probe
/// needs to *undo* it and that function is crate-private.
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

/// The control payload: the level's words permuted so that the decoder's own
/// twiddled lookup lands on the word a **raster** reading would have used.
/// Same bytes, same histogram, same block count - only the word order differs,
/// which is the one hypothesis a smoothness metric can separate.
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

fn main() -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(PACKAGE)?;
    let mut entries: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".gxt"))
        .cloned()
        .collect();
    entries.sort();

    let mut pvrtc = Vec::new();
    let mut ubc2 = Vec::new();
    let mut control = Vec::new();
    let (mut hard, mut words, mut palette, mut texels) = (0u64, 0u64, 0u64, 0u64);
    for entry in entries {
        let Ok(blob) = archive.read_path(&entry) else {
            continue;
        };
        let Ok(parsed) = gxt::Gxt::parse(&blob) else {
            continue;
        };
        let Some(texture) = parsed.only() else {
            continue;
        };
        if texture.width < 32 || texture.height < 32 {
            continue;
        }
        let Ok(rgba) = texture.to_rgba(&blob) else {
            continue;
        };
        let (w, h) = (usize::from(texture.width), usize::from(texture.height));
        let score = roughness(&rgba, w, h);
        match texture.format_byte >> 24 {
            0x83 => {
                pvrtc.push(score);
                let start = texture.data.start;
                let (wx, wy) = (w.div_ceil(4), h.div_ceil(4));
                let word = |x: usize, y: usize| -> u32 {
                    let at =
                        start + (twiddle(x as u32, y as u32, wx as u32, wy as u32) as usize) * 8;
                    blob.get(at + 4..at + 8)
                        .map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
                };
                words += (wx * wy) as u64;
                texels += (w * h) as u64;
                for qy in 0..wy {
                    for qx in 0..wx {
                        let (px, py) = ((qx + wx - 1) % wx, (qy + wy - 1) % wy);
                        if word(px, py) & (1 << 15) == 0 {
                            continue;
                        }
                        hard += 1;
                        // The quad's central 4x4 is four texels from each of
                        // P, Q, R and S; a word in modulation mode 1 sends its
                        // four into the local palette.
                        for (x, y) in [(px, py), (qx, py), (px, qy), (qx, qy)] {
                            palette += 4 * u64::from(word(x, y) & 1 != 0);
                        }
                    }
                }
                // Control: hand the decoder the payload with its twiddle
                // undone, so the words land in the wrong places.
                let start = texture.data.start;
                let level = (w.div_ceil(4) * h.div_ceil(4) * 8).max(16);
                let shuffled = as_if_raster(
                    &blob[start..start + level],
                    u32::from(texture.width),
                    u32::from(texture.height),
                );
                let mut fake = blob.clone();
                fake[start..start + level].copy_from_slice(&shuffled);
                if let Ok(other) = texture.to_rgba(&fake) {
                    control.push(roughness(&other, w, h));
                }
            }
            0x86 => ubc2.push(score),
            _ => {}
        }
    }

    let report = |name: &str, mut v: Vec<f64>| {
        if v.is_empty() {
            println!("{name}: none");
            return;
        }
        v.sort_by(f64::total_cmp);
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        println!(
            "{name}: {} textures, mean {:.2}, median {:.2}, p90 {:.2}, max {:.2}",
            v.len(),
            mean,
            v[v.len() / 2],
            v[v.len() * 9 / 10],
            v[v.len() - 1]
        );
    };
    println!(
        "hard-transition words: {hard} of {words} ({:.4}%); \
         texels reaching the local-palette path: {palette} of {texels} ({:.6}%)",
        100.0 * hard as f64 / words.max(1) as f64,
        100.0 * palette as f64 / texels.max(1) as f64,
    );
    report("PVRTII4BPP", pvrtc);
    report("UBC2 (trusted)", ubc2);
    report("PVRTII4BPP untwiddled control", control);
    Ok(())
}
