//! Scratch: dump one psarc entry to a file.
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let psarc = args.next().unwrap();
    let entry = args.next().unwrap();
    let out = args.next().unwrap();
    let mut archive = oag_assets::psarc::Archive::open(&psarc)?;
    let blob = archive.read_path(&entry)?;
    std::fs::write(&out, &blob)?;
    println!("{} bytes -> {out}", blob.len());
    Ok(())
}
