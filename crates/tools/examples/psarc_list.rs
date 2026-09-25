//! Scratch probe (not wired into any doc): list every path in a PSARC
//! archive, one per line, for grepping asset names.
//!
//! `cargo run -p oag-tools --example psarc_list -- <data.psarc>`

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: psarc_list <data.psarc>");
    let archive =
        oag_assets::psarc::Archive::open(&path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    for p in archive.paths() {
        println!("{p}");
    }
}
