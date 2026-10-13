//! The Workshop census, read three ways on one walk of it: every bundle opens,
//! every SerializedFile parses and every object reads exactly, and every
//! built-in class reads through its typed struct.
//!
//! Census (92 item files under `data/ballisticng/workshop-473770`, counted by
//! `read_dir` over each item directory): 47 `.csf` ship bundles, 32 `.wtrk`
//! track files that wrap a bundle at an offset, 13 `.shp` legacy ships that
//! hold none.
//!
//! **One walk, three claims.** These were three test binaries until
//! 2026-10-13, and each LZMA- or LZ4-unpacked all 79 bundles again, which was
//! nearly all of their time. Each claim keeps its own type and assertions; the
//! walk hands every one the same opened bundle.

mod common;

use std::collections::BTreeMap;

use oag_unity::assets::Assets;
use oag_unity::bundle::Bundle;
use oag_unity::classes::{GameObject, Material, Mesh, MonoBehaviour, Transform};
use oag_unity::serialized::{SerializedFile, class_id};

#[test]
#[ignore = "needs data/ballisticng"]
fn the_workshop_census() {
    let Some(root) = common::workshop() else {
        return;
    };
    let mut bundles = Bundles::default();
    let mut objects = Objects::default();
    let mut classes = Classes::default();
    for item in common::items(&root) {
        *bundles.by_ext.entry(item.ext.clone()).or_default() += 1;
        let Some(bytes) = common::bundle_bytes(&item.path) else {
            assert_eq!(item.ext, "shp", "{} holds no bundle", item.path.display());
            bundles.skipped += 1;
            continue;
        };
        let b = Bundle::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", item.path.display()));
        bundles.count(&b);
        objects.count(&item.id, &b);
        classes.count(&b);
    }
    bundles.census_opens_every_bundle();
    objects.every_object_reads_exactly();
    classes.every_builtin_class_reads();
}

/// Every UnityFS bundle in the census opens, and its shape.
#[derive(Default)]
struct Bundles {
    skipped: usize,
    by_ext: BTreeMap<String, usize>,
    shape: BTreeMap<(u32, String, u32), usize>,
    streams: usize,
    res_s: usize,
    resource: usize,
}

impl Bundles {
    fn count(&mut self, b: &Bundle) {
        self.streams += b.nodes().iter().filter(|n| !n.is_serialized_file()).count();
        self.res_s += b
            .nodes()
            .iter()
            .filter(|n| n.path.ends_with(".resS"))
            .count();
        self.resource += b
            .nodes()
            .iter()
            .filter(|n| n.path.ends_with(".resource"))
            .count();
        *self
            .shape
            .entry((
                b.archive_version,
                b.engine_version.clone(),
                b.block_compression,
            ))
            .or_default() += 1;
    }

    fn census_opens_every_bundle(self) {
        assert_eq!(
            self.by_ext,
            [("csf", 47), ("shp", 13), ("wtrk", 32)]
                .map(|(k, v)| (k.to_string(), v))
                .into_iter()
                .collect()
        );
        assert_eq!(self.skipped, 13);
        // Archive version, engine, block codec (1 = LZMA for tracks, 3 = LZ4HC for ships).
        let want: BTreeMap<(u32, String, u32), usize> = [
            ((6, "2018.3.8f1", 1), 31),
            ((6, "2018.3.8f1", 3), 47),
            ((7, "2020.3.5f1", 1), 1),
        ]
        .map(|((v, e, c), n)| ((v, e.to_string(), c), n))
        .into_iter()
        .collect();
        assert_eq!(self.shape, want);
        // 47 `.resS` (one per ship: texture pixels) and 78 `.resource` (audio) nodes.
        assert_eq!((self.streams, self.res_s, self.resource), (125, 47, 78));
    }
}

/// Every SerializedFile in the census parses and every object reads through its
/// typetree with no leftover bytes.
#[derive(Default)]
struct Objects {
    files: usize,
    objects: usize,
    versions: BTreeMap<u32, usize>,
    failures: Vec<String>,
    streams: usize,
}

impl Objects {
    fn count(&mut self, id: &str, b: &Bundle) {
        for node in b.nodes() {
            if !node.is_serialized_file() {
                self.streams += 1;
                continue;
            }
            let sf = match SerializedFile::parse(b.node_bytes(node)) {
                Ok(sf) => sf,
                Err(e) => {
                    self.failures.push(format!("{id} {}: parse {e}", node.path));
                    continue;
                }
            };
            self.files += 1;
            *self.versions.entry(sf.version).or_insert(0usize) += 1;
            for o in &sf.objects {
                self.objects += 1;
                if let Err(e) = sf.read_object(o) {
                    self.failures.push(format!(
                        "{id} {} path_id {} class {}: {e}",
                        node.path, o.path_id, o.class_id
                    ));
                }
            }
        }
    }

    fn every_object_reads_exactly(self) {
        println!(
            "streams={} files={} objects={} versions={:?} failures={}",
            self.streams,
            self.files,
            self.objects,
            self.versions,
            self.failures.len()
        );
        for f in self.failures.iter().take(40) {
            println!("{f}");
        }
        assert!(self.failures.is_empty());
    }
}

const AUDIO_CLIP: i32 = 83;

/// Every Mesh, Texture2D, Material, GameObject, Transform and MonoBehaviour in
/// the census reads through its typed class, or is refused by name.
#[derive(Default)]
struct Classes {
    ok: BTreeMap<i32, usize>,
    clips: usize,
    formats: BTreeMap<String, usize>,
    refused: BTreeMap<(i32, String), usize>,
}

impl Classes {
    fn count(&mut self, b: &Bundle) {
        let assets = Assets::open(b).unwrap();
        for sf in assets.files() {
            let major: u32 = sf.unity_version.split('.').next().unwrap().parse().unwrap();
            for o in &sf.objects {
                if o.class_id == AUDIO_CLIP {
                    let v = sf.read_object(o).unwrap();
                    let src = v
                        .get("m_Resource")
                        .and_then(|r| r.get("m_Source"))
                        .and_then(|s| s.as_str());
                    assert!(src.is_some_and(|s| s.ends_with(".resource")), "{src:?}");
                    self.clips += 1;
                    continue;
                }
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
                        *self.formats.entry(format!("{:?}", t.format)).or_default() += 1;
                    }),
                    class_id::MATERIAL => Material::from_value(&v).map(|_| ()),
                    class_id::GAME_OBJECT => GameObject::from_value(&v).map(|_| ()),
                    class_id::TRANSFORM => Transform::from_value(&v).map(|_| ()),
                    _ => MonoBehaviour::from_value(&v).map(|_| ()),
                };
                match res {
                    Ok(()) => *self.ok.entry(c).or_default() += 1,
                    Err(e) => *self.refused.entry((c, e.to_string())).or_default() += 1,
                }
            }
        }
    }

    fn every_builtin_class_reads(self) {
        assert!(self.refused.is_empty(), "{:?}", self.refused);
        assert_eq!(
            self.clips, 134,
            "every AudioClip streams from a .resource node"
        );
        assert_eq!(
            self.ok,
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
            self.formats,
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
}
