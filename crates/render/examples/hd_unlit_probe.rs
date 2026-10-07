//! Scratch probe behind `docs/rendering/hd-unlit-programs.md`: for every
//! chunk of one PS3 `.vex`, the raw material state (blend enable, the raw
//! `src`/`dst` factor words, cull bit), the vertex declaration and the span of
//! the diffuse `Uv1` the chunk actually carries.
//!
//! ```sh
//! cargo run -p oag-render --example hd_unlit_probe -- \
//!     'data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC' \
//!     /data/weapons/hd_leachbeam_ball_bloomring.vex
//! ```
//!
//! The span is the question `oag-view --draws` raised: its `uv 0.00x0.00`
//! column on the LeachBall and Plasma spheres could be a decode that lost the
//! attribute or a mesh that really authors one coordinate for every vertex.
//! This prints the declaration beside it so the two are told apart.

use oag_mesh::mesh;
use oag_rcs::rcsmodel;

fn main() -> anyhow::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--program") {
        let args: Vec<String> = std::env::args().skip(2).collect();
        let blob = mesh::read_blob(&args[0], &args[1])?;
        let at = usize::from_str_radix(args[2].trim_start_matches("0x"), 16)?;
        let program = oag_rcs::rcsmaterial::fragment::Program::parse(&blob, at)
            .ok_or_else(|| anyhow::anyhow!("no program at {at:#x}"))?;
        let declared = oag_rcs::rcsmaterial::Declared::parse(&blob, at)
            .ok_or_else(|| anyhow::anyhow!("no SHO block at {at:#x}"))?;
        let names: Vec<&str> = program
            .instructions
            .iter()
            .map(|i| i.name().unwrap_or("?"))
            .collect();
        println!("mnemonics: {names:?}");
        let patched: Vec<u16> = declared
            .parameters
            .iter()
            .flat_map(|&h| program.patches(h).collect::<Vec<_>>())
            .collect();
        let literals: Vec<[f32; 4]> = program
            .instructions
            .iter()
            .filter(|i| i.const_slot.is_some_and(|s| !patched.contains(&s)))
            .filter_map(|i| i.constant)
            .collect();
        println!("literals: {literals:?}");
        println!("parameters: {:08x?}", declared.parameters);
        println!("samplers: {:08x?}", declared.samplers);
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("--built") {
        // The same winding question asked of what `mesh::rcs` hands the GPU,
        // after every node transform: a reflection anywhere in the chain
        // shows up here as triangles wound clockwise about their normal.
        let args: Vec<String> = std::env::args().skip(2).collect();
        let data = mesh::read_blob(&args[0], &args[1])?;
        let (model, _) = mesh::rcs::scene_from(&args[0], &args[1], &data)?
            .ok_or_else(|| anyhow::anyhow!("not a PS3 model"))?;
        for draw in model.transparent_draws.iter().chain(&model.draws) {
            let (mut ccw, mut tris, mut wound_out, mut normal_out) = (0, 0, 0usize, 0usize);
            for t in model.indices[draw.range.start as usize..draw.range.end as usize]
                .as_chunks::<3>()
                .0
            {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| model.vertices[i as usize]);
                let e1 = [
                    b.position[0] - a.position[0],
                    b.position[1] - a.position[1],
                    b.position[2] - a.position[2],
                ];
                let e2 = [
                    c.position[0] - a.position[0],
                    c.position[1] - a.position[1],
                    c.position[2] - a.position[2],
                ];
                let g = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];
                tris += 1;
                if g[0] * a.normal[0] + g[1] * a.normal[1] + g[2] * a.normal[2] > 0.0 {
                    ccw += 1;
                }
                // Against the draw's own centroid: which way the winding and
                // the vertex normal each face on a closed shell.
                let range = &model.indices[draw.range.start as usize..draw.range.end as usize];
                let n = range.len() as f32;
                let mut centre = [0.0f32; 3];
                for &i in range {
                    for (c, p) in centre.iter_mut().zip(model.vertices[i as usize].position) {
                        *c += p / n;
                    }
                }
                let out: f32 = (0..3).map(|k| (a.position[k] - centre[k]) * g[k]).sum();
                let nout: f32 = (0..3)
                    .map(|k| (a.position[k] - centre[k]) * a.normal[k])
                    .sum();
                wound_out += usize::from(out > 0.0);
                normal_out += usize::from(nout > 0.0);
            }
            println!(
                "draw node {:?} culled {}: {ccw} of {tris} counter-clockwise about the normal, \
                 {wound_out} wound outward, {normal_out} with an outward vertex normal",
                draw.node, draw.culled
            );
        }
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("--undeclared") {
        undeclared("data/images/hdfury-ps3-eu-dec.iso");
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("--slot-order") {
        slot_order("data/images/hdfury-ps3-eu-dec.iso");
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("--layouts") {
        for (k, n) in declared_layouts("data/images/hdfury-ps3-eu-dec.iso") {
            println!("{n:>7}  {k}");
        }
        return Ok(());
    }
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/weapons/hd_leachbeam_ball_bloomring.vex".into());
    let data = mesh::read_blob(&spec, &name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("{name}: no sibling .rcsmodel"))?;
    let model = rcsmodel::Model::parse(&geometry).map_err(|e| anyhow::anyhow!("{e}"))?;

    for (i, material) in model.materials.iter().enumerate() {
        println!(
            "material [{i}] {}: state {:#010x} (blend {} alpha-test {} cull {}) src {:#06x} dst {:#06x} -> {:?}",
            material.name,
            material.state,
            material.state & 1,
            material.state >> 1 & 1,
            material.state >> 4 & 1,
            material.src_factor,
            material.dst_factor,
            material.blend(),
        );
    }
    for mesh in &model.meshes {
        println!(
            "mesh {:#010x}: material {} space {:?} render_flags {:#06x}",
            mesh.hash, mesh.material, mesh.space, mesh.render_flags
        );
        if let Some(decl) = &mesh.decl {
            for a in &decl.attributes {
                println!(
                    "  attribute {:#010x} components {} rsx_type {} offset {} (stride {})",
                    a.name_hash, a.components, a.rsx_type, a.offset, decl.stride
                );
            }
        }
        let stride = mesh
            .declared_stride()
            .or_else(|| mesh.solve_stride_by_layout())
            .or_else(|| mesh.solve_stride_by_normals(&geometry))
            .unwrap_or(0);
        // Winding against the authored normal: how many triangles turn
        // counter-clockwise about the way their own vertex normals point,
        // and how many of those normals point away from the mesh's centre.
        for sub in &mesh.submeshes {
            let (Ok(p), Ok(n), Ok(idx)) = (
                mesh.positions(&geometry, sub, stride),
                mesh.normals(&geometry, sub, stride),
                mesh.indices(&geometry, sub),
            ) else {
                continue;
            };
            let centre = p
                .iter()
                .fold([0.0f32; 3], |a, v| [a[0] + v[0], a[1] + v[1], a[2] + v[2]]);
            let centre = centre.map(|c| c / p.len().max(1) as f32);
            let (mut ccw, mut outward, mut tris) = (0, 0, 0);
            for t in idx.as_chunks::<3>().0 {
                let [a, b, c] = [t[0], t[1], t[2]].map(usize::from);
                let (e1, e2) = (
                    [p[b][0] - p[a][0], p[b][1] - p[a][1], p[b][2] - p[a][2]],
                    [p[c][0] - p[a][0], p[c][1] - p[a][1], p[c][2] - p[a][2]],
                );
                let g = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];
                let nn = n[a];
                tris += 1;
                if g[0] * nn[0] + g[1] * nn[1] + g[2] * nn[2] > 0.0 {
                    ccw += 1;
                }
                let r = [
                    p[a][0] - centre[0],
                    p[a][1] - centre[1],
                    p[a][2] - centre[2],
                ];
                if r[0] * nn[0] + r[1] * nn[1] + r[2] * nn[2] > 0.0 {
                    outward += 1;
                }
            }
            println!(
                "  winding: {ccw} of {tris} triangle(s) counter-clockwise about their normal, {outward} normal(s) outward"
            );
        }
        println!(
            "  stride {stride} (declared {:?}, layout {:?}, normals {:?})",
            mesh.declared_stride(),
            mesh.solve_stride_by_layout(),
            mesh.solve_stride_by_normals(&geometry)
        );
        for sub in &mesh.submeshes {
            for v in 0..sub.vertex_count.min(6) {
                let at = sub.vertex_offset + v * stride;
                let bytes = geometry.get(at..at + stride).unwrap_or(&[]);
                println!("  raw vertex {v}: {bytes:02x?} format {:02x?}", sub.format);
            }
            match mesh.texcoords(&geometry, sub, stride) {
                Ok(uv) => {
                    let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
                    for c in &uv {
                        for k in 0..2 {
                            lo[k] = lo[k].min(c[k]);
                            hi[k] = hi[k].max(c[k]);
                        }
                    }
                    println!(
                        "  submesh: {} vertices, uv u {:.4}..{:.4} v {:.4}..{:.4}, first {:?}",
                        uv.len(),
                        lo[0],
                        hi[0],
                        lo[1],
                        hi[1],
                        &uv[..uv.len().min(4)]
                    );
                }
                Err(e) => println!("  submesh: no texcoord ({e})"),
            }
        }
    }
    Ok(())
}

/// Every declared vertex layout on the disc, as `stride: attribute@offset`
/// sorted by offset, with how many chunks carry it - the evidence for where
/// `Uv1` sits in a chunk that declares nothing.
fn declared_layouts(image: &str) -> std::collections::BTreeMap<String, usize> {
    const ARCHIVES: &[&str] = &[
        "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
    ];
    let mut out = std::collections::BTreeMap::new();
    for archive in ARCHIVES {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
        let Ok(mut handle) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = handle
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(data) = handle.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&data) else {
                continue;
            };
            for mesh in &model.meshes {
                let key = match &mesh.decl {
                    Some(decl) => {
                        let mut attrs: Vec<_> = decl
                            .attributes
                            .iter()
                            .map(|a| (a.offset, a.name_hash, a.components, a.rsx_type))
                            .collect();
                        attrs.sort_unstable();
                        let attrs: Vec<String> = attrs
                            .iter()
                            .map(|(o, h, c, t)| format!("{h:08x}@{o}x{c}t{t}"))
                            .collect();
                        format!("declared stride {}: {}", decl.stride, attrs.join(" "))
                    }
                    None => "no declaration".to_string(),
                };
                *out.entry(key).or_default() += 1;
            }
        }
    }
    out
}

/// Every undeclared chunk on the disc, by solved stride: how many read a
/// non-finite texture coordinate out of their last four bytes, and how many
/// of those read a finite one at `+0x0a` instead - the population
/// `oag_rcs::rcsmodel::Mesh::texcoords`' inline fallback answers for.
fn undeclared(image: &str) {
    const ARCHIVES: &[&str] = &[
        "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
    ];
    let half = |data: &[u8], at: usize| {
        oag_rcs::rcsmodel::unpack_half(u16::from_be_bytes([data[at], data[at + 1]]))
    };
    // (stride, tail non-finite, +10 finite) -> chunks
    let mut counts: std::collections::BTreeMap<(usize, bool, bool), usize> = Default::default();
    let mut shown = 0;
    for archive in ARCHIVES {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
        let Ok(mut handle) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = handle
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(data) = handle.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&data) else {
                continue;
            };
            for mesh in model.meshes.iter().filter(|m| m.decl.is_none()) {
                let Some(stride) = mesh.solve_stride_without_a_box(&data) else {
                    *counts.entry((0, false, false)).or_default() += 1;
                    continue;
                };
                let (mut tail_bad, mut ten_ok) = (false, stride >= 14);
                for sub in &mesh.submeshes {
                    for k in 0..sub.vertex_count {
                        let at = sub.vertex_offset + k * stride;
                        if at + stride > data.len() {
                            break;
                        }
                        let tail = at + stride - 4;
                        if !(half(&data, tail).is_finite() && half(&data, tail + 2).is_finite()) {
                            tail_bad = true;
                        }
                        if !(half(&data, at + 10).is_finite() && half(&data, at + 12).is_finite()) {
                            ten_ok = false;
                        }
                    }
                }
                if tail_bad && ten_ok && (stride == 18 || stride == 22) && shown < 200 {
                    shown += 1;
                    println!(
                        "tail non-finite: {archive} {path} mesh {:#010x} stride {stride}",
                        mesh.hash
                    );
                }
                *counts.entry((stride, tail_bad, ten_ok)).or_default() += 1;
            }
        }
    }
    for ((stride, tail_bad, ten_ok), n) in counts {
        println!(
            "stride {stride:>2} tail non-finite {tail_bad:<5} +10 finite {ten_ok:<5} {n} chunk(s)"
        );
    }
}

/// A vertex `SHO` block's attribute table: `(hash, slot)`, in table order.
fn vertex_attributes(blob: &[u8], at: usize) -> Vec<(u32, u32)> {
    let u16_at = |o: usize| -> usize {
        blob.get(at + o..at + o + 2)
            .map_or(0, |b| usize::from(u16::from_be_bytes([b[0], b[1]])))
    };
    let u32_at = |o: usize| -> u32 {
        blob.get(o..o + 4)
            .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    if blob.get(at..at + 4) != Some(b"SHO\x08".as_slice()) {
        return Vec::new();
    }
    let (count, table) = (u16_at(0x0a), u16_at(0x10));
    (0..count)
        .map(|i| {
            let r = at + table + 8 * i;
            (u32_at(r), u32_at(r + 4))
        })
        .collect()
}

/// Whether every declared chunk's attribute **offsets** run in the order of
/// its own material's vertex-program attribute **slots** - the rule an
/// undeclared chunk's layout would follow if the engine packs by slot.
fn slot_order(image: &str) {
    use oag_rcs::rcsmaterial::{self, Class, Features, LIT_RACE_PASS};
    const ARCHIVES: &[&str] = &[
        "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
    ];
    let mut handles: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| {
            oag_assets::psarc::Archive::open(&format!("{image}:PS3_GAME/USRDIR/{a}.PSARC")).ok()
        })
        .collect();
    let mut materials: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
    let (mut agree, mut disagree, mut unresolved) = (0usize, 0usize, 0usize);
    let mut shown = 0;
    for h in 0..handles.len() {
        let paths: Vec<String> = handles[h]
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(data) = handles[h].read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&data) else {
                continue;
            };
            for mesh in &model.meshes {
                let Some(decl) = &mesh.decl else { continue };
                let Some(material) = model.material_of(mesh) else {
                    unresolved += 1;
                    continue;
                };
                let name = format!("/{}", material.name);
                let blob = materials
                    .entry(name.clone())
                    .or_insert_with(|| handles.iter_mut().find_map(|a| a.read_path(&name).ok()))
                    .clone();
                let Some(blob) = blob else {
                    unresolved += 1;
                    continue;
                };
                let Ok(parsed) = rcsmaterial::RcsMaterial::parse(&blob) else {
                    unresolved += 1;
                    continue;
                };
                let key = Features::from_pass_word(Features::chunk_word(LIT_RACE_PASS, Some(decl)));
                let Some(variant) = Class::ALL.into_iter().find_map(|c| parsed.variant(c, key))
                else {
                    unresolved += 1;
                    continue;
                };
                let slots = vertex_attributes(&blob, variant.vertex.offset);
                let mut by_offset: Vec<(u8, u32)> = decl
                    .attributes
                    .iter()
                    .map(|a| (a.offset, a.name_hash))
                    .collect();
                by_offset.sort_unstable();
                let with_slot: Vec<u32> = by_offset
                    .iter()
                    .filter_map(|(_, hash)| slots.iter().find(|(h, _)| h == hash).map(|&(_, s)| s))
                    .collect();
                if with_slot.windows(2).all(|w| w[0] < w[1]) {
                    agree += 1;
                } else {
                    disagree += 1;
                    if shown < 12 {
                        shown += 1;
                        println!(
                            "disagree: {path} {} offsets {:?} slots {:?}",
                            material.name, by_offset, slots
                        );
                    }
                }
            }
        }
    }
    println!("{agree} declared chunk(s) agree, {disagree} disagree, {unresolved} unresolved");
}
