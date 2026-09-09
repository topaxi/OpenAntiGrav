//! Why do 426 of Talon's Junction's 442 materials (96%,
//! `Report::specular_exponent_unresolved`) have no `specular_exponent()`
//! chain? Categorises every resolved fragment block disc-wide into:
//!
//! - `NoLg2`: no `LG2` instruction in the block at all - genuinely no
//!   `pow`/`exp` curve, the expected shape for an unlit or emissive surface.
//! - `Lg2NotDp3Fed`: an `LG2` exists, but the register it reads was not last
//!   fully written by a saturated, literally-named `DP3` - the exact
//!   `0x3b`-fed shape `renderer.md`'s "Ships have no Lambert diffuse
//!   either" found on the ship's own confirmed block. A candidate false
//!   negative population, not a confirmed one.
//! - `Lg2Dp3FedButNoChain`: the `DP3` gate passes but `specular_exponent()`
//!   still returns `None` - the `MUL`/`EX2` half of the chain does not
//!   follow, or the exponent is `log2(e)` (an `exp()` curve, not `pow()`),
//!   or the result never reaches the output colour.
//! - `Resolved`: `specular_exponent()` succeeds.
//!
//! **Lane-aware, not the naive "any nonzero mask" check `Program::
//! dp3_feeding` itself uses.** A first draft of this file copied that naive
//! check verbatim and got a badly wrong picture: `LG2` reads only the lane(s)
//! its own swizzle names for the output lanes its mask selects - always one
//! lane in practice, checked disc-wide (see [`read_mask`]'s own doc
//! comment) - so a "most recent
//! writer of *any* lane of the register" match both credits a `DP3` that
//! wrote a lane `LG2` never reads (a false positive) and, just as often,
//! misses a real `DP3` behind an unrelated write to some *other* lane of the
//! same register (a false negative). `crates/render/examples/
//! hd_dp3_feeding_lane_check.rs` measured the false-positive direction
//! against the shipped gate: of the 6,141 blocks `specular_exponent()`
//! currently resolves, **1,679 (27.3%) are lane-unsound**. This file's own
//! re-tally (below) measures the false-negative direction: `5,556` blocks
//! move from `Lg2NotDp3Fed` to `Lg2Dp3FedButNoChain` once the check is
//! lane-correct - more than three times the false-positive count. Both are
//! findings about `Program::dp3_feeding` itself, filed separately (see the
//! thread file's `## Open`); this diagnostic does not reimplement the
//! shipped gate's bug, it uses the lane-correct check `dp3_feeding` should
//! have.
//!
//! This mirrors what a lane-aware `dp3_feeding` would answer, not
//! `specular_exponent_chain`'s full search - the `Lg2NotDp3Fed`/
//! `Lg2Dp3FedButNoChain` split checks **every** `LG2` in the block
//! (`Lg2NotDp3Fed` means none of them are lane-soundly `DP3`-`SAT`-fed, not
//! just the last one), so the category counts are exact under this gate;
//! only the printed *feeder* (what wrote the register) looks at the block's
//! last `LG2` alone, and a block with several differently-fed `LG2`s reports
//! only that one - good enough to size the feeder population, not to claim
//! it is exhaustive per block.
//!
//! **Measured disc-wide, 76,358 blocks, lane-correct**: `Lg2NotDp3Fed`
//! 56.0%, `NoLg2` 27.6%, `Lg2Dp3FedButNoChain` 8.3%, `Resolved` 8.0%. Within
//! `Lg2NotDp3Fed` (42,773 blocks): `ADD`/`ADD_SAT` 53.4%, `op3B`/`op3B_SAT`
//! combined 23.1%, `TEX` 11.4%, `MOV` 5.9%, unsaturated `DP3` 1.2%, the rest
//! single digits or below. See `renderer.md`'s "Ships have no Lambert
//! diffuse either" for the full write-up and what changed from the earlier,
//! naive-gate numbers (`63.3`/`27.6`/`8.0`/`1.1`, `op3B` `~13.6%`).

use oag_rcs::rcsmaterial::RcsMaterial;
use oag_rcs::rcsmaterial::fragment::{Instruction, Program, Source};

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    NoLg2,
    Lg2NotDp3Fed,
    Lg2Dp3FedButNoChain,
    Resolved,
}

/// The lanes `insn` reads from `reg` through its first operand, as a mask -
/// `None` if that operand is not `reg`. Computed as the union of the source
/// lane each masked *output* lane's own swizzle entry names - the general
/// elementwise reading, and the only one that matches every masked-write
/// instruction seen elsewhere in this microcode. This function is only ever
/// called with an `LG2` as `insn` in this file, so it does not need `DP3`'s
/// three-lane special case.
///
/// **Whether `LG2`'s hardware unit is genuinely elementwise, or a scalar
/// unit that always reads one lane and broadcasts, turns out to be moot in
/// practice, checked disc-wide rather than left open.** Mesa's
/// `nvfx_shader.h` documents a real scalar/vector split for the *vertex*
/// program (a separate `NVFX_VP_INST_SLOT_SCA` opcode table dual-issued
/// against `VEC`) but no analogous split for fragment-program opcodes, which
/// share one flat table - primary source is inconclusive there. What settles
/// it instead: `hd_lg2_swizzle_check.rs` found **`LG2` never writes more
/// than one output lane on this disc - 0 of 120,082 occurrences**. The two
/// readings can only disagree when a multi-lane write reads a differing
/// source lane per output lane, and that shape simply does not occur, so
/// this function's answer is the same either way for every `LG2` this file
/// has ever seen.
fn read_mask(insn: &Instruction, reg: (u8, bool)) -> Option<u8> {
    let (_, sw) = insn
        .operands()
        .zip(insn.swizzles)
        .find(|(s, _)| matches!(s, Source::Register { index, half } if (*index, *half) == reg))?;
    Some(
        (0..4)
            .filter(|&i| insn.mask & (1 << i) != 0)
            .fold(0u8, |acc, i| acc | (1 << sw[i])),
    )
}

/// The single instruction before `at` that fully covers the lanes `needed`
/// selects - `None` when no writer covers all of them in one instruction, or
/// when a later partial write to any of those lanes lands between it and
/// `at` (the value `at` reads is then a mix, not that one writer's output).
/// Lifted from `hd_specular_unresolved_trace.rs`'s `last_writer`, generalised
/// to a mask rather than a 4-lane array since the caller here already knows
/// its reader is `LG2`, never `DP3`.
fn last_writer(program: &Program, at: usize, reg: (u8, bool), needed: u8) -> Option<usize> {
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

/// The lane-sound writer feeding the `LG2` at `at`, if any - `None` if its
/// operand is not a register, or no single instruction lane-soundly covers
/// what it reads.
fn feeding(program: &Program, at: usize) -> Option<usize> {
    let Some(Source::Register { index, half }) = program.instructions[at].operands().next() else {
        return None;
    };
    let needed = read_mask(&program.instructions[at], (index, half))?;
    last_writer(program, at, (index, half), needed)
}

fn dp3_feeds(program: &Program, at: usize) -> bool {
    feeding(program, at).is_some_and(|w| {
        let insn = &program.instructions[w];
        insn.name() == Some("DP3") && insn.saturate
    })
}

fn classify(program: &Program) -> Category {
    if program.specular_exponent().is_some() {
        return Category::Resolved;
    }
    let lg2s: Vec<usize> = program
        .instructions
        .iter()
        .enumerate()
        .filter(|(_, insn)| insn.name() == Some("LG2"))
        .map(|(i, _)| i)
        .collect();
    if lg2s.is_empty() {
        return Category::NoLg2;
    }
    if lg2s.iter().any(|&i| dp3_feeds(program, i)) {
        Category::Lg2Dp3FedButNoChain
    } else {
        Category::Lg2NotDp3Fed
    }
}

/// What actually, lane-soundly wrote the register the last `LG2` in the
/// block reads - `"???"` for an unnamed opcode (with its raw hex), `"no
/// writer"` if no lane-sound writer exists at all (an interpolator/constant
/// read, lanes assembled piecemeal so no single writer covers them, or a
/// writer whose own destination mask misses the lane `LG2` actually reads).
fn feeder(program: &Program) -> String {
    let Some(&lg2_i) = program
        .instructions
        .iter()
        .enumerate()
        .filter(|(_, insn)| insn.name() == Some("LG2"))
        .map(|(i, _)| i)
        .collect::<Vec<_>>()
        .last()
    else {
        return "no LG2".to_string();
    };
    match feeding(program, lg2_i) {
        Some(w) => {
            let insn = &program.instructions[w];
            match insn.name() {
                Some(n) => format!("{n}{}", if insn.saturate { "_SAT" } else { "" }),
                None => format!(
                    "op{:02X}{}",
                    insn.opcode,
                    if insn.saturate { "_SAT" } else { "" }
                ),
            }
        }
        None => "no writer".to_string(),
    }
}

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut materials = 0usize;
    let mut blocks = 0usize;
    let mut tally: Vec<(Category, u32)> = Vec::new();
    let mut feeders: Vec<(String, u32)> = Vec::new();

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
                let c = classify(&program);
                match tally.iter_mut().find(|(cat, _)| *cat == c) {
                    Some((_, n)) => *n += 1,
                    None => tally.push((c, 1)),
                }
                if c == Category::Lg2NotDp3Fed {
                    let f = feeder(&program);
                    match feeders.iter_mut().find(|(name, _)| *name == f) {
                        Some((_, n)) => *n += 1,
                        None => feeders.push((f, 1)),
                    }
                }
            }
        }
    }

    println!("{materials} .rcsmaterial files, {blocks} fragment blocks");
    tally.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for (c, n) in &tally {
        println!(
            "  {c:?}: {n} ({:.1}%)",
            100.0 * f64::from(*n) / blocks as f64
        );
    }
    let lg2_not_dp3_fed: u32 = feeders.iter().map(|(_, n)| n).sum();
    println!("--- what feeds the LG2 in Lg2NotDp3Fed ({lg2_not_dp3_fed} blocks) ---");
    feeders.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for (f, n) in &feeders {
        println!(
            "  {f}: {n} ({:.1}%)",
            100.0 * f64::from(*n) / f64::from(lg2_not_dp3_fed)
        );
    }
    Ok(())
}
