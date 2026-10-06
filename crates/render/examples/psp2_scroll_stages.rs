//! Which shader stage of each 2048 `.rcsmaterial` declares the scroll
//! uniforms (`time`, `TimeScaler`, `speed_multipliaer`, `frameRate`, the
//! emissive pair): vertex (V) or fragment (F). Reads the decoded GXP parameter
//! tables only; no instruction is decoded.
//!
//! ```sh
//! cargo run -p oag-render --example psp2_scroll_stages [package.psarc]
//! ```

use oag_rcs::gxp;
use std::collections::BTreeMap;

const NAMES: &[&str] = &[
    "time",
    "TimeScaler",
    "speed_multipliaer",
    "frameRate",
    "Emissive_UV_Offset",
    "Emissive_UV_Scale",
];

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/extracted/vita/PCSF00007/base/PSP2/data.psarc".into());
    let mut archive = oag_assets::psarc::Archive::open(&path)?;
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmaterial"))
        .cloned()
        .collect();
    // family -> shape string -> files
    let mut shapes: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for entry in paths {
        let blob = archive.read_path(&entry)?;
        let mut shape = Vec::new();
        for n in NAMES {
            let mut stages = String::new();
            let (mut v, mut f) = (false, false);
            for (_, p) in gxp::programs(&blob) {
                if let Ok(p) = p
                    && p.parameter(n).is_some()
                {
                    if p.is_fragment() { f = true } else { v = true }
                }
            }
            if v {
                stages.push('V')
            }
            if f {
                stages.push('F')
            }
            if !stages.is_empty() {
                shape.push(format!("{n}:{stages}"));
            }
        }
        if shape.is_empty() {
            continue;
        }
        let name = entry.rsplit('/').next().unwrap_or(&entry).to_string();
        *shapes
            .entry(name)
            .or_default()
            .entry(shape.join(" "))
            .or_default() += 1;
    }
    for (name, by) in shapes {
        for (shape, n) in by {
            println!("{name} x{n}: {shape}");
        }
    }
    Ok(())
}
