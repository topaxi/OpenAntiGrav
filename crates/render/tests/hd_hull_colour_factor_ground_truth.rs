//! A hull's `VertexColour1` is a factor on its light, read off its programs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; see
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_hull_colour_factor_ground_truth)'
//! ```
//!
//! `diffuse_vcol`'s lit block multiplies `f[TC0]` (the vertex program's
//! `VertexColour1`) into `ambient + sun * N.L`; `mesh.wesl` used to add it as a
//! baked light, which drew `feisar_c1`'s rear housing about 3x too bright.
//! See `docs/rendering/hd-ship-materials.md`, "Finding 3, resolved".

mod archive_cache;

use oag_mesh::mesh;

const IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn hull() -> Option<mesh::Model> {
    let image = oag_testdata::image(IMAGE)?;
    let spec = format!("{}:PS3_GAME/USRDIR/DATA06.PSARC", image.display());
    let path = "/data/ships/feisar_c1/ship.vex";
    let data = archive_cache::read(&spec, path)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    mesh::rcs::build(
        path,
        &data,
        &geometry,
        &mut |name| archive_cache::read(&spec, name),
        |c| c.mesh,
    )
    .ok()
    .map(|(model, _)| model)
}

#[test]
#[ignore = "needs the HD disc image"]
fn the_hulls_vertex_colour_materials_are_factors_and_the_unlit_ones_are_not() {
    let Some(model) = hull() else { return };
    assert!(
        model.material_colour_factor.iter().any(|&f| f),
        "no material of feisar_c1 reads VertexColour1 as a factor"
    );
    let marked = model.vertices.iter().filter(|v| v.sun_mask < 0.0).count();
    assert!(marked > 0, "no vertex carries the factor marker");
    assert!(
        marked < model.vertices.len(),
        "every vertex is marked: glass and emissive chunks carry no VertexColour1"
    );
    assert!(
        model
            .vertices
            .iter()
            .any(|v| v.specular_exponent == mesh::NO_SPECULAR),
        "diffuse_vcol has no LG2, so its chunks draw no specular"
    );
    assert!(
        model
            .vertices
            .iter()
            .any(|v| v.specular_exponent == mesh::DEFAULT_SPECULAR_EXPONENT),
        "the n_vcol material keeps its stand-in exponent"
    );
}
