//! Scratch probe: list or extract the entries of a PSARC whose path contains a
//! substring (case-insensitive).
//!
//! `cargo run -p oag-tools --example psarc_extract -- <data.psarc> <needle> [outdir]`
//! Without `outdir` it only lists the matches.

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: psarc_extract <psarc> <needle> [outdir]");
    let needle = args.next().expect("needle").to_lowercase();
    let out = args.next();
    let mut archive =
        oag_assets::psarc::Archive::open(&path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_lowercase().contains(&needle))
        .cloned()
        .collect();
    for p in paths {
        match &out {
            None => println!("{p}"),
            Some(dir) => {
                let bytes = archive.read_path(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
                let dest = std::path::Path::new(dir).join(p.trim_start_matches('/'));
                std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
                std::fs::write(&dest, &bytes).unwrap();
                println!("{} ({} bytes)", dest.display(), bytes.len());
            }
        }
    }
}
