fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let psarc = args.next().unwrap_or_default();
    let needle = args.next().unwrap_or_default().to_ascii_lowercase();
    let archive = oag_assets::psarc::Archive::open(&psarc)?;
    let mut hits: Vec<&String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().contains(&needle))
        .collect();
    hits.sort();
    for p in hits.iter().take(400) {
        println!("{p}");
    }
    println!("({} of {} entries)", hits.len(), archive.paths().len());
    Ok(())
}
