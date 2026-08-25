//! Scratch probe: what the `kk` byte at a chunk's `+0x07` correlates with.
//!
//! `rcsmodel`'s `LAYOUT_DESCRIBED` has carried it as "takes the values `01`
//! and `02` for a reason nothing here has distinguished". The engine branches
//! on it. This asks the disc what else differs between the two populations.

use oag_formats::rcsmodel;

#[derive(Default, Debug)]
struct Tally {
    chunks: usize,
    described: usize,
    inline: usize,
    multi_surface: usize,
    extra_surfaces: usize,
    submeshes: usize,
    triangles: u64,
    vertices: u64,
    with_decl: usize,
    see_through: usize,
    strides: std::collections::BTreeMap<usize, usize>,
    /// Index counts that are not a multiple of three - a strip would be.
    non_triangular: usize,
    /// Submeshes whose vertex count exceeds what a `u16` index can address.
    over_65536: usize,
    materials: std::collections::BTreeMap<String, usize>,
    /// |bias|, which is a world position on baked geometry and near zero on
    /// anything authored in its own space.
    bias: Vec<f32>,
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let only = std::env::args().nth(2).unwrap_or_default();
    let mut by_kk: std::collections::BTreeMap<u8, Tally> = Default::default();
    // kk by where the model lives, which is the cheapest test of "world
    // geometry versus object geometry".
    let mut by_dir: std::collections::BTreeMap<String, std::collections::BTreeMap<u8, usize>> =
        Default::default();
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
            let be32 = |at: usize| -> u32 {
                blob.get(at..at + 4)
                    .map_or(0, |s| u32::from_be_bytes(s.try_into().unwrap()))
            };
            let table = be32(0x20) as usize;
            for (index, chunk) in model.meshes.iter().enumerate() {
                let at = be32(table + index * 4) as usize;
                let Some(&kk) = blob.get(at + 0x07) else {
                    continue;
                };
                let bucket = path
                    .trim_start_matches('/')
                    .split('/')
                    .take(2)
                    .collect::<Vec<_>>()
                    .join("/");
                *by_dir.entry(bucket).or_default().entry(kk).or_default() += 1;
                let t = by_kk.entry(kk).or_default();
                t.chunks += 1;
                match chunk.layout {
                    rcsmodel::Layout::Described => t.described += 1,
                    rcsmodel::Layout::Inline => t.inline += 1,
                }
                if !chunk.extra_surfaces.is_empty() {
                    t.multi_surface += 1;
                }
                t.extra_surfaces += chunk.extra_surfaces.len();
                t.with_decl += usize::from(chunk.decl.is_some());
                if let Some(stride) = chunk.declared_stride() {
                    *t.strides.entry(stride).or_default() += 1;
                }
                if let Some(m) = model.material_of(chunk) {
                    let name = m
                        .name
                        .rsplit('/')
                        .next()
                        .unwrap_or("?")
                        .trim_end_matches(".rcsmaterial")
                        .to_string();
                    *t.materials.entry(name).or_default() += 1;
                    t.see_through += usize::from(m.transparency().is_some());
                }
                t.bias.push(
                    (chunk.bias[0].powi(2) + chunk.bias[1].powi(2) + chunk.bias[2].powi(2)).sqrt(),
                );
                for s in &chunk.submeshes {
                    t.submeshes += 1;
                    t.triangles += (s.index_count / 3) as u64;
                    t.vertices += s.vertex_count as u64;
                    if s.index_count % 3 != 0 {
                        t.non_triangular += 1;
                    }
                    if s.vertex_count > 65_536 {
                        t.over_65536 += 1;
                    }
                }
            }
        }
    }
    for (bucket, counts) in &by_dir {
        println!("{bucket:<28} {counts:?}");
    }
    println!();
    for (kk, t) in &by_kk {
        println!("=== kk = {kk:#04x}: {} chunk(s)", t.chunks);
        println!(
            "   layout described {} / inline {}; declaration on {}",
            t.described, t.inline, t.with_decl
        );
        println!(
            "   {} multi-surface ({} extra surfaces), {} submesh(es), {} triangle(s), {} vertex(es)",
            t.multi_surface, t.extra_surfaces, t.submeshes, t.triangles, t.vertices,
        );
        println!(
            "   see-through material {}, index count not divisible by 3 on {}, >65536 verts on {}",
            t.see_through, t.non_triangular, t.over_65536,
        );
        let mut strides: Vec<_> = t.strides.iter().collect();
        strides.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        println!(
            "   strides {:?}",
            strides.iter().take(8).collect::<Vec<_>>()
        );
        let mut bias = t.bias.clone();
        bias.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if !bias.is_empty() {
            let q = |f: f64| bias[((bias.len() - 1) as f64 * f) as usize];
            println!(
                "   |bias|: p10 {:.1}  median {:.1}  p90 {:.1}  max {:.1}; {} under 1.0 unit ({:.1}%)",
                q(0.1),
                q(0.5),
                q(0.9),
                bias[bias.len() - 1],
                bias.iter().filter(|b| **b < 1.0).count(),
                bias.iter().filter(|b| **b < 1.0).count() as f64 / bias.len() as f64 * 100.0,
            );
        }
        let mut mats: Vec<_> = t.materials.iter().collect();
        mats.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        println!(
            "   top materials {:?}",
            mats.iter()
                .take(8)
                .map(|(n, c)| format!("{n} x{c}"))
                .collect::<Vec<_>>()
        );
    }
    Ok(())
}
