//! Scratch probe: per-channel statistics of the `.gtf` files one material
//! names, for deciding whether a texture carries a silhouette or a flat plate.

use oag_assets::Container;
use oag_render::mesh;

fn stats(name: &str, blob: &[u8]) -> anyhow::Result<()> {
    let parsed = oag_texture::gtf::Gtf::parse(blob)?;
    let Some(texture) = parsed.only() else {
        println!("{name}: not one texture");
        return Ok(());
    };
    let (w, h) = texture.level_size(0);
    println!(
        "{name}: {w}x{h} {:?} remap {:#010x}",
        texture.format, texture.remap
    );
    let rgba = texture.to_rgba(blob)?;
    if std::env::var("OAG_DUMP_PNG").is_ok() {
        let out = format!("/tmp/{}.png", name.replace(['/', ' '], "_"));
        let flat: Vec<u8> = rgba.iter().flatten().copied().collect();
        std::fs::write(&out, oag_texture::png::encode_rgba(w, h, &flat))?;
        println!("  wrote {out}");
    }
    for (c, label) in ["r", "g", "b", "a"].iter().enumerate() {
        let (mut min, mut max, mut sum) = (255u32, 0u32, 0u64);
        let (mut u32c, mut u64c, mut u128c) = (0usize, 0usize, 0usize);
        for t in &rgba {
            let v = u32::from(t[c]);
            min = min.min(v);
            max = max.max(v);
            sum += u64::from(v);
            u32c += usize::from(v < 32);
            u64c += usize::from(v < 64);
            u128c += usize::from(v < 128);
        }
        let n = rgba.len() as f64;
        println!(
            "  {label}: {min}..{max} mean {:.1}  <32 {:.1}%  <64 {:.1}%  <128 {:.1}%",
            sum as f64 / n,
            u32c as f64 / n * 100.0,
            u64c as f64 / n * 100.0,
            u128c as f64 / n * 100.0
        );
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let vex = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let want = args.next().unwrap_or_else(|| "clouds".into());

    let data = mesh::read_blob(&spec, &vex)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &vex, &data)
        .ok_or_else(|| anyhow::anyhow!("{vex}: no sibling .rcsmodel"))?;
    let model = oag_rcs::rcsmodel::Model::parse(&geometry)?;

    let mut container = Container::open(&spec)?;
    let mut seen = std::collections::BTreeSet::new();
    for (slot, material) in model.materials.iter().enumerate() {
        if !material.name.contains(&want) {
            continue;
        }
        println!(
            "slot {slot} {}\n  texture {:?}\n  second  {:?}",
            material.name, material.texture, material.second_texture
        );
        for path in [
            Some(material.texture.clone()),
            material.second_texture.clone(),
        ]
        .into_iter()
        .flatten()
        {
            if path.is_empty() || !seen.insert(path.clone()) {
                continue;
            }
            match container.read_entry(&path) {
                Ok(blob) => stats(&path, &blob)?,
                Err(e) => println!("  {path}: {e}"),
            }
        }
    }
    Ok(())
}
