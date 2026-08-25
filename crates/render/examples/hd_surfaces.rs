//! Scratch probe: how much of the disc's geometry lives in a chunk's surfaces
//! past the first, and how often those name a different material.
//!
//! See `docs/formats/rcsmodel.md`, "A chunk names one material here and the
//! engine reads several".

use oag_formats::rcsmodel;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let only = std::env::args().nth(2).unwrap_or_default();
    let (mut models, mut chunks, mut multi, mut mixed) = (0usize, 0usize, 0usize, 0usize);
    let (mut tri_first, mut tri_extra) = (0u64, 0u64);
    let (mut sub_first, mut sub_extra) = (0usize, 0usize);
    // Declared against parsed: a surface the parser could not read is skipped
    // silently, which is the failure mode this whole exercise is about.
    let (mut declared, mut parsed, mut worst): (u64, u64, Vec<(usize, String)>) =
        (0, 0, Vec::new());
    for archive_index in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC");
        let Ok(mut psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel") && p.contains(&only))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = psarc.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            models += 1;
            let be32 = |at: usize| -> u32 {
                blob.get(at..at + 4)
                    .map_or(0, |s| u32::from_be_bytes(s.try_into().unwrap()))
            };
            let be16 = |at: usize| -> u16 {
                blob.get(at..at + 2)
                    .map_or(0, |s| u16::from_be_bytes(s.try_into().unwrap()))
            };
            let chunk_table = be32(0x20) as usize;
            let mut missed = 0usize;
            for (index, chunk) in model.meshes.iter().enumerate() {
                let at = be32(chunk_table + index * 4) as usize;
                let want = usize::from(be16(at + 0x10)).max(1);
                let got = 1 + chunk.extra_surfaces.len();
                declared += want as u64;
                parsed += got as u64;
                missed += want.saturating_sub(got);
                chunks += 1;
                let triangles = |m: &rcsmodel::Mesh| -> u64 {
                    m.submeshes.iter().map(|s| (s.index_count / 3) as u64).sum()
                };
                tri_first += triangles(chunk);
                sub_first += chunk.submeshes.len();
                if chunk.extra_surfaces.is_empty() {
                    continue;
                }
                multi += 1;
                for extra in &chunk.extra_surfaces {
                    tri_extra += triangles(extra);
                    sub_extra += extra.submeshes.len();
                }
                if chunk
                    .extra_surfaces
                    .iter()
                    .any(|e| e.material != chunk.material)
                {
                    mixed += 1;
                }
            }
            if missed > 0 {
                worst.push((missed, path.clone()));
            }
        }
    }
    println!("{models} model(s), {chunks} chunk(s)");
    println!(
        "  {declared} surface(s) declared, {parsed} parsed, {} skipped ({:.2}%)",
        declared - parsed,
        (declared - parsed) as f64 / declared as f64 * 100.0,
    );
    worst.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
    for (n, path) in worst.iter().take(6) {
        println!("    {n:>7} skipped in {path}");
    }
    println!("  {multi} declare a surface past the first; {mixed} of those name another material");
    println!(
        "  {sub_first} submesh(es) on first surfaces, {sub_extra} on the rest ({:.1}% more)",
        sub_extra as f64 / sub_first as f64 * 100.0,
    );
    println!(
        "  {tri_first} triangle(s) on first surfaces, {tri_extra} on the rest ({:.1}% more)",
        tri_extra as f64 / tri_first as f64 * 100.0,
    );
    Ok(())
}
