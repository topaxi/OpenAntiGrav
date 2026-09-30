//! Validates that a PS4 submesh record points at its own vertex declaration,
//! against every `.rcsmodel` in Wipeout: Omega Collection's five base archives.
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
//! Keyed by stride, one declaration stood for every chunk of that stride, and
//! on a PS4 circuit that read the diffuse coordinate out of the wrong bytes of
//! 285 of `tech_de_ra`'s 3,186 submeshes (and, for the reversed circuit,
//! decoded 218,672 vertices to non-finite floats). The record's own 8-byte
//! pointer at [`PS4_DECLARATION_POINTER`] is the address of its declaration's
//! header. Two invariants, over every submesh of every file, that a wrong
//! pointer offset cannot keep:
//!
//! - **the pointer resolves, and the stride agrees**: the declaration it
//!   reaches states the stride the buffer packing independently gave;
//! - **the control group**: a submesh's material names an `-lmap.gnf` *if and
//!   only if* its own declaration carries `lightmapUV`. The material half is
//!   read out of the material's own extent and the declaration half out of the
//!   pointer, and neither knows about the other.
//!
//! One archive per test, so the sweep parallelises across tests.

use std::path::{Path, PathBuf};

use oag_rcs::rcsmodel::psp2::{self, vertex_decl};

fn package(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/ps4/omega-eu/uroot")
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

#[derive(Default, Debug, PartialEq, Eq)]
struct Survey {
    files: usize,
    submeshes: usize,
    /// Records whose pointer reached a declaration of the record's own stride.
    resolved: usize,
    /// Submeshes whose own declaration carries `lightmapUV`.
    lightmapped: usize,
    /// Files that state a material count or yielded a material.
    table_files: usize,
    /// Of those, files where the materials read are not exactly the count stated.
    table_disagrees: usize,
    /// Files with at least one such submesh.
    files_with_a_lightmap: usize,
    /// Submeshes on which the two halves of the control group disagree, by file.
    off_diagonal: std::collections::BTreeMap<String, usize>,
}

fn sweep(name: &str) -> Option<Survey> {
    let path = package(name)?;
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let models: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
        .cloned()
        .collect();
    let mut survey = Survey::default();
    for model_path in models {
        let blob = archive.read_path(&model_path).expect("the model reads");
        let model = psp2::parse(&blob).unwrap_or_else(|e| panic!("{model_path}: {e}"));
        let cpu = model.sections[0];
        let cpu_bytes = &blob[cpu.at..cpu.at + cpu.len];
        // The count the header states at `+0x70`, against the materials read.
        let stated = cpu_bytes
            .get(0x70..0x78)
            .map_or(0, |b| u64::from_le_bytes(b.try_into().unwrap()));
        if stated != 0 || !model.materials.is_empty() {
            let agrees = usize::try_from(stated).is_ok_and(|n| n == model.materials.len());
            survey.table_disagrees += usize::from(!agrees);
            survey.table_files += 1;
        }
        if model.submeshes.is_empty() {
            continue;
        }
        survey.files += 1;
        let declarations = vertex_decl::find_by_header(cpu_bytes);
        let mut any = false;
        for s in &model.submeshes {
            survey.submeshes += 1;
            let at = s.record + vertex_decl::PS4_DECLARATION_POINTER;
            let word = u64::from_le_bytes(cpu_bytes[at..at + 8].try_into().unwrap());
            let Some(decl) = usize::try_from(word)
                .ok()
                .and_then(|w| declarations.get(&w))
                .filter(|d| d.stride == s.stride)
            else {
                continue;
            };
            survey.resolved += 1;
            let has_uv = decl.lightmap_texcoord().is_some();
            survey.lightmapped += usize::from(has_uv);
            any |= has_uv;
            if let Some(m) = s.material {
                let names_one = model.materials[m]
                    .textures
                    .iter()
                    .any(|t| t.to_ascii_lowercase().ends_with("-lmap.gnf"));
                if names_one != has_uv {
                    *survey
                        .off_diagonal
                        .entry(format!("{model_path} names:{names_one} decl:{has_uv}"))
                        .or_default() += 1;
                }
            }
        }
        survey.files_with_a_lightmap += usize::from(any);
    }
    Some(survey)
}

/// Every record of every file resolves its own declaration, and the control
/// group is exact: no submesh is off the diagonal.
fn every_record_resolves(name: &str, files: usize, submeshes: usize, lightmapped: usize) {
    let Some(survey) = sweep(name) else { return };
    assert_eq!(
        survey.resolved, survey.submeshes,
        "{name}: a record's pointer did not reach a declaration of its own stride"
    );
    assert_eq!(
        survey.table_disagrees, 0,
        "{name}: a file's materials are not the table its header states"
    );
    assert!(
        survey.off_diagonal.is_empty(),
        "{name}: material and declaration disagree about a lightmap: {:#?}",
        survey.off_diagonal
    );
    assert_eq!(
        (survey.files, survey.submeshes, survey.lightmapped),
        (files, submeshes, lightmapped),
        "{name}"
    );
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data00_records_point_at_their_own_declarations() {
    every_record_resolves("data00.psarc", 579, 38_362, 5_302);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data01_records_point_at_their_own_declarations() {
    every_record_resolves("data01.psarc", 304, 27_942, 8_056);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data02_records_point_at_their_own_declarations() {
    every_record_resolves("data02.psarc", 32, 50_858, 14_914);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data03_records_point_at_their_own_declarations() {
    every_record_resolves("data03.psarc", 143, 802, 0);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn data04_records_point_at_their_own_declarations() {
    every_record_resolves("data04.psarc", 40, 108_417, 21_770);
}
