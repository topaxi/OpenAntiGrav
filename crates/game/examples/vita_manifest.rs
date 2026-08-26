fn main() -> anyhow::Result<()> {
    let archive =
        oag_assets::psarc::Archive::open("data/extracted/vita/PCSF00007/base/PSP2/data.psarc")?;
    let mut args = std::env::args().skip(1);
    let needle = args.next().unwrap_or_default().to_ascii_lowercase();
    let suffix = args.next().unwrap_or_default().to_ascii_lowercase();
    let mut hits: Vec<&String> = archive
        .paths()
        .iter()
        .filter(|p| {
            let low = p.to_ascii_lowercase();
            low.contains(&needle) && low.ends_with(&suffix)
        })
        .collect();
    hits.sort();
    for p in hits.iter().take(40) {
        println!("{p}");
    }
    println!("({} of {} entries)", hits.len(), archive.paths().len());
    Ok(())
}
