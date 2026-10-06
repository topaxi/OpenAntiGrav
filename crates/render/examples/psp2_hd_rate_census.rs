//! Which Vita/PS4 `.rcsmodel` materials author one of the rate parameters
//! Wipeout HD's vertex scroll reads (`USpeed`, `VSpeed`, `Speed`, the unnamed
//! pairs), counted by shader and by which hashes it carries.
//!
//! ```sh
//! cargo run -p oag-render --example psp2_hd_rate_census -- <archive.psarc>...
//! ```

use oag_rcs::rcsmodel::psp2;
use std::collections::BTreeMap;

const RATES: &[(u32, &str)] = &[
    (0x1abb_e1f7, "USpeed"),
    (0x9c2f_9359, "VSpeed"),
    (0x87d7_69dc, "cf_x"),
    (0x2481_ef75, "cf_y"),
    (0x3118_2e0d, "Speed"),
    (0x6829_2521, "bloom_v"),
    (0x33d5_1367, "uvalpha_u"),
    (0x336d_2dcc, "V_Offset"),
];

fn main() -> anyhow::Result<()> {
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for path in std::env::args().skip(1) {
        let mut archive = oag_assets::psarc::Archive::open_file(std::path::Path::new(&path))?;
        let models: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for m in models {
            let Ok(blob) = archive.read_path(&m) else {
                continue;
            };
            let Ok(model) = psp2::parse(&blob) else {
                continue;
            };
            for mat in &model.materials {
                let hits: Vec<String> = RATES
                    .iter()
                    .filter_map(|&(h, n)| {
                        mat.param(h)
                            .map(|v| format!("{n}={:?}", v.first().copied().unwrap_or(0.0)))
                    })
                    .collect();
                if hits.is_empty() {
                    continue;
                }
                let name = mat.name.rsplit(['/', '\\']).next().unwrap_or(&mat.name);
                let key = format!(
                    "{name} [{}] textures={}",
                    hits.iter()
                        .map(|h| h.split('=').next().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join(","),
                    mat.samplers.len()
                );
                *out.entry(key).or_default() += 1;
            }
        }
    }
    for (k, n) in out {
        println!("{n:5} {k}");
    }
    Ok(())
}
