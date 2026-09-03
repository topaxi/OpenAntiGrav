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
//! This mirrors `Program::dp3_feeding`'s own gate (name-and-saturate) rather
//! than reimplementing `specular_exponent_chain`'s full search. The
//! `Lg2NotDp3Fed`/`Lg2Dp3FedButNoChain` split checks **every** `LG2` in the
//! block (`Lg2NotDp3Fed` means none of them are `DP3`-`SAT`-fed, not just
//! the last one), so the category counts are exact under this gate; only
//! the printed *feeder* (what wrote the register) looks at the block's last
//! `LG2` alone, and a block with several differently-fed `LG2`s reports
//! only that one - good enough to size the feeder population, not to claim
//! it is exhaustive per block.
//!
//! **Measured disc-wide, 76,358 blocks**: `Lg2NotDp3Fed` 63.3%, `NoLg2`
//! 27.6%, `Resolved` 8.0%, `Lg2Dp3FedButNoChain` 1.1%. Within
//! `Lg2NotDp3Fed`, the feeder breakdown says the `0x3b` shape is a real but
//! *minority* cause: `ADD`/`MUL`/`MOV`/`TEX`/other named opcodes make up
//! roughly three quarters of it, `op3B`/`op3B_SAT` combined only ~13.6 %,
//! and a plain **unsaturated** `DP3` (a real `DP3` by name, just missing
//! the saturate bit `dp3_feeding` also requires) ~2.8 %. `LG2` is a
//! general-purpose `pow`/`exp` primitive - fog curves, rim falloffs and
//! other unrelated combines use the identical `LG2`/`MUL`/`EX2` shape over
//! a *different* saturated dot, or none at all - so most of `Lg2NotDp3Fed`
//! reads as "this `LG2` was never a specular term" rather than as further
//! confirmed false negatives: `renderer.md`'s own detection of this
//! (excluding Zone's `rim^5`/`rim^10` and the `log2(e)`/`exp()` idiom) was
//! already evidence the bare shape over-matches. The unsaturated-`DP3`
//! slice is the narrower, better-justified candidate for widening the gate,
//! but the one block sampled here (`amphiseum/base_diffusespecular
//! .rcsmaterial`) has four differently-fed `LG2`s in one program, not a
//! single clean term - a real per-block read, not assumed from the shape.

use oag_formats::rcsmaterial::RcsMaterial;
use oag_formats::rcsmaterial::fragment::{Program, Source};

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

/// Mirrors `Program::dp3_feeding`: the most recent full writer of the
/// register `at`'s own first operand names, checked for `DP3` + saturate.
fn dp3_feeds(program: &Program, at: usize) -> bool {
    let Some(Source::Register { index, half }) = program.instructions[at].operands().next() else {
        return false;
    };
    program.instructions[..at]
        .iter()
        .rev()
        .find(|insn| insn.mask != 0 && (insn.dst, insn.dst_half) == (index, half))
        .is_some_and(|insn| insn.name() == Some("DP3") && insn.saturate)
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

/// What actually wrote the register the last `LG2` in the block reads -
/// `"???"` for an unnamed opcode (with its raw hex), `"none"` if no full
/// writer exists at all (an interpolator/constant read, or lanes assembled
/// piecemeal so no single writer covers them).
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
    let Some(Source::Register { index, half }) = program.instructions[lg2_i].operands().next()
    else {
        return "LG2 reads non-register".to_string();
    };
    match program.instructions[..lg2_i]
        .iter()
        .rev()
        .find(|insn| insn.mask != 0 && (insn.dst, insn.dst_half) == (index, half))
    {
        Some(insn) => match insn.name() {
            Some(n) => format!("{n}{}", if insn.saturate { "_SAT" } else { "" }),
            None => format!(
                "op{:02X}{}",
                insn.opcode,
                if insn.saturate { "_SAT" } else { "" }
            ),
        },
        None => "no writer".to_string(),
    }
}

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut materials = 0usize;
    let mut blocks = 0usize;
    let mut tally: Vec<(Category, u32)> = Vec::new();
    let mut feeders: Vec<(String, u32)> = Vec::new();
    let mut unsaturated_samples = 0usize;

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
                    if f == "DP3" && unsaturated_samples < 5 {
                        println!("=== {archive}:{path} unsaturated-DP3-fed LG2 ===");
                        for (i, insn) in program.instructions.iter().enumerate() {
                            let name = insn.name().unwrap_or("???");
                            println!(
                                "  [{i:3}] {name}{} dst=R{}{} mask={:#06b} sat={}",
                                if insn.saturate { "_SAT" } else { "" },
                                insn.dst,
                                if insn.dst_half { "h" } else { "" },
                                insn.mask,
                                insn.saturate,
                            );
                        }
                        unsaturated_samples += 1;
                    }
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
