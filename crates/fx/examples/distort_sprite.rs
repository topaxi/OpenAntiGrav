//! Print the sprite each blend class 8 emitter of an Omega effect displaces by,
//! its GNF channel type, and where its `rg` sits:
//! `cargo run -p oag-fx --example distort_sprite -- <data05.psarc> [EFFECT...]`.
//!
//! The heat-haze program's displacement is `sprite.rg - 0.5`, so the byte that
//! means "no displacement" decides whether the sheet's raw read is right
//! (`docs/ghidra/functions/ps4-omega-eu/heat-haze.md`).

use oag_assets::psarc::Archive;
use oag_pob as pob;
use oag_texture::gnf;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("a .psarc path");
    let mut names: Vec<String> = args.collect();
    if names.is_empty() {
        names = ["WO_ROCKET_EXPLO", "WO_MISSILE_EXPLO", "WO_RB_HEATHAZE"]
            .map(String::from)
            .to_vec();
    }
    let mut archive = Archive::open_file(std::path::Path::new(&path)).expect("the archive opens");
    for name in names {
        let blob = archive
            .read_path(&format!("Data/particles/{name}.pob"))
            .expect("the effect ships");
        let system = pob::ParticleSystem::parse(&blob).expect("it parses");
        for emitter in system.emitters(&blob).expect("emitters") {
            if emitter.blend_class != 8 {
                continue;
            }
            let Some(texture) = system.texture_path(&blob, &emitter) else {
                println!("{name} {}: no texture path", emitter.name);
                continue;
            };
            let stem = texture
                .rsplit(['\\', '/'])
                .next()
                .and_then(|file| file.rsplit_once('.'))
                .map_or("", |(stem, _)| stem);
            let found: Vec<String> = archive
                .paths()
                .iter()
                .filter(|p| {
                    p.to_ascii_lowercase()
                        .ends_with(&format!("{}.gnf", stem.to_ascii_lowercase()))
                })
                .cloned()
                .collect();
            let Some(gnf_path) = found.first() else {
                println!("{name} {}: {stem}.gnf is not in the archive", emitter.name);
                continue;
            };
            let gnf_blob = archive.read_path(gnf_path).expect("it reads");
            let parsed = gnf::Texture::parse(&gnf_blob).expect("a gnf");
            let pixels = parsed.decode(&gnf_blob).expect("it decodes");
            let mut histogram = [[0u32; 256]; 2];
            let mut alpha_sum = 0u64;
            for p in &pixels {
                alpha_sum += u64::from(p[3]);
                for channel in 0..2 {
                    histogram[channel][usize::from(p[channel])] += u32::from(p[3]);
                }
            }
            let median = |channel: usize| {
                let mut seen = 0u64;
                for (value, count) in histogram[channel].iter().enumerate() {
                    seen += u64::from(*count);
                    if seen * 2 >= alpha_sum {
                        return value;
                    }
                }
                0
            };
            let corner = pixels[0];
            println!(
                "{name} {}: {gnf_path} {}x{} {:?}; alpha-weighted median r {} g {}; corner {:?}; \
                 mean alpha {:.1}",
                emitter.name,
                parsed.width,
                parsed.height,
                parsed.channel_type,
                median(0),
                median(1),
                corner,
                alpha_sum as f64 / pixels.len() as f64
            );
        }
    }
}
