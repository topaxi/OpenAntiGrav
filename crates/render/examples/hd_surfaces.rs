//! Scratch probe: the per-chunk surface table at `+0x10`/`+0x18` that
//! `oag_formats::rcsmodel` does not read, and whether a chunk's surfaces ever
//! name more than one material.
//!
//! `Mesh::material` is one index per chunk, taken from `+0x20`. The engine also
//! walks a surface table and indexes the material table per *surface*
//! (`0x003faf00`, `0x003fb330`). If a chunk's surfaces always agree with its
//! `+0x20`, nothing is lost; if they disagree, part of that chunk is being
//! painted with the wrong material.

use oag_formats::rcsmodel;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let mut chunks_total = 0usize;
    let mut multi = 0usize;
    let mut disagree = 0usize;
    let mut mixed = 0usize;
    let mut models = 0usize;
    let mut worst: Vec<(usize, String, u32, Vec<u32>)> = Vec::new();
    let (mut aligned, mut misaligned) = (0usize, 0usize);
    for archive_index in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC");
        let Ok(mut psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(b) = psarc.read_path(&path) else {
                continue;
            };
            let be32 = |at: usize| -> Option<u32> {
                b.get(at..at + 4)
                    .map(|s| u32::from_be_bytes(s.try_into().unwrap()))
            };
            let be16 = |at: usize| -> Option<u16> {
                b.get(at..at + 2)
                    .map(|s| u16::from_be_bytes(s.try_into().unwrap()))
            };
            let (Some(count), Some(table), Some(materials)) = (be32(0x1c), be32(0x20), be32(0x2c))
            else {
                continue;
            };
            if count == 0 || count > 100_000 {
                continue;
            }
            models += 1;
            let parsed = rcsmodel::Model::parse(&b).ok();
            for i in 0..count as usize {
                let Some(at) = be32(table as usize + i * 4) else {
                    continue;
                };
                let at = at as usize;
                let (Some(surfaces), Some(surface_table), Some(material)) =
                    (be16(at + 0x10), be32(at + 0x18), be32(at + 0x20))
                else {
                    continue;
                };
                chunks_total += 1;
                if surfaces <= 1 {
                    continue;
                }
                multi += 1;
                let named: Vec<u32> = (0..surfaces as usize)
                    .filter_map(|s| be32(surface_table as usize + s * 4))
                    .filter_map(|off| be32(off as usize))
                    .filter(|m| *m < materials)
                    .collect();
                if named.len() != surfaces as usize {
                    continue;
                }
                if named.iter().any(|m| *m != material) {
                    disagree += 1;
                }
                if named.windows(2).any(|w| w[0] != w[1]) {
                    mixed += 1;
                    let submeshes = parsed
                        .as_ref()
                        .and_then(|m| m.meshes.get(i))
                        .map_or(0, |m| m.submeshes.len());
                    if submeshes == surfaces as usize {
                        aligned += 1;
                    } else {
                        misaligned += 1;
                        if worst.len() < 10 {
                            worst.push((i, path.clone(), material, named.clone()));
                        }
                    }
                    let _ = submeshes;
                }
            }
        }
    }
    println!("{models} model(s), {chunks_total} chunk(s)");
    println!("  {multi} chunk(s) declare more than one surface");
    println!("  {disagree} have a surface naming a material other than the chunk's own +0x20");
    println!("  {mixed} have surfaces naming more than one material between them");
    println!("  of those, {aligned} have exactly one submesh per surface and {misaligned} do not");
    println!("  the first few that do not:");
    for (i, path, material, named) in &worst {
        println!("    {path} chunk {i}: +0x20 = {material}, surfaces {named:?}");
    }
    Ok(())
}
