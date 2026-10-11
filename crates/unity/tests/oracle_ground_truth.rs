//! Mesh and texture reads against a second implementation (UnityPy).
//!
//! `OAG_UNITY_ORACLE=<table>` names a table written by
//! `scripts/unity-oracle.py` over the whole Workshop census. Without it the
//! test prints that and passes, so a plain `just test-data` stays hermetic; the
//! committed facts in `known_items.rs` cover the two named items regardless.

mod common;

use std::collections::HashMap;

use oag_unity::assets::Assets;
use oag_unity::bundle::Bundle;
use oag_unity::classes::{Mesh, Texture2d};
use oag_unity::serialized::class_id;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-3 * (1.0 + a.abs().max(b.abs()))
}

#[test]
#[ignore = "needs data/unity and OAG_UNITY_ORACLE"]
fn meshes_and_textures_match_unitypy() {
    let Some(root) = common::workshop() else {
        return;
    };
    let Some(table) = std::env::var_os("OAG_UNITY_ORACLE") else {
        println!("skipping: OAG_UNITY_ORACLE not set");
        return;
    };
    let table = std::fs::read_to_string(table).expect("oracle table");
    let mut rows: HashMap<(String, String, i64), Vec<String>> = HashMap::new();
    for l in table.lines() {
        let c: Vec<String> = l.split('\t').map(str::to_string).collect();
        rows.insert((c[1].clone(), c[2].clone(), c[3].parse().unwrap()), c);
    }
    let (mut meshes, mut textures, mut oracle_failed, mut diffs) = (0, 0, 0, Vec::new());
    for item in common::items(&root) {
        let Some(bytes) = common::bundle_bytes(&item.path) else {
            continue;
        };
        let b = Bundle::parse(&bytes).unwrap();
        let assets = Assets::open(&b).unwrap();
        for (fi, sf) in assets.files().iter().enumerate() {
            let major: u32 = sf.unity_version.split('.').next().unwrap().parse().unwrap();
            for o in &sf.objects {
                let Some(row) =
                    rows.get(&(item.id.clone(), assets.names()[fi].to_string(), o.path_id))
                else {
                    continue;
                };
                let tag = format!("{} {} {}", item.id, assets.names()[fi], o.path_id);
                if row[0] == "E" {
                    // UnityPy's exporter refused this object; ours must still be
                    // structurally sound (exact buffer length is checked by the
                    // reader itself), so say what it is.
                    if o.class_id == class_id::MESH {
                        let m = Mesh::from_value(&sf.read_object(o).unwrap(), major).unwrap();
                        let finite = m
                            .positions()
                            .unwrap()
                            .is_none_or(|p| p.iter().flatten().all(|c| c.is_finite()));
                        let tris: usize = (0..m.submeshes.len())
                            .map(|s| m.triangles(s).unwrap().len())
                            .sum();
                        println!(
                            "oracle refused mesh {tag}: {:?} vertices={} triangles={tris} finite={finite}",
                            m.name, m.vertex_count
                        );
                        assert!(
                            finite && m.vertex_count == 0,
                            "UnityPy refused a non-empty mesh"
                        );
                    }
                    oracle_failed += 1;
                    continue;
                }
                match o.class_id {
                    class_id::MESH => {
                        let m = Mesh::from_value(&sf.read_object(o).unwrap(), major).unwrap();
                        let tris: usize = (0..m.submeshes.len())
                            .map(|s| m.triangles(s).unwrap().len())
                            .sum();
                        let sum = |v: Option<Vec<Vec<f32>>>, n: usize| -> Vec<f64> {
                            let mut s = vec![0.0f64; n];
                            for r in v.unwrap_or_default() {
                                for i in 0..n {
                                    s[i] += f64::from(r[i].abs());
                                }
                            }
                            s
                        };
                        let pos = sum(
                            m.positions()
                                .unwrap()
                                .map(|v| v.iter().map(|p| p.to_vec()).collect()),
                            3,
                        );
                        let nrm = sum(
                            m.normals()
                                .unwrap()
                                .map(|v| v.iter().map(|p| p.to_vec()).collect()),
                            3,
                        );
                        let uv = sum(
                            m.uvs(0)
                                .unwrap()
                                .map(|v| v.iter().map(|p| p.to_vec()).collect()),
                            2,
                        );
                        let want = |i: usize| row[i].parse::<f64>().unwrap();
                        let mut ok = m.vertex_count == want(4) as usize && tris == want(5) as usize;
                        for i in 0..3 {
                            ok &= close(pos[i], want(8 + i)) && close(nrm[i], want(11 + i));
                        }
                        for (i, u) in uv.iter().enumerate() {
                            ok &= close(*u, want(14 + i));
                        }
                        if !ok {
                            diffs.push(format!("mesh {tag}: ours v={} f={tris} pos={pos:?} nrm={nrm:?} uv={uv:?}; oracle {:?}", m.vertex_count, &row[4..]));
                        }
                        meshes += 1;
                    }
                    class_id::TEXTURE_2D => {
                        let t: Texture2d = assets.texture(&sf.read_object(o).unwrap()).unwrap();
                        let got = vec![
                            t.width.to_string(),
                            t.height.to_string(),
                            format!("{}", format_id(&t)),
                            t.mip_count.to_string(),
                            t.data.len().to_string(),
                        ];
                        if got != row[4..9] {
                            diffs.push(format!(
                                "texture {tag}: ours {got:?}, oracle {:?}",
                                &row[4..9]
                            ));
                        }
                        textures += 1;
                    }
                    _ => {}
                }
            }
        }
    }
    println!(
        "meshes={meshes} textures={textures} oracle_failed={oracle_failed} diffs={}",
        diffs.len()
    );
    for d in diffs.iter().take(20) {
        println!("{d}");
    }
    assert!(diffs.is_empty());
    assert!(meshes > 1000 && textures > 2000);
}

fn format_id(t: &Texture2d) -> i32 {
    use oag_unity::classes::TextureFormat as F;
    match t.format {
        F::Alpha8 => 1,
        F::Argb4444 => 2,
        F::Rgb24 => 3,
        F::Rgba32 => 4,
        F::Argb32 => 5,
        F::Rgb565 => 7,
        F::R16 => 9,
        F::Dxt1 => 10,
        F::Dxt3 => 11,
        F::Dxt5 => 12,
        F::Rgba4444 => 13,
        F::Bgra32 => 14,
        F::Other(i) => i,
    }
}
