//! Writes one entry out of the first archive that holds it.
//!
//! ```sh
//! cargo run -p oag-rcs --example psarc_extract -- <entry> <out file> <archive.psarc>...
//! ```

use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [entry, out, archives @ ..] = args.as_slice() else {
        eprintln!("usage: psarc_extract <entry> <out file> <archive.psarc>...");
        std::process::exit(2);
    };
    let blob = archives
        .iter()
        .find_map(|a| {
            oag_assets::psarc::Archive::open_file(Path::new(a))
                .ok()?
                .read_path(entry)
                .ok()
        })
        .expect("entry in an archive");
    std::fs::write(out, &blob).expect("writes");
    println!("{out}: {} bytes", blob.len());
}
