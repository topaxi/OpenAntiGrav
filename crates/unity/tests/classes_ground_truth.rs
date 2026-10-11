//! Every Mesh, Texture2D, Material, GameObject, Transform and MonoBehaviour in
//! the census reads through its typed class, or is refused by name.

mod common;

use std::collections::BTreeMap;

use oag_unity::assets::Assets;
use oag_unity::bundle::Bundle;
use oag_unity::classes::{GameObject, Material, Mesh, MonoBehaviour, Transform};
use oag_unity::serialized::class_id;

#[test]
#[ignore = "needs data/unity"]
fn every_builtin_class_reads() {
    let Some(root) = common::workshop() else {
        return;
    };
    let mut ok: BTreeMap<i32, usize> = BTreeMap::new();
    let mut formats: BTreeMap<String, usize> = BTreeMap::new();
    let mut refused: BTreeMap<(i32, String), usize> = BTreeMap::new();
    for item in common::items(&root) {
        let Some(bytes) = common::bundle_bytes(&item.path) else {
            continue;
        };
        let b = Bundle::parse(&bytes).unwrap();
        let assets = Assets::open(&b).unwrap();
        for sf in assets.files() {
            let major: u32 = sf.unity_version.split('.').next().unwrap().parse().unwrap();
            for o in &sf.objects {
                let r = match o.class_id {
                    class_id::MESH
                    | class_id::TEXTURE_2D
                    | class_id::MATERIAL
                    | class_id::GAME_OBJECT
                    | class_id::TRANSFORM
                    | class_id::MONO_BEHAVIOUR => sf.read_object(o).map(|v| (o.class_id, v)),
                    _ => continue,
                };
                let (c, v) = r.unwrap();
                let res = match c {
                    class_id::MESH => Mesh::from_value(&v, major).map(|_| ()),
                    class_id::TEXTURE_2D => assets.texture(&v).map(|t| {
                        *formats.entry(format!("{:?}", t.format)).or_default() += 1;
                    }),
                    class_id::MATERIAL => Material::from_value(&v).map(|_| ()),
                    class_id::GAME_OBJECT => GameObject::from_value(&v).map(|_| ()),
                    class_id::TRANSFORM => Transform::from_value(&v).map(|_| ()),
                    _ => MonoBehaviour::from_value(&v).map(|_| ()),
                };
                match res {
                    Ok(()) => *ok.entry(c).or_default() += 1,
                    Err(e) => *refused.entry((c, e.to_string())).or_default() += 1,
                }
            }
        }
    }
    assert!(refused.is_empty(), "{refused:?}");
    assert_eq!(
        ok,
        [
            (class_id::GAME_OBJECT, 8459),
            (class_id::TRANSFORM, 8306),
            (class_id::MATERIAL, 805),
            (class_id::TEXTURE_2D, 2302),
            (class_id::MESH, 1570),
            (class_id::MONO_BEHAVIOUR, 88703),
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        formats,
        [
            ("Argb32", 584),
            ("Dxt1", 1218),
            ("Dxt3", 1),
            ("Dxt5", 228),
            ("Rgb24", 177),
            ("Rgba32", 94)
        ]
        .map(|(k, v)| (k.to_string(), v))
        .into_iter()
        .collect()
    );
}
