//! Scratch sweep for `docs/rendering/water.md`: which `.vex` files of the
//! PSP and PS2 discs carry batches with header byte 3 bit `0x04`, the bit
//! Pulse's `Mesh_CompileBatchSet` sorts into its caustic pass
//! (`docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`), and which embed a
//! texture whose name says water.
//!
//! Prints only, so it is `#[ignore]`d and returns early unless `OAG_SWEEP` is
//! set:
//!
//! ```sh
//! OAG_SWEEP=1 cargo nextest run -p oag-vex --run-ignored all --no-capture \
//!     -E 'test(/^water_caustic_sweep::/)'
//! ```

use std::collections::BTreeMap;

use oag_disc::DiscImage;
use oag_vex::vex;

use crate::wad_walk;

/// Header byte 3 bit the caustic pass selects on.
const CAUSTIC_BIT: u8 = 0x04;

const SOURCES: [(&str, &[&str]); 3] = [
    ("pulse-psp-eu.chd", &["PSP_GAME/USRDIR/Data.wad"]),
    ("pure-psp-eu.chd", &["PSP_GAME/USRDIR/Data.wad"]),
    ("pulse-ps2-eu.chd", &["54748/WADS2.WAD", "54748/WADSP.WAD"]),
];

fn water_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [
        "water", "sea", "wave", "caustic", "ocean", "lake", "river", "ripple", "pool",
    ]
    .iter()
    .any(|k| n.contains(k))
}

#[test]
#[ignore = "scratch sweep: prints only; set OAG_SWEEP to run it"]
fn caustic_bit_and_water_textures_per_vex() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    for (image, archives) in SOURCES {
        let Some(path) = oag_testdata::image(&format!("data/images/{image}")) else {
            println!("{image}: absent");
            continue;
        };
        let mut disc = DiscImage::open(&path).expect("image opens");
        for archive in archives {
            let blobs = wad_walk::blobs(&mut disc, archive, wad_walk::VEX_HEAD, vex::has_magic);
            for (index, bytes) in &blobs {
                let Ok(nodes) = vex::nodes(bytes) else {
                    continue;
                };
                let mut total = 0usize;
                let mut flagged = 0usize;
                let mut vertices = 0usize;
                let mut y = (f32::MAX, f32::MIN);
                let mut by_node: BTreeMap<String, usize> = BTreeMap::new();
                for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
                    let range = node.payload();
                    if range.end > bytes.len() {
                        continue;
                    }
                    for list in 0..2u8 {
                        let Ok(batches) = vex::mesh_batches(&bytes[range.clone()], list) else {
                            continue;
                        };
                        for batch in &batches {
                            total += 1;
                            if batch.header_flags & CAUSTIC_BIT == 0 {
                                continue;
                            }
                            flagged += 1;
                            vertices += batch.vertices.len();
                            y.0 = y.0.min(batch.bounds.0[1]);
                            y.1 = y.1.max(batch.bounds.1[1]);
                            *by_node
                                .entry(node.name.clone().unwrap_or_else(|| "?".into()))
                                .or_default() += 1;
                        }
                    }
                }
                let all: Vec<String> = vex::textures(bytes)
                    .unwrap_or_default()
                    .into_iter()
                    .flatten()
                    .filter_map(|t| t.name.or(t.asset_path))
                    .collect();
                let first = all.first().cloned().unwrap_or_default();
                let textures: Vec<&String> = all.iter().filter(|n| water_name(n)).collect();
                if flagged == 0 && textures.is_empty() {
                    continue;
                }
                let root = nodes
                    .iter()
                    .find_map(|n| n.name.clone())
                    .unwrap_or_else(|| "?".into());
                let names: Vec<String> = by_node
                    .iter()
                    .take(10)
                    .map(|(n, c)| format!("{n}x{c}"))
                    .collect();
                println!(
                    "{image} {archive} entry {index} root {root} first texture {first}: caustic batches {flagged}/{total}, \
                     {vertices} vertices, y {:.1}..{:.1}, nodes [{}] ({} nodes), water textures {textures:?}",
                    y.0,
                    y.1,
                    names.join(", "),
                    by_node.len()
                );
            }
        }
    }
}
