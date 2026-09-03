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
//! aware and clobber-checked**: [`last_writer`] only reports an instruction
//! that both wrote *every* lane the reader's own swizzle selects and had
//! nothing else touch any of those lanes afterward, so a value assembled
//! across several partial writes reports as "no single writer" rather than
//! naming whichever one happened to sit closest.
//!
//! **The honest result of that check: this operand trace does not
//! discriminate.** Two earlier, less careful passes each claimed a shape
//! ("every operand carries a normalize tail", then "one confirmed `32`
//! block's operand comes from `EX2`, unlike the disputed values") - both
//! were artifacts of a `last_writer` that ignored write masks and,
//! afterward, ignored a later clobber. Corrected, **164 of 168 operands**
//! across all 84 `200`/`250`/`260`/`35` occurrences have `NoSingleWriter`:
//! this microcode overwhelmingly builds a vector's lanes across several
//! separate, partial-mask instructions rather than one full write, so "the
//! last instruction that wrote this register" is very rarely a real
//! answer. Run against the confirmed `32` bucket for comparison, the ratio
//! is the same - `5,354` of `5,528` (97 %) - so this is not a property of
//! the four disputed values, it is a property of the microcode itself, and
//! a single-writer heuristic cannot tell a real specular dot from anything
//! else here. Real provenance would need a per-lane dataflow trace (the
//! most recent writer of *each individual lane*, merged), which is a
//! bigger analysis than this pass built - left as the open item, not a
//! result this pass can report either way.
//!
//! **Unnamed opcodes are printed as `op3B`/`op3C`/`op3D`, never guessed at.**
//! `Instruction::name()` returns `None` for these three specifically because
//! they are not in nouveau's table (`fragment.rs`'s own doc comment); an
//! earlier pass here annotated one as "normalize (mul by 1/|sum|)" from its
//! position alone, which is exactly the guess-dressed-as-a-name CLAUDE.md's
//! confidence rubric rules out below 50. `arity()` also defaults an unnamed
//! opcode to 2 operands, so the second operand this prints for one may not
//! be real hardware behaviour - flagged, not resolved, by this pass.

use oag_formats::rcsmaterial;
use oag_formats::rcsmaterial::RcsMaterial;
use oag_formats::rcsmaterial::fragment::{Instruction, Program, Source};

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
/// `200`/`250`/`260`/`35` occurrences (168 operands): `NoSingleWriter` 164,
/// `Normalize` 2, `Neither` 2, `Sum` 0 - see this module's own doc comment
/// for why `NoSingleWriter` dominating means the trace does not
/// discriminate, rather than meaning "not a specular term".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Evidence {
    /// The operand is `normalize(A + B)` for two distinct registers/inputs -
    /// the half-vector shape `renderer.md`'s ship reading already carries.
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

fn classify(program: &Program, at: usize, src: Source, swizzle: [u8; 4]) -> Option<Evidence> {
    let Source::Register { index, half } = src else {
        return None;
    };
    let Some(w) = last_writer(program, at, (index, half), swizzle) else {
        return Some(Evidence::NoSingleWriter);
    };
    let writer = &program.instructions[w];
    // A normalize tail: an instruction reading the same register (the
    // un-normalized vector) and a scalar this project does not further
    // resolve (a reciprocal-length term) - `nvfx_shader.h`'s RSQ/RCP, or one
    // of the two unnamed opcodes this project has seen adjacent to a DP3
    // self-dot on this exact shape.
    let reads_same_reg = writer
        .operands()
        .any(|s| matches!(s, Source::Register{index: i, half: h} if (i, h) == (index, half)));
    if !reads_same_reg {
        return Some(Evidence::Neither);
    }
    // Was the un-normalized vector itself a sum of two distinct sources?
    let Some(pre) = last_writer(program, w, (index, half), [0, 1, 2, 3]) else {
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
