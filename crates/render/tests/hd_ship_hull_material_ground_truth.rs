//! Every ship hull's drawn materials resolve to a shipped shader variant.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_ship_hull_material_ground_truth)'
//! ```
//!
//! # What this pins
//!
//! Before 2026-09-13, 78 of 178 drawn ship materials disc-wide (every
//! `/data/ships/*/ship.vex`) never resolved a shader variant, and every hull
//! drew part of itself through `mesh.wgsl`'s unresolved fallback. Measured
//! and diagnosed in `crates/render/examples/hd_ship_class_census.rs`: the
//! cause was `Features::chunk_word` treating a ship's `VertexColour1`
//! attribute as HD's baked-light colour set, asking every one of those
//! materials for the `IleVertex` shader permutation, which no ship material
//! on the disc ships. Fixed in `crates/rcs/src/rcsmodel/vertex_decl.rs` with
//! a new `light_colour_set()`, hash-restricted to the two hashes established
//! to be that colour set (`colorSet1`, the unnamed `0x1aaf7631`) - kept
//! separate from `vertex_colour()` itself, which stays a shape match because
//! `oag_game::livery::flare::alpha_ramp` reads `VertexColour1` through it for
//! the engine flame's own, unrelated alpha ramp.
//!
//! This test builds a sample of ship hulls through the real
//! `mesh::rcs::build` path - the code path `oag_game::livery` uses - and
//! pins that every drawn material resolves and every drawn chunk is covered,
//! so the regression this fixed cannot silently return.

use std::path::{Path, PathBuf};

use oag_mesh::mesh;

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// A sample of ship hulls: the two teams the handover thread's own repro
/// command names (`feisar_c1`, `feisar`), one more base team, and one Fury
/// `_n1` variant - enough to cover the `Static`/`RigidBody`-shared class
/// path and the `_c1`/`_n1`/base directory shapes without building all 37.
const SHIPS: &[(&str, &str)] = &[
    ("DATA06.PSARC", "/data/ships/feisar_c1/ship.vex"),
    ("DATA02.PSARC", "/data/ships/feisar/ship.vex"),
    ("DATA02.PSARC", "/data/ships/qirex/ship.vex"),
    ("DATA06.PSARC", "/data/ships/triakis_n1/ship.vex"),
];

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

fn build(image: &Path, archive: &str, path: &str) -> Option<(mesh::Model, mesh::rcs::Report)> {
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let data = mesh::read_blob(&spec, path).ok()?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    mesh::rcs::build(
        path,
        &data,
        &geometry,
        &mut |name| mesh::read_blob(&spec, name).ok(),
        |c| c.mesh,
    )
    .ok()
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_drawn_ship_material_resolves_a_shader_variant() {
    let Some(image) = image() else { return };
    let mut hulls = 0usize;
    for (archive, path) in SHIPS {
        let Some((_, report)) = build(&image, archive, path) else {
            continue;
        };
        hulls += 1;
        println!(
            "{path}: {} of {} material(s) resolved, {} of {} chunk(s) covered",
            report.variants_resolved,
            report.variants_resolved + report.variants_unshipped,
            report.variant_chunks,
            report.variant_chunks + report.variant_chunks_missed,
        );
        assert_eq!(
            report.variants_unshipped, 0,
            "{path}: a drawn material fell back to the unresolved fallback \
             (VertexColour1 mis-read as a colour set, or a genuinely new gap - \
             see hd_ship_class_census.rs)"
        );
        assert_eq!(
            report.variant_chunks_missed, 0,
            "{path}: a drawn chunk is covered by an unresolved material"
        );
        assert!(
            report.variants_resolved > 0,
            "{path}: nothing resolved at all, which is a build failure hiding \
             behind a vacuously-true assertion above"
        );
    }
    assert_eq!(hulls, SHIPS.len(), "every sample ship hull should build");
}
