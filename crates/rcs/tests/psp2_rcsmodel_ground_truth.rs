//! Validates the [`rcsmodel::psp2`](oag_rcs::rcsmodel::psp2) decoder against
//! every `.rcsmodel` Wipeout 2048 ships.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! The container was settled by arithmetic that cannot come out even unless the
//! reading is right, and these tests are that argument over **993 files**:
//!
//! - **every file's sections add up to its own length**, which is what says the
//!   header's section table is read correctly;
//! - **every submesh's index buffer is exactly its own contents**, `count * 2`
//!   bytes rounded up to four, with the count divisible by three;
//! - **every vertex buffer's length divides by its own vertex count**, which is
//!   the only thing that gives a stride - no field states one;
//! - **no index names a vertex outside its own submesh**, over 33 million of
//!   them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_rcs::rcsmodel::psp2;

const PACKAGES: [&str; 3] = [
    "vita/PCSF00007/base/PSP2/data.psarc",
    "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn package(name: &str) -> Option<PathBuf> {
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
    files: usize,
    without_geometry: usize,
    submeshes: usize,
    triangles: usize,
    vertices: usize,
    strides: BTreeMap<usize, usize>,
    unpaired: usize,
}

fn survey(check: &mut impl FnMut(&str, &psp2::Model)) -> Survey {
    let mut out = Survey::default();
    for name in PACKAGES {
        let Some(path) = package(name) else { continue };
        let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
            .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
        let mut entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
            .cloned()
            .collect();
        entries.sort();
        for entry in entries {
            let blob = archive
                .read_path(&entry)
                .unwrap_or_else(|e| panic!("reading {entry}: {e}"));
            let decoded = psp2::parse(&blob).unwrap_or_else(|e| panic!("{entry}: {e}"));
            out.files += 1;
            if !decoded.has_geometry() {
                out.without_geometry += 1;
            }
            out.submeshes += decoded.submeshes.len();
            out.unpaired += decoded.unpaired_pointers;
            for mesh in &decoded.submeshes {
                out.triangles += mesh.triangle_count();
                out.vertices += mesh.positions.len();
                *out.strides.entry(mesh.stride).or_default() += 1;
            }
            check(&entry, &decoded);
        }
    }
    out
}

/// Every shipped file decodes, and the decode accounts for the whole file.
///
/// [`psp2::parse`] refuses sections that do not add up, so a file that parses at
/// all is one whose section table was read correctly.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn every_shipped_model_decodes_and_its_sections_add_up() {
    let found = survey(&mut |name, decoded| {
        assert!(!decoded.sections.is_empty(), "{name}: no sections at all");
        assert!(
            decoded.sections.len() <= 2,
            "{name}: {} sections, and only the CPU/GPU pair is understood",
            decoded.sections.len()
        );
    });
    if found.files == 0 {
        return;
    }
    println!(
        "{} file(s), {} with no GPU section, {} submesh(es), {} triangle(s), {} vertices",
        found.files, found.without_geometry, found.submeshes, found.triangles, found.vertices
    );
    println!("vertex strides: {:?}", found.strides);
    println!(
        "{} GPU pointer(s) this reading does not account for",
        found.unpaired
    );
    assert_eq!(found.files, 993, "the three EU packages ship 993 .rcsmodel");
    assert!(found.submeshes > 200_000, "{} submeshes", found.submeshes);
}

/// Every submesh's buffers close on its own two counts.
///
/// The index buffer is the check that makes finding records through the
/// relocation table safe: it is `index_count * 2` bytes rounded up to four, and
/// the count divides by three. The vertex buffer's length divides by the vertex
/// count, which is the **only** source for a stride - the format states none.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn every_submesh_closes_on_its_own_counts() {
    let found = survey(&mut |name, decoded| {
        for mesh in &decoded.submeshes {
            assert_eq!(
                mesh.indices.len() % 3,
                0,
                "{name}: {} indices is not a triangle list",
                mesh.indices.len()
            );
            assert!(
                mesh.stride >= 12 && mesh.stride % 4 == 0,
                "{name}: stride {} is not a word-aligned vertex",
                mesh.stride
            );
            for &index in &mesh.indices {
                assert!(
                    usize::from(index) < mesh.positions.len(),
                    "{name}: index {index} names one of {} vertices",
                    mesh.positions.len()
                );
            }
        }
    });
    if found.files == 0 {
        return;
    }
    // 16 to 64 in steps of four, on the whole corpus. A stride outside that is
    // not forbidden by the format - it would just be new, and worth seeing.
    for stride in found.strides.keys() {
        assert!(
            (16..=64).contains(stride),
            "stride {stride} is outside the range the corpus showed"
        );
    }
}

/// Positions decode as a finite `f32[3]` at each vertex's `+0`.
///
/// Asserted as a **rate** rather than absolutely: 456 of 33,335,682 vertices
/// across the corpus land outside a generous world box, and those are recorded
/// rather than explained - see `docs/formats/2048-rcsmodel.md`. A reading that
/// was wrong about the offset would not miss by 0.0014 %.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn positions_decode_as_a_float_triple_at_the_start_of_every_vertex() {
    let mut sane = 0usize;
    let mut total = 0usize;
    let found = survey(&mut |_, decoded| {
        for position in decoded.positions() {
            total += 1;
            if position.iter().all(|c| c.is_finite() && c.abs() < 1.0e5) {
                sane += 1;
            }
        }
    });
    if found.files == 0 {
        return;
    }
    println!("{sane}/{total} vertices decode as a finite in-range f32[3]");
    assert!(
        sane * 10_000 >= total * 9_999,
        "only {sane} of {total} positions are in range"
    );
}

/// Normals decode as unit vectors - a self-consistency check the whole
/// corpus can run, not just the twelve HD-ported circuits.
///
/// **The real confirmation is elsewhere**: an index-exact vertex
/// correspondence against Wipeout HD (1,504 vertices, zero ambiguity) scores
/// this exact decode - three signed bytes, `byte/127.0`, x/y/z at
/// [`psp2::NORMAL_OFFSET`] - at 100% within 18 degrees, mean dot 0.994,
/// against every other candidate tried at or below chance. See
/// `docs/formats/2048-rcsmodel.md`. What this test adds is reach: the ported
/// circuits are 12 of 993 files, and unit length is a property every decoded
/// normal should have regardless of which file it came from.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn normals_decode_as_unit_vectors() {
    let mut unit = 0usize;
    let mut total = 0usize;
    let found = survey(&mut |_, decoded| {
        for normal in decoded.normals() {
            total += 1;
            let len = (0..3).map(|a| normal[a] * normal[a]).sum::<f32>().sqrt();
            if (len - 1.0).abs() < 0.02 {
                unit += 1;
            }
        }
    });
    if found.files == 0 {
        return;
    }
    println!("{unit}/{total} normals are unit-length within 2%");
    assert!(total > 0, "no normals decoded at all");
    assert!(
        unit * 100 >= total * 99,
        "only {unit} of {total} normals are unit-length"
    );
}

/// A circuit's positions are world space and a craft's are its own.
///
/// The check that says a caller composes no transform onto either: `altima`'s
/// track model spans the box its `track_col.col` sibling states, and `Assegai`'s
/// hull sits inside a few units of its own origin.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn a_circuit_is_world_space_and_a_craft_is_not() {
    let Some(path) = package(PACKAGES[0]) else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .expect("opening the base package");
    let mut bounds = |entry: &str| {
        let blob = archive
            .read_path(entry)
            .unwrap_or_else(|e| panic!("{entry}: {e}"));
        let decoded = psp2::parse(&blob).unwrap_or_else(|e| panic!("{entry}: {e}"));
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for position in decoded.positions() {
            for a in 0..3 {
                lo[a] = lo[a].min(position[a]);
                hi[a] = hi[a].max(position[a]);
            }
        }
        (lo, hi)
    };

    let (lo, hi) = bounds("data/art/published/environments/altima/track.rcsmodel");
    // Altima's own collision file states a box centred near the origin and
    // roughly 2,000 x 770 x 2,500 units across; the render geometry is the
    // scenery too, so it is larger and in the same place.
    assert!(lo[0] < -900.0 && hi[0] > 700.0, "x {}..{}", lo[0], hi[0]);
    assert!(
        hi[1] > 500.0,
        "a circuit reaches well above the road: y {}",
        hi[1]
    );
    assert!(lo[2] < -700.0 && hi[2] > 1400.0, "z {}..{}", lo[2], hi[2]);

    let (lo, hi) = bounds("data/art/published/hdships/Assegai/ship.rcsmodel");
    for a in 0..3 {
        assert!(
            lo[a] > -20.0 && hi[a] < 20.0,
            "a craft is authored about its own origin: axis {a} spans {}..{}",
            lo[a],
            hi[a]
        );
    }
}
