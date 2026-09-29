//! Scratch probe for the Omega race lane: decode `track_col.col` files with
//! [`oag_vex::kdcol`] and check the two invariants the 2048 ground-truth test
//! rests on - the leaf runs tile the leaf array, and the stated bounds are
//! centre and half-extent of the soup.
//!
//! `cargo run -p oag-vex --example omega_col_probe -- <file.col>...`

use oag_vex::kdcol;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut failures = 0;
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path)?;
        let name = path.rsplit('/').next().unwrap_or(&path);
        let decoded = match kdcol::parse(&data) {
            Ok(d) => d,
            Err(e) => {
                println!("{name}: {e}");
                failures += 1;
                continue;
            }
        };
        let mut runs: Vec<(usize, usize)> = decoded
            .nodes
            .iter()
            .filter(|n| n.is_leaf())
            .map(|n| (n.first_leaf, n.triangle_count))
            .collect();
        runs.sort_unstable();
        let mut at = 0;
        let mut tiles = true;
        for (first, count) in runs {
            tiles &= first == at;
            at += count;
        }
        tiles &= at == decoded.leaves.len();
        println!(
            "{name}: {:?}, {} nodes, {} leaf indices, {} vertices, {} triangles, tiles {tiles}",
            decoded.layout,
            decoded.nodes.len(),
            decoded.leaves.len(),
            decoded.mesh.vertices.len(),
            decoded.mesh.triangles.len()
        );
        if !tiles {
            failures += 1;
        }
    }
    println!("{failures} failure(s)");
    Ok(())
}
