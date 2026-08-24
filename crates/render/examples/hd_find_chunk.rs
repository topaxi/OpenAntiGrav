//! Scratch probe: which `.rcsmodel` on the disc carries a given chunk hash.
//!
//! For the nodes a circuit's own model does not answer for - Talon's
//! Junction's sky traffic among them - the question is whether the geometry
//! is absent from the disc or merely absent from *this* file.

use oag_assets::Container;
use oag_formats::rcsmodel;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let wanted: Vec<u32> = args
        .map(|a| u32::from_str_radix(a.trim_start_matches("0x"), 16))
        .collect::<Result<_, _>>()?;
    if wanted.is_empty() {
        anyhow::bail!("give one or more chunk hashes");
    }

    // The archive reader has no listing call, so the names come from
    // `scripts/psarc.py list` through a file named by `$OAG_MODEL_LIST`.
    let list = std::env::var("OAG_MODEL_LIST").unwrap_or_else(|_| "/tmp/models.txt".to_string());
    let listing = std::fs::read_to_string(&list)?;
    let names: Vec<&str> = listing
        .lines()
        .map(str::trim)
        .filter(|n| n.to_ascii_lowercase().ends_with(".rcsmodel"))
        .collect();
    let mut container = Container::open(&spec)?;
    println!("scanning {} .rcsmodel in {spec}", names.len());
    let mut found: std::collections::BTreeMap<u32, Vec<String>> = Default::default();
    for &name in &names {
        let Ok(blob) = container.read_entry(name) else {
            continue;
        };
        let Ok(model) = rcsmodel::Model::parse(&blob) else {
            continue;
        };
        for &hash in &wanted {
            if model.mesh(hash).is_some() {
                found.entry(hash).or_default().push(name.to_string());
            }
        }
    }
    for &hash in &wanted {
        match found.get(&hash) {
            None => println!("{hash:#010x}: in none of them"),
            Some(v) => {
                println!("{hash:#010x}: {} model(s)", v.len());
                for n in v.iter().take(6) {
                    println!("    {n}");
                }
            }
        }
    }
    Ok(())
}
