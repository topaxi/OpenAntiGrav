//! Every UnityFS bundle in the Workshop census opens.
//!
//! Census (92 item files under `data/ballisticng/workshop-473770`, counted by
//! `read_dir` over each item directory): 47 `.csf` ship bundles, 32 `.wtrk`
//! track files that wrap a bundle at an offset, 13 `.shp` legacy ships that
//! hold none.

mod common;

use std::collections::BTreeMap;

use oag_unity::bundle::Bundle;

#[test]
#[ignore = "needs data/ballisticng"]
fn census_opens_every_bundle() {
    let Some(root) = common::workshop() else {
        return;
    };
    let (mut skipped, mut by_ext) = (0, BTreeMap::<String, usize>::new());
    let mut shape = BTreeMap::<(u32, String, u32), usize>::new();
    let (mut streams, mut res_s, mut resource) = (0, 0, 0);
    for item in common::items(&root) {
        *by_ext.entry(item.ext.clone()).or_default() += 1;
        let Some(bytes) = common::bundle_bytes(&item.path) else {
            assert_eq!(item.ext, "shp", "{} holds no bundle", item.path.display());
            skipped += 1;
            continue;
        };
        let b = Bundle::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", item.path.display()));
        streams += b.nodes().iter().filter(|n| !n.is_serialized_file()).count();
        res_s += b
            .nodes()
            .iter()
            .filter(|n| n.path.ends_with(".resS"))
            .count();
        resource += b
            .nodes()
            .iter()
            .filter(|n| n.path.ends_with(".resource"))
            .count();
        *shape
            .entry((
                b.archive_version,
                b.engine_version.clone(),
                b.block_compression,
            ))
            .or_default() += 1;
    }
    assert_eq!(
        by_ext,
        [("csf", 47), ("shp", 13), ("wtrk", 32)]
            .map(|(k, v)| (k.to_string(), v))
            .into_iter()
            .collect()
    );
    assert_eq!(skipped, 13);
    // Archive version, engine, block codec (1 = LZMA for tracks, 3 = LZ4HC for ships).
    let want: BTreeMap<(u32, String, u32), usize> = [
        ((6, "2018.3.8f1", 1), 31),
        ((6, "2018.3.8f1", 3), 47),
        ((7, "2020.3.5f1", 1), 1),
    ]
    .map(|((v, e, c), n)| ((v, e.to_string(), c), n))
    .into_iter()
    .collect();
    assert_eq!(shape, want);
    // 47 `.resS` (one per ship: texture pixels) and 78 `.resource` (audio) nodes.
    assert_eq!((streams, res_s, resource), (125, 47, 78));
}
