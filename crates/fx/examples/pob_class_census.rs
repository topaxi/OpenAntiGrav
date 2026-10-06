//! Lists every emitter of every `.pob` in the PSARC archives named on the
//! command line whose blend class is not 2 or 3, or all emitters of the
//! effects named after `--effect`.
//!
//! ```sh
//! cargo run -q -p oag-fx --example pob_class_census -- ARCHIVE...
//! ```

use oag_assets::psarc::Archive;
use oag_pob as pob;

fn main() {
    for rel in std::env::args().skip(1) {
        let mut archive = Archive::open_file(std::path::Path::new(&rel)).expect("archive opens");
        let entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".pob"))
            .cloned()
            .collect();
        for entry in entries {
            let blob = archive.read_path(&entry).expect("reads");
            let Ok(system) = pob::ParticleSystem::parse(&blob) else {
                println!("{rel} {entry}: system does not parse");
                continue;
            };
            let Ok(emitters) = system.emitters(&blob) else {
                println!("{rel} {entry}: emitters do not walk");
                continue;
            };
            for e in emitters {
                if !matches!(e.blend_class, 2 | 3) {
                    println!(
                        "{rel} {entry} emitter {:?}: blend {} render {} flags {:#x} sprite? dur {}",
                        e.name, e.blend_class, e.render_mode, e.flags, e.duration_ticks
                    );
                }
            }
        }
    }
}
