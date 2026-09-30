//! `Texture::block_levels` - a `.gnf`'s own BC7 blocks and mip chain, still
//! compressed - against every BC7 `Thin_1DThin` `.gnf` in the Omega archives.
//!
//! **`#[ignore]`d and never run in CI**; needs the extracted PS4 archives
//! `omega_gnf_pixels_ground_truth.rs` names. Run with `just test-data`.
//!
//! What it holds the chain to, each an arithmetic fact rather than a taste:
//!
//! 1. **The level count is the descriptor's** and every level is exactly
//!    `ceil(w/4) * ceil(h/4)` blocks of 16 bytes.
//! 2. **The base level is the decode's, byte for byte**: the blocks this
//!    hands the GPU decode to the same texels `Texture::decode` draws today,
//!    so passing them through changes no base-level pixel.
//! 3. **The authored chain is the base's own filtered down**, to within BC7's
//!    own quantisation: level 1 decoded sits within a stated mean absolute
//!    difference of a 2x2 box filter of level 0 decoded - which is what the
//!    renderer synthesises when it is not given a chain. This is the
//!    difference the passthrough introduces and the only one; see
//!    `MEAN_LIMIT` for the ceiling and the measurement it was set against.

use std::path::PathBuf;

use oag_assets::psarc::Archive;
use oag_texture::gnf::{self, SurfaceFormat, Texture};

/// Level-0 texel count above which the decode comparisons are skipped, not the
/// structure checks: a debug-build BC7 decode of a 4096-square is minutes.
const DECODE_LIMIT: u32 = 1024 * 1024;

/// Mean absolute difference, in 8-bit channel units, that level 1 decoded may
/// sit from a box filter of level 0 decoded, averaged over every texture the
/// archive offers. Measured on the whole corpus and printed by each test.
const MEAN_LIMIT: f64 = 4.0;

fn open(dir: &str, name: &str) -> Option<Archive> {
    let path: PathBuf = oag_testdata::exact(&format!("data/extracted/ps4/{dir}/uroot/{name}"))?;
    Some(Archive::open_file(&path).unwrap_or_else(|e| panic!("open {name}: {e}")))
}

fn box_down(rgba: &[[u8; 4]], w: u32, h: u32) -> Vec<[u8; 4]> {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let mut out = Vec::with_capacity((nw * nh) as usize);
    for y in 0..nh {
        for x in 0..nw {
            let at = |sx: u32, sy: u32| rgba[(sy.min(h - 1) * w + sx.min(w - 1)) as usize];
            let taps = [
                at(x * 2, y * 2),
                at(x * 2 + 1, y * 2),
                at(x * 2, y * 2 + 1),
                at(x * 2 + 1, y * 2 + 1),
            ];
            out.push(std::array::from_fn(|c| {
                ((taps.iter().map(|t| u32::from(t[c])).sum::<u32>() + 2) / 4) as u8
            }));
        }
    }
    out
}

fn check(dir: &str, name: &str) {
    let Some(mut archive) = open(dir, name) else {
        return;
    };
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".gnf"))
        .cloned()
        .collect();
    let (mut ok, mut corrupt, mut short, mut compared) = (0usize, 0usize, 0usize, 0usize);
    let (mut sum, mut worst) = (0.0f64, (0.0f64, String::new()));
    for path in &paths {
        let blob = archive.read_path(path).unwrap();
        let Ok(t) = Texture::parse(&blob) else {
            continue;
        };
        if t.surface_format != SurfaceFormat::Bc7 || t.tile_mode.0 != 13 {
            continue;
        }
        let levels = match t.block_levels(&blob) {
            Ok(levels) => levels,
            Err(gnf::Error::CorruptBlocks { .. }) => {
                corrupt += 1;
                continue;
            }
            Err(gnf::Error::ChainLayout { .. } | gnf::Error::DataOutOfBounds { .. }) => {
                short += 1;
                continue;
            }
            Err(e) => panic!("{name}: {path}: unexpected {e}"),
        };
        ok += 1;
        assert_eq!(
            levels.len(),
            usize::from(t.last_mip_level - t.base_mip_level) + 1,
            "{path}"
        );
        for (level, blocks) in levels.iter().enumerate() {
            let wb = (t.width >> level).max(1).div_ceil(4);
            let hb = (t.height >> level).max(1).div_ceil(4);
            assert_eq!(
                blocks.len(),
                (wb * hb) as usize * 16,
                "{path} level {level}"
            );
        }
        if t.width * t.height > DECODE_LIMIT {
            continue;
        }
        let base = gnf::decode_bc7_level(&levels[0], t.width, t.height).unwrap();
        assert_eq!(
            base,
            t.decode(&blob).unwrap(),
            "{path}: base level differs from decode()"
        );
        if levels.len() < 2 || t.width < 8 || t.height < 8 {
            continue;
        }
        let (w1, h1) = (t.width / 2, t.height / 2);
        let authored = gnf::decode_bc7_level(&levels[1], w1, h1).unwrap();
        let filtered = box_down(&base, t.width, t.height);
        let total: u64 = authored
            .iter()
            .zip(&filtered)
            .flat_map(|(a, b)| a.iter().zip(b))
            .map(|(a, b)| u64::from(a.abs_diff(*b)))
            .sum();
        let mad = total as f64 / (authored.len() * 4) as f64;
        sum += mad;
        compared += 1;
        if mad > worst.0 {
            worst = (mad, path.clone());
        }
    }
    let mean = if compared > 0 {
        sum / compared as f64
    } else {
        0.0
    };
    println!(
        "{name}: {ok} chains untiled, {corrupt} refused corrupt, {short} not one 2D chain; level 1 vs box(level 0) over {compared}: mean MAD {mean:.3}, worst {:.3} ({})",
        worst.0, worst.1
    );
    assert!(
        mean < MEAN_LIMIT,
        "{name}: mean MAD {mean:.3} over the limit"
    );
}

macro_rules! per_archive {
    ($($test:ident: $dir:literal, $name:literal;)*) => {$(
        #[test]
        #[ignore]
        fn $test() {
            check($dir, $name);
        }
    )*};
}

per_archive! {
    data00_chains: "omega-eu", "data00.psarc";
    data01_chains: "omega-eu", "data01.psarc";
    data02_chains: "omega-eu", "data02.psarc";
    data03_chains: "omega-eu", "data03.psarc";
    data04_chains: "omega-eu", "data04.psarc";
    patch_data05_chains: "omega-eu-patch", "data05.psarc";
    patch_data07_chains: "omega-eu-patch", "data07.psarc";
    patch_data08_chains: "omega-eu-patch", "data08.psarc";
    patch_data09_chains: "omega-eu-patch", "data09.psarc";
}
