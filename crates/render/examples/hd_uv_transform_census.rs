//! What Wipeout HD's `uvScale`/`uvOffset` parameters actually hold.
//!
//! They are the disc's two commonest material parameters, 1,364 uses each and
//! always paired - `oag_render::examples::hd_param_names` recovered both names
//! by preimage. This asks the question that decides whether reading them is
//! worth anything: **how many are the identity?** A parameter that is
//! `(1, 1)` and `(0, 0)` everywhere costs nothing to ignore.
//!
//! ```sh
//! cargo run -p oag-render --example hd_uv_transform_census
//! ```

use oag_formats::rcsmodel;

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

/// `~crc32("uvScale")`.
const UV_SCALE: u32 = 0xea1d_cc4c;
/// `~crc32("uvOffset")`.
const UV_OFFSET: u32 = 0x1eb1_3436;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut records = 0usize;
    let mut carries = 0usize;
    let mut identity = 0usize;
    let mut moved: Vec<(String, [f32; 4], [f32; 4])> = Vec::new();
    let mut values: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();

    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let models: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in models {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            for material in &model.materials {
                records += 1;
                let find = |hash: u32| {
                    material
                        .parameters
                        .iter()
                        .find(|p| p.hash == hash)
                        .map(|p| p.value)
                };
                let (Some(scale), Some(offset)) = (find(UV_SCALE), find(UV_OFFSET)) else {
                    continue;
                };
                carries += 1;
                *values
                    .entry(format!(
                        "scale ({:.3}, {:.3})  offset ({:.3}, {:.3})",
                        scale[0], scale[1], offset[0], offset[1]
                    ))
                    .or_default() += 1;
                let is_identity = (scale[0] - 1.0).abs() < 1e-6
                    && (scale[1] - 1.0).abs() < 1e-6
                    && offset[0].abs() < 1e-6
                    && offset[1].abs() < 1e-6;
                if is_identity {
                    identity += 1;
                } else if moved.len() < 20 {
                    let short = material
                        .name
                        .rsplit(['/', '\\'])
                        .next()
                        .unwrap_or(&material.name)
                        .to_string();
                    moved.push((format!("{path} {short}"), scale, offset));
                }
            }
        }
    }

    println!("{records} material records, {carries} carry the pair, {identity} of those are the identity");
    println!("\ndistinct values, commonest first:");
    let mut rows: Vec<_> = values.into_iter().collect();
    rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (text, n) in rows.iter().take(20) {
        println!("  x{n:<5} {text}");
    }
    println!("  ... {} distinct", rows.len());
    println!("\nnon-identity examples:");
    for (name, scale, offset) in &moved {
        println!("  {name}\n    scale {scale:?}\n    offset {offset:?}");
    }
    Ok(())
}
