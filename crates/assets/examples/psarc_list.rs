//! Scratch tool: list every path in a loose `.psarc`, one per line.
//!
//! `cargo run -p oag-assets --example psarc_list -- <path.psarc> [substring]`

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: psarc_list <path> [substring]");
    let filter = args.next();

    let archive =
        oag_assets::psarc::Archive::open(&path).unwrap_or_else(|e| panic!("open {path}: {e}"));

    for p in archive.paths() {
        if filter
            .as_deref()
            .is_none_or(|f| p.to_ascii_lowercase().contains(&f.to_ascii_lowercase()))
        {
            println!("{p}");
        }
    }
    eprintln!("{} entries total", archive.paths().len());
}
