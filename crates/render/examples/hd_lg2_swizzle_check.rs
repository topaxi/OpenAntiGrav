//! Does `LG2` ever write more than one output lane with a *differing*
//! swizzle across those specific written lanes? That is the only shape
//! where the scalar-broadcast reading and the general elementwise reading
//! of what `LG2` reads (see `hd_specular_unresolved_reasons.rs`'s
//! `read_mask` doc comment) actually disagree - if `LG2`'s own mask selects
//! only one output lane, as it almost always does feeding a scalar constant
//! `MUL`, the other three swizzle slots are never consulted by either
//! reading and the distinction is moot for that instruction regardless of
//! what values sit in the unused slots.
//!
//! Mesa's `nvfx_shader.h` (fetched directly, not through a summarising
//! fetch) documents a genuine scalar/vector split for the *vertex* program:
//! a separate `NVFX_VP_INST_SLOT_SCA` opcode table (`RCP`/`RSQ`/`LG2`/
//! `EX2`/`LIT`, dual-issued against a `VEC` slot). Fragment-program opcodes
//! share one flat `NVFX_FP_OP_OPCODE_*` table with no analogous split
//! documented, though. Primary source is inconclusive for the fragment
//! case, so this checks the disc's own shipped microcode directly instead.

use oag_rcs::rcsmaterial::RcsMaterial;
use oag_rcs::rcsmaterial::fragment::Program;

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut total = 0usize;
    let mut multi_lane = 0usize;
    let mut multi_lane_diverging = 0usize;
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
            for variant in &material.variants {
                let Some(program) = Program::parse(&data, variant.fragment.offset) else {
                    continue;
                };
                for insn in &program.instructions {
                    if insn.name() != Some("LG2") {
                        continue;
                    }
                    total += 1;
                    let sw = insn.swizzles[0];
                    let written: Vec<u8> = (0..4)
                        .filter(|&i| insn.mask & (1 << i) != 0)
                        .map(|i| sw[i])
                        .collect();
                    if written.len() > 1 {
                        multi_lane += 1;
                        if written.iter().any(|&l| l != written[0]) {
                            multi_lane_diverging += 1;
                        }
                    }
                }
            }
        }
    }
    println!("{total} LG2 instructions");
    println!("{multi_lane} write more than one output lane");
    println!(
        "{multi_lane_diverging} of those read a differing source lane across their own written lanes"
    );
    Ok(())
}
