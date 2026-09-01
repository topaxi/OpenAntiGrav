//! Scratch tool: print one entry of a loose `.psarc` to stdout.
//!
//! `cargo run -p oag-assets --example psarc_cat -- <path.psarc> <entry-path>`

use std::io::Write;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: psarc_cat <path> <entry-path>");
    let entry = args.next().expect("usage: psarc_cat <path> <entry-path>");

    let mut archive =
        oag_assets::psarc::Archive::open(&path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    let bytes = archive
        .read_path(&entry)
        .unwrap_or_else(|e| panic!("read {entry}: {e}"));
    std::io::stdout().write_all(&bytes).unwrap();
}
