//! Disc-wide corroboration for naming opcodes `0x3B`/`0x3D`, behind
//! `docs/rendering/pads.md`'s "the two nouveau's own opcode table has no
//! entry for" blocker.
//!
//! `scripts/ps3-microcode.py`/`fragment.rs` left both unnamed: absent from
//! Mesa's `nvfx_shader.h` (the NV30/NV40-era header this decoder is built
//! from), and `renderer.md`'s own `op3B`-as-`NRM` hypothesis stayed below
//! this project's confidence-70 rename line because a second usage shape
//! (`op3B R0.w, R0, R0`, no preceding self-`DP3`) did not fit "normalize".
//!
//! RPCS3's own `FPOpcodes.h` (`rpcs3/Emu/RSX/Program/Assembler/FPOpcodes.h`,
//! GPLv2, independently reverse-engineered against real hardware/games) names
//! the same two opcode numbers: `RSX_FP_OPCODE_DIVSQ = 0x3B // Divide by
//! Square Root` and `RSX_FP_OPCODE_FENCT = 0x3D // Fence T?`. This checks
//! both against every fragment program the disc ships:
//!
//! - **DIVSQ** (`a / sqrt(b)`) unifies both `op3B` usage shapes at once:
//!   `DP3(v,v)->d; op3B(v,d)` is `v * rsqrt(d)` (normalize), and `op3B(x,x)`
//!   is `x / sqrt(x) = sqrt(x)` - the standard single-instruction square
//!   root idiom on hardware with no native `SQRT`, which is exactly the
//!   "same register twice" shape `renderer.md` could not reconcile with a
//!   dedicated `NRM`.
//! - **FENCT**: every real use should write destination register 63 (the
//!   6-bit field's all-ones value, `NV40_FP_OP_OUT_NONE`'s literal encoding
//!   per Mesa's `nvfx_shader.h`) - i.e. no real destination - regardless of
//!   its (irrelevant) source operands.

use oag_rcs::rcsmaterial::RcsMaterial;
use oag_rcs::rcsmaterial::fragment::{Program, Source};

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut materials = 0usize;
    let mut blocks = 0usize;

    let mut op3b_total = 0usize;
    let mut op3b_same_register = 0usize;
    let mut op3b_different_register = 0usize;
    let mut op3b_other_shape = 0usize;

    let mut op3d_total = 0usize;
    let mut op3d_dst_is_63 = 0usize;
    let mut op3d_counterexamples: Vec<String> = Vec::new();

    for archive in ARCHIVES {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
        let Ok(mut handle) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = handle
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmaterial"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(data) = handle.read_path(&path) else {
                continue;
            };
            let Ok(material) = RcsMaterial::parse(&data) else {
                continue;
            };
            materials += 1;
            for variant in &material.variants {
                let Some(program) = Program::parse(&data, variant.fragment.offset) else {
                    continue;
                };
                blocks += 1;
                for insn in &program.instructions {
                    if insn.opcode == 0x3b {
                        op3b_total += 1;
                        match (insn.sources[0], insn.sources[1]) {
                            (
                                Source::Register {
                                    index: i0,
                                    half: h0,
                                },
                                Source::Register {
                                    index: i1,
                                    half: h1,
                                },
                            ) if i0 == i1 && h0 == h1 => op3b_same_register += 1,
                            (Source::Register { .. }, Source::Register { .. }) => {
                                op3b_different_register += 1;
                            }
                            _ => op3b_other_shape += 1,
                        }
                    }
                    if insn.opcode == 0x3d {
                        op3d_total += 1;
                        if insn.dst == 63 {
                            op3d_dst_is_63 += 1;
                        } else if op3d_counterexamples.len() < 10 {
                            op3d_counterexamples.push(format!(
                                "{archive}:{path} dst={} dst_half={} sources={:?}",
                                insn.dst, insn.dst_half, insn.sources
                            ));
                        }
                    }
                }
            }
        }
    }

    println!("{materials} .rcsmaterial files, {blocks} fragment blocks decoded");
    println!();
    println!(
        "op3B (0x3b): {op3b_total} total - {op3b_same_register} same-register operands \
         (the `x / sqrt(x) = sqrt(x)` shape), {op3b_different_register} different-register \
         operands (the `v * rsqrt(dot(v,v))` normalize shape), {op3b_other_shape} neither \
         (constant/input operand or similar)"
    );
    println!();
    println!(
        "op3D (0x3d): {op3d_total} total - {op3d_dst_is_63} write destination register 63 \
         (no real destination)"
    );
    if op3d_counterexamples.is_empty() {
        println!("  no counterexamples: every single use writes register 63");
    } else {
        println!("  counterexamples (destination register is not 63):");
        for c in &op3d_counterexamples {
            println!("    {c}");
        }
    }

    Ok(())
}
