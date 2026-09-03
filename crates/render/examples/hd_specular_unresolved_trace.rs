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
//! `DP3`'s own two operands plus, for each operand that is a register, the
//! last instruction before the `DP3` that wrote it - the same "read the
//! instructions by hand" step `renderer.md`'s own worked examples (the
//! ship's `pow(N.H, 40)`, `track_surface`'s block #7) already used, applied
//! here to the four values that were never read that way.

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

fn fmt_insn(i: usize, insn: &Instruction) -> String {
    let name = insn.name().unwrap_or("???");
    let dst = format!("{}{}", if insn.dst_half { "H" } else { "R" }, insn.dst);
    let sat = if insn.saturate { "_SAT" } else { "" };
    let ops: Vec<String> = insn
        .operands()
        .zip(insn.swizzles)
        .map(|(s, sw)| fmt_source(s, sw, insn.input))
        .collect();
    format!("[{i:3}] {name}{sat} {dst}, {}", ops.join(", "))
}

/// The last instruction before `at` that wrote `reg` (by register identity),
/// or `None` if nothing before `at` did.
fn last_writer(program: &Program, at: usize, reg: (u8, bool)) -> Option<usize> {
    program.instructions[..at]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, insn)| insn.mask != 0 && (insn.dst, insn.dst_half) == reg)
        .map(|(i, _)| i)
}

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut found = 0usize;
    let mut histogram: Vec<(f32, u32)> = Vec::new();
    let mut materials_checked = 0usize;

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
                let patched_by_specular_power =
                    program.specular_exponent_slot().is_some_and(|slot| {
                        program
                            .patches(rcsmaterial::SPECULAR_POWER)
                            .any(|s| s == slot)
                    });
                println!(
                    "=== {archive}:{path} exponent {exponent} declares_sun={} declares_specular_power={} patched_by_specular_power={patched_by_specular_power} declares_zone={}",
                    program
                        .declared
                        .parameters
                        .contains(&rcsmaterial::SUN_DIRECTION),
                    program
                        .declared
                        .parameters
                        .contains(&rcsmaterial::SPECULAR_POWER),
                    declares_zone(&program),
                );
                let dp3 = &program.instructions[dp3_i];
                let start = dp3_i.saturating_sub(8);
                for (i, insn) in program.instructions[start..=dp3_i].iter().enumerate() {
                    println!("  {}", fmt_insn(start + i, insn));
                }
                for (opi, (src, sw)) in dp3.operands().zip(dp3.swizzles).enumerate() {
                    println!("    dp3 operand {opi}: {}", fmt_source(src, sw, dp3.input));
                    if let Source::Register { index, half } = src {
                        match last_writer(&program, dp3_i, (index, half)) {
                            Some(w) => println!(
                                "      last written by {}",
                                fmt_insn(w, &program.instructions[w])
                            ),
                            None => {
                                println!("      never written before this point in the program")
                            }
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
    Ok(())
}
