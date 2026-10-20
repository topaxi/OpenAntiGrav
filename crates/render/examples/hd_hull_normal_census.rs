//! Scratch census: which ship hull chunks resolve to a program that reads a
//! texture unit as a tangent-space normal, and what tangent-ish attributes
//! those chunks author (the `tangent` hash and the second `0x868f8229`).
//!
//! ```sh
//! cargo run -p oag-render --example hd_hull_normal_census -- data/images/hdfury-ps3-eu-dec.iso
//! ```
use oag_mesh::mesh;
use oag_rcs::{
    rcsmaterial::{self, Class, Declared, LIT_RACE_PASS, fragment::Program},
    rcsmodel,
};
use std::collections::BTreeMap;

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];
const TANGENT: u32 = 0xdbe5_f417;
const SECOND: u32 = 0x868f_8229;

#[derive(Default)]
struct Row {
    chunks: usize,
    verts: usize,
    with_tangent: usize,
    with_second: usize,
    perp_sum: f64,
    perp_n: usize,
    len_min: f64,
    len_max: f64,
    len_sum: f64,
    w: BTreeMap<u8, usize>,
    second_shape: BTreeMap<String, usize>,
    ships: std::collections::BTreeSet<String>,
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let mut vexes = std::collections::BTreeSet::new();
    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        for p in psarc.paths() {
            if p.starts_with("/data/ships/")
                && p.ends_with("/ship.vex")
                && p.matches('/').count() == 4
            {
                vexes.insert(p.clone());
            }
        }
    }
    let find = |path: &str| -> Option<(String, Vec<u8>)> {
        ARCHIVES.iter().find_map(|a| {
            let spec = format!("{image}:{a}");
            mesh::read_blob(&spec, path).ok().map(|d| (spec, d))
        })
    };
    // key: (material leaf, normal-unit-or-none)
    let mut rows: BTreeMap<(String, String), Row> = BTreeMap::new();
    for vex in &vexes {
        let Some((spec, data)) = find(vex) else {
            continue;
        };
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, vex, &data) else {
            continue;
        };
        let Ok(model) = rcsmodel::Model::parse(&geometry) else {
            continue;
        };
        let mut blobs: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
        let ship = vex
            .trim_start_matches("/data/ships/")
            .trim_end_matches("/ship.vex")
            .to_string();
        for m in &model.meshes {
            let Some(material) = model.materials.get(m.material as usize) else {
                continue;
            };
            let leaf = material.name.rsplit('/').next().unwrap_or("").to_string();
            let blob = blobs
                .entry(material.name.clone())
                .or_insert_with(|| find(&format!("/{}", material.name)).map(|x| x.1))
                .clone();
            let mut normal_unit = "unresolved".to_string();
            if let Some(blob) = blob
                && let Ok(parsed) = rcsmaterial::RcsMaterial::parse(&blob)
            {
                let word = rcsmaterial::Features::chunk_word(LIT_RACE_PASS, m.decl.as_ref());
                let key = rcsmaterial::Features::from_pass_word(word);
                if let Some(variant) = Class::ALL.into_iter().find_map(|c| parsed.variant(c, key)) {
                    normal_unit = "none".into();
                    if let (Some(d), Some(p)) = (
                        Declared::parse(&blob, variant.fragment.offset),
                        Program::parse(&blob, variant.fragment.offset),
                    ) {
                        let mut us: Vec<String> = d
                            .samplers
                            .iter()
                            .filter(|&&(_, u)| p.samples_as_normal(u as u8))
                            .map(|&(_, u)| format!("unit{u}"))
                            .collect();
                        us.sort();
                        if !us.is_empty() {
                            normal_unit = us.join("+");
                        }
                    }
                }
            }
            let row = rows.entry((leaf, normal_unit)).or_default();
            row.chunks += 1;
            row.ships.insert(ship.clone());
            let Some(decl) = m.decl.as_ref() else {
                continue;
            };
            let tangent = decl.attributes.iter().find(|a| a.name_hash == TANGENT);
            let second = decl.attributes.iter().find(|a| a.name_hash == SECOND);
            if let Some(a) = second {
                row.with_second += 1;
                *row.second_shape
                    .entry(format!(
                        "{}x type{} @{}",
                        a.components, a.rsx_type, a.offset
                    ))
                    .or_default() += 1;
            }
            if let Some(t) = tangent {
                row.with_tangent += 1;
                for sm in &m.submeshes {
                    row.verts += sm.vertex_count;
                    let Ok(normals) = m.normals(&geometry, sm, decl.stride) else {
                        continue;
                    };
                    for (k, n) in normals.iter().enumerate() {
                        let at = sm.vertex_offset + k * decl.stride + usize::from(t.offset);
                        let b = &geometry[at..at + 4];
                        *row.w.entry(b[3]).or_default() += 1;
                        let tv = [0, 1, 2].map(|i| f64::from(b[i]) / 255.0 * 2.0 - 1.0);
                        let len = (tv[0] * tv[0] + tv[1] * tv[1] + tv[2] * tv[2]).sqrt();
                        let d = tv[0] * f64::from(n[0])
                            + tv[1] * f64::from(n[1])
                            + tv[2] * f64::from(n[2]);
                        row.len_min = if row.perp_n == 0 {
                            len
                        } else {
                            row.len_min.min(len)
                        };
                        row.len_max = row.len_max.max(len);
                        row.len_sum += len;
                        if len > 0.0 {
                            row.perp_sum += (d / len).abs();
                            row.perp_n += 1;
                        }
                    }
                }
            }
        }
    }
    for ((leaf, nu), r) in &rows {
        let mut w: Vec<_> = r.w.iter().collect();
        w.sort_by(|a, b| b.1.cmp(a.1));
        println!(
            "{leaf:60} normal={nu:12} chunks={:4} ships={:2} tangent={:4} second={:4} verts={:7} |t|min/mean/max={:.3}/{:.3}/{:.3} |t.n|={:.3} w_top={:?} second_shape={:?}",
            r.chunks,
            r.ships.len(),
            r.with_tangent,
            r.with_second,
            r.verts,
            r.len_min,
            r.len_sum / r.perp_n.max(1) as f64,
            r.len_max,
            if r.perp_n > 0 {
                r.perp_sum / r.perp_n as f64
            } else {
                f64::NAN
            },
            &w[..w.len().min(4)],
            r.second_shape
        );
    }
    Ok(())
}
