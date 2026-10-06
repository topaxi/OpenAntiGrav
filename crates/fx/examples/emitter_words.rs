//! Print the words at `+0xc80..+0xc90` of every blend class 8 emitter in an
//! Omega `.psarc`: `cargo run -p oag-fx --example emitter_words -- <data05.psarc>`.
//!
//! `+0xc84` is the `kColourScale` the heat-haze program is fed
//! (`docs/ghidra/functions/ps4-omega-eu/heat-haze.md`).

use oag_assets::psarc::Archive;
use oag_pob as pob;

const WORDS: [usize; 5] = [0xc80, 0xc84, 0xc88, 0xc8c, 0xc90];

fn main() {
    let path = std::env::args().nth(1).expect("a .psarc path");
    let mut archive = Archive::open_file(std::path::Path::new(&path)).expect("the archive opens");
    let entries: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".pob") && p.contains("particles"))
        .cloned()
        .collect();
    for entry in entries {
        let blob = archive.read_path(&entry).expect("the entry reads");
        let Ok(system) = pob::ParticleSystem::parse(&blob) else {
            continue;
        };
        let base = system.resource_base();
        let Ok(emitters) = system.emitters(&blob) else {
            continue;
        };
        for emitter in emitters {
            let at = base + emitter.offset;
            let word = |k: usize| {
                let bytes = blob[at + k..at + k + 4].try_into().expect("four bytes");
                f32::from_bits(u32::from_le_bytes(bytes))
            };
            if emitter.blend_class != 8 {
                continue;
            }
            let values: Vec<String> = WORDS
                .iter()
                .map(|&k| format!("{k:#x}={}", word(k)))
                .collect();
            println!("{entry} {} {}", emitter.name, values.join(" "));
        }
    }
}
