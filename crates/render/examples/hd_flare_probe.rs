//! Scratch probe: what a Wipeout HD craft's `engineflare` pair authors - its
//! material and blend, its chunks, and the per-node span of the colour set's
//! fourth byte, which `flame_test.rcsmaterial`'s own program reads as the
//! flame's opacity ramp.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_flare_probe /data/ships/feisar/engineflare
//! ```
//!
//! The evidence reproducer for
//! `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`; `hd_flare_tree`
//! beside it prints the node tree the groups come from.
use oag_formats::rcsmodel;
use oag_render::mesh;

const ISO: &str = "data/images/hdfury-ps3-eu-dec.iso";
const ARCHIVES: [&str; 7] = [
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

fn main() -> anyhow::Result<()> {
    let stem = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/data/ships/feisar/engineflare".into());
    for archive in ARCHIVES {
        let spec = format!("{ISO}:PS3_GAME/USRDIR/{archive}.PSARC");
        let Ok(geometry) = mesh::read_blob(&spec, &format!("{stem}.rcsmodel")) else {
            continue;
        };
        println!("== {archive} {stem}.rcsmodel: {} bytes", geometry.len());
        let model = rcsmodel::Model::parse(&geometry).map_err(|e| anyhow::anyhow!("{e}"))?;
        println!(
            "  {} material(s), {} mesh(es)",
            model.materials.len(),
            model.meshes.len()
        );
        for (index, material) in model.materials.iter().enumerate() {
            println!(
                "  [{index}] {} state={:#x} blend={:?}",
                material.name,
                material.state,
                material.blend()
            );
            println!("       tex0={}", material.texture);
            println!("       tex1={:?}", material.second_texture);
        }
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
        println!(
            "  {} triangle(s), centre {:?}, radius {:.3}",
            built.indices.len() / 3,
            built.centre,
            built.radius
        );
        println!(
            "  textures: {:?}",
            built
                .textures
                .iter()
                .map(|t| t.as_ref().map(|t| (t.label.as_str(), t.width, t.height)))
                .collect::<Vec<_>>()
        );
        // Per node, because the ramp is what says which end of the flame is
        // opaque - and the two groups have to be looked at separately.
        for draw in built
            .draws
            .iter()
            .chain(&built.alpha_tested_draws)
            .chain(&built.transparent_draws)
        {
            let range = draw.range.start as usize..draw.range.end as usize;
            let (mut low, mut high) = (f32::INFINITY, f32::NEG_INFINITY);
            // The `Uv1` span decides whether the program's doubled lookup
            // (`2u`, `2v + noise`) covers the texture once or wraps it twice -
            // asset-side evidence for a shader-side reading. See
            // engine-flare.md, "the surface scroll".
            let (mut u0, mut u1) = (f32::INFINITY, f32::NEG_INFINITY);
            let (mut v0, mut v1) = (f32::INFINITY, f32::NEG_INFINITY);
            for &index in &built.indices[range] {
                let vertex = &built.vertices[index as usize];
                low = low.min(vertex.sun_mask);
                high = high.max(vertex.sun_mask);
                u0 = u0.min(vertex.texcoord[0]);
                u1 = u1.max(vertex.texcoord[0]);
                v0 = v0.min(vertex.texcoord[1]);
                v1 = v1.max(vertex.texcoord[1]);
            }
            println!(
                "  node {:?}: {} triangle(s), VertexColour1.w spans {low:.3} to {high:.3}, \
                 Uv1.x {u0:.3}..{u1:.3}, Uv1.y {v0:.3}..{v1:.3}",
                draw.node,
                (draw.range.end - draw.range.start) / 3
            );
        }
        return Ok(());
    }
    println!("{stem}.rcsmodel: in none of the archives");
    Ok(())
}
