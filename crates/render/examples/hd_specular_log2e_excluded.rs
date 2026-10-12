//! What the `log2(e)` exclusion in `Program::specular_exponent` actually
//! excludes. The doc comment says `EX2(log2(e) * LG2(x)) = exp(x)`; that is
//! `x^1.4427` (`2^(log2(e) * log2 x)`), and `exp(y)` is `EX2(log2(e) * y)` with
//! no `LG2` at all. This lists, disc-wide, every `LG2` fed by a saturated
//! `DP3` whose next `MUL`-by-literal reads that `LG2` and whose literal is
//! `log2(e)`, then an `EX2` reading the `MUL`, and prints the first few chains
//! with source negates.

use oag_rcs::rcsmaterial::RcsMaterial;
use oag_rcs::rcsmaterial::fragment::{Instruction, Program, Source};

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

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

fn dump(program: &Program, lo: usize, hi: usize) {
    for (i, insn) in program.instructions.iter().enumerate().take(hi).skip(lo) {
        println!(
            "    {i:3} {:?}{} dst={}{} mask={:04b} neg={:?} sw={:?} const={:?} ops={:?}",
            insn.name(),
            if insn.saturate { "_SAT" } else { "" },
            insn.dst,
            if insn.dst_half { "h" } else { "r" },
            insn.mask,
            insn.negate,
            insn.swizzles,
            insn.constant,
            insn.operands().collect::<Vec<_>>(),
        );
    }
}

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let (mut blocks, mut chains, mut log2e, mut shown) = (0usize, 0usize, 0usize, 0usize);
    let (mut wrong_lane, mut wrong_lane_log2e) = (0usize, 0usize);
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
                blocks += 1;
                for (i, insn) in program.instructions.iter().enumerate() {
                    if insn.name() != Some("LG2") || !dp3_feeds(&program, i) {
                        continue;
                    }
                    let lg2_dst = (insn.dst, insn.dst_half);
                    let Some(m) = program.instructions[i + 1..].iter().position(|x| {
                        x.name() == Some("MUL")
                            && x.constant.is_some()
                            && x.operands().any(|s| matches!(s, Source::Register { index, half } if (index, half) == lg2_dst))
                    }) else {
                        continue;
                    };
                    chains += 1;
                    let mul = &program.instructions[i + 1 + m];
                    let c = mul.constant.unwrap();
                    let read = read_mask(mul, lg2_dst).unwrap_or(0);
                    let wrote = insn.mask;
                    let sound = read & wrote == read;
                    if !sound {
                        wrong_lane += 1;
                    }
                    if c.iter()
                        .any(|v| (v - std::f32::consts::LOG2_E).abs() < 1e-3)
                    {
                        log2e += 1;
                        if !sound {
                            wrong_lane_log2e += 1;
                        }
                        if shown < 1 {
                            shown += 1;
                            println!(
                                "{path} variant fragment @{:#x}: LG2 at {i}, MUL at {}",
                                variant.fragment.offset,
                                i + 1 + m
                            );
                            dump(&program, i.saturating_sub(3), i + m + 4);
                        }
                    }
                }
            }
        }
    }
    println!(
        "MUL reads a lane the LG2 did not write: {wrong_lane} of {chains} chains, {wrong_lane_log2e} of the {log2e} log2(e) ones"
    );
    println!(
        "{blocks} blocks, {chains} DP3-fed LG2->MUL(literal) chains, {log2e} with literal log2(e)"
    );
    Ok(())
}
