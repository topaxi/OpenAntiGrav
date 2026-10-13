//! The game install's own SerializedFiles (`level*`, `*.assets`,
//! `globalgamemanagers`): which this reader opens, and which it refuses by name.
//!
//! A shipped player strips type trees, so these parse to a header, a type table
//! and an object table, and `read_object` refuses each with `NoTypeTree`.

use crate::common;

use std::collections::BTreeMap;

use oag_unity::error::Error;
use oag_unity::serialized::SerializedFile;

#[test]
#[ignore = "needs data/unity"]
fn install_serialized_files_open_and_refuse_by_name() {
    let Some(root) = common::install() else {
        return;
    };
    let dir = root.join("BallisticNG_Data");
    let mut rows = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap() {
        let path = e.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let serialized = name.starts_with("level") && !name.ends_with(".resS")
            || name.ends_with(".assets")
            || name.starts_with("globalgamemanagers");
        if serialized {
            rows.push((name, std::fs::read(&path).unwrap()));
        }
    }
    rows.sort();
    let mut tally = BTreeMap::<String, usize>::new();
    let (mut objects, mut sampled, mut refused) = (0usize, 0usize, 0usize);
    for (name, bytes) in &rows {
        let f = SerializedFile::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        *tally
            .entry(format!("v{} trees={}", f.version, f.has_type_trees))
            .or_default() += 1;
        objects += f.objects.len();
        for o in f.objects.iter().take(50) {
            sampled += 1;
            match f.read_object(o) {
                Err(Error::NoTypeTree { .. }) => refused += 1,
                other => panic!("{name}: expected NoTypeTree, got {other:?}"),
            }
        }
    }
    assert_eq!(rows.len(), 27);
    assert_eq!(
        tally,
        [("v22 trees=false".to_string(), 27)].into_iter().collect()
    );
    assert_eq!((objects, sampled, refused), (278_778, 878, 878));
}
