//! Which of Wipeout HD's surfaces scroll their texture off the engine clock.
//!
//! HD has no per-material texture-transform *keyframe* block - the mechanism
//! `oag_vex::vex::mesh_tex_transform` reads on the PSP, whose home is the
//! mesh payload HD emptied out. It scrolls in the **fragment program**
//! instead, off an engine-supplied `time` parameter
//! (`~crc32("time") == 0x906b67ba`, a preimage over `EBOOT.elf`).
//!
//! This counts what that reaches: materials whose fragment blocks declare
//! `time`, and the chunks of each circuit drawn through one.
//!
//! ```sh
//! cargo run -p oag-render --example hd_uv_time_census
//! ```

use oag_formats::{rcsmaterial, rcsmodel};

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
];

/// `~crc32("time")`, the engine clock every scrolling program is fed.
const TIME: u32 = 0x906b_67ba;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut timed: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut materials_seen = 0usize;
    let mut materials_timed = 0usize;

    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmaterial"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(material) = rcsmaterial::RcsMaterial::parse(&blob) else {
                continue;
            };
            materials_seen += 1;
            let takes_time = material.variants.iter().any(|v| {
                [v.fragment, v.vertex].iter().any(|b| {
                    rcsmaterial::Declared::parse(&blob, b.offset)
                        .is_some_and(|d| d.parameters.contains(&TIME))
                })
            });
            if takes_time {
                materials_timed += 1;
                let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                *timed.entry(name).or_default() += 1;
            }
        }

        // How much of each circuit those materials paint.
        let tracks: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.starts_with("/data/environments/") && p.ends_with("/track.rcsmodel"))
            .cloned()
            .collect();
        for path in tracks {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            let mut hit = 0usize;
            for material in &model.materials {
                let entry = normalise(&material.name);
                let Ok(mat) = open.read_path(&entry) else {
                    continue;
                };
                let Ok(parsed) = rcsmaterial::RcsMaterial::parse(&mat) else {
                    continue;
                };
                if parsed.variants.iter().any(|v| {
                    rcsmaterial::Declared::parse(&mat, v.fragment.offset)
                        .is_some_and(|d| d.parameters.contains(&TIME))
                }) {
                    hit += 1;
                }
            }
            println!(
                "{path}: {hit} of {} materials scroll off the clock",
                model.materials.len()
            );
        }
    }

    println!("\n{materials_timed} of {materials_seen} .rcsmaterial declare `time`");
    let mut rows: Vec<_> = timed.into_iter().collect();
    rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (name, n) in rows.iter().take(30) {
        println!("  x{n:<4} {name}");
    }
    println!("  ... {} distinct names", rows.len());
    Ok(())
}

/// A material record's stored path, as the archive spells its entry.
fn normalise(name: &str) -> String {
    let slashed = name.replace('\\', "/");
    if slashed.starts_with('/') {
        slashed
    } else {
        format!("/{slashed}")
    }
}
