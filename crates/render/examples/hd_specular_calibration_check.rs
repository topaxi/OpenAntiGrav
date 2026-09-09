//! One-off, and the answer turned out to be yes, for a reason worth keeping
//! precise: is the block `renderer.md`'s "Ships have no Lambert diffuse
//! either" reads by hand - `detonator_ship_rich_iridescent.rcsmaterial`,
//! file offset `0x3a00` - the same block `hd_specular_unresolved_trace.rs`'s
//! `40`-bucket sweep classifies?
//!
//! **It is not, and the reason is a real gate miss, not a wrong address.**
//! `0x3a00` decodes to a real `SHO` block whose instructions match the doc's
//! own `@0x13`/`@0x17`/`@0x40`/`@0x43`/`@0x46` citations exactly, once those
//! are read as `byte_position / 16` (a paragraph count, confirmed against
//! six independent details: address, destination lane, saturation, the
//! constant operand, and the literal `40.0`) - but `specular_exponent()`
//! returns `None` on it. Its `LG2` is fed by an unnamed `0x3b` instruction,
//! not a literally-named `DP3`, and `Program::dp3_feeding` requires the
//! latter. So this block never reaches `hd_specular_unresolved_trace.rs`'s
//! `40.0` bucket at all - the eight occurrences that sweep does find are a
//! disjoint population from the one hand-confirmed case, which is why that
//! tool's own attempted calibration against "a `40.0` block in this file"
//! was never actually checking the documented one.
//!
//! Prints every variant's fragment offset in this file (unfiltered), the
//! fragment/vertex discriminator word at four offsets, and the full
//! instruction stream - operands, masks, constants, byte and paragraph
//! position - at `0x3a00` and at the two blocks that do resolve to `40.0`,
//! so this can be checked rather than trusted.

use oag_rcs::rcsmaterial::RcsMaterial;
use oag_rcs::rcsmaterial::fragment::{Program, Source};

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let spec = format!("{image}:PS3_GAME/USRDIR/DATA00.PSARC");
    let mut handle = oag_assets::psarc::Archive::open(&spec)?;
    let path = "/data/materials/ships/detonator_ship_rich_iridescent.rcsmaterial";
    let data = handle.read_path(path)?;
    let material = RcsMaterial::parse(&data)?;

    println!("{} variants, unfiltered:", material.variants.len());
    for (i, v) in material.variants.iter().enumerate() {
        let exponent = Program::parse(&data, v.fragment.offset).and_then(|p| p.specular_exponent());
        println!(
            "variant {i}: vertex={:#x} fragment={:#x} class={:?} feature={:#x} exponent={exponent:?}",
            v.vertex.offset, v.fragment.offset, v.class, v.feature_hash
        );
    }

    // The "1 for a fragment program" word `Declared`'s own doc comment names
    // but never checks - is 0x3a00 actually a fragment block at all?
    for offset in [0x3370usize, 0x3a00, 0x57e0, 0x7340] {
        let word = u32::from_be_bytes(data[offset + 4..offset + 8].try_into()?);
        println!("word at {offset:#x}+4 (fragment discriminator) = {word}");
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
            let c = insn
                .constant
                .map(|c| format!(" const={c:?}"))
                .unwrap_or_default();
            let ops: Vec<String> = insn
                .operands()
                .map(|s| match s {
                    Source::Register { index, half } => {
                        format!("{}{index}", if half { "H" } else { "R" })
                    }
                    Source::Input => "f[]".to_string(),
                    Source::Constant => "{const}".to_string(),
                    Source::Unknown => "?".to_string(),
                })
                .collect();
            println!(
                "[{i:3}] @{byte_pos:#04x} paragraph={:#04x} opcode={:#04x} {name}{} dst=R{}{} mask={:#06b} sat={}{c} ops={ops:?}",
                byte_pos / 16,
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
