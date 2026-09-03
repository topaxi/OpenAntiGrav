//! **History: this file found the bug, and now verifies the fix.** It
//! originally measured `Program::dp3_feeding`'s naive "most recent write to
//! any lane of the register, any mask" gate against a lane-correct
//! reimplementation and found it lane-unsound in 1,679 of 6,141
//! then-resolved blocks (27.3%) - see `renderer.md`'s "Ships have no
//! Lambert diffuse either" for that write-up and the companion false
//! negative (5,556 blocks) `hd_specular_unresolved_reasons.rs` found the
//! same way. `dp3_feeding` itself was then fixed to be lane-aware and
//! clobber-checked (`crates/formats/src/rcsmaterial/fragment.rs`).
//!
//! What remains useful here after that fix landed: an **independent**
//! reimplementation of the lane-correct check, written from this project's
//! own description of the gate rather than sharing code with the library,
//! cross-verifying every block `specular_exponent()` now resolves. If this
//! ever finds a lane-unsound block again, either the library regressed or
//! this file's own independent copy has drifted from it - either way, worth
//! knowing.

use oag_formats::rcsmaterial::RcsMaterial;
use oag_formats::rcsmaterial::fragment::{Instruction, Program, Source};

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// An independent reimplementation of the lane-correct writer search
/// `Program::dp3_feeding` now uses internally - written from this project's
/// own description of the gate, not by calling the private method, so this
/// file cross-checks the library rather than restating it.
fn lane_correct_writer(program: &Program, at: usize, reg: (u8, bool), needed: u8) -> Option<usize> {
    let (w, _) = program.instructions[..at]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, insn)| (insn.dst, insn.dst_half) == reg && insn.mask & needed == needed)?;
    let clobbered = program.instructions[w + 1..at]
        .iter()
        .any(|insn| (insn.dst, insn.dst_half) == reg && insn.mask & needed != 0);
    (!clobbered).then_some(w)
}

fn needed_lanes(insn: &Instruction) -> u8 {
    let sw = insn.swizzles[0];
    (0..4)
        .filter(|&i| insn.mask & (1 << i) != 0)
        .fold(0u8, |acc, i| acc | (1 << sw[i]))
}

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut resolved = 0usize;
    let mut agrees = 0usize;
    let mut mismatches: Vec<String> = Vec::new();

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
                if program.specular_exponent().is_none() {
                    continue;
                }
                let Some(dp3_i) = program.specular_exponent_dp3() else {
                    continue;
                };
                resolved += 1;

                let dp3 = &program.instructions[dp3_i];
                let dp3_reg = (dp3.dst, dp3.dst_half);
                // The LG2 this dp3_i must be feeding, by the library's own
                // contract: a saturated DP3 whose index specular_exponent_dp3
                // returns. Locate it independently by scanning for the LG2
                // whose own lane-correct writer is exactly dp3_i.
                let Some(lg2_i) = (0..program.instructions.len()).rev().find(|&i| {
                    let insn = &program.instructions[i];
                    if insn.name() != Some("LG2") {
                        return false;
                    }
                    let Some(Source::Register { index, half }) = insn.operands().next() else {
                        return false;
                    };
                    if (index, half) != dp3_reg {
                        return false;
                    }
                    lane_correct_writer(&program, i, dp3_reg, needed_lanes(insn)) == Some(dp3_i)
                }) else {
                    mismatches.push(format!(
                        "{archive}:{path} - no LG2 independently re-derives dp3_i={dp3_i} as its writer"
                    ));
                    continue;
                };
                let lg2 = &program.instructions[lg2_i];
                let needed = needed_lanes(lg2);
                if dp3.mask & needed == needed {
                    agrees += 1;
                } else {
                    mismatches.push(format!(
                        "{archive}:{path} - dp3_i={dp3_i} mask={:#06b} lg2_i={lg2_i} needs={:#06b}",
                        dp3.mask, needed
                    ));
                }
            }
        }
    }

    println!("{resolved} blocks resolved by specular_exponent()");
    println!("{agrees} independently verified lane-sound");
    println!(
        "{} mismatches (would indicate a regression)",
        resolved - agrees
    );
    for (i, ex) in mismatches.iter().enumerate().take(20) {
        println!("  [{i}] {ex}");
    }
    Ok(())
}
