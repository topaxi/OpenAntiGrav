//! Scratch probe: the raw bytes of a chunk's surface records, beside the
//! submesh descriptors this parser already reads, so the two can be
//! correlated. See `docs/formats/rcsmodel.md`, "A chunk names ONE material
//! here and the engine reads SEVERAL".

use oag_formats::rcsmodel;
use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let path = args
        .next()
        .unwrap_or_else(|| "/data/environments/amphiseum/track.rcsmodel".into());
    let want: Vec<usize> = args.filter_map(|s| s.parse().ok()).collect();
    let b = mesh::read_blob(&spec, &path)?;
    let model = rcsmodel::Model::parse(&b).map_err(|e| anyhow::anyhow!("{e}"))?;
    let be32 = |at: usize| u32::from_be_bytes(b[at..at + 4].try_into().unwrap());
    let be16 = |at: usize| u16::from_be_bytes(b[at..at + 2].try_into().unwrap());
    let count = be32(0x1c) as usize;
    let table = be32(0x20) as usize;
    println!("{path}: {count} chunk(s), {} material(s)", be32(0x2c));

    for index in want {
        let at = be32(table + index * 4) as usize;
        let surfaces = be16(at + 0x10) as usize;
        let surface_table = be32(at + 0x18) as usize;
        let chunk = &model.meshes[index];
        println!(
            "\nchunk {index} at {at:#x}: hash {:#010x}, +0x20 material {}, {surfaces} surface(s) via {surface_table:#x}, {} submesh(es)",
            chunk.hash,
            chunk.material,
            chunk.submeshes.len(),
        );
        for (n, s) in chunk.submeshes.iter().enumerate() {
            println!(
                "  submesh {n}: {:6} vert at {:#010x}, {:6} index at {:#010x}",
                s.vertex_count, s.vertex_offset, s.index_count, s.index_offset,
            );
        }
        let offsets: Vec<usize> = (0..surfaces)
            .map(|s| be32(surface_table + s * 4) as usize)
            .collect();
        let stride = offsets
            .windows(2)
            .map(|w| w[1].wrapping_sub(w[0]))
            .collect::<Vec<_>>();
        println!("  surface offsets {offsets:#x?}");
        println!("  gaps {stride:?}");
        for (n, &off) in offsets.iter().enumerate() {
            let len = stride.first().copied().unwrap_or(0x20).clamp(0x10, 0x40);
            print!("  surface {n} at {off:#x}:");
            for w in 0..len / 4 {
                if w % 8 == 0 {
                    print!("\n    +{:#04x} ", w * 4);
                }
                print!(" {:08x}", be32(off + w * 4));
            }
            println!();
        }
    }
    Ok(())
}
