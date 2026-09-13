//! Scratch: every `.gtf` path bound to sampler hash `EmissiveTexture`
//! (0xb1f2a176) disc-wide, and its dimensions.
use oag_rcs::rcsmodel;

const EMISSIVE_TEXTURE: u32 = 0xb1f2_a176;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let mut paths: std::collections::BTreeSet<String> = Default::default();
    for archive_index in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC");
        let Ok(mut psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let model_paths: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in model_paths {
            let Ok(blob) = psarc.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            for material in &model.materials {
                for (hash, p) in &material.samplers {
                    if *hash == EMISSIVE_TEXTURE
                        && let Some(p) = p
                    {
                        paths.insert(p.clone());
                    }
                }
            }
        }
    }
    println!(
        "{} distinct path(s) bound to EmissiveTexture disc-wide",
        paths.len()
    );
    for p in paths.iter().take(40) {
        println!("  {p}");
    }
    Ok(())
}
