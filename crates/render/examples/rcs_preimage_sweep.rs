//! The preimage sweep behind `docs/formats/rcsmaterial.md`'s "Open" section:
//! hash every identifier-shaped candidate from four sources and see which
//! land on a disc hash that [`oag_rcs::rcsmaterial::names`] does not already
//! carry.
//!
//! # The two populations, and why their scope is not "the whole disc"
//!
//! - **Sampler hashes**: [`Declared::samplers`] read off `DATA00.PSARC`'s
//!   `.rcsmaterial` fragment blocks alone, deduplicated by block offset -
//!   **125** distinct hashes there, matching `rcsmaterial.md`'s own count
//!   exactly (and its `10,276` fragment-block figure, which is the same scope
//!   scanned a second way). Widening to all seven archives moves this to 191
//!   and is reported separately, never folded into the headline number, so a
//!   count computed here stays comparable to the one already in the docs.
//! - **Parameter hashes**: [`rcsmodel::material::Parameter::hash`] read off
//!   `DATA00.PSARC` + `DATA02.PSARC`'s `.rcsmodel` records - **300** distinct
//!   hashes there, the same scope `hd_param_names.rs` already sweeps (this
//!   tool supersedes it for parameters and adds the sampler side it never
//!   had).
//!
//! # The four candidate sources
//!
//! a. `EBOOT.elf`'s own identifier-shaped strings (path argument; SELF is
//!    encrypted, see `docs/formats/ps3-disc.md` - decrypt with
//!    `rpcs3 --decrypt EBOOT.BIN`).
//! b. `DFEngine.sprx`'s, once decrypted the same way (also a SELF; `rpcs3
//!    --decrypt` works on it too, producing a `.prx`. Optional argument).
//! c. Every identifier-shaped NUL-terminated run inside every
//!    `.rcsmaterial`/`.rcsmodel`/`.vex` file on the disc - material names,
//!    `.gtf` basenames, node names - scanned as raw bytes, so this needs
//!    nothing from `oag-vex`'s own `.vex` decoder.
//! d. A generated vocabulary: every token the names in (a) recovered so far
//!    are built from, combined pairwise in camelCase/PascalCase/snake_case,
//!    with digits `0`-`9` and the affixes `_vp`/`_fp`/`Tex`/`Texture`/`Map`/
//!    `Colour`/`Color` appended.
//!
//! Every candidate is hashed once and kept in a `hash -> [(text, source)]`
//! map; a hash with more than one *distinct* text is reported as ambiguous
//! rather than resolved, per this project's rule that a preimage is evidence
//! and a collision is not silently broken by picking one.
//!
//! ```sh
//! cargo run --release -p oag-render --example rcs_preimage_sweep -- \
//!     data/images/hdfury-ps3-eu-dec.iso \
//!     /path/to/EBOOT.elf \
//!     /path/to/DFEngine.prx
//! ```

use oag_rcs::rcsmaterial::{Declared, RcsMaterial, names};
use oag_rcs::rcsmodel;
use std::collections::{BTreeMap, BTreeSet};

/// Where the "125 distinct sampler hashes" baseline comes from.
const SAMPLER_ARCHIVE: &str = "PS3_GAME/USRDIR/DATA00.PSARC";
/// Where the "300 distinct parameter hashes" baseline comes from.
const PARAMETER_ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
];
/// Every archive on the disc, for the broader figure reported separately.
const ALL_ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

/// One candidate string, tagged with where it came from.
#[derive(Debug, Clone)]
struct Candidate {
    text: String,
    source: &'static str,
}

/// `hash -> every distinct candidate that produced it`, across all sources.
type Candidates = BTreeMap<u32, Vec<Candidate>>;

fn add(map: &mut Candidates, text: &str, source: &'static str) {
    if text.is_empty() {
        return;
    }
    let hash = oag_rcs::rcsmaterial::name_hash(text);
    let entry = map.entry(hash).or_default();
    if !entry.iter().any(|c| c.text == text) {
        entry.push(Candidate {
            text: text.to_string(),
            source,
        });
    }
}

/// The same identifier-shape predicate `hd_param_names.rs` uses: 3-64 bytes,
/// alphanumeric/underscore (and `.` for path-shaped runs).
fn identifier_shaped(run: &[u8], allow_dot: bool) -> Option<&str> {
    if !(3..=64).contains(&run.len()) {
        return None;
    }
    if !run
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || (allow_dot && *b == b'.'))
    {
        return None;
    }
    std::str::from_utf8(run).ok()
}

fn scan_executable(
    map: &mut Candidates,
    path: &str,
    source: &'static str,
) -> anyhow::Result<usize> {
    let bytes = std::fs::read(path)?;
    let mut found = 0usize;
    for run in bytes.split(|&b| b == 0) {
        if let Some(text) = identifier_shaped(run, false) {
            add(map, text, source);
            found += 1;
        }
    }
    Ok(found)
}

/// Splits camelCase/PascalCase/snake_case into lowercase tokens.
fn tokenize(name: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }
        if c.is_uppercase() && !current.is_empty() {
            let prev_lower = chars[i - 1].is_lowercase() || chars[i - 1].is_numeric();
            if prev_lower {
                tokens.push(std::mem::take(&mut current));
            }
        }
        current.push(c);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens.into_iter().map(|t| t.to_lowercase()).collect()
}

fn capitalize(word: &str) -> String {
    let mut c = word.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Every token the known sampler and parameter names are built from, plus
/// pairwise camelCase/PascalCase/snake_case combinations, digit suffixes and
/// the affixes this format's own names use elsewhere.
fn generate_vocabulary() -> Vec<String> {
    let mut tokens: BTreeSet<String> = BTreeSet::new();
    for name in names::KNOWN_SAMPLER_NAMES
        .iter()
        .chain(names::KNOWN_PARAMETER_NAMES.iter())
    {
        for t in tokenize(name) {
            tokens.insert(t);
        }
    }
    let tokens: Vec<String> = tokens.into_iter().collect();

    let mut out: BTreeSet<String> = BTreeSet::new();
    let affixes = ["_vp", "_fp", "Tex", "Texture", "Map", "Colour", "Color"];

    for a in &tokens {
        let pascal_a = capitalize(a);
        out.insert(a.clone());
        out.insert(pascal_a.clone());
        for d in 0..=9 {
            out.insert(format!("{a}{d}"));
            out.insert(format!("{pascal_a}{d}"));
        }
        for suffix in &affixes {
            out.insert(format!("{a}{suffix}"));
            out.insert(format!("{pascal_a}{suffix}"));
        }
        for b in &tokens {
            if a == b {
                continue;
            }
            let pascal_b = capitalize(b);
            out.insert(format!("{a}{pascal_b}")); // camelCase
            out.insert(format!("{pascal_a}{pascal_b}")); // PascalCase
            out.insert(format!("{a}_{b}")); // snake_case
            for d in 0..=9 {
                out.insert(format!("{a}{pascal_b}{d}"));
            }
        }
    }
    out.into_iter().collect()
}

/// The distinct `Declared::samplers` hashes across every fragment block of
/// every `.rcsmaterial` in `archives`, deduplicated by block offset within a
/// file (content is constant across variants sharing one). Materials named
/// per hash are capped for the report.
fn sampler_population(
    image: &str,
    archives: &[&str],
) -> anyhow::Result<BTreeMap<u32, (usize, Vec<String>)>> {
    let mut population: BTreeMap<u32, (usize, Vec<String>)> = BTreeMap::new();
    for archive in archives {
        let spec = format!("{image}:{archive}");
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmaterial"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(mat) = RcsMaterial::parse(&blob) else {
                continue;
            };
            let mut seen_offsets = BTreeSet::new();
            let short = path.rsplit('/').next().unwrap_or(&path).to_string();
            for v in &mat.variants {
                if !seen_offsets.insert(v.fragment.offset) {
                    continue;
                }
                let Some(decl) = Declared::parse(&blob, v.fragment.offset) else {
                    continue;
                };
                for (hash, _unit) in &decl.samplers {
                    let row = population.entry(*hash).or_default();
                    row.0 += 1;
                    if row.1.len() < 4 && !row.1.contains(&short) {
                        row.1.push(short.clone());
                    }
                }
            }
        }
    }
    Ok(population)
}

/// The distinct `Parameter::hash` values across every `.rcsmodel` in
/// `archives`.
fn parameter_population(
    image: &str,
    archives: &[&str],
) -> anyhow::Result<BTreeMap<u32, (usize, Vec<String>)>> {
    let mut population: BTreeMap<u32, (usize, Vec<String>)> = BTreeMap::new();
    for archive in archives {
        let spec = format!("{image}:{archive}");
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            for material in &model.materials {
                let short = material
                    .name
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(&material.name)
                    .to_string();
                for p in &material.parameters {
                    let row = population.entry(p.hash).or_default();
                    row.0 += 1;
                    if row.1.len() < 4 && !row.1.contains(&short) {
                        row.1.push(short.clone());
                    }
                }
            }
        }
    }
    Ok(population)
}

/// Scans every `.rcsmaterial`/`.rcsmodel`/`.vex` file in `archives` for
/// identifier-shaped NUL-terminated runs - source (c). Raw bytes only, so
/// nothing here reaches into `oag-vex`'s own decoder.
fn scan_disc_strings(
    map: &mut Candidates,
    image: &str,
    archives: &[&str],
) -> anyhow::Result<usize> {
    let mut found = 0usize;
    for archive in archives {
        let spec = format!("{image}:{archive}");
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| {
                p.ends_with(".rcsmaterial") || p.ends_with(".rcsmodel") || p.ends_with(".vex")
            })
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            for run in blob.split(|&b| b == 0) {
                if let Some(text) = identifier_shaped(run, false) {
                    add(map, text, "disc-strings");
                    found += 1;
                }
            }
        }
    }
    Ok(found)
}

fn report(
    label: &str,
    population: &BTreeMap<u32, (usize, Vec<String>)>,
    known: fn(u32) -> Option<&'static str>,
    candidates: &Candidates,
) {
    println!("\n=== {label}: {} distinct hashes ===", population.len());
    let mut already_named = 0usize;
    let mut newly_named = 0usize;
    let mut ambiguous = 0usize;
    let mut by_source: BTreeMap<&str, usize> = BTreeMap::new();
    let mut unresolved: Vec<u32> = Vec::new();

    for (hash, (count, materials)) in population {
        if let Some(name) = known(*hash) {
            already_named += 1;
            let _ = (name, count, materials);
            continue;
        }
        let Some(found) = candidates.get(hash) else {
            unresolved.push(*hash);
            continue;
        };
        newly_named += 1;
        let distinct_texts: BTreeSet<&str> = found.iter().map(|c| c.text.as_str()).collect();
        let sources: BTreeSet<&str> = found.iter().map(|c| c.source).collect();
        for s in &sources {
            *by_source.entry(s).or_default() += 1;
        }
        let flag = if distinct_texts.len() > 1 {
            ambiguous += 1;
            " AMBIGUOUS"
        } else {
            ""
        };
        println!(
            "  NEW  {hash:#010x} x{count:<6} {:<32} sources={sources:?}{flag} materials={materials:?}",
            distinct_texts
                .iter()
                .copied()
                .collect::<Vec<_>>()
                .join(" / "),
        );
    }

    println!(
        "  {already_named} already named, {newly_named} newly named this sweep ({ambiguous} ambiguous), {} still unresolved",
        unresolved.len()
    );
    println!("  per-source coverage of the newly named: {by_source:?}");
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let eboot = args.next();
    let dfengine = args.next();

    let mut candidates: Candidates = BTreeMap::new();

    if let Some(eboot) = &eboot {
        let found = scan_executable(&mut candidates, eboot, "eboot")?;
        println!("(a) {eboot}: {found} identifier-shaped strings");
    } else {
        println!("(a) no EBOOT.elf given - skipped (see docs/formats/ps3-disc.md)");
    }

    if let Some(dfengine) = &dfengine {
        let found = scan_executable(&mut candidates, dfengine, "dfengine")?;
        println!("(b) {dfengine}: {found} identifier-shaped strings");
    } else {
        println!("(b) no DFEngine.prx given - skipped");
    }

    let disc_strings = scan_disc_strings(&mut candidates, &image, ALL_ARCHIVES)?;
    println!(
        "(c) {disc_strings} identifier-shaped strings across every .rcsmaterial/.rcsmodel/.vex on the disc"
    );

    let vocabulary = generate_vocabulary();
    for word in &vocabulary {
        add(&mut candidates, word, "generated");
    }
    println!("(d) {} generated candidates", vocabulary.len());

    println!("{} distinct hashes across all candidates", candidates.len());

    let sampler_pop = sampler_population(&image, &[SAMPLER_ARCHIVE])?;
    let sampler_pop_all = sampler_population(&image, ALL_ARCHIVES)?;
    let param_pop = parameter_population(&image, PARAMETER_ARCHIVES)?;
    let param_pop_all = parameter_population(&image, ALL_ARCHIVES)?;

    report(
        "Sampler hashes (Declared::samplers, DATA00.PSARC fragment blocks - the '125' baseline)",
        &sampler_pop,
        names::sampler_name,
        &candidates,
    );
    println!(
        "  (all seven archives instead: {} distinct - reported separately, not the headline count)",
        sampler_pop_all.len()
    );

    report(
        "Parameter hashes (Material::parameters, DATA00+DATA02.PSARC - the '300' baseline)",
        &param_pop,
        names::parameter_name,
        &candidates,
    );
    println!(
        "  (all seven archives instead: {} distinct - reported separately, not the headline count)",
        param_pop_all.len()
    );

    println!("\n=== The discriminating target: pads.md's two const-slot hashes ===");
    for (hash, label) in [(0x7611_a2d8u32, "Speedup Pad"), (0xce5c_4410, "Weapon Pad")] {
        let known = names::parameter_name(hash);
        let found = candidates.get(&hash);
        println!("  {hash:#010x} ({label}): known={known:?} candidates={found:?}");
    }

    Ok(())
}
