//! Scratch probe: the vertices a `/data/ribboneffects/*_triangle` template
//! authors - the cross-section the engine extrudes along the position history.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_ribbon_probe \
//!     /data/ribboneffects/enginetrail_bluered_triangle
//! ```
use oag_mesh::mesh;
use oag_rcs::rcsmodel;

const ISO: &str = "data/images/hdfury-ps3-eu-dec.iso";
const ARCHIVES: [&str; 7] = [
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

fn main() -> anyhow::Result<()> {
    let stem = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/data/ribboneffects/enginetrail_bluered_triangle".into());
    for archive in ARCHIVES {
        let spec = format!("{ISO}:PS3_GAME/USRDIR/{archive}.PSARC");
        let Ok(geometry) = mesh::read_blob(&spec, &format!("{stem}.rcsmodel")) else {
            continue;
        };
        println!("== {archive} {stem}.rcsmodel: {} bytes", geometry.len());
        let model = rcsmodel::Model::parse(&geometry).map_err(|e| anyhow::anyhow!("{e}"))?;
        let vex = mesh::read_blob(&spec, &format!("{stem}.vex"))?;
        let (built, report) = mesh::rcs::build(
            &format!("{stem}.vex"),
            &vex,
            &geometry,
            &mut |path| {
                ARCHIVES.iter().find_map(|a| {
                    mesh::read_blob(&format!("{ISO}:PS3_GAME/USRDIR/{a}.PSARC"), path).ok()
                })
            },
            |classes| classes.mesh,
        )?;
        println!("  {}", report.describe());
        for (index, material) in model.materials.iter().enumerate() {
            println!(
                "  material[{index}] {} state={:#x}",
                material.name, material.state
            );
        }
        println!(
            "  {} vertices, {} indices",
            built.vertices.len(),
            built.indices.len()
        );
        for (i, v) in built.vertices.iter().enumerate() {
            println!(
                "  v{i}  pos=({:9.4}, {:9.4}, {:9.4})  n=({:7.4}, {:7.4}, {:7.4})  uv=({:7.4}, {:7.4})  rgba=({:5.3}, {:5.3}, {:5.3}, {:5.3})",
                v.position[0],
                v.position[1],
                v.position[2],
                v.normal[0],
                v.normal[1],
                v.normal[2],
                v.texcoord[0],
                v.texcoord[1],
                v.colour[0],
                v.colour[1],
                v.colour[2],
                v.colour[3],
            );
        }
        println!("  indices: {:?}", built.indices);
    }
    Ok(())
}
