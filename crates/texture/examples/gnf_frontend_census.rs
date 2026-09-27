//! Scratch probe: for the front-end sprite sheet and campaign hex textures -
//! the actual `.gnf` files `crates/game/src/boot/sprites.rs`'s
//! `gnf_sibling_report` and `crates/game/src/campaign.rs::load_omega` need
//! to draw - reports whether [`oag_texture::gnf::Texture::decode`] actually
//! draws each one, using the shipped decoder directly rather than
//! re-implementing its scan. A whole-file byte scan over-counts corruption
//! (it walks every later mip level's own tile-alignment padding too); this
//! reports the real, base-level-only outcome instead.
//!
//! ```sh
//! cargo run -q --release -p oag-texture --example gnf_frontend_census
//! ```

use std::collections::BTreeMap;

use oag_assets::psarc::Archive;
use oag_texture::gnf::{Error, Texture};

const OMEGA_BASE: &str = "data/extracted/ps4/omega-eu/uroot";
const OMEGA_PATCH: &str = "data/extracted/ps4/omega-eu-patch/uroot";
const BASE_ARCHIVES: &[&str] = &["data00", "data01", "data02", "data03", "data04"];
const PATCH_ARCHIVES: &[&str] = &["data05", "data07", "data08", "data09"];

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let targets_lower = [
        "data/fe/",
        "data/plugins/frontend/",
        "hexagon_hd",
        "hexmedal",
    ];

    let mut decoded = 0usize;
    let mut corrupt = 0usize;
    let mut unsupported = 0usize;
    let mut other_err = 0usize;
    let mut by_reason: BTreeMap<String, usize> = BTreeMap::new();

    for (spec, label) in &archives {
        let Ok(mut archive) = Archive::open_file(std::path::Path::new(spec)) else {
            continue;
        };
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| {
                let lower = p.to_ascii_lowercase();
                lower.ends_with(".gnf") && targets_lower.iter().any(|t| lower.contains(t))
            })
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = archive.read_path(&path) else {
                println!("{label} {path}: read failed");
                continue;
            };
            let texture = match Texture::parse(&blob) {
                Ok(t) => t,
                Err(e) => {
                    println!("{label} {path}: parse failed: {e}");
                    continue;
                }
            };
            match texture.decode(&blob) {
                Ok(_) => {
                    decoded += 1;
                    println!(
                        "{label} {path}: DRAWS {}x{} {:?}",
                        texture.width, texture.height, texture.surface_format
                    );
                }
                Err(Error::CorruptBlocks { count }) => {
                    corrupt += 1;
                    println!(
                        "{label} {path}: refused, {count} corrupt base-level block(s) ({}x{} {:?})",
                        texture.width, texture.height, texture.surface_format
                    );
                }
                Err(Error::UnsupportedFormat { format }) => {
                    unsupported += 1;
                    *by_reason
                        .entry(format!("unsupported format {format:?}"))
                        .or_default() += 1;
                }
                Err(e) => {
                    other_err += 1;
                    *by_reason.entry(format!("{e}")).or_default() += 1;
                }
            }
        }
    }

    println!(
        "\n=== {decoded} draw, {corrupt} refused (corrupt base level), {unsupported} unsupported format, {other_err} other error ==="
    );
    for (reason, n) in &by_reason {
        println!("  {n:>4}  {reason}");
    }

    Ok(())
}
