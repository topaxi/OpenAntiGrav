//! Scratch probe: the full `0x80`-byte submesh descriptor of the chunks that
//! use one material, as bytes and as big-endian floats.
//!
//! Roughly `0x24` of those `0x80` bytes are read today. If a surface addresses
//! a region of its atlas that carries nothing, a per-submesh texture-coordinate
//! scale and bias - the exact shape the chunk header already uses for
//! positions at `+0x30`/`+0x40` - would be the thing that moves it, and it
//! would live in the unread tail.

use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let want: u32 = args.next().unwrap_or_else(|| "331".into()).parse()?;

    let data = mesh::read_blob(&spec, &name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("{name}: no sibling .rcsmodel"))?;
    let model = oag_formats::rcsmodel::Model::parse(&geometry)?;
    let word = |at: usize| {
        u32::from_be_bytes([
            geometry[at],
            geometry[at + 1],
            geometry[at + 2],
            geometry[at + 3],
        ]) as usize
    };
    let count = word(0x1c);
    let table = word(0x20);

    let mut shown = 0;
    for i in 0..count {
        let chunk = word(table + i * 4);
        if chunk + 0x80 > geometry.len() || word(chunk + 0x20) != want as usize {
            continue;
        }
        let hash = word(chunk);
        println!("chunk {hash:#010x} header at {chunk:#x}");
        for row in 0..6 {
            let base = chunk + row * 0x10;
            let bytes = &geometry[base..base + 0x10];
            let words: Vec<String> = bytes
                .chunks_exact(4)
                .map(|w| {
                    let f = f32::from_be_bytes([w[0], w[1], w[2], w[3]]);
                    if f.is_finite() && f != 0.0 && f.abs() > 1e-6 && f.abs() < 1e6 {
                        format!("{f:>12.4}")
                    } else {
                        "            ".into()
                    }
                })
                .collect();
            let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
            println!(
                "  +{:02x}  {}  {}",
                row * 0x10,
                hex.join(" "),
                words.join(" ")
            );
        }
        if let Some(mesh) = model.meshes.iter().find(|m| m.hash == hash as u32) {
            println!(
                "  declaration: stride {}",
                mesh.declared_stride().unwrap_or(0)
            );
            for a in mesh.decl.iter().flat_map(|d| d.attributes.iter()) {
                println!(
                    "    {:#010x} {:<12} {} component(s) rsx type {} at +{:#04x}",
                    a.name_hash,
                    a.name().unwrap_or("-"),
                    a.components,
                    a.rsx_type,
                    a.offset
                );
            }
        }
        let submeshes = word(chunk + 0x50).min(8);
        for n in 0..submeshes {
            let at = chunk + 0x60 + n * 0x80;
            if at + 0x80 > geometry.len() {
                continue;
            }
            println!("chunk {hash:#010x} submesh {n} descriptor at {at:#x}");
            for row in 0..8 {
                let base = at + row * 0x10;
                let bytes = &geometry[base..base + 0x10];
                let words: Vec<String> = bytes
                    .chunks_exact(4)
                    .map(|w| {
                        let f = f32::from_be_bytes([w[0], w[1], w[2], w[3]]);
                        if f.is_finite() && f != 0.0 && f.abs() > 1e-6 && f.abs() < 1e6 {
                            format!("{f:>12.4}")
                        } else {
                            "            ".into()
                        }
                    })
                    .collect();
                let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
                println!(
                    "  +{:02x}  {}  {}",
                    row * 0x10,
                    hex.join(" "),
                    words.join(" ")
                );
            }
            shown += 1;
            if shown >= 3 {
                return Ok(());
            }
        }
    }
    Ok(())
}
