//! Does a pad's vertex stream author a tangent frame, and does a frame built
//! from the texture coordinates agree with it?
//!
//! The pads' fragment programs build `N = nx*TC3 + ny*TC0 + nz*TC2` out of the
//! `_ne` normal map (`hd_pad_ne_tint_probe.rs`), so the vertex program hands
//! the pixel a tangent frame. `mesh.wesl` has none. Two ways to give it one:
//! decode the authored tangent (`rcsmodel`'s stride-22 `+10` field, a unit
//! vector perpendicular to the normal, `rcsmodel_vertex_ground_truth.rs`
//! claim 7) or derive it per pixel from the texture coordinates. This
//! measures, per pad chunk, which the chunk has and how far apart they sit:
//! the authored tangent's alignment with `dP/du` of the triangle it belongs
//! to, and the handedness (`cross(N, T) . dP/dv` sign), the two things a
//! wrong frame would flip without any pixel-count test noticing.
//!
//! `cargo run -p oag-render --example hd_pad_tangent_probe -- <archive> <vex>`

use oag_mesh::mesh;
use oag_rcs::rcsmodel;
use oag_vex::vex;

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt().max(1e-12);
    [a[0] / l, a[1] / l, a[2] / l]
}

fn main() -> anyhow::Result<()> {
    let spec = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC".into());
    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/environments/12_sol_2/track.vex".into());
    let data = mesh::read_blob(&spec, &name)?;
    let model_blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
    let model = rcsmodel::Model::parse(&model_blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    let classes = vex::classes_of(&data)?;
    let nodes = vex::nodes(&data)?;
    let order = vex::byte_order(&data);

    for (label, class_id) in [
        ("Speedup Pad", classes.speedup_pad),
        ("Weapon Pad", classes.weapon_pad),
    ] {
        let Some(class_id) = class_id else { continue };
        for node in nodes.iter().filter(|n| n.class_id == class_id).take(2) {
            let payload = &data[node.payload()];
            let hash = order.u32(payload, 0x30);
            let Some(chunk) = model.mesh(hash) else {
                continue;
            };
            for surface in chunk.surfaces() {
                let Some(stride) = surface.declared_stride() else {
                    println!("{label}: no declared stride");
                    continue;
                };
                let attrs: Vec<String> = surface
                    .decl
                    .iter()
                    .flat_map(|d| d.attributes.iter())
                    .map(|a| {
                        format!(
                            "{:#010x}:{}x{}@{}",
                            a.name_hash, a.components, a.rsx_type, a.offset
                        )
                    })
                    .collect();
                println!("{label} {hash:#010x} stride {stride} attrs {attrs:?}");
                for sub_mesh in &surface.submeshes {
                    let (Ok(points), Ok(indices), Ok(normals), Ok(uvs)) = (
                        surface.positions(&model_blob, sub_mesh, stride),
                        surface.indices(&model_blob, sub_mesh),
                        surface.normals(&model_blob, sub_mesh, stride),
                        surface.texcoords(&model_blob, sub_mesh, stride),
                    ) else {
                        continue;
                    };
                    if stride < 14 {
                        continue;
                    }
                    // The pad's vertex program reads this attribute (`0xdbe5f417`,
                    // 4 x ubyte-normalised) as `v[2].xyz * 2 - 1` into `o[TC3]`
                    // (the tangent) and uses `v[2].w` as a multiplier on
                    // `cross(N, T)` into `o[TC0]` (the bitangent): the tangent
                    // frame is authored, as four unsigned bytes.
                    let raw: Vec<[u8; 4]> = (0..sub_mesh.vertex_count)
                        .map(|k| {
                            let at = sub_mesh.vertex_offset + k * stride + 10;
                            [
                                model_blob[at],
                                model_blob[at + 1],
                                model_blob[at + 2],
                                model_blob[at + 3],
                            ]
                        })
                        .collect();
                    let tangents: Vec<[f32; 3]> = raw
                        .iter()
                        .map(|b| [0, 1, 2].map(|i| f32::from(b[i]) / 255.0 * 2.0 - 1.0))
                        .collect();
                    let w_values: std::collections::BTreeMap<u8, usize> =
                        raw.iter().fold(Default::default(), |mut m, b| {
                            *m.entry(b[3]).or_default() += 1;
                            m
                        });
                    println!("  w byte histogram: {w_values:?}");
                    let (mut unit, mut perp, mut along_du, mut anti_du, mut right, mut left) =
                        (0, 0, 0, 0, 0, 0);
                    let mut dots: Vec<f32> = Vec::new();
                    let zero_w = raw.iter().filter(|b| b[3] == 0).count();
                    for (t, n) in tangents.iter().zip(&normals) {
                        unit += usize::from((dot(*t, *t).sqrt() - 1.0).abs() < 0.05);
                        perp += usize::from(dot(*t, *n).abs() < 0.05);
                    }
                    for tri in indices.as_chunks::<3>().0 {
                        let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
                        let (e1, e2) = (sub(points[b], points[a]), sub(points[c], points[a]));
                        let (du1, du2) = (uvs[b][0] - uvs[a][0], uvs[c][0] - uvs[a][0]);
                        let (dv1, dv2) = (uvs[b][1] - uvs[a][1], uvs[c][1] - uvs[a][1]);
                        let det = du1 * dv2 - du2 * dv1;
                        if det.abs() < 1e-9 {
                            continue;
                        }
                        let r = 1.0 / det;
                        let dpdu = norm([
                            (e1[0] * dv2 - e2[0] * dv1) * r,
                            (e1[1] * dv2 - e2[1] * dv1) * r,
                            (e1[2] * dv2 - e2[2] * dv1) * r,
                        ]);
                        let dpdv = norm([
                            (e2[0] * du1 - e1[0] * du2) * r,
                            (e2[1] * du1 - e1[1] * du2) * r,
                            (e2[2] * du1 - e1[2] * du2) * r,
                        ]);
                        let t = tangents[a];
                        let d = dot(t, dpdu);
                        dots.push(d);
                        if d > 0.7 {
                            along_du += 1;
                        } else if d < -0.7 {
                            anti_du += 1;
                        }
                        let handed = dot(cross(normals[a], t), dpdv);
                        if handed > 0.5 {
                            right += 1;
                        } else if handed < -0.5 {
                            left += 1;
                        }
                    }
                    let mean = dots.iter().sum::<f32>() / dots.len().max(1) as f32;
                    let min = dots.iter().cloned().fold(1.0, f32::min);
                    println!(
                        "  authored w byte is 0 on {zero_w} of {} vertices; mean dot(T, dP/du) \
                         {mean:.4}, min {min:.4} over {} triangles",
                        sub_mesh.vertex_count,
                        dots.len()
                    );
                    println!(
                        "  {} verts: tangent unit {unit}, perpendicular to N {perp}; \
                         per triangle: T along +dP/du {along_du}, along -dP/du {anti_du}; \
                         cross(N,T).dP/dv > 0 {right}, < 0 {left}",
                        sub_mesh.vertex_count
                    );
                }
            }
        }
    }
    Ok(())
}
