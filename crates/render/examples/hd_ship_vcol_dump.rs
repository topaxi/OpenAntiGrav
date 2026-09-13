//! Scratch: for a ship's `.vex`, dump every mesh's vertex declaration
//! attributes, and which one `VertexDecl::vertex_colour()` matches - to check
//! whether it is really the same `colorSet1`/`0x1aaf7631` colour set the
//! doc comment measures over world/track chunks, or a different attribute
//! that happens to share its shape (4 components, `RSX_UBYTE_NORM`).
use oag_rcs::rcsmodel;
use oag_render::mesh;

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/ships/feisar/ship.vex".into());

    let Some((spec, data)) = ARCHIVES.iter().find_map(|archive| {
        let spec = format!("{image}:{archive}");
        mesh::read_blob(&spec, &name).ok().map(|d| (spec, d))
    }) else {
        anyhow::bail!("{name}: not found");
    };
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("no sibling .rcsmodel"))?;
    let model = rcsmodel::Model::parse(&geometry).map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut seen: std::collections::HashSet<u32> = Default::default();
    for mesh in &model.meshes {
        let Some(decl) = mesh.decl.as_ref() else {
            continue;
        };
        if !seen.insert(mesh.material) {
            continue;
        }
        let mat = &model.materials[mesh.material as usize];
        println!("material [{}] {}", mesh.material, mat.name);
        for a in &decl.attributes {
            let marker = if decl.vertex_colour().map(|vc| vc.name_hash) == Some(a.name_hash) {
                " <- vertex_colour() match"
            } else {
                ""
            };
            println!(
                "    hash {:#010x} name {:?} components {} rsx_type {}{}",
                a.name_hash,
                a.name(),
                a.components,
                a.rsx_type,
                marker,
            );
        }
    }
    Ok(())
}
