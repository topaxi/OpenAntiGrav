//! Writes one mesh as OBJ plus a wireframe PNG, and one texture as PNG, into
//! the directory `OAG_UNITY_EXPORT` names, so a person can look at them. Passes
//! without writing anything when the variable is unset.

use crate::common;

use oag_unity::assets::Assets;
use oag_unity::bundle::Bundle;
use oag_unity::classes::{Mesh, Texture2d, TextureFormat};
use oag_unity::serialized::class_id;

fn decode_dxt(t: &Texture2d) -> Vec<u8> {
    let (w, h) = (t.width as usize, t.height as usize);
    let mut out = vec![0u8; w * h * 4];
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    let size = if t.format == TextureFormat::Dxt1 {
        8
    } else {
        16
    };
    for by in 0..bh {
        for bx in 0..bw {
            let at = (by * bw + bx) * size;
            let texels = if size == 8 {
                oag_texture::bcn::dxt1(t.data[at..at + 8].try_into().unwrap())
            } else {
                oag_texture::bcn::dxt45(t.data[at..at + 16].try_into().unwrap())
            };
            for (i, px) in texels.iter().enumerate() {
                let (x, y) = (bx * 4 + i % 4, by * 4 + i / 4);
                if x < w && y < h {
                    // Unity stores the bottom row first.
                    let row = h - 1 - y;
                    out[(row * w + x) * 4..][..4].copy_from_slice(px);
                }
            }
        }
    }
    out
}

fn wireframe(m: &Mesh) -> (u32, u32, Vec<u8>) {
    let pos = m.positions().unwrap().unwrap();
    let (w, h) = (900usize, 450usize);
    let mut img = vec![255u8; w * h * 4];
    // Side view (x, y) on the left half, top view (x, z) on the right.
    let views: [(usize, usize, usize); 2] = [(0, 1, 0), (0, 2, w / 2)];
    for (ax, ay, x0) in views {
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for p in &pos {
            for (k, a) in [ax, ay].into_iter().enumerate() {
                lo[k] = lo[k].min(p[a]);
                hi[k] = hi[k].max(p[a]);
            }
        }
        let s = ((w / 2 - 20) as f32 / (hi[0] - lo[0]).max(1e-6))
            .min((h - 20) as f32 / (hi[1] - lo[1]).max(1e-6));
        let map = |p: &[f32; 3]| {
            (
                x0 as f32 + 10.0 + (p[ax] - lo[0]) * s,
                h as f32 - 10.0 - (p[ay] - lo[1]) * s,
            )
        };
        for sm in 0..m.submeshes.len() {
            for t in m.triangles(sm).unwrap() {
                for e in [(0, 1), (1, 2), (2, 0)] {
                    let (a, b) = (map(&pos[t[e.0] as usize]), map(&pos[t[e.1] as usize]));
                    let n = ((b.0 - a.0).abs().max((b.1 - a.1).abs()) as usize).max(1);
                    for i in 0..=n {
                        let (x, y) = (
                            a.0 + (b.0 - a.0) * i as f32 / n as f32,
                            a.1 + (b.1 - a.1) * i as f32 / n as f32,
                        );
                        let (x, y) = (x as usize, y as usize);
                        if x < w && y < h {
                            img[(y * w + x) * 4..][..3].copy_from_slice(&[20, 40, 160]);
                        }
                    }
                }
            }
        }
    }
    (w as u32, h as u32, img)
}

fn obj(m: &Mesh) -> String {
    let pos = m.positions().unwrap().unwrap();
    let nrm = m.normals().unwrap();
    let uv = m.uvs(0).unwrap();
    let mut s = format!("o {}\n", m.name);
    for p in &pos {
        s += &format!("v {} {} {}\n", p[0], p[1], p[2]);
    }
    for n in nrm.iter().flatten() {
        s += &format!("vn {} {} {}\n", n[0], n[1], n[2]);
    }
    for t in uv.iter().flatten() {
        s += &format!("vt {} {}\n", t[0], t[1]);
    }
    for sm in 0..m.submeshes.len() {
        for t in m.triangles(sm).unwrap() {
            let f = |i: u32| {
                let i = i + 1;
                match (uv.is_some(), nrm.is_some()) {
                    (true, true) => format!("{i}/{i}/{i}"),
                    (true, false) => format!("{i}/{i}"),
                    (false, true) => format!("{i}//{i}"),
                    _ => format!("{i}"),
                }
            };
            s += &format!("f {} {} {}\n", f(t[0]), f(t[1]), f(t[2]));
        }
    }
    s
}

#[test]
#[ignore = "needs data/unity"]
fn export_one_mesh_and_one_texture() {
    let Some(root) = common::workshop() else {
        return;
    };
    let Some(dir) = std::env::var_os("OAG_UNITY_EXPORT") else {
        println!("skipping: OAG_UNITY_EXPORT not set");
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (id, mesh_name, tex_name) in [
        ("2050936018", "CliffhawkBody", "ShipCliffhawkClean"),
        ("1392209921", "Lion8_floor", "Lion8_diffuse"),
    ] {
        let item = common::items(&root)
            .into_iter()
            .find(|i| i.id == id)
            .unwrap();
        let bytes = common::bundle_bytes(&item.path).unwrap();
        let b = Bundle::parse(&bytes).unwrap();
        let a = Assets::open(&b).unwrap();
        for sf in a.files() {
            for o in &sf.objects {
                if o.class_id == class_id::MESH {
                    let m = Mesh::from_value(&sf.read_object(o).unwrap(), 2018).unwrap();
                    if m.name == mesh_name && m.vertex_count > 100 {
                        std::fs::write(dir.join(format!("{id}-{mesh_name}.obj")), obj(&m)).unwrap();
                        let (w, h, img) = wireframe(&m);
                        std::fs::write(
                            dir.join(format!("{id}-{mesh_name}-wire.png")),
                            oag_texture::png::encode_rgba(w, h, &img),
                        )
                        .unwrap();
                    }
                } else if o.class_id == class_id::TEXTURE_2D {
                    let t = a.texture(&sf.read_object(o).unwrap()).unwrap();
                    if t.name == tex_name {
                        let rgba = match t.format {
                            TextureFormat::Dxt1 | TextureFormat::Dxt5 => decode_dxt(&t),
                            _ => t.base_level_rgba8().unwrap(),
                        };
                        std::fs::write(
                            dir.join(format!("{id}-{tex_name}.png")),
                            oag_texture::png::encode_rgba(t.width, t.height, &rgba),
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
}
