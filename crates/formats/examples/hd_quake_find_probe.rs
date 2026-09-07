//! Scratch probe: locate and extract Wipeout HD/Fury's Quake-equivalent
//! `.pob` (`WO_QUAKE_DETONATOR_TRAILS`, see `docs/formats/pob.md`) from the
//! seven PSARC archives, for the "is the Quake's wave the right colour"
//! investigation.
//!
//! ```sh
//! cargo run -q -p oag-formats --example hd_quake_find_probe
//! ```

const ISO: &str = "data/images/hdfury-ps3-eu-dec.iso";
const ARCHIVES: usize = 7;

fn main() {
    let iso = std::path::Path::new(ISO);
    if !iso.exists() {
        println!("skipping: {ISO} not present");
        return;
    }
    for n in 0..ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", iso.display());
        let Ok(mut archive) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_lowercase().contains("quake"))
            .cloned()
            .collect();
        for path in paths {
            println!("DATA0{n}: {path}");
            if let Ok(blob) = archive.read_path(&path) {
                let out = format!("/tmp/{}", path.replace('/', "_"));
                std::fs::write(&out, &blob).unwrap();
                println!("  wrote {out} ({} bytes)", blob.len());
            }
        }
    }
}
