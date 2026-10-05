//! Which variant row this project resolves for Amphiseum's four ceiling
//! materials, and where that variant's fragment program lives in the file -
//! the input `scripts/ps3-microcode.py fp-file` needs to dump the exact
//! block this renderer draws, rather than block 0 or every block.
//!
//! `lane-hd-ceiling`'s own brief: "a per-material microcode sweep of the
//! four ceiling materials for which named engine parameters (if any) their
//! own fragment programs actually declare" (`docs/ghidra/functions/
//! ps3-hdfury-eu/renderer.md`, "The rows map to real materials..."). The
//! four are named there: `animhexlights`, `cf_diff_spec`, `lambert`
//! (non-lightmap slot), `base_diffusespecular` (non-lightmap slot). Extended
//! 2026-09-18 to the two `uvanim_*emissive*` materials
//! (`lane-hd-ambient-light`'s Next Steps: "Read `uvanim_diffuse_emissive`'s
//! own colour") - same idiom, same reason, a fifth/sixth material sharing
//! the ceiling materials' code-path defect that needed its own slot lookup
//! to feed `hd_amphiseum_ceiling_vertex_light.rs`.
//!
//! `mesh::rcs::skin::variants` already resolves one variant per material
//! slot from the real model - this is a dump of that public field
//! (`Model::material_variants`), the same idiom `hd_material_probe_dump.rs`
//! uses for roles, not a second reading of the variant-selection logic.

use oag_mesh::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/amphiseum/track.vex".into());

    let data = mesh::read_blob(&spec, &name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("{name}: no sibling .rcsmodel"))?;
    let source = oag_rcs::rcsmodel::Model::parse(&geometry)?;
    let (model, _) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("{name}: not a PS3 model"))?;

    println!(
        "slot\tmaterial\tclass\tclass_hash\tfeature_hash\tvertex_offset\tfragment_offset\tfragment_len\tfragment_hash"
    );
    for (slot, material) in source.materials.iter().enumerate() {
        let is_ceiling = material.name.ends_with("animhexlights.rcsmaterial")
            || material.name.ends_with("cf_diff_spec.rcsmaterial")
            || material.name.ends_with("lambert.rcsmaterial")
            || material.name.ends_with("base_diffusespecular.rcsmaterial")
            || material
                .name
                .ends_with("uvanim_diffuse_emissive.rcsmaterial")
            || material.name.ends_with("cf_uvanim_emssive.rcsmaterial");
        if !is_ceiling {
            continue;
        }
        let variant = model.material_variants.get(slot).copied().flatten();
        match variant {
            Some(v) => println!(
                "{slot}\t{}\t{:?}\t{:#010x}\t{:#010x}\t{:#x}\t{:#x}\t{}\t{:#010x}",
                material.name,
                v.class,
                v.class_hash,
                v.feature_hash,
                v.vertex.offset,
                v.fragment.offset,
                v.fragment.len,
                v.fragment.program_hash,
            ),
            None => println!("{slot}\t{}\t<unresolved>", material.name),
        }
    }
    Ok(())
}
