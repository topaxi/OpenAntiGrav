//! Does opcode `0x3C` - `NVFX_FP_OP_OPCODE_LITEX2_NV40` per Mesa's
//! `nvfx_shader.h`, confirmed against the primary source 2026-09-03 -
//! actually appear in any fragment program on this disc? `fragment.rs`
//! decodes it as unnamed (`op3C`, folded into the same "not in nouveau's
//! table" comment as `0x3B`/`0x3D`, which is wrong only for this one: the
//! compiler that emits `LIT_EX2_NV40` targets it from `TGSI_OPCODE_LIT`,
//! Mesa's `nvfx_fragprog.c` shows) - a real specular/lighting-coefficient
//! idiom `Program::specular_exponent`'s `LG2`/`MUL`/`EX2` search would never
//! recognise, since `LIT_EX2_NV40` is one instruction, not three.

use oag_rcs::rcsmaterial::RcsMaterial;
use oag_rcs::rcsmaterial::fragment::Program;

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut materials = 0usize;
    let mut blocks = 0usize;
    let mut hits = 0usize;
    let mut hit_files: Vec<String> = Vec::new();

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
                if program.instructions.iter().any(|insn| insn.opcode == 0x3c) {
                    hits += 1;
                    if hit_files.len() < 10 {
                        hit_files.push(format!("{archive}:{path}"));
                    }
                }
            }
        }
    }

    println!("{materials} .rcsmaterial files, {blocks} fragment blocks decoded");
    println!("{hits} blocks contain opcode 0x3c");
    for f in &hit_files {
        println!("  {f}");
    }
    Ok(())
}
