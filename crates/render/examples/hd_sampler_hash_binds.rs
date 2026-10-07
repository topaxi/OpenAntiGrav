//! Census: every (family, texture path) a sampler hash binds across all 28
//! circuit models and the ships, with the entry index and whether the
//! material is lightmapped - the evidence a "this hash is only ever a normal
//! map" row needs.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_sampler_hash_binds -- data/images/hdfury-ps3-eu-dec.iso 0x48f37f5a
//! ```

use oag_mesh::mesh;
use oag_rcs::rcsmodel;

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
];

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().unwrap();
    let hash = u32::from_str_radix(args.next().unwrap().trim_start_matches("0x"), 16)?;
    let mut paths = std::collections::BTreeSet::new();
    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        for path in psarc.paths() {
            let wanted = (path.starts_with("/data/ships/") && path.ends_with("/ship.vex"))
                || (path.starts_with("/data/environments/")
                    && (path.ends_with("/track.vex") || path.ends_with("/track_reversed.vex")));
            if !wanted || path.matches('/').count() != 4 {
                continue;
            }
            let Ok(data) = mesh::read_blob(&spec, path) else {
                continue;
            };
            let Some(geometry) = mesh::rcs::sibling_geometry(&spec, path, &data) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&geometry) else {
                continue;
            };
            for material in &model.materials {
                for (i, (h, p)) in material.samplers.iter().enumerate() {
                    if *h == hash {
                        paths.insert(format!(
                            "{:<45} entry {i} lightmapped {:<5} {}",
                            material.name.rsplit('/').next().unwrap_or(""),
                            material.lightmap_entry().is_some(),
                            p.as_deref().unwrap_or("-").rsplit('/').next().unwrap_or(""),
                        ));
                    }
                }
            }
        }
    }
    for line in &paths {
        println!("{line}");
    }
    Ok(())
}
