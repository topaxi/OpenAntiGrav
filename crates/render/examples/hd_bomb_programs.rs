//! The lit-race fragment program each HD Bomb detonation model resolves to:
//! offset, declared parameters, mnemonic sequence and the literals the file
//! authors. See `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`, 2026-10-07.
//!
//! ```sh
//! cargo run -p oag-render --example hd_bomb_programs
//! ```

use oag_mesh::mesh;
use oag_rcs::rcsmaterial::{self, Class, Declared, Features, LIT_RACE_PASS, fragment::Program};
use oag_rcs::rcsmodel;

const SPEC: &str = "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC";

fn main() -> anyhow::Result<()> {
    let models = [
        "hd_bomb_sphere",
        "hd_bomb_sphere_white",
        "hd_bomb_sphere_bloomring",
        "hd_bomb_shockwaves",
        "hd_bomb_halo",
        "hd_missile_explosion",
        "hd_bomb",
    ];
    for name in models {
        let blob = mesh::read_blob(SPEC, &format!("/data/weapons/{name}.rcsmodel"))?;
        let model = rcsmodel::Model::parse(&blob)?;
        println!("== {name}");
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material as usize];
            let mpath = format!("/{}", material.name);
            let mblob = mesh::read_blob(SPEC, &mpath)?;
            let parsed = rcsmaterial::RcsMaterial::parse(&mblob)?;
            let key =
                Features::from_pass_word(Features::chunk_word(LIT_RACE_PASS, mesh.decl.as_ref()));
            let Some(variant) = Class::ALL.into_iter().find_map(|c| parsed.variant(c, key)) else {
                println!("  {} : no variant", material.name);
                continue;
            };
            let at = variant.fragment.offset;
            println!(
                "  state {:#x} alpha_func {:#06x} alpha_ref {}",
                material.state, material.alpha_func, material.alpha_ref
            );
            println!("  vertex @{:#x}", variant.vertex.offset);
            let program =
                Program::parse(&mblob, at).ok_or_else(|| anyhow::anyhow!("no program"))?;
            let declared = Declared::parse(&mblob, at).ok_or_else(|| anyhow::anyhow!("no sho"))?;
            let names: Vec<&str> = program
                .instructions
                .iter()
                .map(|i| i.name().unwrap_or("?"))
                .collect();
            println!(
                "  {} @{at:#x} params {:08x?}",
                material.name, declared.parameters
            );
            println!("  mnemonics {names:?}");
            let consts: Vec<_> = program
                .instructions
                .iter()
                .filter_map(|i| i.const_slot.zip(i.constant))
                .collect();
            println!("  constants (slot, value) {consts:?}");
            println!(
                "  rim_glow_bit {:#x}",
                mesh::rcs::rim_glow_bit(&declared, &program, None)
            );
            for p in &material.parameters {
                println!("  model param {:#010x} = {:?}", p.hash, p.value);
            }
        }
    }
    Ok(())
}
