//! A ship hull's own normal map no longer glows; a real circuit glow is
//! untouched.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_hull_glow_role_ground_truth)'
//! ```
//!
//! # What this pins
//!
//! `mesh::rcs::emissive::emissive` used to treat any material whose fragment
//! program structurally accumulates unit 1 (`Program::accumulates(1)`) as a
//! glow, whatever texture `Pick::aux` actually loaded there. On a ship hull
//! that texture is routinely the ship's own normal map -
//! `docs/rendering/hd-ship-materials.md`, Finding 1 - and adding a raw,
//! blue-dominant tangent-space normal at full weight is what gave `feisar_c1`
//! its blue-purple cast. Fixed by reading the role of the sampler `Pick::aux`
//! actually resolved to, refusing the accumulate only where that role is a
//! disc-measured normal or specular map.
//!
//! Two things have to hold at once, on the *same* hull and the *same*
//! circuits, or the fix either does nothing or repeats the regression two
//! earlier attempts hit (`hd-ship-materials.md`, "two fixes tried and both
//! failed"):
//!
//! 1. **The hull's own normal-map accumulate is gone.**
//!    `diffuse_with_specular_from_alpha_n_vcol` no longer carries
//!    `ADD_SECOND`, and the load report counts the refusal.
//! 2. **A material this project cannot settle a role for is left exactly as
//!    it drew before.** `carbonfibre`'s own second texture is a plain diffuse
//!    (`carbon.gtf`), not a normal map - `CLAUDE.md`'s rule against inventing
//!    a role from a name means this one is *not* touched, and the load report
//!    counts it as unresolved rather than silently dropping it too.
//! 3. **A real, working circuit glow still glows.** `scroller_glow_v3`,
//!    `tunnel_fx_noalpha` and `mageffect08` are three of the ten families the
//!    first (`aux_traced`) attempt regressed; all three still carry
//!    `ADD_SECOND` on Talon's Junction after this fix.

mod archive_cache;

use std::path::{Path, PathBuf};

use oag_mesh::mesh;

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

fn build_ship(image: &Path, archive: &str, path: &str) -> Option<(mesh::Model, mesh::rcs::Report)> {
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
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
}

fn build_circuit(
    image: &Path,
    archive: &str,
    path: &str,
) -> Option<(mesh::Model, oag_rcs::rcsmodel::Model, mesh::rcs::Report)> {
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let data = archive_cache::read(&spec, path)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    let geometry_model = oag_rcs::rcsmodel::Model::parse(&geometry).ok()?;
    let (model, report) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archive_cache::read(&spec, name)
    })
    .ok()?;
    Some((model, geometry_model, report))
}

/// Which material slots of `geometry_model` carry `ADD_SECOND` in `model`,
/// keyed by the material's own leaf file name (several slots can share a
/// name across different chunks of the same shader).
fn add_second_families(
    model: &mesh::Model,
    geometry_model: &oag_rcs::rcsmodel::Model,
) -> std::collections::BTreeSet<String> {
    model
        .material_slots
        .iter()
        .enumerate()
        .filter(|(_, packed)| **packed & mesh::slots::ADD_SECOND != 0)
        .filter_map(|(slot, _)| geometry_model.materials.get(slot))
        .map(|m| {
            m.name
                .rsplit('/')
                .next()
                .unwrap_or(&m.name)
                .trim_end_matches(".rcsmaterial")
                .to_string()
        })
        .collect()
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_ship_hulls_own_normal_map_no_longer_glows_but_an_unsettled_role_still_does() {
    let Some(image) = image() else { return };
    let Some((model, report)) =
        build_ship(&image, "DATA06.PSARC", "/data/ships/feisar_c1/ship.vex")
    else {
        panic!("feisar_c1 should build");
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA06.PSARC", image.display());
    let path = "/data/ships/feisar_c1/ship.vex";
    let data = archive_cache::read(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("a sibling .rcsmodel");
    let geometry_model = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");

    let families = add_second_families(&model, &geometry_model);
    println!("feisar_c1 ADD_SECOND families: {families:?}");
    println!(
        "feisar_c1: {} excluded as a known normal/specular map, {} left unresolved (kept as before)",
        report.emissive_surface_map_excluded, report.emissive_role_unresolved
    );

    assert!(
        !families.contains("diffuse_with_specular_from_alpha_n_vcol"),
        "feisar_c1's hull paint still glows its own normal map: {families:?}"
    );
    assert!(
        report.emissive_surface_map_excluded > 0,
        "feisar_c1 should refuse at least one normal-map accumulate"
    );

    // carbonfibre's own second texture (`carbon.gtf`) is a plain diffuse, not
    // a normal map - this project has no disc-measured role for it, so
    // CLAUDE.md's rule against inventing one means it is left exactly as it
    // drew before this fix: still `ADD_SECOND`, still counted as unresolved
    // rather than silently dropped too.
    assert!(
        families.contains("carbonfibre"),
        "carbonfibre should be left on the pre-existing path: {families:?}"
    );
    assert!(
        report.emissive_role_unresolved > 0,
        "carbonfibre's own second texture has no settled role and should be counted, not guessed at"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn talons_junctions_real_glows_still_glow_after_the_role_fix() {
    let Some(image) = image() else { return };
    let Some((model, geometry_model, _report)) = build_circuit(
        &image,
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
    ) else {
        panic!("Talon's Junction should build");
    };
    let families = add_second_families(&model, &geometry_model);
    println!("Talon's Junction ADD_SECOND families: {families:?}");

    // Three of the ten families the first (`aux_traced`) attempt regressed -
    // `docs/rendering/hd-ship-materials.md`, "two fixes tried and both
    // failed". `tunnel_fx_noalpha` is the one that would have been dropped by
    // resolving hardware unit 1 independently of `Pick::aux`: it declares its
    // *specular* map at unit 1 and its own emissive texture at unit 2, and
    // `Pick::aux` (no lightmap, an untraced alpha lane) resolves to the
    // emissive one.
    for family in ["scroller_glow_v3", "tunnel_fx_noalpha", "mageffect08"] {
        assert!(
            families.contains(family),
            "{family} should still glow on Talon's Junction: {families:?}"
        );
    }
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn tech_de_ras_mountains_no_longer_glow_their_own_normal_map() {
    let Some(image) = image() else { return };
    let Some((model, geometry_model, report)) = build_circuit(
        &image,
        "DATA00.PSARC",
        "/data/environments/tech_de_ra/track.vex",
    ) else {
        panic!("Tech De Ra should build");
    };
    let families = add_second_families(&model, &geometry_model);
    println!("Tech De Ra ADD_SECOND families: {families:?}");
    println!(
        "Tech De Ra: {} excluded as a known normal/specular map, {} left unresolved (kept as before)",
        report.emissive_surface_map_excluded, report.emissive_role_unresolved
    );

    // The same Finding-1 shape on circuit scenery: `tech_de_ra_rocks`' slot
    // 343 - the 58-chunk mountain range - is the one rock slot with no
    // lightmap path, so `Pick::aux` landed on `rocks_01_normal_alpha.gtf` and
    // the mountains drew cyan/purple. The hash (`0x0cddca48`) binds that one
    // normal map and nothing else disc-wide - `emissive.rs`,
    // `CIRCUIT_SURFACE_MAP_SAMPLERS`.
    assert!(
        !families.contains("tech_de_ra_rocks"),
        "Tech De Ra's rocks still glow their own normal map: {families:?}"
    );
    assert!(
        report.emissive_surface_map_excluded > 0,
        "Tech De Ra should refuse at least one normal-map accumulate"
    );

    // A named glow on the same circuit is untouched: `EmissiveTexture` on
    // the rail's scrolling emissive.
    assert!(
        families.contains("lambert_spec_mult_emissive_scroll"),
        "lambert_spec_mult_emissive_scroll should still glow on Tech De Ra: {families:?}"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn zone_1s_floor_and_pads_no_longer_glow_their_normal_maps() {
    let Some(image) = image() else { return };
    let Some((model, geometry_model, report)) = build_circuit(
        &image,
        "DATA00.PSARC",
        "/data/environments/zone_1/track.vex",
    ) else {
        panic!("Zone 1 should build");
    };
    let families = add_second_families(&model, &geometry_model);
    println!("Zone 1 ADD_SECOND families: {families:?}");

    // The Zone tracks ship no lightmaps, so every `Pick::aux` there is the
    // material's own entry 1 - `tracknormal.gtf` under the floor's
    // `tracktexture_with_normal` (a 1x1 black diffuse, the Zone look being
    // the unread Zone shader's) and the pads' `_ne` normal map under
    // `weapon_pads`. Both were drawn as a lavender glow; both hashes bind a
    // picture under *other* families, which is why the table is keyed on
    // (family, hash) rather than hash alone.
    for family in ["tracktexture_with_normal", "weapon_pads"] {
        assert!(
            !families.contains(family),
            "{family} still glows its own normal map on Zone 1: {families:?}"
        );
    }
    assert!(
        report.emissive_surface_map_excluded >= 2,
        "Zone 1 should refuse the floor's and the pads' accumulates, got {}",
        report.emissive_surface_map_excluded
    );
}
