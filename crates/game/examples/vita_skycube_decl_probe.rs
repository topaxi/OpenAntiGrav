//! Scratch probe: does the sky dome's stride-20 declaration carry `Uv1`
//! (diffuse UV) or `VertexColour1` (`docs/formats/2048-rcsmodel.md` says
//! stride 20 keeps position/normal/VertexColour1 on the corpus it measured -
//! this checks whether the sky file is the same shape or a different one).

use oag_rcs::rcsmodel::psp2::vertex_decl;

const PACKAGE: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";

fn main() -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(PACKAGE)?;
    for env in ["altima", "arena", "tower"] {
        let path = format!("data/art/published/environments/{env}/skycube.rcsmodel");
        let blob = archive.read_path(&path)?;
        // Section 0 is the CPU section; header_len (0x0c) then descriptors.
        let header_len = u32::from_le_bytes(blob[0x0c..0x10].try_into().unwrap()) as usize;
        let cpu_len = u32::from_le_bytes(blob[0x24..0x28].try_into().unwrap()) as usize;
        let cpu = &blob[header_len..header_len + cpu_len];
        let decls = vertex_decl::find_by_stride(cpu);
        println!("=== {env}: {} declaration(s) ===", decls.len());
        for (stride, decl) in &decls {
            println!("  stride {stride}: restated stride {}", decl.stride);
            for a in &decl.attributes {
                println!(
                    "    name_hash={:#010x} components={} gxm_type={} offset={}",
                    a.name_hash, a.components, a.gxm_type, a.offset
                );
            }
        }
        println!(
            "  position={:#x} normal={:#x} tangent={:#x} uv1={:#x} lightmap={:#x}",
            vertex_decl::POSITION_HASH,
            vertex_decl::NORMAL_HASH,
            vertex_decl::TANGENT_HASH,
            vertex_decl::UV1_HASH,
            vertex_decl::LIGHTMAP_HASH,
        );
    }
    Ok(())
}
