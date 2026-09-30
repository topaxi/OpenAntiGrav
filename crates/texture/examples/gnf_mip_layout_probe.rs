//! Scratch probe: does a contiguous, tile-padded mip chain account for every
//! byte of a BC7 `.gnf` past its header?
use std::collections::BTreeMap;

use oag_assets::psarc::Archive;
use oag_texture::gnf::{SurfaceFormat, Texture};

fn predicted(t: &Texture) -> usize {
    let mut total = 0usize;
    for level in 0..=(t.last_mip_level - t.base_mip_level) {
        let w = (t.width >> level).max(1).div_ceil(4);
        let h = (t.height >> level).max(1).div_ceil(4);
        total += (w.div_ceil(8) * h.div_ceil(8)) as usize * 1024;
    }
    total
}

fn main() {
    let mut hist: BTreeMap<i64, usize> = BTreeMap::new();
    let mut examples: BTreeMap<i64, String> = BTreeMap::new();
    for (dir, names) in [
        (
            "omega-eu",
            vec!["data00", "data01", "data02", "data03", "data04"],
        ),
        (
            "omega-eu-patch",
            vec!["data05", "data07", "data08", "data09"],
        ),
    ] {
        for name in names {
            let spec = format!("data/extracted/ps4/{dir}/uroot/{name}.psarc");
            let Ok(mut archive) = Archive::open_file(std::path::Path::new(&spec)) else {
                continue;
            };
            let paths: Vec<String> = archive
                .paths()
                .iter()
                .filter(|p| p.to_ascii_lowercase().ends_with(".gnf"))
                .cloned()
                .collect();
            for path in paths {
                let Ok(blob) = archive.read_path(&path) else {
                    continue;
                };
                let Ok(t) = Texture::parse(&blob) else {
                    continue;
                };
                if t.surface_format != SurfaceFormat::Bc7 || t.tile_mode.0 != 13 {
                    continue;
                }
                let have = blob.len() as i64 - t.data_offset as i64;
                let diff = have - predicted(&t) as i64;
                *hist.entry(diff).or_default() += 1;
                examples.entry(diff).or_insert_with(|| {
                    format!("{path} {}x{} mips {}", t.width, t.height, t.last_mip_level)
                });
            }
        }
    }
    for (d, n) in &hist {
        println!("diff {d:>8}: {n:>5}  e.g. {}", examples[d]);
    }
}
