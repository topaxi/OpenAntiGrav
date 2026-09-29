//! Wipeout: Omega Collection races on its own data: its circuit's spline, its
//! collision, its craft, its geometry and its materials.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(omega_race_ground_truth)'
//! ```
//!
//! `OAG_OMEGA_SOURCE=<dir>` reads a different directory pair than
//! `data/extracted/ps4`.
//!
//! # These skip on the extraction they cannot be measured on
//!
//! **A short-read extraction of `omega-ps4-eu{,-patch}.pkg` is short of most of
//! its bytes** (`data/extracted/ps4` was one until 2026-09-29, and is not any
//! more) - `docs/formats/omega-status.md`, "An extraction-tool bug" - and
//! on it `tech_de_ra\track.vex` has no `WO Track` node at all, which is what
//! this crate reported as "a clean negative" for a month. Every test here
//! measures a property of the *corrected* extraction, so [`source`] opens the
//! tree it was pointed at (`$OAG_OMEGA_SOURCE`, else `data/extracted/ps4`),
//! checks that one file, and **skips with a printed reason - even under
//! `OAG_REQUIRE_GAME_DATA`** - when it is the corrupt one. That is deliberate:
//! the variable exists to turn a *missing* image into a failure, and a corrupt
//! copy is a known property of one maintainer's disk, not a missing image. A
//! green run that printed the skip is not a green run; `data/extracted/ps4`
//! holds the corrected extraction since 2026-09-29, so it should not print.

use std::path::{Path, PathBuf};

use oag_game::race;
use oag_vex::{kdcol, track, vex};

/// The directory pair to read, or `None` with the reason printed.
fn source() -> Option<String> {
    let path: PathBuf = match std::env::var_os("OAG_OMEGA_SOURCE") {
        // Relative to the repository, not to the crate a test binary runs in.
        Some(path) => oag_testdata::repo_root().join(path),
        None => oag_testdata::exact("data/extracted/ps4")?,
    };
    let source = path.display().to_string();
    let mut archives = oag_omega::open(&source)
        .unwrap_or_else(|e| panic!("opening the Omega source {source}: {e}"));
    let sentinel = archives
        .read_name(r"Data\environments\tech_de_ra\track.vex")
        .unwrap_or_default();
    let intact = vex::nodes(&sentinel)
        .ok()
        .is_some_and(|nodes| track::find_node(&sentinel, &nodes).is_some());
    if !intact {
        println!(
            "skipping: {source} is the short-read extraction - tech_de_ra\\track.vex has no \
             WO Track node there. Point OAG_OMEGA_SOURCE at a corrected one; see \
             docs/formats/omega-status.md"
        );
        return None;
    }
    Some(source)
}

/// Every `dataNN.psarc` under `source`, base and patch alike.
fn psarc_files(source: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && depth < 3 {
                walk(&path, out, depth + 1);
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("data") && n.ends_with(".psarc"))
            {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(Path::new(source), &mut out, 0);
    out.sort();
    out
}

/// The archive named `data{number}.psarc`, opened loose.
fn archive(source: &str, number: &str) -> oag_assets::psarc::Archive {
    let path = psarc_files(source)
        .into_iter()
        .find(|p| {
            p.file_name()
                .is_some_and(|n| *n == *format!("data{number}.psarc"))
        })
        .unwrap_or_else(|| panic!("no data{number}.psarc under {source}"));
    oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()))
}

/// `tech_de_ra`'s `WO Track` is 2 paths, 2 junctions and 835 control points,
/// and the arithmetic closes: the decoded length is the payload's own, to the
/// byte. The same reader HD, Pulse and 2048 use, unmodified.
#[test]
#[ignore = "needs the corrected Omega extraction"]
fn tech_de_ra_authors_a_spline_this_project_already_reads() {
    let Some(source) = source() else { return };
    let mut archives = oag_omega::open(&source).expect("open");
    let blob = archives
        .read_name(r"Data\environments\tech_de_ra\track.vex")
        .expect("read track.vex");
    let nodes = vex::nodes(&blob).expect("walk the nodes");
    let node = track::find_node(&blob, &nodes).expect("a WO Track node");
    let payload = &blob[node.payload()];
    let ai = track::parse(payload).expect("parse the WO Track");
    assert_eq!(ai.paths.len(), 2);
    assert_eq!(ai.junctions.len(), 2);
    assert_eq!(ai.point_count(), 835);
    assert_eq!(ai.encoded_len(), payload.len());
    assert_eq!(
        track::byte_order(payload),
        Some(oag_formats::ByteOrder::Little)
    );
}

/// All 38 `track_col.col` files decode as the packed tree, each with its leaf
/// runs tiling its leaf array exactly - the invariant that says the packed
/// node's count and start fields are the ones a 2048 node has.
#[test]
#[ignore = "needs the corrected Omega extraction"]
fn every_omega_collision_file_decodes_and_its_leaf_runs_tile() {
    let Some(source) = source() else { return };
    let mut seen = 0;
    for number in ["00", "01", "02", "03", "04", "05", "07", "08"] {
        let mut archive = archive(&source, number);
        let names: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().contains("/track_col"))
            .filter(|p| p.to_ascii_lowercase().ends_with(".col"))
            .cloned()
            .collect();
        for name in names {
            let blob = archive.read_path(&name).expect("read");
            let decoded = kdcol::parse(&blob).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(decoded.layout, kdcol::NodeLayout::Packed, "{name}");
            let mut runs: Vec<(usize, usize)> = decoded
                .nodes
                .iter()
                .filter(|n| n.is_leaf())
                .map(|n| (n.first_leaf, n.triangle_count))
                .collect();
            runs.sort_unstable();
            let mut at = 0;
            for (first, count) in runs {
                assert_eq!(first, at, "{name}: a leaf run starts at {first}, not {at}");
                at += count;
            }
            assert_eq!(at, decoded.leaves.len(), "{name}: the runs stop short");
            seen += 1;
        }
    }
    assert_eq!(seen, 38, "38 collision files across the nine archives");
}

/// Omega's `environments2048\altima\track_col.col` is 2048's own file with
/// the node array re-encoded: everything after it - leaf indices, bounds, the
/// triangle soup and the final tag, 415,286 bytes - is byte-identical, and the
/// file is smaller by exactly five bytes a node. The evidence that the packed
/// layout is a re-encoding and not a different format.
#[test]
#[ignore = "needs the corrected Omega extraction and the extracted 2048 package"]
fn altima_collision_is_2048s_file_with_five_bytes_less_a_node() {
    let Some(source) = source() else { return };
    let Some(vita) = oag_testdata::exact("data/extracted/vita/PCSF00007/base/PSP2/data.psarc")
    else {
        return;
    };
    let mut vita = oag_assets::psarc::Archive::open(&vita.display().to_string()).expect("open");
    let theirs = vita
        .read_path("data/art/published/environments/altima/track_col.col")
        .expect("2048's altima collision");
    let mut omega = oag_omega::open(&source).expect("open");
    let ours = omega
        .read_name(r"Data\environments2048\altima\track_col.col")
        .expect("Omega's altima collision");

    let nodes = |file: &[u8]| {
        let stride = u32::from_le_bytes(file[12..16].try_into().unwrap()) as usize;
        let count = u32::from_le_bytes(file[16..20].try_into().unwrap()) as usize;
        (stride, count, 0x14 + stride * count)
    };
    let (their_stride, their_count, their_end) = nodes(&theirs);
    let (our_stride, our_count, our_end) = nodes(&ours);
    assert_eq!((their_stride, our_stride), (24, 19));
    assert_eq!(their_count, our_count);
    assert_eq!(their_count, 27_199);
    assert_eq!(theirs.len() - ours.len(), 5 * their_count);
    assert_eq!(theirs[their_end..], ours[our_end..], "past the node array");
}

/// The Omega hulls that 2048 also ships as HD ports decode, through the same
/// reader, to the same submesh, triangle and material counts. Not the same
/// vertex counts - Omega re-welded them, by +41 on Feisar and -496 on Qirex -
/// so those are deliberately not compared.
#[test]
#[ignore = "needs the corrected Omega extraction and the extracted 2048 package"]
fn omegas_hd_hulls_match_2048s_ports_on_submeshes_triangles_and_materials() {
    let Some(source) = source() else { return };
    let Some(vita) = oag_testdata::exact("data/extracted/vita/PCSF00007/base/PSP2/data.psarc")
    else {
        return;
    };
    let mut vita = oag_assets::psarc::Archive::open(&vita.display().to_string()).expect("open");
    let mut omega = archive(&source, "01");
    for (team, submeshes, triangles, materials) in
        [("Feisar", 17, 22_527, 5), ("Qirex", 15, 29_643, 4)]
    {
        let read = |archive: &mut oag_assets::psarc::Archive, path: String| {
            let blob = archive
                .read_path(&path)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            oag_rcs::rcsmodel::psp2::parse(&blob).unwrap_or_else(|e| panic!("{path}: {e}"))
        };
        let theirs = read(
            &mut vita,
            format!("data/art/published/hdships/{team}/ship.rcsmodel"),
        );
        let ours = read(
            &mut omega,
            format!("Data/art/published/hdships/{team}/ship.rcsmodel"),
        );
        for (label, model) in [("2048", &theirs), ("Omega", &ours)] {
            let tris: usize = model.submeshes.iter().map(|s| s.triangle_count()).sum();
            assert_eq!(
                (model.submeshes.len(), tris, model.materials.len()),
                (submeshes, triangles, materials),
                "{team} on {label}"
            );
            assert_eq!(model.unpaired_pointers, 0, "{team} on {label}");
        }
    }
}

/// Every `.rcsmodel` in one archive decodes through the 2048 reader: the three
/// sections add up to the file, the relocation sites pair into submeshes (with
/// the handful of leftovers each archive is known to have), and every
/// submesh names a material inside the table. Its own test per archive so the
/// nine run in parallel instead of end to end.
fn census(number: &str, files: usize, submeshes: usize, unpaired: usize) {
    let Some(source) = source() else { return };
    let mut archive = archive(&source, number);
    let names: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
        .cloned()
        .collect();
    let (mut seen, mut found, mut leftover) = (0, 0, 0);
    for name in &names {
        let blob = archive.read_path(name).expect("read");
        assert!(oag_rcs::rcsmodel::psp2::is_ps4(&blob), "{name}: not PS4's");
        let model = oag_rcs::rcsmodel::psp2::parse(&blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        seen += 1;
        found += model.submeshes.len();
        leftover += model.unpaired_pointers;
        for s in &model.submeshes {
            assert!(s.material.is_some(), "{name}: a submesh names no material");
        }
    }
    assert_eq!(
        (seen, found, leftover),
        (files, submeshes, unpaired),
        "data{number}"
    );
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data00_rcsmodels_decode_with_only_the_crowd_rigs_unpaired() {
    census("00", 625, 38_362, 82);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data01_rcsmodels_decode_and_close_completely() {
    census("01", 316, 27_942, 0);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data02_rcsmodels_decode_and_close_completely() {
    census("02", 32, 50_858, 0);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data03_rcsmodels_decode_and_close_completely() {
    census("03", 147, 802, 0);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data04_rcsmodels_decode_with_one_circuit_leaving_two_unpaired() {
    census("04", 40, 108_417, 2);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data08_rcsmodels_decode_with_the_front_end_scenes_leaving_six_unpaired() {
    census("08", 110, 12_708, 6);
}

/// The patch's `.EnvSettings` is the newer copy and the mount order serves it:
/// 29 of 40 sampled race-relevant collisions between the base and the patch
/// differ, and the tripwire below names which archive wins so a reorder of
/// `oag_omega::EXTRA_CANDIDATES` fails here by name.
#[test]
#[ignore = "needs the corrected Omega extraction"]
fn the_patch_copy_of_a_circuits_envsettings_is_the_one_served() {
    let Some(source) = source() else { return };
    let archives = oag_omega::open(&source).expect("open");
    let served = archives
        .locate(r"Data\environments\tech_de_ra\track.EnvSettings")
        .expect("served by something");
    assert!(
        served.contains("data08.psarc"),
        "tech_de_ra's .EnvSettings is served by {served}, expected the patch's data08.psarc"
    );
}

/// **A race starts on Omega's own data**, with the derived ribbon standing in
/// for the circuit's geometry so the test does not decode 460 circuit
/// textures: the real spline, the real collision and the real craft.
///
/// Slow (about a minute and a half unoptimised, all of it the eight hulls'
/// textures) and one test for that reason rather than several.
#[test]
#[ignore = "needs the corrected Omega extraction"]
fn an_omega_race_loads_its_spline_collision_and_craft() {
    let Some(source) = source() else { return };
    let loaded = race::load(&race::Options {
        source: source.clone(),
        ribbon: true,
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading {source}: {e:#}"));
    let said = |needle: &str| loaded.report.iter().any(|line| line.contains(needle));

    assert!(
        said("racing on Wipeout: Omega Collection"),
        "{:#?}",
        loaded.report
    );
    assert!(
        said(
            r"Data\environments\tech_de_ra\track.vex: 2 path(s), 2 junction(s), 835 control point(s)"
        ),
        "{:#?}",
        loaded.report
    );
    assert!(
        said("track_col.col: 12894 vertices, 20777 triangles, 29154 k-d node(s)"),
        "{:#?}",
        loaded.report
    );
    assert!(
        said(
            r"hdships\ag_systems\Ship.vex: 22666 triangle(s) over 12 submesh(es), 12/12 draw(s) textured from 4 of 4 material(s)"
        ),
        "{:#?}",
        loaded.report
    );
    assert_eq!(loaded.setup.ai.paths.len(), 2);
    assert_eq!(loaded.setup.ai.point_count(), 835);
    assert!(!loaded.liveries.is_empty());
    assert!(loaded.liveries[0].hull.indices.len() >= 22_666 * 3);
}
