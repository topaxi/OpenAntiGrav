fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    for p in &a[2..] {
        let mut arc = oag_assets::psarc::Archive::open_file(std::path::Path::new(p))?;
        let hits: Vec<String> = arc
            .paths()
            .iter()
            .filter(|x| x.to_lowercase().contains(&a[1]))
            .cloned()
            .collect();
        for h in hits {
            let b = arc.read_path(&h)?;
            println!("=== {p} {h}\n{}", String::from_utf8_lossy(&b));
        }
    }
    Ok(())
}
