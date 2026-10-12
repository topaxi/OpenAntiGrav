//! The fields of `Section`, `Tile` and `TrackDataStorage` this layer does not
//! read, across the 32 tracks: each with how many objects carry it and how many
//! hold a non-default value. A field the tools add, or one that starts to
//! matter, fails here instead of being skipped silently.

use std::collections::BTreeMap;

use oag_ballisticng::track::READ_FIELDS;
use oag_ballisticng::wtrk::Wtrk;
use oag_unity::assets::Assets;
use oag_unity::bundle::Bundle;
use oag_unity::serialized::class_id;
use oag_unity::value::{PPtr, Value};

fn non_default(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::UInt(u) => *u != 0,
        Value::Float(f) => *f != 0.0,
        Value::Double(f) => *f != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Bytes(b) => !b.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Struct(s) => s.fields.iter().any(|(_, f)| non_default(f)),
        Value::PPtr(p) => !p.is_null(),
    }
}

/// Fields every `MonoBehaviour` carries; not the game's.
const BEHAVIOUR: [&str; 4] = ["m_Enabled", "m_GameObject", "m_Name", "m_Script"];

#[test]
#[ignore = "needs data/ballisticng"]
fn the_fields_this_layer_does_not_read_are_frozen() {
    let Some(root) = oag_testdata::exact("data/ballisticng/workshop-473770") else {
        return;
    };
    let mut dirs: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    // class -> field -> (objects, non-default)
    let mut unread: BTreeMap<String, BTreeMap<String, (usize, usize)>> = BTreeMap::new();
    let mut read_seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for d in dirs {
        for e in std::fs::read_dir(&d).unwrap() {
            let path = e.unwrap().path();
            if path.extension().and_then(|x| x.to_str()) != Some("wtrk") {
                continue;
            }
            let file = std::fs::read(&path).unwrap();
            let w = Wtrk::parse(&file).unwrap();
            let b = Bundle::parse(w.bundle(&file)).unwrap();
            let a = Assets::open(&b).unwrap();
            for (fi, sf) in a.files().iter().enumerate() {
                for o in sf
                    .objects
                    .iter()
                    .filter(|o| o.class_id == class_id::MONO_BEHAVIOUR)
                {
                    let t = &sf.types[o.type_index];
                    let Some(&(file_id, path_id)) = usize::try_from(t.script_type_index)
                        .ok()
                        .and_then(|i| sf.script_types.get(i))
                    else {
                        continue;
                    };
                    let Some((_, script)) = a.read(fi, PPtr { file_id, path_id }).unwrap() else {
                        continue;
                    };
                    let class = script.get("m_ClassName").unwrap().as_str().unwrap();
                    let Some((_, read)) = READ_FIELDS.iter().find(|(c, _)| *c == class) else {
                        continue;
                    };
                    let Value::Struct(st) = sf.read_object(o).unwrap() else {
                        continue;
                    };
                    for (name, value) in &st.fields {
                        if BEHAVIOUR.contains(&&**name) {
                            continue;
                        }
                        if read.contains(&&**name) {
                            let seen = read_seen.entry(class.to_string()).or_default();
                            if !seen.iter().any(|s| s == &**name) {
                                seen.push(name.to_string());
                            }
                            continue;
                        }
                        let ent = unread
                            .entry(class.to_string())
                            .or_default()
                            .entry(name.to_string())
                            .or_default();
                        ent.0 += 1;
                        ent.1 += usize::from(non_default(value));
                    }
                }
            }
        }
    }
    // Every field the reader names exists in the census (a typo would not).
    for (class, read) in READ_FIELDS {
        let seen = &read_seen[*class];
        for f in *read {
            assert!(seen.iter().any(|s| s == f), "{class}.{f} never seen");
        }
    }
    let flat: Vec<(String, String, usize, usize)> = unread
        .into_iter()
        .flat_map(|(c, fs)| {
            fs.into_iter()
                .map(move |(f, (n, nd))| (c.clone(), f, n, nd))
        })
        .collect();
    let table: Vec<String> = flat
        .iter()
        .map(|(c, f, n, nd)| format!("{c}.{f} {n} {nd}"))
        .collect();
    assert_eq!(
        table,
        [
            "Section.LockOnToDistance 17608 17608",
            "Section.NextAiOvverride 10052 0",
            "Section.Previous 17608 0",
            "Section.Tiles 17608 17608",
            "Tile.Color 66549 0",
            "Tile.ForcedShadow 66549 0",
            "Tile.MapOrder 66549 66549",
            "Tile.Swooshable 66549 316",
            "Tile.UvFlip 66549 190",
            "Tile.UvOrder 66549 0",
            "TrackDataStorage.DataParent 32 0",
            "TrackDataStorage.FloorTilesObject 32 32",
            "TrackDataStorage.MappedFloorTIles 32 32",
            "TrackDataStorage.MappedWallTiles 32 32",
            "TrackDataStorage.SectionsObject 32 32",
            "TrackDataStorage.SpawnTileReplacementIndex 32 13",
            "TrackDataStorage.TrackFloor 32 32",
            "TrackDataStorage.TrackTextures 32 32",
            "TrackDataStorage.TrackWall 32 32",
            "TrackDataStorage.UseNewUvSytstem 32 32",
            "TrackDataStorage.WallTilesObject 32 32",
            "TrackDataStorage.WeaponPadReplacementIndex 32 13",
        ]
    );
}
