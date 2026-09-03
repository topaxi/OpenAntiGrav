//! One-off: does the block `renderer.md`'s "Ships have no Lambert diffuse
//! either" actually reads by hand - `detonator_ship_rich_iridescent
//! .rcsmaterial`'s fragment block #2, file offset `0x3a00` - match one of the
//! two instruction shapes `hd_specular_unresolved_trace.rs`'s `40`-bucket run
//! found, and does `specular_exponent_dp3()` pick the same `DP3` the doc's
//! own `@0x24` names? Prints every variant's fragment offset in the file and
//! the full instruction stream at `0x3a00` so this can be checked rather than
//! assumed.

use oag_formats::rcsmaterial::RcsMaterial;
use oag_formats::rcsmaterial::fragment::Program;

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let spec = format!("{image}:PS3_GAME/USRDIR/DATA00.PSARC");
    let mut handle = oag_assets::psarc::Archive::open(&spec)?;
    let path = "/data/materials/ships/detonator_ship_rich_iridescent.rcsmaterial";
    let data = handle.read_path(path)?;
    let material = RcsMaterial::parse(&data)?;

    println!("{} variants", material.variants.len());
    for (i, v) in material.variants.iter().enumerate() {
        if let Some(program) = Program::parse(&data, v.fragment.offset)
            && let Some(e) = program.specular_exponent()
            && (e - 40.0).abs() < 1e-3
        {
            println!(
                "variant {i}: fragment.offset={:#x} class={:?} feature={:#x}",
                v.fragment.offset, v.class, v.feature_hash
            );
        }
    }

    for offset in [0x3a00usize, 0x57e0, 0x7340] {
        println!("\n=== decoding fragment block at file offset {offset:#x} ===");
        let Some(program) = Program::parse(&data, offset) else {
            println!("{offset:#x} does not open a block");
            continue;
        };
        let mut byte_pos = 0usize;
        for (i, insn) in program.instructions.iter().enumerate() {
            let name = insn.name().unwrap_or("???");
            println!(
                "[{i:3}] @{byte_pos:#04x} opcode={:#04x} {name}{} dst=R{}{} mask={:#06b} sat={}",
                insn.opcode,
                if insn.dst_half { " (half)" } else { "" },
                insn.dst,
                if insn.dst_half { "h" } else { "" },
                insn.mask,
                insn.saturate,
            );
            byte_pos += if insn.constant.is_some() { 32 } else { 16 };
        }
        println!("specular_exponent() = {:?}", program.specular_exponent());
        println!(
            "specular_exponent_dp3() = {:?}",
            program.specular_exponent_dp3()
        );
    }
    Ok(())
}
