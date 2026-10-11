//! Every SerializedFile in the census parses and every object reads through its
//! typetree with no leftover bytes.

mod common;

use oag_unity::bundle::Bundle;
use oag_unity::serialized::SerializedFile;

#[test]
#[ignore = "needs data/unity"]
fn every_object_reads_exactly() {
    let Some(root) = common::workshop() else {
        return;
    };
    let (mut files, mut objects, mut versions) =
        (0usize, 0usize, std::collections::BTreeMap::new());
    let mut failures = Vec::new();
    let mut streams = 0usize;
    for item in common::items(&root) {
        let Some(bytes) = common::bundle_bytes(&item.path) else {
            continue;
        };
        let b = Bundle::parse(&bytes).unwrap();
        for node in b.nodes() {
            if !node.is_serialized_file() {
                streams += 1;
                continue;
            }
            let sf = match SerializedFile::parse(b.node_bytes(node)) {
                Ok(sf) => sf,
                Err(e) => {
                    failures.push(format!("{} {}: parse {e}", item.id, node.path));
                    continue;
                }
            };
            files += 1;
            *versions.entry(sf.version).or_insert(0usize) += 1;
            for o in &sf.objects {
                objects += 1;
                if let Err(e) = sf.read_object(o) {
                    failures.push(format!(
                        "{} {} path_id {} class {}: {e}",
                        item.id, node.path, o.path_id, o.class_id
                    ));
                }
            }
        }
    }
    println!(
        "streams={streams} files={files} objects={objects} versions={versions:?} failures={}",
        failures.len()
    );
    for f in failures.iter().take(40) {
        println!("{f}");
    }
    assert!(failures.is_empty());
}
