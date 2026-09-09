//! Validates [`gxp`](oag_rcs::gxp) against every `GXP\0` Wipeout 2048
//! ships - **both** eboots and all three `.psarc` packages.
//!
//! Both builds are swept because they disagree: the base `eboot.elf` embeds
//! **111** programs and the v1.04 patch's embeds **67**, against the same 67
//! `_vp`/`_fp` shader-name strings in either. Sweeping one would have made
//! whichever it was look like the number.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The tests skip with a printed message when the packages are absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves
//! nothing.
//!
//! # What this is for
//!
//! `docs/rendering/shadows.md` could not read a single one of 2048's shader
//! programs, and the shadow half of its plan was blocked on that. The claim
//! this file pins is the one that unblocked it:
//!
//! > **97,899 of 97,899 `GXP` containers on the disc decode**, and each one
//! > accounts for itself from its first instruction to its last name byte.
//!
//! # Why closure, and not "the parser returned `Ok`"
//!
//! `docs/formats/gxp.md` records the derivation; the short version is that two
//! candidate field assignments *both* pass "the parameter table lands inside
//! the program", because one of them reads a field that is zero and zero
//! parameters fit anywhere. What separates them is arithmetic that cannot come
//! out even unless the reading is right, and this is that argument over the
//! whole disc:
//!
//! - **the six declared tables tile the space between the code and the
//!   parameters exactly** - no gap, no overlap - which is what says the
//!   literal, uniform-image and container strides are all read correctly at
//!   once, since one wrong stride shifts every table after it;
//! - **the last parameter name's NUL is the program's last byte**, which is
//!   what says the parameter stride is 16 and the name offsets are
//!   self-relative;
//! - **`+0x74` re-states the primary program's end** and agrees with
//!   `+0x40 + 8 * +0x3c` every time, which is what fixes the instruction
//!   width at 8 bytes without decoding a single instruction.
//!
//! # The corpus size is pinned too
//!
//! A sweep whose interesting answer is a count has to fail on "walked
//! nothing" rather than return a tidy zero, so [`WALKED`] - every file opened,
//! not just the 1,262 that carry one - and [`PROGRAMS`] are asserted
//! alongside the decode rate.
//!
//! # Three magics are not containers, and are counted apart
//!
//! `GXP\0` is four bytes and 97,902 of them turn up; three sit inside
//! `.probes` files and declare a "size" that runs past the file holding them.
//! They are counted as rejected magics rather than as failed programs -
//! folding them together would let a real regression hide inside a known false
//! positive.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_rcs::gxp::{self, Category, Error, Program};

/// The executable and the three packages, in sweep order.
const SOURCES: [&str; 5] = [
    "vita/PCSF00007/base/eboot.elf",
    "vita/PCSF00007/patch-v104/eboot.elf",
    "vita/PCSF00007/base/PSP2/data.psarc",
    "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

/// Every file the sweep opens: `eboot.elf` plus every entry of all three
/// archives. Pinned so that a sweep which walks nothing fails rather than
/// returning a tidy zero.
const WALKED: usize = 22_056;

/// Of those, the ones that carry at least one container that decodes. Three
/// further files hold a stray magic and nothing else.
const CARRYING: usize = 1262;

/// Containers found, the three stray magics excluded.
const PROGRAMS: usize = 97_899;

/// Magics that are not containers at all. All three are inside `.probes`.
const NOT_CONTAINERS: usize = 3;

/// The vertex / fragment split, by bit 0 of `+0x14`.
const VERTEX: usize = 78_989;
const FRAGMENT: usize = 18_910;

/// Programs embedded in each build's executable. The two disagree, which is
/// the whole reason both are swept.
const BASE_EBOOT_PROGRAMS: usize = 111;
const PATCH_EBOOT_PROGRAMS: usize = 67;

fn source(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted")
        .join(name);
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// What one sweep of the corpus found.
#[derive(Default)]
struct Survey {
    walked: usize,
    carrying: usize,
    programs: usize,
    not_containers: usize,
    vertex: usize,
    fragment: usize,
    parameters: usize,
    /// Gap between the last code block and the primary program, per program.
    padding: BTreeMap<usize, usize>,
    /// Programs that bind a sampler of that name.
    samplers: BTreeMap<String, usize>,
}

/// Walks every source, handing each decoded program to `check`.
fn survey(check: &mut impl FnMut(&str, &Program)) -> Survey {
    let mut out = Survey::default();
    for name in SOURCES {
        let Some(path) = source(name) else { continue };
        if path.extension().is_some_and(|e| e == "elf") {
            let blob = std::fs::read(&path).expect("reading the executable");
            out.walked += 1;
            sweep(&mut out, &path.display().to_string(), &blob, check);
            continue;
        }
        let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
            .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
        let mut entries: Vec<String> = archive.paths().to_vec();
        entries.sort();
        for entry in entries {
            let blob = archive
                .read_path(&entry)
                .unwrap_or_else(|e| panic!("reading {entry}: {e}"));
            out.walked += 1;
            sweep(&mut out, &entry, &blob, check);
        }
    }
    out
}

fn sweep(out: &mut Survey, label: &str, blob: &[u8], check: &mut impl FnMut(&str, &Program)) {
    let found = gxp::programs(blob);
    if found.iter().any(|(_, p)| p.is_ok()) {
        out.carrying += 1;
    }
    for (at, decoded) in found {
        match decoded {
            Err(Error::NotAContainer { .. }) => out.not_containers += 1,
            Err(e) => panic!("{label} @{at:#x}: {e}"),
            Ok(program) => {
                out.programs += 1;
                if program.is_fragment() {
                    out.fragment += 1;
                } else {
                    out.vertex += 1;
                }
                out.parameters += program.parameters.len();
                *out.padding.entry(program.unaccounted).or_default() += 1;
                for p in &program.parameters {
                    if p.category == Category::Sampler {
                        *out.samplers.entry(p.name.clone()).or_default() += 1;
                    }
                }
                check(label, &program);
            }
        }
    }
}

/// Every container on the disc decodes, and every closure check holds.
///
/// [`Program::parse`] refuses anything that does not close, so a program that
/// decodes at all is one whose header accounts for itself exactly.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn every_gxp_program_on_the_disc_decodes_and_closes() {
    let mut versions = BTreeMap::new();
    let survey = survey(&mut |_, program| {
        *versions.entry(program.version).or_insert(0usize) += 1;
    });
    if survey.walked == 0 {
        return;
    }
    assert_eq!(
        survey.walked, WALKED,
        "the corpus moved; the counts below are stale"
    );
    assert_eq!(survey.carrying, CARRYING);
    assert_eq!(survey.programs, PROGRAMS);
    assert_eq!(survey.not_containers, NOT_CONTAINERS);
    assert_eq!(survey.vertex + survey.fragment, PROGRAMS);
    assert_eq!(survey.vertex, VERTEX);
    assert_eq!(survey.fragment, FRAGMENT);
    // One toolchain built the whole disc, which is why one header layout fits
    // all of it.
    assert_eq!(versions, BTreeMap::from([((1, 4), PROGRAMS)]));
}

/// The only bytes no header field claims are 4 or 8 of alignment padding.
///
/// Stated as a distribution rather than as a rule the parser enforces: a third
/// value would be a fact about the container worth learning, not a decode
/// failure.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn the_only_unaccounted_bytes_are_alignment_padding() {
    let survey = survey(&mut |_, _| {});
    if survey.walked == 0 {
        return;
    }
    let widths: Vec<usize> = survey.padding.keys().copied().collect();
    assert_eq!(widths, vec![4, 8], "padding widths: {:?}", survey.padding);
    assert_eq!(survey.padding.values().sum::<usize>(), PROGRAMS);
}

/// Bit 0 of `+0x14` says fragment, and the evidence is one check.
///
/// A vertex program declares vertex inputs and a fragment program cannot, so
/// the bit and [`Category::Attribute`] must agree on every one of the 97,899.
///
/// The tempting second check is asserted here too, as the thing it actually
/// is rather than as corroboration: the two builds embed **111** and **67**
/// programs against the **same 67** `_vp`/`_fp` shader-name strings, so a
/// container-to-name correspondence is not structural, however neatly the
/// patch build's 32 / 35 split lines up with the 32 `_vp` / 35 `_fp` names.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn the_fragment_bit_agrees_with_what_each_program_declares() {
    let mut disagreed = Vec::new();
    let survey = survey(&mut |label, program| {
        let attributes = program
            .parameters
            .iter()
            .any(|p| p.category == Category::Attribute);
        if attributes == program.is_fragment() {
            disagreed.push(format!("{label}: flags {:#010x}", program.flags));
        }
    });
    if survey.walked == 0 {
        return;
    }
    assert!(
        disagreed.is_empty(),
        "{} disagreed: {:?}",
        disagreed.len(),
        &disagreed[..1]
    );

    assert_eq!(embedded(SOURCES[0]), Some(BASE_EBOOT_PROGRAMS));
    assert_eq!(embedded(SOURCES[1]), Some(PATCH_EBOOT_PROGRAMS));
}

/// How many containers one executable embeds.
fn embedded(name: &str) -> Option<usize> {
    let blob = std::fs::read(source(name)?).expect("reading the executable");
    Some(
        gxp::programs(&blob)
            .iter()
            .filter(|(_, p)| p.is_ok())
            .count(),
    )
}

/// What `docs/rendering/shadows.md` was blocked on: 2048's shadow term is a
/// **single-channel sampler** a circuit material binds next to its lightmap,
/// not a depth map the renderer projects.
///
/// `shadowMap` is bound by 1,799 programs across the three packages and
/// is `comp = 1` on every one of them, while `lightmap` beside it is `comp =
/// 4`. Neither count is guessable from the executable's string table, which is
/// all that page had.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn the_shadow_term_is_a_single_channel_sampler_beside_the_lightmap() {
    let mut wide = 0usize;
    let survey = survey(&mut |_, program| {
        if let Some(p) = program.parameter("shadowMap")
            && p.components != 1
        {
            wide += 1;
        }
    });
    if survey.walked == 0 {
        return;
    }
    assert_eq!(
        wide, 0,
        "shadowMap is not single-channel on {wide} programs"
    );
    assert_eq!(
        survey.samplers.get("shadowMap").copied(),
        Some(SHADOW_MAP_BINDINGS)
    );
    assert_eq!(
        survey.samplers.get("lightmap").copied(),
        Some(LIGHTMAP_BINDINGS)
    );
}

/// Fragment programs binding `shadowMap`, across all three packages.
const SHADOW_MAP_BINDINGS: usize = 1_799;

/// Fragment programs binding `lightmap`, for scale.
const LIGHTMAP_BINDINGS: usize = 4_467;
