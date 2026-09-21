//! Scratch probe: census every real `.gnf`'s `SurfaceFormat`/`TileMode`/size
//! across all nine Omega Collection archives (five base + four patch), and
//! find same-basename, same-dimension oracle pairs against Wipeout HD's own
//! `.gtf` copies and 2048's own `.gxt` copies of the same re-shipped art -
//! the same cross-title method `docs/formats/gxt.md`'s `PVRTII4BPP`/`UBC1`/
//! `UBC3` sections already used, generalised to a third codec and a third
//! container.
//!
//! Not part of the shipped decoder - this is the evidence-gathering pass
//! that decides which `TileMode`s and which `SurfaceFormat`s the real
//! untiler/BC7 work needs to cover, and how big the oracle set is before any
//! of that code exists.
//!
//! ```sh
//! cargo run -q --release -p oag-texture --example gnf_census
//! ```

use std::collections::{BTreeMap, BTreeSet};

use oag_assets::psarc::Archive;
use oag_texture::gnf::{self, SurfaceFormat};

const OMEGA_BASE: &str = "data/extracted/ps4/omega-eu/uroot";
const OMEGA_PATCH: &str = "data/extracted/ps4/omega-eu-patch/uroot";
const BASE_ARCHIVES: &[&str] = &["data00", "data01", "data02", "data03", "data04"];
const PATCH_ARCHIVES: &[&str] = &["data05", "data07", "data08", "data09"];

const HD_DIR: &str = "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR";
const HD_COUNT: usize = 7;

const VITA_2048: &[&str] = &[
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

#[derive(Default)]
struct Bucket {
    count: usize,
    min_w: u32,
    max_w: u32,
    min_h: u32,
    max_h: u32,
    archives: BTreeSet<String>,
    example: Option<String>,
}

impl Bucket {
    fn add(&mut self, archive: &str, path: &str, w: u32, h: u32) {
        if self.count == 0 {
            self.min_w = w;
            self.max_w = w;
            self.min_h = h;
            self.max_h = h;
            self.example = Some(path.to_string());
        } else {
            self.min_w = self.min_w.min(w);
            self.max_w = self.max_w.max(w);
            self.min_h = self.min_h.min(h);
            self.max_h = self.max_h.max(h);
        }
        self.count += 1;
        self.archives.insert(archive.to_string());
    }
}

fn format_name(f: SurfaceFormat) -> String {
    match f {
        SurfaceFormat::Format8_8_8_8 => "Format8_8_8_8".to_string(),
        SurfaceFormat::Bc1 => "Bc1".to_string(),
        SurfaceFormat::Bc2 => "Bc2".to_string(),
        SurfaceFormat::Bc3 => "Bc3".to_string(),
        SurfaceFormat::Bc4 => "Bc4".to_string(),
        SurfaceFormat::Bc5 => "Bc5".to_string(),
        SurfaceFormat::Bc6 => "Bc6".to_string(),
        SurfaceFormat::Bc7 => "Bc7".to_string(),
        SurfaceFormat::Other(v) => format!("Other(0x{v:02x})"),
        _ => "Unknown".to_string(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut census: BTreeMap<(String, u8), Bucket> = BTreeMap::new();
    let mut bad_magic = 0usize;
    let mut too_short = 0usize;
    let mut total_entries = 0usize;

    // (archive spec, human label)
    let mut archives: Vec<(String, String)> = Vec::new();
    for name in BASE_ARCHIVES {
        archives.push((format!("{OMEGA_BASE}/{name}.psarc"), format!("base/{name}")));
    }
    for name in PATCH_ARCHIVES {
        archives.push((
            format!("{OMEGA_PATCH}/{name}.psarc"),
            format!("patch/{name}"),
        ));
    }

    // gnf path (lowercased, full) -> (width, height, tile, format, mips)
    let mut gnf_index: BTreeMap<String, (u32, u32, u8, SurfaceFormat, u8)> = BTreeMap::new();

    for (spec, label) in &archives {
        let Ok(mut archive) = Archive::open_file(std::path::Path::new(spec)) else {
            println!("skip {label}: cannot open {spec}");
            continue;
        };
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".gnf"))
            .cloned()
            .collect();
        println!("{label}: {} .gnf entries", paths.len());
        for path in &paths {
            total_entries += 1;
            let Ok(bytes) = archive.read_path(path) else {
                continue;
            };
            match gnf::Texture::parse(&bytes) {
                Ok(t) => {
                    let mips = t.last_mip_level.saturating_sub(t.base_mip_level) + 1;
                    let key = (format_name(t.surface_format), t.tile_mode.0);
                    census
                        .entry(key)
                        .or_default()
                        .add(label, path, t.width, t.height);
                    gnf_index.insert(
                        path.to_ascii_lowercase(),
                        (t.width, t.height, t.tile_mode.0, t.surface_format, mips),
                    );
                }
                Err(gnf::Error::BadMagic { .. }) => bad_magic += 1,
                Err(gnf::Error::TooShort { .. }) => too_short += 1,
                Err(_) => too_short += 1,
            }
        }
    }

    println!(
        "\n=== SurfaceFormat x TileMode census ({total_entries} entries examined, {bad_magic} bad magic, {too_short} too short) ==="
    );
    for ((fmt, tile), bucket) in &census {
        println!(
            "  {fmt:16} tile=0x{tile:02x}  n={:<6} {:>4}x{:<4} .. {:>4}x{:<4}  archives={:?}  e.g. {}",
            bucket.count,
            bucket.min_w,
            bucket.min_h,
            bucket.max_w,
            bucket.max_h,
            bucket.archives,
            bucket.example.as_deref().unwrap_or("-"),
        );
    }

    // Oracle: HD .gtf basenames -> (w, h)
    let mut hd: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for index in 0..HD_COUNT {
        let path = format!("{HD_DIR}/DATA0{index}.PSARC");
        let Ok(mut archive) = Archive::open_file(std::path::Path::new(&path)) else {
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
            let Some(t) = parsed.only() else { continue };
            hd.insert(name, (u32::from(t.width), u32::from(t.height)));
        }
    }
    println!("\n{} HD .gtf basenames indexed", hd.len());

    // Oracle: 2048 .gxt basenames -> (w, h)
    let mut vita: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for spec in VITA_2048 {
        let Ok(mut archive) = Archive::open_file(std::path::Path::new(spec)) else {
            continue;
        };
        let entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".gxt"))
            .cloned()
            .collect();
        for entry in entries {
            let name = basename(&entry);
            if vita.contains_key(&name) {
                continue;
            }
            let Ok(blob) = archive.read_path(&entry) else {
                continue;
            };
            let Ok(parsed) = oag_texture::gxt::Gxt::parse(&blob) else {
                continue;
            };
            let Some(t) = parsed.only() else { continue };
            let (w, h) = t.level_size(0);
            vita.insert(name, (w, h));
        }
    }
    println!("{} Vita 2048 .gxt basenames indexed", vita.len());

    let mut hd_pairs = 0usize;
    let mut vita_pairs = 0usize;
    let mut hd_pairs_by_bucket: BTreeMap<(String, u8), usize> = BTreeMap::new();
    let mut vita_pairs_by_bucket: BTreeMap<(String, u8), usize> = BTreeMap::new();
    for (path, (w, h, tile, fmt, _mips)) in &gnf_index {
        let name = basename(path);
        let key = (format_name(*fmt), *tile);
        if let Some((hw, hh)) = hd.get(&name)
            && hw == w
            && hh == h
        {
            hd_pairs += 1;
            *hd_pairs_by_bucket.entry(key.clone()).or_default() += 1;
        }
        if let Some((vw, vh)) = vita.get(&name)
            && vw == w
            && vh == h
        {
            vita_pairs += 1;
            *vita_pairs_by_bucket.entry(key).or_default() += 1;
        }
    }

    println!(
        "\n=== Oracle pairs: {hd_pairs} .gnf<->.gtf (HD), {vita_pairs} .gnf<->.gxt (2048) same-name same-size ==="
    );
    println!("by (format, tile) for HD pairs:");
    for (k, n) in &hd_pairs_by_bucket {
        println!("  {k:?}: {n}");
    }
    println!("by (format, tile) for Vita pairs:");
    for (k, n) in &vita_pairs_by_bucket {
        println!("  {k:?}: {n}");
    }

    Ok(())
}
