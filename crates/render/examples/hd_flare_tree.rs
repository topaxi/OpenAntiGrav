//! Scratch probe: the node tree of a Wipeout HD `engineflare.vex`, so the
//! `EF_Main` / `EF_Boost` split is read off the file rather than off a hunch.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_flare_tree /data/ships/feisar/engineflare
//! ```
//!
//! Prints every node with its parent, and then the named `f32` attributes any
//! of them carries - which is where `EF_Main`'s unread `AnimEnd` shows up. The
//! companion of `hd_flare_probe`; both are cited by
//! `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`.
use oag_formats::vex;
use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let iso = "data/images/hdfury-ps3-eu-dec.iso";
    let stem = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/data/ships/feisar/engineflare".into());
    for archive in [
        "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
    ] {
        let spec = format!("{iso}:PS3_GAME/USRDIR/{archive}.PSARC");
        let Ok(data) = mesh::read_blob(&spec, &format!("{stem}.vex")) else {
            continue;
        };
        let classes = vex::classes_of(&data).map_err(|e| anyhow::anyhow!("{e}"))?;
        let nodes = vex::nodes(&data).map_err(|e| anyhow::anyhow!("{e}"))?;
        println!(
            "{archive} {stem}.vex: version {}, {} node(s), mesh class {:?}, \
             transform {:#x}",
            classes.version,
            nodes.len(),
            classes.mesh,
            vex::CLASS_TRANSFORM
        );
        for (index, node) in nodes.iter().enumerate() {
            println!(
                "  [{index:3}] d{} class={:#x} parent={:?} payload={} name={:?}",
                node.depth, node.class_id, node.parent, node.data_size, node.name
            );
        }
        for (index, node) in nodes.iter().enumerate() {
            let attributes = vex::node_attributes(&data, node);
            if !attributes.is_empty() {
                println!("  [{index:3}] {:?} attributes: {attributes:?}", node.name);
            }
        }
        return Ok(());
    }
    println!("{stem}.vex: in none of the archives");
    Ok(())
}
