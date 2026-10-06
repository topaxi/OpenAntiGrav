//! Which of Wipeout HD's circuit materials author motion, and through which
//! parameter. HD has no keyframe block (see `hd_uv_time_census`); the motion is
//! a parameter the material record authors or a clock the fragment program
//! declares. This lists, per circuit, every material that does either, grouped
//! by shader name, so a family nothing reads is visible as a row.
//!
//! ```sh
//! cargo run -p oag-render --example hd_anim_family_census
//! ```

use oag_rcs::{rcsmaterial, rcsmodel};
use std::collections::BTreeMap;

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];
const TIME: u32 = 0x906b_67ba;
/// Authored parameter names (preimages) that read as a motion rate or offset.
const MOTION_PARAMS: &[&str] = &[
    "speed",
    "Speed",
    "V_Offset",
    "VSpeed",
    "UV_offset",
    "W_Cycle",
    "TimeScaler",
    "USpeed",
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    // (shader, motion-param names, frag declares time) -> circuits -> count
    type Key = (String, String, &'static str);
    let mut fam: BTreeMap<Key, BTreeMap<String, usize>> = BTreeMap::new();
    for archive in ARCHIVES {
        let Ok(mut open) = oag_assets::psarc::Archive::open(&format!("{image}:{archive}")) else {
            continue;
        };
        let tracks: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.starts_with("/data/environments/") && p.ends_with("/track.rcsmodel"))
            .cloned()
            .collect();
        for path in tracks {
            let circuit = path.split('/').nth(3).unwrap_or("?").to_string();
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            for material in &model.materials {
                let entry = {
                    let s = material.name.replace('\\', "/");
                    if s.starts_with('/') {
                        s
                    } else {
                        format!("/{s}")
                    }
                };
                let mut motion: Vec<&str> = material
                    .parameters
                    .iter()
                    .filter_map(|p| rcsmaterial::names::parameter_name(p.hash))
                    .filter(|n| MOTION_PARAMS.contains(n))
                    .collect();
                motion.sort();
                motion.dedup();
                let mut takes_time = "";
                if let Ok(mat) = open.read_path(&entry)
                    && let Ok(parsed) = rcsmaterial::RcsMaterial::parse(&mat)
                {
                    {
                        let any = |pick: fn(&rcsmaterial::Variant) -> rcsmaterial::Block| {
                            parsed.variants.iter().any(|v| {
                                rcsmaterial::Declared::parse(&mat, pick(v).offset)
                                    .is_some_and(|d| d.parameters.contains(&TIME))
                            })
                        };
                        takes_time = match (any(|v| v.vertex), any(|v| v.fragment)) {
                            (true, true) => "vertex+fragment",
                            (true, false) => "vertex",
                            (false, true) => "fragment",
                            _ => "",
                        };
                    }
                }
                if motion.is_empty() && takes_time.is_empty() {
                    continue;
                }
                let shader = entry.rsplit('/').next().unwrap_or(&entry).to_string();
                *fam.entry((shader, motion.join("+"), takes_time))
                    .or_default()
                    .entry(circuit.clone())
                    .or_default() += 1;
            }
        }
    }
    for ((shader, motion, time), circuits) in &fam {
        let total: usize = circuits.values().sum();
        println!(
            "{shader:<48} params=[{motion}] time=[{time}] total={total} circuits={}",
            circuits.len()
        );
        let per: Vec<String> = circuits.iter().map(|(c, n)| format!("{c}:{n}")).collect();
        println!("    {}", per.join(" "));
    }
    Ok(())
}
