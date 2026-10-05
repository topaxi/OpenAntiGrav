//! Scratch probe: which vertex attributes an HD circuit's chunks declare, and
//! what a four-byte normalised one actually holds - the question being whether
//! the disc bakes per-vertex light into a colour set the mesh reader drops.

use oag_mesh::mesh;
use oag_rcs::rcsmodel;

fn main() -> anyhow::Result<()> {
    let spec = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/environments/talons_junction/track.rcsmodel".into());
    let blob = mesh::read_blob(&spec, &name)?;
    let model = rcsmodel::Model::parse(&blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut census: std::collections::BTreeMap<(u32, u8, u8), usize> = Default::default();
    let mut chunks_with_colour = 0usize;
    let mut sampled = 0usize;
    for chunk in &model.meshes {
        let Some(decl) = chunk.decl.as_ref() else {
            continue;
        };
        let mut has_colour = false;
        for a in &decl.attributes {
            *census
                .entry((a.name_hash, a.components, a.rsx_type))
                .or_default() += 1;
            if matches!(a.name(), Some("colorSet1" | "VertexColour1")) || a.name_hash == 0x1aaf_7631
            {
                has_colour = true;
            }
        }
        if !has_colour {
            continue;
        }
        chunks_with_colour += 1;
        if sampled >= 10 {
            continue;
        }
        let Some(a) = decl.attributes.iter().find(|a| {
            matches!(a.name(), Some("colorSet1" | "VertexColour1")) || a.name_hash == 0x1aaf_7631
        }) else {
            continue;
        };
        let Some(sub) = chunk.submeshes.first() else {
            continue;
        };
        sampled += 1;
        let stride = decl.stride;
        let at = sub.vertex_offset + usize::from(a.offset);
        let mut values = Vec::new();
        for k in 0..sub.vertex_count.min(2000) {
            let o = at + k * stride;
            if o + 4 > blob.len() {
                break;
            }
            values.push([blob[o], blob[o + 1], blob[o + 2], blob[o + 3]]);
        }
        let n = values.len().max(1) as f32;
        let mean: Vec<f32> = (0..4)
            .map(|i| values.iter().map(|v| f32::from(v[i])).sum::<f32>() / n)
            .collect();
        let material = model
            .material_of(chunk)
            .map(|m| m.name.clone())
            .unwrap_or_default();
        println!(
            "chunk {:#010x} material {material:<40} attr {} type {}x{} at +{:#04x}: {} vertex/es, mean [{:.0} {:.0} {:.0} {:.0}], first {:?}",
            chunk.hash,
            a.name().unwrap_or("?"),
            a.components,
            a.rsx_type,
            a.offset,
            values.len(),
            mean[0],
            mean[1],
            mean[2],
            mean[3],
            &values[..values.len().min(4)],
        );
    }
    let mut both = 0usize;
    let mut only_lm = 0usize;
    let mut only_col = 0usize;
    let mut neither = 0usize;
    let mut tris_col = 0usize;
    let mut tris_lm = 0usize;
    let mut tris_neither = 0usize;
    for chunk in &model.meshes {
        let Some(decl) = chunk.decl.as_ref() else {
            continue;
        };
        let lm = decl
            .attributes
            .iter()
            .any(|a| a.name() == Some("lightmapUV"));
        let col = decl.attributes.iter().any(|a| {
            matches!(a.name(), Some("colorSet1" | "VertexColour1")) || a.name_hash == 0x1aaf_7631
        });
        let tris: usize = chunk.submeshes.iter().map(|s| s.index_count / 3).sum();
        match (lm, col) {
            (true, true) => {
                both += 1;
                tris_lm += tris;
            }
            (true, false) => {
                only_lm += 1;
                tris_lm += tris;
            }
            (false, true) => {
                only_col += 1;
                tris_col += tris;
            }
            (false, false) => {
                neither += 1;
                tris_neither += tris;
            }
        }
    }
    println!("\nlightmapUV only {only_lm}, colour only {only_col}, both {both}, neither {neither}");
    println!("triangles: lightmapped {tris_lm}, colour-only {tris_col}, neither {tris_neither}");
    println!(
        "\n{chunks_with_colour} of {} chunk(s) declare a colour set",
        model.meshes.len()
    );
    println!("\nattribute census (hash, components, rsx type) -> chunks:");
    for ((hash, components, ty), count) in &census {
        let name = rcsmodel::vertex_decl::Attribute {
            name_hash: *hash,
            components: *components,
            rsx_type: *ty,
            offset: 0,
        }
        .name()
        .unwrap_or("?");
        println!("  {hash:#010x} {name:<14} {components}x type {ty}  {count}");
    }
    Ok(())
}
