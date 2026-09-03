//! Re-derives `renderer.md`'s "population, excluding Zone-declaring blocks"
//! table (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "Ships have no
//! Lambert diffuse either") after the 2026-09-04 `Program::dp3_feeding`
//! lane-correctness fix, to see which of the six-plus-four published values
//! moved. **They all did, substantially** - every population number in that
//! section of the page was measured under the buggy gate. Re-deriving the
//! full narrative (which values are Zone rim exponents, which are
//! `SpecularPower` patched at draw time, what `200`/`250`/`260`/`35` actually
//! are now that the population roughly doubled) is its own task with its own
//! verification needs, tracked in a new handover thread rather than finished
//! in the same pass that found the gate bug - this file is that thread's
//! starting evidence, kept rather than thrown away once its first measurement
//! landed.
//!
//! Also prints the *unfiltered* histogram (Zone-declaring blocks included)
//! alongside the excluded one, to check `declares_zone` itself rather than
//! assume it: `5`/`10` reappear in bulk unfiltered (134/76) and vanish
//! entirely once Zone is excluded, exactly as the pre-fix sweep found - the
//! Zone-declaring share growing from a small minority to the majority
//! (6,464 of 10,087, 64%) is a real consequence of the fix reaching more rim
//! chains, not a filter bug.

use oag_formats::rcsmaterial;
use oag_formats::rcsmaterial::RcsMaterial;
use oag_formats::rcsmaterial::fragment::Program;

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

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    let mut histogram: Vec<(f32, u32)> = Vec::new();
    let mut unfiltered_histogram: Vec<(f32, u32)> = Vec::new();
    let mut total = 0usize;
    let mut zone_excluded = 0usize;

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
                let Some(exponent) = program.specular_exponent() else {
                    continue;
                };
                total += 1;
                match unfiltered_histogram
                    .iter_mut()
                    .find(|(v, _)| (*v - exponent).abs() < 1e-3)
                {
                    Some((_, n)) => *n += 1,
                    None => unfiltered_histogram.push((exponent, 1)),
                }
                if declares_zone(&program) {
                    zone_excluded += 1;
                    continue;
                }
                match histogram
                    .iter_mut()
                    .find(|(v, _)| (*v - exponent).abs() < 1e-3)
                {
                    Some((_, n)) => *n += 1,
                    None => histogram.push((exponent, 1)),
                }
            }
        }
    }

    println!(
        "{total} blocks resolved by specular_exponent() total, {zone_excluded} Zone-declaring (excluded below)"
    );
    println!("--- unfiltered (includes Zone-declaring blocks) ---");
    unfiltered_histogram.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for (v, n) in &unfiltered_histogram {
        println!("  {v}: {n}");
    }
    println!("--- Zone-excluded ---");
    histogram.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for (v, n) in &histogram {
        println!("  {v}: {n}");
    }
    Ok(())
}
