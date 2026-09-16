//! Byte coverage of every `.pob` particle system on the PSP and PS2 discs.
//!
//! **`#[ignore]`d and never run in CI.** Needs game content this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # Why this is a separate sweep from `pob_ground_truth.rs`
//!
//! That test proves the emitter-tree *layout* against the corpus - every
//! record decodes to a sane schedule, on both platforms. This asks a
//! different question: once the tree is walked, what fraction of the file
//! did reaching it actually claim? `docs/formats/pob.md`'s own "Not
//! determined" already names the slot-resolved records as a dead end after
//! two passes, so [`oag_vex::pob_coverage::coverage`] measures around them
//! rather than trying a third: a string target is claimed, a non-string one
//! is an honest, reported gap.

use std::path::PathBuf;

use oag_assets::Archive;
use oag_vex::{pob, pob_coverage};

/// `.pob` blobs on the PSP disc, per `docs/formats/pob.md`.
const PSP_SYSTEMS: usize = 35;

/// And on the PS2 disc's main archive.
const PS2_SYSTEMS: usize = 41;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Every `SYSP` blob in `archive`, decompressed, in directory order. Same
/// discovery-by-content `pob_ground_truth.rs` uses, duplicated rather than
/// shared across the two test binaries.
fn particle_systems(archive: &mut Archive) -> Vec<Vec<u8>> {
    let count = archive.directory().entries.len();
    let mut out = Vec::new();
    for index in 0..count {
        let Ok(head) = archive.peek(index, 4) else {
            continue;
        };
        if !pob::looks_like_particle_system(&head) {
            continue;
        }
        let Ok(blob) = archive.read(index) else {
            continue;
        };
        out.push(blob);
    }
    out
}

/// Sweeps one archive's worth of `.pob` blobs and reports the corpus totals.
fn sweep(label: &str, spec: &str, expected: usize) {
    let mut archive = Archive::open(spec).expect("open archive");
    let blobs = particle_systems(&mut archive);
    assert_eq!(
        blobs.len(),
        expected,
        "{label}: found {} SYSP blobs, expected {expected}",
        blobs.len()
    );

    let (mut total, mut claimed, mut string_targets, mut non_string_gaps) =
        (0u64, 0u64, 0usize, 0usize);
    let mut worst: Option<(usize, String)> = None;
    for blob in &blobs {
        let seen = pob_coverage::coverage(blob);
        if seen.is_empty() {
            continue;
        }
        total += seen.len() as u64;
        claimed += seen.claimed() as u64;
        for gap in seen.gaps(1) {
            if gap.len < 4 {
                continue;
            }
            non_string_gaps += 1;
        }
        let gap_len: usize = seen.gaps(1).iter().map(|g| g.len).sum();
        if gap_len > 0 && worst.as_ref().is_none_or(|(n, _)| gap_len > *n) {
            worst = Some((gap_len, seen.describe(1)));
        }
        let system = pob::ParticleSystem::parse(blob).expect("parses, per pob_ground_truth.rs");
        for index in 0..system.slots.len() {
            if let Ok(Some(_)) = system.resolve_slot(blob, index) {
                string_targets += 1;
            }
        }
    }
    let pct = 100.0 * claimed as f64 / total.max(1) as f64;
    println!(
        "{label}: {} file(s), {claimed} of {total} byte(s) ({pct:.2}%), \
         {non_string_gaps} unclaimed run(s) of 4+ bytes, {string_targets} resolved slot(s) total",
        blobs.len()
    );
    if let Some((_, describe)) = &worst {
        println!("  worst: {describe}");
    }
    assert!(claimed > 0, "{label}: nothing claimed at all");
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn psp_particle_systems_close_around_the_named_dead_end() {
    let Some(image) = image("pulse-psp-usa.chd") else {
        return;
    };
    sweep(
        "psp",
        &format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()),
        PSP_SYSTEMS,
    );
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn ps2_particle_systems_close_the_same_way() {
    let Some(image) = image("pulse-ps2-eu.chd") else {
        return;
    };
    sweep(
        "ps2",
        &format!("{}:54748/WADS2.WAD", image.display()),
        PS2_SYSTEMS,
    );
}
