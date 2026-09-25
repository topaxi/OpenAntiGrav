//! Scratch probe (not wired into any doc): grep every `.xml` entry in a
//! PSARC archive for a substring, printing the path and the matching lines.
//!
//! `cargo run -p oag-tools --example psarc_grep -- <data.psarc> <needle>`

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: psarc_grep <data.psarc> <needle>");
    let needle = args.next().expect("usage: psarc_grep <data.psarc> <needle>");
    let mut archive =
        oag_assets::psarc::Archive::open(&path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_lowercase().ends_with(".xml"))
        .cloned()
        .collect();
    for p in paths {
        let Ok(bytes) = archive.read_path(&p) else {
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        if text.to_lowercase().contains(&needle.to_lowercase()) {
            for line in text.lines() {
                if line.to_lowercase().contains(&needle.to_lowercase()) {
                    println!("{p}: {}", line.trim());
                }
            }
        }
    }
}
