//! Lists, per 2048 circuit, every material that authors a scroll uniform, and
//! (with an HD image) the same-named HD material's parameter values, so the
//! two titles' families can be compared by name and value.
//!
//! ```sh
//! cargo run -p oag-render --example psp2_scroll_census > out.txt
//! ```

use oag_rcs::rcsmaterial::{self, name_hash};
use oag_rcs::rcsmodel::{self, psp2};
use std::collections::BTreeMap;

const NAMES: &[&str] = &[
    "TimeScaler",
    "time",
    "speed_multipliaer",
    "frameRate",
    "Emissive_UV_Offset",
    "Emissive_UV_Scale",
    "GlowTint",
];

fn norm(name: &str) -> String {
    name.trim_start_matches(|c: char| c.is_ascii_digit() || c == '_')
        .to_ascii_lowercase()
}

fn main() -> anyhow::Result<()> {
    let _ = ();
    let hd_image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut hd: BTreeMap<String, Vec<(String, Vec<(String, Vec<f32>)>)>> = BTreeMap::new();
    for archive in [
        "PS3_GAME/USRDIR/DATA00.PSARC",
        "PS3_GAME/USRDIR/DATA02.PSARC",
        "PS3_GAME/USRDIR/DATA03.PSARC",
        "PS3_GAME/USRDIR/DATA06.PSARC",
    ] {
        let Ok(mut open) = oag_assets::psarc::Archive::open(&format!("{hd_image}:{archive}"))
        else {
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
            let mut rows = Vec::new();
            for m in &model.materials {
                let ps: Vec<(String, Vec<f32>)> = m
                    .parameters
                    .iter()
                    .map(|p| {
                        (
                            rcsmaterial::names::parameter_name(p.hash)
                                .map_or(format!("{:#010x}", p.hash), str::to_string),
                            p.value.to_vec(),
                        )
                    })
                    .collect();
                rows.push((m.name.clone(), ps));
            }
            hd.insert(circuit, rows);
        }
    }
    let mut archive =
        oag_assets::psarc::Archive::open("data/extracted/vita/PCSF00007/base/PSP2/data.psarc")?;
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.contains("/environments/") && p.ends_with("/track.rcsmodel"))
        .cloned()
        .collect();
    for path in paths {
        let circuit = path.split('/').rev().nth(1).unwrap_or("?").to_string();
        let Ok(blob) = archive.read_path(&path) else {
            continue;
        };
        let Ok(model) = psp2::parse(&blob) else {
            continue;
        };
        let hd_rows = hd
            .iter()
            .find(|(k, _)| norm(k) == norm(&circuit))
            .map(|(_, v)| v);
        println!("== {circuit} (HD has it: {})", hd_rows.is_some());
        let mut seen = std::collections::BTreeSet::new();
        for m in &model.materials {
            let vals: Vec<String> = NAMES
                .iter()
                .filter_map(|n| m.param(name_hash(n)).map(|v| format!("{n}={v:?}")))
                .collect();
            if vals.is_empty() || !seen.insert((m.name.clone(), vals.clone())) {
                continue;
            }
            println!("  2048 {} {}", m.name, vals.join(" "));
            if let Some(rows) = hd_rows {
                let short = m.name.rsplit(['/', '\\']).next().unwrap_or(&m.name);
                for (hn, ps) in rows {
                    let hs = hn.rsplit(['/', '\\']).next().unwrap_or(hn);
                    if hs.trim_end_matches(".rcsmaterial") == short.trim_end_matches(".rcsmaterial")
                    {
                        println!("     HD same name {hn}: {ps:?}");
                    }
                }
            }
        }
    }
    Ok(())
}
