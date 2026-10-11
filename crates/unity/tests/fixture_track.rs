//! The project-authored track fixture (`tests/fixtures`, regenerate per its
//! README): no game or Workshop content, so it is committed and runs in plain
//! `just test`. Built with Unity 6000.6.5f1: UnityFS version 8, SerializedFile 23.

use oag_unity::assets::Assets;
use oag_unity::bundle::Bundle;
use oag_unity::classes::{Material, Mesh, MonoBehaviour, TextureFormat};
use oag_unity::serialized::class_id;
use oag_unity::value::Value;

fn read(codec: &str, name: &str) -> Vec<u8> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(codec)
        .join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn all_three_compressions_open_and_read_exactly() {
    for codec in ["lzma", "lz4", "raw"] {
        let (mut meshes, mut textures, mut mats, mut scripts) = (Vec::new(), 0, 0, Vec::new());
        let mut xsums: Vec<f64> = Vec::new();
        for name in ["fixture_loop", "fixture_loop_scene"] {
            let bytes = read(codec, name);
            let b = Bundle::parse(&bytes).unwrap_or_else(|e| panic!("{codec}/{name}: {e}"));
            assert_eq!(
                (b.archive_version, b.engine_version.as_str()),
                (8, "6000.6.5f1"),
                "{codec}"
            );
            let a = Assets::open(&b).unwrap();
            for sf in a.files() {
                assert_eq!(sf.version, 23);
                for o in &sf.objects {
                    let v = sf
                        .read_object(o)
                        .unwrap_or_else(|e| panic!("{codec}/{name} class {}: {e}", o.class_id));
                    match o.class_id {
                        class_id::MESH => {
                            let m = Mesh::from_value(&v, 6000).unwrap();
                            let tris: usize = (0..m.submeshes.len())
                                .map(|s| m.triangles(s).unwrap().len())
                                .sum();
                            meshes.push((m.name.clone(), m.vertex_count, tris));
                            let x: f64 = m
                                .positions()
                                .unwrap()
                                .unwrap()
                                .iter()
                                .map(|p| f64::from(p[0].abs()))
                                .sum();
                            xsums.push(x);
                        }
                        class_id::TEXTURE_2D => {
                            let t = a.texture(&v).unwrap();
                            assert_eq!(
                                (t.width, t.height, t.format),
                                (64, 64, TextureFormat::Dxt1)
                            );
                            assert_eq!((t.mip_count, t.data.len()), (7, 2744));
                            textures += 1;
                        }
                        class_id::MATERIAL => {
                            Material::from_value(&v).unwrap();
                            mats += 1;
                        }
                        class_id::MONO_BEHAVIOUR => {
                            scripts.push(MonoBehaviour::from_value(&v).unwrap())
                        }
                        _ => {}
                    }
                }
            }
        }
        // Both bundles carry the same three meshes, texture, material and script.
        meshes.sort();
        let want: Vec<_> = ["TrackFloor", "TrackWallLeft", "TrackWallRight"]
            .iter()
            .flat_map(|n| [(n.to_string(), 258, 256), (n.to_string(), 258, 256)])
            .collect();
        // Sum of |x| over each mesh, as UnityPy reads them (floor, left wall, right wall; twice each).
        xsums.sort_by(f64::total_cmp);
        for (got, want) in xsums.iter().zip([
            23220.049919719997,
            23220.049919719997,
            23966.415718129996,
            23966.415718129996,
            24712.781516539988,
            24712.781516539988,
        ]) {
            assert!((got - want).abs() < 1e-3 * want, "{codec}: {got} vs {want}");
        }
        assert_eq!(meshes, want, "{codec}");
        assert_eq!((textures, scripts.len()), (2, 2), "{codec}");
        assert!(mats >= 2, "{codec}");
        for m in &scripts {
            let v = &m.value;
            let pts = v
                .get("controlPoints")
                .and_then(Value::as_array)
                .expect("controlPoints");
            assert_eq!(pts.len(), 8);
            assert_eq!(pts[1].as_vec3(), Some([120.0, 0.0, 10.0]));
            assert_eq!(pts[3].as_vec3(), Some([190.0, 12.0, 190.0]));
            assert_eq!(v.get("width").and_then(Value::as_f32), Some(12.0));
            assert_eq!(v.get("wallHeight").and_then(Value::as_f32), Some(2.0));
            assert_eq!(v.get("samples").and_then(Value::as_i64), Some(128));
            assert_eq!(v.get("startSample").and_then(Value::as_i64), Some(0));
            assert_eq!(
                v.get("trackName").and_then(Value::as_str),
                Some("Fixture Loop")
            );
            assert!(!m.script.is_null());
        }
    }
}

#[test]
fn the_three_compressions_hold_the_same_objects() {
    let mut shapes = Vec::new();
    for codec in ["lzma", "lz4", "raw"] {
        let bytes = read(codec, "fixture_loop_scene");
        let b = Bundle::parse(&bytes).unwrap();
        let a = Assets::open(&b).unwrap();
        assert_eq!(
            a.files().len(),
            2,
            "scene bundle: a scene file and shared assets"
        );
        shapes.push(
            a.files()
                .iter()
                .map(|f| f.objects.len())
                .collect::<Vec<_>>(),
        );
    }
    assert!(shapes.windows(2).all(|w| w[0] == w[1]), "{shapes:?}");
}
