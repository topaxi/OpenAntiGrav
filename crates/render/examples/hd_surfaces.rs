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
            for chunk in &model.meshes {
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
        }
    }
    println!("{models} model(s), {chunks} chunk(s)");
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
