//! Scratch: pulls one named entry out of a loose `.psarc` into a file.
//!
//! `cargo run -p oag-rcs --example omega_extract -- <a.psarc> <entry path> <out>`

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [archive, entry, out] = args.as_slice() else {
        return Err("usage: <a.psarc> <entry path> <out>".into());
    };
    let mut archive = oag_assets::psarc::Archive::open(archive)?;
    let blob = archive.read_path(entry)?;
    std::fs::write(out, &blob)?;
    println!("{entry}: {} bytes -> {out}", blob.len());
    Ok(())
}
