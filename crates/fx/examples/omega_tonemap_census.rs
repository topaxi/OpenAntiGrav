//! Which Omega circuits author a complete `Tonemap` block, and so get the
//! tone-map chain that composites the distortion offsets:
//! `cargo run -p oag-fx --example omega_tonemap_census -- <uroot-dir>...`
//! (for example `data/extracted/ps4/omega-eu/uroot` and the patch's).
//!
//! The race builds `oag_post::omega_tonemap::Chain` only from this block
//! (`oag_raceplay::load::environment::light::envsettings_tonemap`), and the
//! chain is the only consumer of `oag_fx::psys::Pipeline::encode_distort`.

use oag_assets::psarc::Archive;
use oag_tables::envsettings::EnvSettings;

fn main() {
    let mut with = 0;
    let mut without = Vec::new();
    for dir in std::env::args().skip(1) {
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .expect("the directory reads")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "psarc"))
            .collect();
        files.sort();
        for file in files {
            let mut archive = Archive::open_file(&file).expect("the archive opens");
            let entries: Vec<String> = archive
                .paths()
                .iter()
                .filter(|p| {
                    let lower = p.to_ascii_lowercase();
                    lower.ends_with(".envsettings")
                        && lower.contains("environments")
                        && !lower.contains("/fe/")
                })
                .cloned()
                .collect();
            for entry in entries {
                let blob = archive.read_path(&entry).expect("the entry reads");
                let Ok(text) = String::from_utf8(blob) else {
                    without.push(format!("{entry} (not text)"));
                    continue;
                };
                match EnvSettings::parse(&text) {
                    Ok(env) if env.tonemap("Tonemap").is_some() => with += 1,
                    Ok(_) => without.push(format!("{} in {}", entry, file.display())),
                    Err(e) => without.push(format!("{entry} ({e})")),
                }
            }
        }
    }
    println!("{with} circuit file(s) author a complete Tonemap block");
    println!("{} do not:", without.len());
    for entry in without {
        println!("  {entry}");
    }
}
