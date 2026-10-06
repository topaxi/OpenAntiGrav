//! Pulse's weather anchors, pinned against the disc and a live capture.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! Fort Gale White read live on PPSSPP 1.20.4 (software renderer,
//! 2026-10-02): `Weather_Update`'s covered mask `DAT_08b3bfd0`/`d4` was
//! `0x43fc78001c`. Its first anchor, at section 2, sat at
//! `(672.375, 115.587, -169.979)` in the weather node's own matrix block.
//! See `docs/ghidra/functions/psp-pulse-usa/weather.md`.

use oag_vex::vex;
use oag_vex::weather::{self, Anchor};

fn disc() -> Option<oag_assets::Archives> {
    let path = oag_testdata::image("pulse-psp-usa.chd")?;
    Some(oag_pulse::open(path.to_str().expect("utf-8 path")).expect("open"))
}

/// `None` where the disc has no such circuit file.
fn anchors_in(archives: &mut oag_assets::Archives, file: &str) -> Option<Vec<Anchor>> {
    let blob = archives
        .read_name(&format!(r"Data\Environments\{file}"))
        .ok()?;
    let nodes = vex::nodes(&blob).expect("nodes");
    Some(weather::anchors(&blob, &nodes))
}

#[test]
#[ignore = "needs a Pulse PSP disc image under data/images/"]
fn fort_gale_covers_the_sections_the_original_covered_live() {
    let Some(mut archives) = disc() else {
        return;
    };
    let anchors = anchors_in(&mut archives, r"14_Track\track.vex").expect("track.vex");
    assert_eq!(weather::covered_mask(&anchors), 0x43_fc78_001c);
    assert_eq!(anchors.len(), 16);
    let two = anchors.iter().find(|a| a.section == 2).expect("section 2");
    assert_eq!(&two.world[12..15], &[672.3753, 115.58707, -169.97923]);
}

#[test]
#[ignore = "needs a Pulse PSP disc image under data/images/"]
fn the_covered_test_counts_an_unpublished_section_as_covered() {
    assert!(weather::covered(0, -1));
    assert!(!weather::covered(0, 0));
    assert!(weather::covered(1 << 5, 5));
    assert!(!weather::covered(u64::MAX, 64));
}

#[test]
#[ignore = "needs a Pulse PSP disc image under data/images/"]
fn every_circuit_authors_anchors_though_only_two_read_them() {
    let Some(mut archives) = disc() else {
        return;
    };
    let mut with = Vec::new();
    for n in 1..=16 {
        for file in ["track.vex", "track_reversed.vex"] {
            let name = format!(r"{n:02}_Track\{file}");
            if anchors_in(&mut archives, &name).is_some_and(|a| !a.is_empty()) {
                with.push(n);
            }
        }
    }
    with.dedup();
    // Every circuit authors them; only 07 and 14 author a `Weather` element for
    // them to serve (`oag_tables::trackstartup`).
    assert_eq!(with, [1, 2, 3, 4, 5, 6, 7, 9, 10, 13, 14, 16]);
}
