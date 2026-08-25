//! Scratch probe: PSARC paths matching a substring, across all seven archives.

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let needle = std::env::args().nth(2).unwrap_or_default().to_lowercase();
    for archive_index in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC");
        let Ok(psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        for path in psarc.paths() {
            if path.to_lowercase().contains(&needle) {
                println!("DATA{archive_index:02} {path}");
            }
        }
    }
    Ok(())
}
