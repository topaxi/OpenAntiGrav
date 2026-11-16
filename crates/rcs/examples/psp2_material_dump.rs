//! Lists the materials a 2048-lineage `.rcsmodel` uses and the submeshes under a world point.
//!
//! ```sh
//! cargo run -p oag-rcs --example psp2_material_dump -- <entry> <x,z,radius> <archive.psarc>...
//! ```
//!
//! For each material: ordinal, path, technique, state word, textures, lightmap, uniform
//! parameters, how many submeshes and triangles use it. Then every submesh whose position
//! box covers the point (x, z) within `radius`, with its material, node and box.
//!
//! Environment switches: `DECL_SUB=<submesh>` prints that submesh's vertex declaration (attribute
//! names where they are known, type, offset) and the value range of each coordinate attribute
//! (PS4 model); `DUMP_VERTS=1` with `DUMP_MAT=<material ordinal>` prints the vertices of that
//! material near the point with the texture and lightmap coordinates the loader reads.

use std::collections::BTreeMap;
use std::path::Path;

use oag_rcs::rcsmodel::psp2;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [entry, at, archives @ ..] = args.as_slice() else {
        eprintln!("usage: psp2_material_dump <entry> <x,z,radius> <archive.psarc>...");
        std::process::exit(2);
    };
    let at: Vec<f32> = at.split(',').map(|v| v.parse().expect("number")).collect();
    let blob = archives
        .iter()
        .find_map(|a| {
            oag_assets::psarc::Archive::open_file(Path::new(a))
                .ok()?
                .read_path(entry)
                .ok()
        })
        .expect("entry in an archive");
    let model = psp2::parse(&blob).expect("parses");
    let mut use_of: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    for s in &model.submeshes {
        if let Some(m) = s.material {
            let e = use_of.entry(m).or_default();
            e.0 += 1;
            e.1 += s.triangle_count();
        }
    }
    for (m, (subs, tris)) in &use_of {
        let mat = &model.materials[*m];
        println!(
            "#{m} {} | tech {:?} | state {:?} | {subs} submesh(es) {tris} tri(s)",
            mat.name, mat.technique, mat.state
        );
        println!("    textures {:?}", mat.textures);
        println!("    lightmap {:?}", mat.lightmap);
        println!("    samplers {:08x?}", mat.samplers);
        if let Some(file) = archives.iter().find_map(|a| {
            oag_assets::psarc::Archive::open_file(Path::new(a))
                .ok()?
                .read_path(&mat.name)
                .ok()
        }) {
            let signed = oag_rcs::gcn::signed_samplers(&file);
            println!(
                "    diffuse {:?} | signed samplers {:?} unresolved {} | choice {:?}",
                mat.diffuse_texture(),
                signed.names,
                signed.unresolved,
                psp2::normal_map::choose(mat, &signed)
            );
        } else {
            println!("    material file not found");
        }
        for p in &mat.params {
            let v: Vec<f32> = p.bits.iter().map(|&b| f32::from_bits(b)).collect();
            println!("    param {:08x} {v:?}", p.hash);
        }
    }
    if let Ok(which) = std::env::var("DECL_SUB") {
        let which: usize = which.parse().expect("submesh index");
        let s = &model.submeshes[which];
        let cpu = model.sections[0];
        let gpu = model.sections[1];
        let cpu_bytes = &blob[cpu.at..cpu.at + cpu.len];
        let decls = psp2::vertex_decl::find_by_header(cpu_bytes);
        let at = s.record + psp2::vertex_decl::PS4_DECLARATION_POINTER;
        let target = u64::from_le_bytes(cpu_bytes[at..at + 8].try_into().unwrap()) as usize;
        let decl = decls.get(&target).expect("declaration");
        println!("sub {which} stride {} decl at {target:#x}", decl.stride);
        let names = [
            "position",
            "normal",
            "tangent",
            "Uv1",
            "uv1",
            "Uv2",
            "uvMarkings",
            "uv1_uvDirt",
            "uvDirt",
            "lightmapUV",
            "Uv1_Uv2",
            "VertexColour1",
            "NOVAColor",
            "uv2",
            "uvDiffuse",
        ];
        for a in &decl.attributes {
            let name = names
                .iter()
                .find(|n| oag_rcs::rcsmaterial::name_hash(n) == a.name_hash)
                .copied()
                .unwrap_or("?");
            println!(
                "  attr {:08x} {name} comps {} type {} offset {}",
                a.name_hash, a.components, a.gxm_type, a.offset
            );
        }
        let gpu_bytes = &blob[gpu.at..gpu.at + gpu.len];
        let first: Vec<u8> = s.positions[0]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let second: Vec<u8> = s.positions[1]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let start = (0..gpu_bytes.len() - decl.stride - 12)
            .find(|&i| {
                &gpu_bytes[i..i + 12] == first.as_slice()
                    && &gpu_bytes[i + decl.stride..i + decl.stride + 12] == second.as_slice()
            })
            .expect("vertex data");
        println!("  vertex data at gpu+{start:#x}");
        for a in decl
            .attributes
            .iter()
            .filter(|a| a.components >= 2 && a.gxm_type <= 1)
        {
            let name = names
                .iter()
                .find(|n| oag_rcs::rcsmaterial::name_hash(n) == a.name_hash)
                .copied()
                .unwrap_or("?");
            let mut lo = [f32::MAX; 4];
            let mut hi = [f32::MIN; 4];
            for v in 0..s.positions.len() {
                let o = start + v * decl.stride + usize::from(a.offset);
                for k in 0..usize::from(a.components).min(4) {
                    let f = if a.gxm_type == 1 {
                        oag_rcs::rcsmodel::unpack_half(u16::from_le_bytes([
                            gpu_bytes[o + 2 * k],
                            gpu_bytes[o + 2 * k + 1],
                        ]))
                    } else {
                        f32::from_le_bytes(gpu_bytes[o + 4 * k..o + 4 * k + 4].try_into().unwrap())
                    };
                    lo[k] = lo[k].min(f);
                    hi[k] = hi[k].max(f);
                }
            }
            println!("  {name} range {lo:.3?}..{hi:.3?}");
        }
    }
    println!(
        "--- submeshes covering ({}, {}) within {}",
        at[0], at[1], at[2]
    );
    for (i, s) in model.submeshes.iter().enumerate() {
        if s.positions.is_empty() {
            continue;
        }
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in &s.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        if at[0] + at[2] >= lo[0]
            && at[0] - at[2] <= hi[0]
            && at[1] + at[2] >= lo[2]
            && at[1] - at[2] <= hi[2]
        {
            let name = s.material.map_or("-", |m| {
                model.materials[m].name.rsplit('/').next().unwrap_or("")
            });
            let range = |uv: &[[f32; 2]]| {
                let mut lo = [f32::MAX; 2];
                let mut hi = [f32::MIN; 2];
                for t in uv {
                    for k in 0..2 {
                        lo[k] = lo[k].min(t[k]);
                        hi[k] = hi[k].max(t[k]);
                    }
                }
                if uv.is_empty() {
                    String::from("-")
                } else {
                    format!("{lo:.2?}..{hi:.2?}")
                }
            };
            if std::env::var_os("DUMP_VERTS").is_some()
                && s.material
                    == Some(
                        std::env::var("DUMP_MAT")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(980),
                    )
            {
                for (k, p) in s.positions.iter().enumerate() {
                    if (p[0] - at[0]).abs() < 40.0 && (p[2] - at[1]).abs() < 40.0 {
                        println!(
                            "    v{k} {:.1?} uv {:.3?} lm {:.3?}",
                            p,
                            s.texcoords.get(k),
                            s.lightmap_texcoords.get(k)
                        );
                    }
                }
            }
            println!(
                "    uv {} lmuv {} packed {}",
                range(&s.texcoords),
                range(&s.lightmap_texcoords),
                s.packed_texcoords.len()
            );
            println!(
                "sub {i} mat {:?} {name} node {:?} tris {} lm_uv {} tangents {} colours {} box {lo:.0?}..{hi:.0?}",
                s.material,
                s.node,
                s.triangle_count(),
                s.lightmap_texcoords.len(),
                s.tangents.len(),
                s.colours.len(),
            );
        }
    }
}
