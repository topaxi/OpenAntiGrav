//! Three of `hd-specular-exponent-population-needs-re-measuring.md`'s open
//! items, answered in one disc walk rather than three, since all three read
//! the same `(archive, path, variant)` population:
//!
//! 1. **What the two `NoSingleWriter` operands
//!    (`hd_specular_unresolved_trace.rs`) actually resolve to.** Both belong
//!    to `nitro_perspex_new.rcsmaterial`'s `260` blocks. This dumps, per
//!    lane the winning `DP3` reads, every instruction before it that writes
//!    any part of that lane - not just the single full-mask writer
//!    `last_writer` requires - so "no single writer" reads as "assembled
//!    from N partial writes", the shape this microcode uses everywhere else
//!    (`op3B .xyz` / `EX2 .w`), rather than as "nothing writes it".
//!
//! 2. **The orthogonality question**: does a `declares_zone=false` variant
//!    ever resolve a specular exponent at all? The disc-wide count answers
//!    the plain reading (yes - `200`/`260`/`35` are entirely non-Zone,
//!    per renderer.md's 2026-09-05 entry). The sharper question
//!    `01_normal_diffuse_specularonalpha.rcsmaterial` left open is
//!    file-scoped: among files whose *own* variants include both a
//!    Zone-declaring and a non-Zone one (the toggle this file showed), does
//!    the non-Zone side ever resolve, broken down by which value? That is
//!    what actually answers "is the flag orthogonal to this specific file's
//!    chain" rather than "is it orthogonal somewhere on the disc".
//!
//! 3. **Whether the four-parameter Zone cluster
//!    (`zoneColourTint`/`zoneEffectInner`/`zoneBaseInner`/`zoneBaseAltInner`)
//!    the 13-material by-hand read found is the disc-wide norm or a sample
//!    artefact.** A census of every Zone-declaring variant's *exact* declared
//!    subset of the 16 known `zone*` parameter names, tallied by archive and
//!    by circuit directory, in place of a bigger hand sample.
//!
//! **What this does not do**: settle *meaning*. Every number here is
//! structural (who declares what, who resolves what, which instruction wrote
//! a lane) - not a claim about which values are real specular exponents. See
//! `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "Ships have no Lambert
//! diffuse either", for why that distinction has already cost four drafts.
//!
//! Not wired into any test - a one-off read, kept as evidence for the
//! thread's dated history entry, per this project's "record the evidence"
//! rule.

use std::collections::BTreeMap;

use oag_rcs::rcsmaterial;
use oag_rcs::rcsmaterial::RcsMaterial;
use oag_rcs::rcsmaterial::fragment::{Instruction, Program, Source};

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// Every declared parameter name `renderer.md`'s own listing spells with a
/// `zone` prefix.
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
            program
                .declared
                .parameters
                .contains(&rcsmaterial::name_hash(n))
        })
        .copied()
        .collect()
}

/// The circuit or category directory a `.rcsmaterial` path sits under, read
/// off the path shape every sampled file so far has shared
/// (`/data/environments/<circuit>/materials.../...`,
/// `/data/materials/ships/...`, `/data/weapons/materials/...`,
/// `/data/fe/.../materials/...`) - a string label for grouping, not a parsed
/// field this reading claims authority over.
fn category(path: &str) -> String {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    // Every sampled path so far starts `data/<kind>/...` - `environments`
    // (circuits, keyed by the circuit directory that follows), `materials`
    // (ships, keyed by the next segment), `weapons`, or `fe` (front end).
    let rest = if parts.first().copied() == Some("data") {
        &parts[1..]
    } else {
        &parts[..]
    };
    match rest.first().copied() {
        Some("environments") => rest
            .get(1)
            .map_or_else(|| "environments".to_string(), |&c| c.to_string()),
        Some("materials") => format!("materials/{}", rest.get(1).unwrap_or(&"?")),
        Some(other) => other.to_string(),
        None => "?".to_string(),
    }
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

/// Ported from `hd_specular_unresolved_trace.rs` - see that module's own doc
/// comment for the three-draft history behind the lane width and clobber
/// rules. Kept local rather than shared, matching the sibling examples'
/// existing pattern of a self-contained file.
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

/// Item 1: for a `NoSingleWriter` operand, print every instruction before
/// `at` that writes *any* part of `reg`, and which of the three lanes `at`'s
/// `DP3` reads (`x`/`y`/`z`, by lane index) each one covers - the per-lane
/// story `last_writer`'s single-instruction requirement can't tell.
fn dump_partial_writers(program: &Program, at: usize, reg: (u8, bool), needed_lanes: [u8; 3]) {
    let lane_char = |l: u8| "xyzw".as_bytes()[l as usize] as char;
    println!(
        "    per-lane writer search for {}{} , lanes needed: {}{}{}",
        if reg.1 { "H" } else { "R" },
        reg.0,
        lane_char(needed_lanes[0]),
        lane_char(needed_lanes[1]),
        lane_char(needed_lanes[2])
    );
    let mut covered = [false; 3];
    for (i, insn) in program.instructions[..at].iter().enumerate().rev() {
        if (insn.dst, insn.dst_half) != reg {
            continue;
        }
        let hits: Vec<char> = needed_lanes
            .iter()
            .filter(|&&l| insn.mask & (1 << l) != 0)
            .map(|&l| lane_char(l))
            .collect();
        if hits.is_empty() {
            continue;
        }
        println!("      {} covers {:?}", fmt_insn(i, insn), hits);
        for (slot, &l) in needed_lanes.iter().enumerate() {
            if insn.mask & (1 << l) != 0 {
                covered[slot] = true;
            }
        }
        if covered.iter().all(|&c| c) {
            println!("      (all needed lanes now covered by the writers above)");
            break;
        }
    }
    if !covered.iter().all(|&c| c) {
        println!(
            "      (some needed lane never written before this point - true absence, not just partial)"
        );
    }
}

const TARGETS: &[f32] = &[200.0, 250.0, 260.0, 35.0];

#[derive(Default)]
struct ValueTally(BTreeMap<u32, u32>);
impl ValueTally {
    fn add(&mut self, v: f32) {
        *self.0.entry(v.to_bits()).or_default() += 1;
    }
    fn print(&self, indent: &str) {
        let mut rows: Vec<(f32, u32)> = self
            .0
            .iter()
            .map(|(&b, &n)| (f32::from_bits(b), n))
            .collect();
        rows.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        for (v, n) in rows {
            println!("{indent}{v}: {n}");
        }
    }
}

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";

    let mut materials_checked = 0usize;
    let mut variants_checked = 0usize;
    let mut variants_unparsed = 0usize;

    // Item 2a: disc-wide, plain reading.
    let mut nonzone_resolved = ValueTally::default();
    let mut zone_resolved = ValueTally::default();

    // Item 2b: files whose own variants include both a Zone-declaring and a
    // non-Zone one - the toggle `01_normal_diffuse_specularonalpha` showed.
    // Per file: (any zone variant seen, any nonzone variant seen, nonzone
    // variant that resolves, by value).
    struct FileRecord {
        has_zone: bool,
        has_nonzone: bool,
        nonzone_resolves: ValueTally,
    }
    let mut files: BTreeMap<String, FileRecord> = BTreeMap::new();

    // Item 3: exact declared zone-parameter subsets, by archive and by
    // category (circuit / ships / weapons / front-end).
    let mut zone_sets_overall: BTreeMap<Vec<&'static str>, u32> = BTreeMap::new();
    let mut zone_sets_by_category: BTreeMap<String, BTreeMap<Vec<&'static str>, u32>> =
        BTreeMap::new();

    // Item 1: NoSingleWriter operands within the already-open TARGETS
    // population, dumped in full once found.
    let mut no_single_writer_found = 0usize;

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
            let file_key = format!("{archive}:{path}");
            let cat = category(&path);

            for variant in &material.variants {
                let Some(program) = Program::parse(&data, variant.fragment.offset) else {
                    variants_unparsed += 1;
                    continue;
                };
                variants_checked += 1;

                let zone_names = declared_zone_names(&program);
                let is_zone = !zone_names.is_empty();
                let exponent = program.specular_exponent();

                if is_zone {
                    *zone_sets_overall.entry(zone_names.clone()).or_default() += 1;
                    *zone_sets_by_category
                        .entry(cat.clone())
                        .or_default()
                        .entry(zone_names.clone())
                        .or_default() += 1;
                }

                if let Some(v) = exponent {
                    if is_zone {
                        zone_resolved.add(v);
                    } else {
                        nonzone_resolved.add(v);
                    }
                }

                let rec = files.entry(file_key.clone()).or_insert_with(|| FileRecord {
                    has_zone: false,
                    has_nonzone: false,
                    nonzone_resolves: ValueTally::default(),
                });
                if is_zone {
                    rec.has_zone = true;
                } else {
                    rec.has_nonzone = true;
                    if let Some(v) = exponent {
                        rec.nonzone_resolves.add(v);
                    }
                }

                // Item 1: reproduce the TARGETS-population operand trace and
                // dump the per-lane story the first time a NoSingleWriter
                // operand turns up.
                if let Some(v) = exponent
                    && TARGETS.iter().any(|t| (v - t).abs() < 1e-3)
                    && let Some(dp3_i) = program.specular_exponent_dp3()
                {
                    let dp3 = program.instructions[dp3_i];
                    for (src, sw) in dp3.operands().zip(dp3.swizzles) {
                        if classify(&program, dp3_i, src, sw) == Some(Evidence::NoSingleWriter) {
                            no_single_writer_found += 1;
                            let Source::Register { index, half } = src else {
                                continue;
                            };
                            println!(
                                "=== NoSingleWriter #{no_single_writer_found}: {archive}:{path} exponent {v} declares_zone={is_zone}"
                            );
                            println!("  winning DP3: {}", fmt_insn(dp3_i, &dp3));
                            dump_partial_writers(
                                &program,
                                dp3_i,
                                (index, half),
                                [sw[0], sw[1], sw[2]],
                            );
                            println!();
                        }
                    }
                }
            }
        }
    }

    println!(
        "--- {materials_checked} .rcsmaterial files, {variants_checked} variants parsed, {variants_unparsed} variants failed to parse"
    );
    println!();

    println!("=== Item 2a: disc-wide specular_exponent() by declares_zone, plain reading ===");
    println!("  declares_zone=false, resolved:");
    nonzone_resolved.print("    ");
    println!("  declares_zone=true, resolved:");
    zone_resolved.print("    ");
    println!();

    println!("=== Item 2b: files with variants on both sides (Zone and non-Zone) ===");
    let toggle_files: Vec<(&String, &FileRecord)> = files
        .iter()
        .filter(|(_, r)| r.has_zone && r.has_nonzone)
        .collect();
    println!(
        "  {} of {} files have at least one Zone-declaring and one non-Zone variant",
        toggle_files.len(),
        files.len()
    );
    let mut toggle_resolving = 0usize;
    for (key, rec) in &toggle_files {
        if !rec.nonzone_resolves.0.is_empty() {
            toggle_resolving += 1;
            print!("    {key}: non-Zone resolves ");
            rec.nonzone_resolves.print("");
        }
    }
    println!(
        "  {toggle_resolving} of {} toggle files have a resolving non-Zone variant",
        toggle_files.len()
    );
    println!();

    println!("=== Item 3: exact declared zone-parameter subsets, disc-wide ===");
    let mut overall_rows: Vec<(&Vec<&'static str>, &u32)> = zone_sets_overall.iter().collect();
    overall_rows.sort_by_key(|&(_, n)| std::cmp::Reverse(*n));
    for (set, n) in &overall_rows {
        println!("  {n}: {set:?}");
    }
    println!();
    println!("=== Item 3: by category (circuit / ships / weapons / front-end) ===");
    for (cat, sets) in &zone_sets_by_category {
        let mut rows: Vec<(&Vec<&'static str>, &u32)> = sets.iter().collect();
        rows.sort_by_key(|&(_, n)| std::cmp::Reverse(*n));
        let total: u32 = rows.iter().map(|&(_, n)| *n).sum();
        println!("  {cat} ({total} Zone-declaring variants):");
        for (set, n) in rows.iter().take(10) {
            println!("    {n}: {set:?}");
        }
        if rows.len() > 10 {
            println!("    ... {} more distinct sets", rows.len() - 10);
        }
    }
    println!();
    println!(
        "--- {no_single_writer_found} NoSingleWriter operands found and dumped above (item 1)"
    );

    Ok(())
}
