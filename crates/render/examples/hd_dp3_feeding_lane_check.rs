//! Does `Program::dp3_feeding`'s naive "most recent write to any lane of the
//! register, any mask" gate ever credit a `DP3` that did not actually write
//! the lane(s) the `LG2` reads? **Yes: measured disc-wide, 1,679 of the
//! 6,141 blocks `specular_exponent()` currently resolves (27.3%) are
//! lane-unsound** - the gate's answer for those blocks may not be the
//! exponent the `LG2`/`MUL`/`EX2` chain actually consumes. See
//! `renderer.md`'s "Ships have no Lambert diffuse either" for the write-up;
//! this is a confirmed correctness gap in shipped detection, filed as an
//! `## Open` item rather than fixed here - a gate change moves every number
//! measured against the current `6,141` and needs its own verification pass.
//!
//! `dp3_feeding`'s own search already guarantees there is no clobber gap to
//! find (it takes the most recent writer of *any* lane, by construction), so
//! the only possible failure is lane **coverage**: the `DP3` it found wrote
//! some lanes of the register but not the specific lane(s) the `LG2` reads.
//! That reduces to a mask test, not a full `last_writer` re-walk - and the
//! LG2 has to be found the same way `specular_exponent_chain` finds it
//! (reverse scan, first `LG2` whose own naive-gate writer is `dp3_i`), not by
//! a forward scan from `dp3_i`, since a block can have several `LG2`s on the
//! same register - the first draft of this file did exactly that forward
//! scan and it silently paired a `DP3` with the wrong `LG2` on any block
//! with more than one.

use oag_formats::rcsmaterial::RcsMaterial;
use oag_formats::rcsmaterial::fragment::{Program, Source};

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// Reproduces `Program::dp3_feeding`'s own naive gate exactly (private in
/// the library), so this check finds the *same* LG2/DP3 pairing
/// `specular_exponent_chain` used, not a plausible-looking different one.
fn naive_dp3_feeding(program: &Program, at: usize) -> Option<usize> {
    let Some(Source::Register { index, half }) = program.instructions[at].operands().next() else {
        return None;
    };
    let (i, insn) = program.instructions[..at]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, insn)| insn.mask != 0 && (insn.dst, insn.dst_half) == (index, half))?;
    (insn.name() == Some("DP3") && insn.saturate).then_some(i)
}

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut resolved = 0usize;
    let mut lane_sound = 0usize;
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

                // The same reverse search `specular_exponent_chain` performs:
                // the last LG2 in the block whose naive-gate writer is dp3_i.
                let Some(lg2_i) = (0..program.instructions.len()).rev().find(|&i| {
                    program.instructions[i].name() == Some("LG2")
                        && naive_dp3_feeding(&program, i) == Some(dp3_i)
                }) else {
                    mismatches.push(format!(
                        "{archive}:{path} - no LG2 reverse-matches dp3_i={dp3_i} at all (unexpected)"
                    ));
                    continue;
                };
                let lg2 = &program.instructions[lg2_i];
                let dp3 = &program.instructions[dp3_i];
                let sw = lg2.swizzles[0];
                // Elementwise reading (see `hd_specular_unresolved_reasons.rs`'s
                // `read_mask` doc comment): whether LG2 is actually a
                // scalar-broadcast unit is unsettled, and 27.0% of LG2s
                // disc-wide have a non-uniform swizzle where the two
                // readings disagree - this may over-demand coverage on that
                // slice and inflate the lane-unsound count somewhat.
                let needed: u8 = (0..4)
                    .filter(|&i| lg2.mask & (1 << i) != 0)
                    .fold(0u8, |acc, i| acc | (1 << sw[i]));
                if dp3.mask & needed == needed {
                    lane_sound += 1;
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
    println!("{lane_sound} lane-sound (DP3 actually wrote the lane(s) the LG2 reads)");
    println!(
        "{} lane-unsound (false positives in the shipped gate)",
        resolved - lane_sound
    );
    for (i, ex) in mismatches.iter().enumerate().take(20) {
        println!("  [{i}] {ex}");
    }
    println!("{} total mismatch entries", mismatches.len());
    Ok(())
}
