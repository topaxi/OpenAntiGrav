//! How much of each file this crate's parsers actually read.
//!
//! # The bug this exists to catch
//!
//! **A hand-written parser cannot fail on a field it does not know about.**
//! `rcsmodel` read the surface record embedded in a chunk header and returned;
//! the count at `+0x10` and the offset table at `+0x18` naming the other
//! surface records went unread, and a quarter of the disc's chunks silently
//! lost their remaining geometry - 25,972 submeshes and 7.1 million triangles.
//! Nothing errored. Every diagnostic this project had compared what it drew
//! against what it decided to draw, which cannot surface an absence.
//!
//! A coverage sweep can, and this is it. It is a **ratchet, not a target**:
//! real files carry padding, string pools and sections this project has
//! decided not to decode, so the floors below are what holds today and the
//! assertion is that it does not get worse.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

mod rcsmodel_common;

use oag_formats::{rcsmodel, vex};
use rcsmodel_common::image;

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// Runs `each` over every entry on the disc whose path ends in `suffix`.
fn for_each(suffix: &str, mut each: impl FnMut(&str, &[u8])) {
    let image = image().expect("checked by the caller");
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(suffix))
            .cloned()
            .collect();
        for path in paths {
            if let Ok(blob) = open.read_path(&path) {
                each(&path, &blob);
            }
        }
    }
}

/// **The `.rcsmodel` reader reaches 97.7% of the disc, and must not reach less.**
///
/// A floor rather than an equality, because coverage rising is the direction
/// this is for - and it already has. The sweep's first run read 96.06%, and
/// following its largest gap found that `rcsmodel::STRIDES` listed three of
/// the seven widths the disc declares, so a chunk of one of the other four
/// with no declaration could not be solved by any search. Widening it to the
/// declared set took the disc to **97.69%** and the surfaces no rule can
/// decode from 371 to 115. See `rcsmodel_stride_ground_truth.rs`.
///
/// What is left is string pools, alignment slack, and the variable-length tail
/// of a material record: this crate reads the first `0x18` bytes of one that
/// runs to 768.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_rcsmodel_reader_reaches_nearly_all_of_every_file() {
    if image().is_none() {
        return;
    }
    let (mut total, mut read) = (0u64, 0u64);
    let mut worst: Option<(f64, String)> = None;
    let mut named: Option<f64> = None;
    for_each(".rcsmodel", |path, blob| {
        let seen = rcsmodel::coverage(blob);
        if seen.is_empty() {
            return;
        }
        total += seen.len() as u64;
        read += seen.claimed() as u64;
        if path == "/data/environments/talons_junction/track.rcsmodel" {
            named = Some(seen.fraction());
            println!("{path}\n{}", seen.describe(64));
        }
        if worst.as_ref().is_none_or(|(f, _)| seen.fraction() < *f) {
            worst = Some((seen.fraction(), path.to_string()));
        }
    });
    let overall = read as f64 / total as f64;
    println!(
        "{read} of {total} .rcsmodel byte(s) read ({:.2}%); worst file {:?}",
        overall * 100.0,
        worst,
    );
    assert!(
        overall >= 0.972,
        "the reader now reaches {:.2}% of the disc's .rcsmodel bytes, down from 97.69%",
        overall * 100.0,
    );
    let named = named.expect("Talon's Junction is on the disc");
    assert!(
        named >= 0.975,
        "Talon's Junction's own model is {:.2}% read, down from 98.08%",
        named * 100.0,
    );
}

/// **The `.vex` node walk reaches every byte of every file, and must keep
/// doing so.**
///
/// A hard assertion rather than a floor, because it currently holds exactly:
/// the file header declares a tree length and a texture length, and those two
/// plus the header are the whole file. So the failure mode that cost `rcsmodel`
/// 40% of its geometry - a region nothing visits - does not exist here.
///
/// **This test's own first run was a false alarm worth remembering.** It
/// reported 3.0 MB across 666 files as unreachable; that was the embedded
/// texture block the header names at `+0x08`, which a different entry point
/// reads. A gap is a lead, and the first thing to check about one is whether
/// the format's own header already accounts for it.
///
/// What this does **not** check is whether a *payload* is fully read. Most
/// `.vex` node classes are undecoded on purpose, so per-class coverage would
/// report a deliberate decision as a defect on every file.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_vex_node_walk_leaves_no_region_unvisited() {
    if image().is_none() {
        return;
    }
    let (mut files, mut unreached) = (0usize, Vec::new());
    for_each(".vex", |path, blob| {
        let seen = vex::coverage(blob);
        if seen.is_empty() {
            return;
        }
        files += 1;
        let gaps = seen.gaps(64);
        if !gaps.is_empty() {
            unreached.push((
                gaps.iter().map(|g| g.len).sum::<usize>(),
                path.to_string(),
                seen.describe(64),
            ));
        }
    });
    assert_eq!(files, 742, "every .vex on the disc");
    assert!(
        unreached.is_empty(),
        "{} file(s) carry a region the walk never reaches, worst:\n{}",
        unreached.len(),
        unreached
            .iter()
            .max_by_key(|(n, ..)| *n)
            .map(|(_, path, describe)| format!("{path}\n{describe}"))
            .unwrap_or_default(),
    );
}
