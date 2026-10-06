//! Lists every entry of a PS3 disc's seven PSARCs whose path contains a
//! needle (case-insensitive).
//!
//! ```sh
//! cargo run -p oag-render --example hd_psarc_find -- data/images/hdfury-ps3-eu-dec.iso psys rain
//! ```

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: <image> <needle>..."))?;
    let needles: Vec<String> = args.map(|n| n.to_lowercase()).collect();
    for n in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA0{n}.PSARC");
        let archive = oag_assets::psarc::Archive::open(&spec)?;
        let total = archive.paths().len();
        let mut hits = 0;
        for path in archive.paths() {
            let lower = path.to_lowercase();
            if needles.iter().all(|needle| lower.contains(needle)) {
                println!("DATA0{n}: {path}");
                hits += 1;
            }
        }
        eprintln!("DATA0{n}: {hits} of {total} entries match");
    }
    Ok(())
}
