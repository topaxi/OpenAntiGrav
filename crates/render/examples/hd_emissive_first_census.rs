//! Scratch census: how many materials disc-wide have `EmissiveTexture`
//! positionally first among their populated sampler entries, in a material
//! that also names a lightmap - the exact shape `picks()`'s lightmap branch
//! resolves by position, and the one Amphiseum's `track_wall.rcsmaterial`
//! (slot 549) hits.
use oag_rcs::rcsmodel;

const EMISSIVE_TEXTURE: u32 = 0xb1f2_a176;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let (mut total, mut emissive_first, mut has_better) = (0usize, 0usize, 0usize);
    let mut examples = Vec::new();
    for archive_index in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC");
        let Ok(mut psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = psarc.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            for material in &model.materials {
                if material.lightmap_entry().is_none() {
                    continue;
                }
                total += 1;
                let first_populated = material.samplers.iter().find(|(_, p)| p.is_some());
                let Some((hash, _)) = first_populated else {
                    continue;
                };
                if *hash == EMISSIVE_TEXTURE {
                    emissive_first += 1;
                    let better = material.samplers.iter().skip(1).any(|(h, p)| {
                        p.is_some()
                            && *h != EMISSIVE_TEXTURE
                            && *h != oag_rcs::rcsmaterial::LIGHTMAP_SAMPLER
                    });
                    if better {
                        has_better += 1;
                        if !examples.contains(&material.name) {
                            examples.push(material.name.clone());
                        }
                    }
                }
            }
        }
    }
    println!("{total} lightmapped material(s) total");
    println!("{emissive_first} have EmissiveTexture positionally first");
    println!("{has_better} of those also name a later, non-lightmap, non-emissive entry");
    for e in examples {
        println!("  e.g. {e}");
    }
    Ok(())
}
