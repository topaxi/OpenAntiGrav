//! Amphiseum's trackside wall panel draws its own diffuse, not its glow decal.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_amphiseum_wall_material_ground_truth)'
//! ```
//!
//! # What this pins
//!
//! Before 2026-09-13, `data/environments/amphiseum/materials/track_wall.rcsmaterial`
//! drew flat black over both trackside wall panels. The material's own sampler
//! table names four entries - `EmissiveTexture` (a near-solid-black glow decal)
//! at entry 0, the real diffuse+specular texture (`Texture1`) at entry 1, a
//! normal map (`Texture2`) at entry 2, and the circuit's own lightmap atlas at
//! entry 3 - and `mesh::rcs::skin::picks`'s lightmap branch chose its albedo
//! by position rather than by sampler role, landing on entry 0 every time a
//! lightmapped material's first populated entry happened to be an
//! `EmissiveTexture` rather than a picture. See
//! `docs/formats/rcsmaterial.md`, "A trackside wall panel drew solid black
//! because its picture was its glow decal", and
//! `crates/mesh/src/mesh/rcs/skin.rs`'s `NOT_A_PICTURE`.
//!
//! This test builds Amphiseum's real track through the production
//! `mesh::rcs::scene_from` path and pins two things: the circuit's own
//! resolved-material count (so a regression in variant resolution shows here
//! too, the same shape `hd_ship_hull_material_ground_truth.rs` pins for ship
//! hulls), and that every drawn `track_wall.rcsmaterial` slot's own bound
//! albedo texture is *not* the `EmissiveTexture` entry.

mod archive_cache;

use std::path::PathBuf;

use oag_mesh::mesh;

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

const TRACK: &str = "/data/environments/amphiseum/track.vex";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_track_wall_slot_binds_its_own_diffuse_not_its_glow_decal() {
    let Some(image) = image() else { return };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let Some(data) = archive_cache::read(&spec, TRACK) else {
        return;
    };
    let Some((model, report)) =
        mesh::rcs::scene_from(&spec, TRACK, &data).expect("Amphiseum's track should build")
    else {
        panic!("{TRACK}: not a PS3 model");
    };

    println!("{}", report.describe());

    // The resolver-side floor this fix does not touch: a regression here
    // means the *variant* resolution broke, which is a different bug from
    // the one this file pins - see `hd_ship_hull_material_ground_truth.rs`
    // for that shape of check. Amphiseum reads 640 of 640 both before and
    // after this fix (it is a `picks()` bug, not a `variants()` one), so this
    // is an exact pin rather than a floor. `oag-game`'s own race load report
    // quotes 641 of 641 for the same circuit because it also counts the
    // speedup/weapon pad passes' own re-use of the resolved table; this test
    // builds `track.vex` alone, through `scene_from`, which is 640.
    assert_eq!(
        report.variants_resolved, 640,
        "Amphiseum's resolved-material count moved - re-check whether this \
         is the resolver regressing or the count genuinely growing"
    );
    assert_eq!(
        report.variants_unshipped, 0,
        "Amphiseum's unresolved-material count moved"
    );
    assert_eq!(
        report.variant_chunks, 2424,
        "Amphiseum's resolved-material chunk coverage moved"
    );

    let geometry = mesh::rcs::sibling_geometry(&spec, TRACK, &data)
        .expect(".rcsmodel should sit beside the .vex");
    let source = oag_rcs::rcsmodel::Model::parse(&geometry).expect(".rcsmodel should parse");

    const EMISSIVE_TEXTURE: &str =
        "data/environments/amphiseum/textures/dds/track/track_wall_emissive.gtf";

    let mut wall_slots = 0usize;
    for (slot, material) in source.materials.iter().enumerate() {
        if !material.name.ends_with("materials/track_wall.rcsmaterial") {
            continue;
        }
        let Some(Some(texture)) = model.textures.get(slot) else {
            continue;
        };
        wall_slots += 1;
        assert_ne!(
            texture.label, EMISSIVE_TEXTURE,
            "slot {slot} ({}) bound its glow decal as its whole picture again",
            material.name
        );
    }
    assert!(
        wall_slots > 0,
        "no track_wall.rcsmaterial slot decoded a texture at all - the sample \
         is not exercising the fix"
    );
}
