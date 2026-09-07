//! Ad hoc sampler for the specular-exponent re-measurement work: dumps
//! a handful of Zone-declaring blocks from the `32` and `0` buckets by hand,
//! since `hd_specular_population_recheck.rs`'s own histogram split shows the
//! 64% Zone-declaring jump is concentrated there (`32`: 1444 -> 5134
//! unfiltered, `0`: 1965 -> 4503), not in `5`/`10` (already confirmed rim,
//! 134/76 total - under 4% of the 6,464 Zone-declaring population) nor in
//! `200`/`260`/`35` (identical filtered/unfiltered counts - zero
//! Zone-declaring blocks there at all). This checks whether the `32`/`0`
//! growth is the same `rim^N` idiom under a different resolved value, or an
//! unrelated specular/patched-zero chain in a material that happens to also
//! declare an unrelated Zone parameter (e.g. a colour tint) elsewhere.
//!
//! Also samples `5`/`10` (the confirmed rim exponents) as a positive control -
//! this page's own history warns that comparing an uncontrolled sample
//! against nothing is exactly how the four-draft operand-shape trace went
//! wrong the first three times - and prints `declares_zone` across *every*
//! variant of the first sampled `0`/`32` material, to tell apart "the
//! four-parameter cluster is boilerplate on the whole file" from "it is
//! scoped to a Zone-mode-only variant of the shader", which the original
//! per-winning-variant-only sample could not distinguish (every printed
//! block declared zone by construction, since that is the sampler's own
//! filter).
//!
//! Not wired into any test - a one-off read, kept as evidence for the
//! thread's dated history entry, per this project's "record the evidence"
//! rule.

use oag_formats::rcsmaterial;
use oag_formats::rcsmaterial::RcsMaterial;
use oag_formats::rcsmaterial::fragment::{Instruction, Program, Source};

/// Ported from `hd_specular_unresolved_trace.rs` - see that module's own doc
/// comment for why the lane-aware, clobber-checked read is necessary and
/// what the categories mean. Kept here rather than shared so this sampler
/// stays a single self-contained file, per the pattern the sibling example
/// already sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Evidence {
    Sum,
    Normalize,
    Neither,
    NoSingleWriter,
}

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
    let dp3_read = [swizzle[0], swizzle[1], swizzle[2], swizzle[0]];
    let Some(w) = last_writer(program, at, (index, half), dp3_read) else {
        return Some(Evidence::NoSingleWriter);
    };
    let writer = &program.instructions[w];
    let Some(read) = read_lanes(writer, (index, half)) else {
        return Some(Evidence::Neither);
    };
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

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

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

fn declared_zone_names(program: &Program) -> Vec<&'static str> {
    ZONE_PARAMETER_NAMES
        .iter()
        .filter(|n| {
            let h = rcsmaterial::name_hash(n);
            program.declared.parameters.contains(&h)
        })
        .copied()
        .collect()
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

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut seen_paths: Vec<String> = Vec::new();
    let mut per_variant_dumped = false;

    for archive in ARCHIVES {
        // Reset per archive, so the sample spreads across all seven PSARCs
        // instead of exhausting on whichever is walked first. `5`/`10` are a
        // positive control - the confirmed rim exponents this sampler's
        // `0`/`32` readings need something real to be compared against.
        let mut remaining: Vec<(f32, usize)> = vec![(32.0, 2), (0.0, 2), (5.0, 1), (10.0, 1)];
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
            if remaining.iter().all(|(_, n)| *n == 0) {
                break;
            }
            let Ok(data) = handle.read_path(&path) else {
                continue;
            };
            let Ok(material) = RcsMaterial::parse(&data) else {
                continue;
            };
            let key = format!("{archive}:{path}");
            if seen_paths.contains(&key) {
                continue;
            }
            let mut used_this_file = false;
            for (vi, variant) in material.variants.iter().enumerate() {
                if used_this_file {
                    break;
                }
                let Some(program) = Program::parse(&data, variant.fragment.offset) else {
                    continue;
                };
                let Some(exponent) = program.specular_exponent() else {
                    continue;
                };
                let Some(slot) = remaining.iter_mut().find(|(v, n)| *v == exponent && *n > 0)
                else {
                    continue;
                };
                let zone_names = declared_zone_names(&program);
                if zone_names.is_empty() {
                    continue;
                }
                let Some(dp3_i) = program.specular_exponent_dp3() else {
                    continue;
                };
                slot.1 -= 1;
                used_this_file = true;
                seen_paths.push(key.clone());
                println!(
                    "=== {archive}:{path} variant {vi} exponent {exponent} zone_params={zone_names:?}"
                );
                let dp3 = &program.instructions[dp3_i];
                let start = dp3_i.saturating_sub(10);
                for (i, insn) in program.instructions[start..=dp3_i].iter().enumerate() {
                    println!("  {}", fmt_insn(start + i, insn));
                }
                for (opi, (src, sw)) in dp3.operands().zip(dp3.swizzles).enumerate() {
                    let evidence = classify(&program, dp3_i, src, sw);
                    println!(
                        "    dp3 operand {opi}: {} -> {evidence:?}",
                        fmt_source(src, sw, dp3.input)
                    );
                }
                println!();

                // Once, on the very first material sampled: does every
                // variant of this *file* declare the same Zone cluster (a
                // property of the file, i.e. boilerplate on the whole
                // material), or only some (a property of a specific
                // Zone-mode variant)? The main loop above cannot tell this
                // apart on its own - every block it prints declares zone by
                // construction, since that is its own filter.
                if !per_variant_dumped {
                    per_variant_dumped = true;
                    println!("--- per-variant declares_zone for {archive}:{path} ---");
                    for (ovi, ovariant) in material.variants.iter().enumerate() {
                        let Some(oprogram) = Program::parse(&data, ovariant.fragment.offset) else {
                            println!("  variant {ovi}: (does not parse)");
                            continue;
                        };
                        let oexponent = oprogram.specular_exponent();
                        println!(
                            "  variant {ovi}: declares_zone={} specular_exponent={oexponent:?}",
                            !declared_zone_names(&oprogram).is_empty()
                        );
                    }
                    println!();
                }
            }
        }
    }
    Ok(())
}
