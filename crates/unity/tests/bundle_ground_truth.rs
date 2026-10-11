//! Every UnityFS bundle in the Workshop census opens.

mod common;

use oag_unity::bundle::Bundle;

#[test]
#[ignore = "needs data/unity"]
fn census_opens_every_bundle() {
    let Some(root) = common::workshop() else {
        return;
    };
    let (mut bundles, mut skipped) = (0, 0);
    for item in common::items(&root) {
        let Some(bytes) = common::bundle_bytes(&item.path) else {
            skipped += 1;
            continue;
        };
        let t = std::time::Instant::now();
        let b = Bundle::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", item.path.display()));
        println!(
            "{} {} v{} {} blocks={} comp={} nodes={} {:?}",
            item.id,
            item.ext,
            b.archive_version,
            b.engine_version,
            b.block_count,
            b.block_compression,
            b.nodes().len(),
            t.elapsed()
        );
        bundles += 1;
    }
    println!("bundles={bundles} skipped={skipped}");
}
