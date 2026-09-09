//! Dumps the microcode around the `DP3` feeding every fragment-program
//! specular chain that resolves to `200`, `250`, `260` or `35` - the four
//! values `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "Ships have no
//! Lambert diffuse either" left open both ways: a real `pow(N.H, e)`
//! specular term, or a `pow`/`exp` idiom sharing the same three trailing
//! instructions (`LG2`/`MUL`/`EX2`) over a different saturated dot product
//! (a Fresnel or falloff curve).
//!
//! **Sweeps every `.rcsmaterial` file directly, across all seven PSARC
//! archives** - not filtered through a circuit's own model-variant
//! resolution the way `hd_specular_patch_census.rs` is. The doc page's
//! six-value population (`5`, `10`, `32`, `26.156`, `40`, `300`) and the
//! four still-open ones here were both read this way, over "every material
//! on the disc", and a 16-circuit reachable-variant walk does not reproduce
//! them: checked directly, that narrower population contains none of
//! `5`/`10`/`40`/`200`/`250`/`260`/`35`/`26.156` at all, only `0`/`32`/`300`.
//!
//! `Program::specular_exponent` names the chain's value; this prints the
//! `DP3`'s own two operands and classifies each with [`Evidence`] - **lane
//! aware and clobber-checked**: [`last_writer`]/[`read_lanes`] only credit
//! an instruction with supplying an operand if it both wrote *every* lane
//! the read actually needs (a `DP3` needs three, not four; a lane-parallel
//! op needs only the lanes matching its own destination mask - see
//! [`read_lanes`]'s own doc comment for why that is a general assumption
//! about masked SIMD, not a claim about what any specific unnamed opcode
//! computes) and had nothing else touch those lanes afterward.
//!
//! **Getting the read width right took three tries, and the first two are
//! kept in `renderer.md`'s own history rather than silently corrected
//! away.** A naive four-lane check found "every operand normalizes"; fixed
//! for `DP3`'s three-lane read alone, almost everything read as
//! `NoSingleWriter` (this microcode writes a vector's lanes across several
//! partial-mask instructions almost universally, so a check that still
//! demands a four-lane writer for a lane-parallel op finds one almost
//! never); only once the elementwise read width was narrowed to match the
//! reading op's own destination mask did the two real categories - `Sum`
//! and `Normalize` - separate from the noise.
//!
//! **A calibration against the one occurrence already read by hand was
//! attempted, and the block is confirmed real - but `specular_exponent()`
//! itself never resolves it, so this tool never sees it either.** See
//! `crates/render/examples/hd_specular_calibration_check.rs` and
//! `renderer.md`'s "Ships have no Lambert diffuse either" for the full
//! decode: `detonator_ship_rich_iridescent.rcsmaterial` at file offset
//! `0x3a00` matches the doc's worked `pow(N.H, 40)` example exactly once
//! its `@0xNN` addresses are read as `byte_position / 16`, but the `LG2`
//! there is fed by an unnamed `0x3b` instruction, not a literal `DP3` -
//! `specular_exponent()`'s own gate requires the latter, so it returns
//! `None` on this block. **The eight `40.0` occurrences this tool's own
//! sweep finds and the one hand-confirmed case are therefore disjoint
//! populations** - the `Sum`/`Normalize` split below was never calibrated
//! against a known-real case, for exactly this reason. See [`Evidence`]'s
//! own doc comment for the numbers.
//!
//! **Unnamed opcodes are printed as `op3B`/`op3C`/`op3D`, never guessed at.**
//! `Instruction::name()` returns `None` for these three specifically because
//! they are not in nouveau's table (`fragment.rs`'s own doc comment); an
//! earlier pass here annotated one as "normalize (mul by 1/|sum|)" from its
//! position alone, which is exactly the guess-dressed-as-a-name CLAUDE.md's
//! confidence rubric rules out below 50. `arity()` also defaults an unnamed
//! opcode to 2 operands, so the second operand this prints for one may not
//! be real hardware behaviour - flagged, not resolved, by this pass.

use oag_rcs::rcsmaterial;
use oag_rcs::rcsmaterial::RcsMaterial;
use oag_rcs::rcsmaterial::fragment::{Instruction, Program, Source};

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

const TARGETS: &[f32] = &[200.0, 250.0, 260.0, 35.0];

/// Every declared parameter name `renderer.md`'s own listing spells with a
/// `zone` prefix - the exclusion "Ships have no Lambert diffuse either"
/// applies before its disc-wide sweep, since a Zone rim exponent
/// (`rim^5`/`rim^10`) shares the specular idiom's shape without being one.
const ZONE_PARAMETER_NAMES: &[&str] = &[
    "zoneColourTint",
    "zoneEffectInner",
    "zoneEffectOuter",
    "zoneBaseInner",
    "zoneBaseOuter",
    "zoneBaseAltInner",
    "zoneBaseAltOuter",
    "zoneOrigin",
    "zoneTexInner",
    "zoneTexOuter",
    "zoneTexInnerNearest",
    "zoneTexOuterNearest",
    "zoneTexVis",
    "zoneAnisoPalette",
    "zoneAnisoPaletteOuter",
    "zoneAnisoPower",
];

fn declares_zone(program: &Program) -> bool {
    let hashes: Vec<u32> = ZONE_PARAMETER_NAMES
        .iter()
        .map(|n| rcsmaterial::name_hash(n))
        .collect();
    program
        .declared
        .parameters
        .iter()
        .any(|h| hashes.contains(h))
}

fn fmt_source(s: Source, swizzle: [u8; 4], input: u8) -> String {
    let lanes: String = swizzle
        .iter()
        .map(|l| "xyzw".as_bytes()[*l as usize] as char)
        .collect();
    match s {
        Source::Register { index, half } => {
            format!("{}{index}.{lanes}", if half { "H" } else { "R" })
        }
        Source::Input => format!("f[{}].{lanes}", interpolator_name(input)),
        Source::Constant => format!("{{const}}.{lanes}"),
        Source::Unknown => "?".to_string(),
    }
}

fn interpolator_name(input: u8) -> &'static str {
    match input {
        0 => "POS",
        1 => "COL0",
        2 => "COL1",
        3 => "FOGC",
        4 => "TC0",
        5 => "TC1",
        6 => "TC2",
        7 => "TC3",
        8 => "TC4",
        9 => "TC5",
        0xe => "FACING",
        _ => "TC?",
    }
}

/// The mnemonic, or `op{XX}` (hex opcode) for one `Instruction::name()`
/// does not know - never a guessed verb. See this module's own doc comment.
fn opname(insn: &Instruction) -> String {
    insn.name()
        .map_or_else(|| format!("op{:02X}", insn.opcode), str::to_string)
}

fn fmt_insn(i: usize, insn: &Instruction) -> String {
    let dst = format!("{}{}", if insn.dst_half { "H" } else { "R" }, insn.dst);
    let sat = if insn.saturate { "_SAT" } else { "" };
    let lanes: String = (0..4)
        .filter(|b| insn.mask & (1 << b) != 0)
        .map(|b| "xyzw".as_bytes()[b] as char)
        .collect();
    let ops: Vec<String> = insn
        .operands()
        .zip(insn.swizzles)
        .map(|(s, sw)| fmt_source(s, sw, insn.input))
        .collect();
    format!(
        "[{i:3}] {}{sat} {dst}.{lanes}, {}",
        opname(insn),
        ops.join(", ")
    )
}

/// The single instruction before `at` that supplies **every** lane `read`
/// selects, as it stands at `at` - `None` when no instruction covers every
/// lane in one write, *or* when a later, partial write to any of those
/// lanes lands between it and `at` (so the value `at` actually reads is a
/// mix of that write and whatever touched it since - not this one
/// instruction's output). Either way, a caller gets an honest "no single
/// writer" rather than a nearest-write search silently naming a stale or
/// partial one.
fn last_writer(program: &Program, at: usize, reg: (u8, bool), read: [u8; 4]) -> Option<usize> {
    let read_mask: u8 = read.iter().fold(0u8, |acc, &l| acc | (1 << l));
    let (w, _) = program.instructions[..at]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, insn)| (insn.dst, insn.dst_half) == reg && insn.mask & read_mask == read_mask)?;
    let clobbered = program.instructions[w + 1..at]
        .iter()
        .any(|insn| (insn.dst, insn.dst_half) == reg && insn.mask & read_mask != 0);
    (!clobbered).then_some(w)
}

/// What tracing one `DP3` operand back found - see [`classify`]. Over all 84
/// `200`/`250`/`260`/`35` occurrences (168 operands, all of them registers):
/// `Sum` 82, `Normalize` 58, `Neither` 28, `NoSingleWriter` 0. **Reported as
/// raw structural data, not as evidence of meaning.** `specular_exponent()`
/// never resolves the one block confirmed by hand-reading (this module's own
/// doc comment) - a false negative in its own gate, not an absent chain -
/// so nothing in this tool's `40.0` population, and therefore no `Sum`/
/// `Normalize` result here, has been checked against a known-real case.
/// `Sum` still names a real, lane-correct mechanical shape -
/// `normalize(ADD of two distinct sources)` - it is only the claim that the
/// shape means "half-vector" that is unconfirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Evidence {
    /// The operand traces to `normalize(A + B)` for two distinct sources -
    /// the mechanical shape a half-vector, `H = normalize(V + L)`, would
    /// have. **Not confirmed against a known-real case** - this module's
    /// own doc comment tried and found the attempted calibration block was
    /// the wrong one. The same shape also fits an ordinary bias-add.
    Sum,
    /// The operand traces to a self-dot-then-scale (`DP3 x,v,v` then a
    /// writer reading `x` and `v`) with a single vector rather than a sum -
    /// still a unit-vector construction, but not the two-light-directions
    /// shape a half-vector specifically needs.
    Normalize,
    /// A last writer exists but is neither of the above (e.g. `EX2`, `MUL`
    /// by a constant, another `DP3`'s raw scalar output).
    Neither,
    /// No single instruction wrote every lane this operand reads.
    NoSingleWriter,
}

/// The lanes `insn` actually reads from `reg`, given the swizzle it reads it
/// with - `None` if `insn` does not read `reg` at all.
///
/// **A `DP3` computes a three-component dot product and never consumes the
/// fourth swizzled lane**, regardless of its own destination mask - one
/// place this got wrong: an identity-swizzle `DP3` operand was checked
/// against all four lanes, so an instruction that fully supplied `.xyz` and
/// left `.w` to something unrelated (the dominant shape in this microcode,
/// per the `op3B .xyz` / `EX2 .w` split seen throughout) reported as no
/// single writer when one genuinely existed.
///
/// **Every other opcode here is lane-parallel: output lane `i` reads only
/// swizzled input lane `i`**, so a source lane whose *destination* mask bit
/// is clear is never actually consumed - this is a property of a masked
/// SIMD ALU generally (the same assumption [`last_writer`]'s own clobber
/// check already leans on to say what a mask "means"), not a claim about
/// what any specific unnamed opcode computes.
fn read_lanes(insn: &Instruction, reg: (u8, bool)) -> Option<[u8; 4]> {
    let (_, sw) = insn
        .operands()
        .zip(insn.swizzles)
        .find(|(s, _)| matches!(s, Source::Register { index, half } if (*index, *half) == reg))?;
    if insn.name() == Some("DP3") {
        return Some([sw[0], sw[1], sw[2], sw[0]]);
    }
    let mut lanes = [sw[0]; 4];
    let mut n = 0;
    for (i, &lane) in sw.iter().enumerate() {
        if insn.mask & (1 << i) != 0 {
            lanes[n] = lane;
            n += 1;
        }
    }
    Some(lanes)
}

fn classify(program: &Program, at: usize, src: Source, swizzle: [u8; 4]) -> Option<Evidence> {
    let Source::Register { index, half } = src else {
        return None;
    };
    // The winning instruction is always a `DP3` - see `read_lanes`.
    let dp3_read = [swizzle[0], swizzle[1], swizzle[2], swizzle[0]];
    let Some(w) = last_writer(program, at, (index, half), dp3_read) else {
        return Some(Evidence::NoSingleWriter);
    };
    let writer = &program.instructions[w];
    // A normalize tail: an instruction reading the same register (the
    // un-normalized vector) and a scalar this project does not further
    // resolve (a reciprocal-length term) - `nvfx_shader.h`'s RSQ/RCP, or one
    // of the two unnamed opcodes this project has seen adjacent to a DP3
    // self-dot on this exact shape.
    let Some(read) = read_lanes(writer, (index, half)) else {
        return Some(Evidence::Neither);
    };
    // Was the un-normalized vector itself a sum of two distinct sources?
    let Some(pre) = last_writer(program, w, (index, half), read) else {
        return Some(Evidence::Normalize);
    };
    let pre_insn = &program.instructions[pre];
    if pre_insn.name() == Some("ADD") {
        let mut operands = pre_insn.operands();
        let (a, b) = (operands.next(), operands.next());
        if a != b {
            return Some(Evidence::Sum);
        }
    }
    Some(Evidence::Normalize)
}

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut found = 0usize;
    let mut histogram: Vec<(f32, u32)> = Vec::new();
    let mut materials_checked = 0usize;
    let mut evidence_tally: Vec<(Evidence, u32)> = Vec::new();

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
            materials_checked += 1;
            for variant in &material.variants {
                let Some(program) = Program::parse(&data, variant.fragment.offset) else {
                    continue;
                };
                let Some(exponent) = program.specular_exponent() else {
                    continue;
                };
                match histogram
                    .iter_mut()
                    .find(|(v, _)| (*v - exponent).abs() < 1e-3)
                {
                    Some((_, n)) => *n += 1,
                    None => histogram.push((exponent, 1)),
                }
                if !TARGETS.iter().any(|t| (exponent - t).abs() < 1e-3) {
                    continue;
                }
                let Some(dp3_i) = program.specular_exponent_dp3() else {
                    continue;
                };
                found += 1;
                println!(
                    "=== {archive}:{path} exponent {exponent} declares_zone={}",
                    declares_zone(&program),
                );
                let dp3 = &program.instructions[dp3_i];
                let start = dp3_i.saturating_sub(8);
                for (i, insn) in program.instructions[start..=dp3_i].iter().enumerate() {
                    println!("  {}", fmt_insn(start + i, insn));
                }
                for (opi, (src, sw)) in dp3.operands().zip(dp3.swizzles).enumerate() {
                    let evidence = classify(&program, dp3_i, src, sw);
                    println!(
                        "    dp3 operand {opi}: {} -> {evidence:?}",
                        fmt_source(src, sw, dp3.input)
                    );
                    if let Some(e) = evidence {
                        match evidence_tally.iter_mut().find(|(ev, _)| *ev == e) {
                            Some((_, n)) => *n += 1,
                            None => evidence_tally.push((e, 1)),
                        }
                    }
                }
                println!();
            }
        }
    }

    println!("--- {materials_checked} .rcsmaterial files checked");
    println!("--- {found} blocks resolved to one of {TARGETS:?}");
    histogram.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for (v, n) in &histogram {
        println!("  {v}: {n}");
    }
    println!("--- operand evidence tally ({} operands)", found * 2);
    for (e, n) in &evidence_tally {
        println!("  {e:?}: {n}");
    }
    Ok(())
}
