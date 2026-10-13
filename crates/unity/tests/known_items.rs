//! Two items read end to end: a track (Lion's Den) and a ship (Cliffhawk),
//! both tagged `Original`. The numbers are facts about the files, cross-checked
//! with UnityPy at the time of writing, not file contents.

use crate::common;

use oag_unity::assets::Assets;
use oag_unity::bundle::Bundle;
use oag_unity::classes::{GameObject, Material, Mesh, TextureFormat, Transform};
use oag_unity::serialized::class_id;
use oag_unity::value::PPtr;

fn open(id: &str) -> Option<Vec<u8>> {
    let root = common::workshop()?;
    let item = common::items(&root).into_iter().find(|i| i.id == id)?;
    common::bundle_bytes(&item.path)
}

#[test]
#[ignore = "needs data/unity"]
fn lions_den() {
    let Some(bytes) = open("1392209921") else {
        return;
    };
    let b = Bundle::parse(&bytes).unwrap();
    assert_eq!(
        (
            b.archive_version,
            b.engine_version.as_str(),
            b.block_compression,
            b.block_count
        ),
        (6, "2018.3.8f1", 1, 1)
    );
    let a = Assets::open(&b).unwrap();
    assert_eq!(
        a.names(),
        [
            "BuildPlayer-Lion's Den.sharedAssets",
            "BuildPlayer-Lion's Den"
        ]
    );
    assert_eq!(a.files()[0].objects.len(), 162);
    assert_eq!(a.files()[1].objects.len(), 2409);

    let (mut meshes, mut tris, mut verts) = (Vec::new(), 0, 0);
    let mut formats = std::collections::BTreeMap::<String, usize>::new();
    let mut empty = 0;
    for sf in a.files() {
        for o in &sf.objects {
            match o.class_id {
                class_id::MESH => {
                    let m = Mesh::from_value(&sf.read_object(o).unwrap(), 2018).unwrap();
                    tris += (0..m.submeshes.len())
                        .map(|s| m.triangles(s).unwrap().len())
                        .sum::<usize>();
                    verts += m.vertex_count;
                    meshes.push((
                        m.name.clone(),
                        m.vertex_count,
                        (0..m.submeshes.len())
                            .map(|s| m.triangles(s).unwrap().len())
                            .sum::<usize>(),
                    ));
                }
                class_id::TEXTURE_2D => {
                    let t = a.texture(&sf.read_object(o).unwrap()).unwrap();
                    empty += usize::from(t.is_empty());
                    *formats.entry(format!("{:?}", t.format)).or_default() += 1;
                }
                _ => {}
            }
        }
    }
    assert_eq!((meshes.len(), tris, verts), (43, 26009, 38604));
    for want in [
        ("Lion8_floor", 4128, 1376),
        ("Lion8_wall", 3900, 1300),
        ("DesertRoadside", 3800, 6398),
    ] {
        assert!(
            meshes.iter().any(|m| (m.0.as_str(), m.1, m.2) == want),
            "{want:?}"
        );
    }
    assert_eq!(
        formats,
        [("Argb32", 34), ("Dxt1", 36), ("Dxt5", 6), ("Rgba32", 3)]
            .map(|(k, v)| (k.to_string(), v))
            .into_iter()
            .collect()
    );
    assert_eq!(empty, 1, "the dynamic font atlas is the one 0x0 texture");
}

#[test]
#[ignore = "needs data/unity"]
fn cliffhawk() {
    let Some(bytes) = open("2050936018") else {
        return;
    };
    let b = Bundle::parse(&bytes).unwrap();
    assert_eq!((b.archive_version, b.block_compression), (6, 3));
    let a = Assets::open(&b).unwrap();
    assert_eq!(a.files().len(), 1);
    let sf = &a.files()[0];

    let mut names = std::collections::BTreeMap::new();
    let mut gos = Vec::new();
    for o in &sf.objects {
        let v = sf.read_object(o).unwrap();
        match o.class_id {
            class_id::GAME_OBJECT => {
                let g = GameObject::from_value(&v).unwrap();
                names.insert(o.path_id, g.name.clone());
                gos.push((o.path_id, g));
            }
            class_id::MESH => {
                let m = Mesh::from_value(&v, 2018).unwrap();
                if m.name == "CliffhawkBody" {
                    assert!(
                        [(671, 766), (631, 742)]
                            .contains(&(m.vertex_count, m.triangles(0).unwrap().len()))
                    );
                }
            }
            class_id::TEXTURE_2D => {
                let t = a.texture(&v).unwrap();
                assert_eq!(
                    (t.width, t.height, t.format, t.mip_count),
                    (
                        if t.name.starts_with("Ship") {
                            1024
                        } else {
                            t.width
                        },
                        if t.name.starts_with("Ship") {
                            1024
                        } else {
                            t.height
                        },
                        TextureFormat::Dxt1,
                        1
                    ),
                    "{}",
                    t.name
                );
            }
            _ => {}
        }
    }
    assert_eq!(gos.len(), 17);

    let (_, ingame_engine) = gos
        .iter()
        .find(|(_, g)| g.name == "Engine Flare Main")
        .expect("engine flare");
    let transform = ingame_engine
        .components
        .iter()
        .find_map(|p| {
            let l = a.resolve(0, *p)?;
            (l.info.class_id == class_id::TRANSFORM).then_some((*p, l))
        })
        .expect("its transform");
    let t = Transform::from_value(&sf.read_object(transform.1.info).unwrap()).unwrap();
    let path = a
        .transform_path(
            0,
            PPtr {
                file_id: 0,
                path_id: transform.1.info.path_id,
            },
        )
        .unwrap();
    assert!(
        t.scale.iter().all(|s| *s > 0.0 && *s < 1.0),
        "{:?}",
        t.scale
    );

    let mat = sf
        .objects
        .iter()
        .filter(|o| o.class_id == class_id::MATERIAL)
        .map(|o| Material::from_value(&sf.read_object(o).unwrap()).unwrap())
        .find(|m| m.name == "MatCliffhawkClean")
        .expect("MatCliffhawkClean");
    for (slot, want) in [
        ("_MainTex", "ShipCliffhawkClean"),
        ("_Illum", "ShipCliffhawkIllumMap"),
        ("_ReflectionMask", "ShipCliffhawkReflectMap"),
    ] {
        let p = mat.texture(slot).unwrap_or_else(|| panic!("slot {slot}"));
        let (_, tex) = a.read(0, p).unwrap().expect("texture resolves");
        assert_eq!(a.texture(&tex).unwrap().name, want, "{slot}");
    }
    assert_eq!(path, "Ingame Ship/Engine Flare Main");
}
