//! A PS2 circuit's blended meshes are drawn far to near, as the PSP's are.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^ps2_blended_depth_ground_truth::/)'
//! ```
//!
//! Talon's Junction in Zone on the PS2 drew its translucent grid in file order,
//! so a far window row showed through nearer glass (maintainer report,
//! 2026-10-31). `DrawCall::depth_point` is what makes `Model::blended_order`
//! sort a mesh's list-B batches by view depth; the PS2 VIF batches carried none.
//! The PS2 submit is unread, so the PSP's key is **inherited**.

use oag_mesh::mesh::{self, Model};

fn circuit(image: &str, entry: &str) -> Option<Model> {
    let image = oag_testdata::image(image)?;
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("the disc opens");
    let blob = archives
        .read_name(entry)
        .unwrap_or_else(|e| panic!("{entry}: {e}"));
    Some(mesh::build_with_textures(entry, &blob, None).unwrap_or_else(|e| panic!("{entry}: {e}")))
}

fn keyed(model: &Model) -> usize {
    model
        .transparent_draws
        .iter()
        .filter(|d| d.depth_point.is_some())
        .count()
}

fn check(image: &str, entry: &str) {
    let Some(model) = circuit(image, entry) else {
        return;
    };
    assert!(
        keyed(&model) > 0,
        "{image} {entry}: no blended draw carries a depth point"
    );
    let near = [0.0, 0.0, 1.0, 0.0];
    let far = [0.0, 0.0, -1.0, 0.0];
    assert_ne!(
        model.blended_order(near),
        model.blended_order(far),
        "{image} {entry}: the order does not follow the camera"
    );
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn ps2_talons_junction_blended_meshes_are_depth_ordered() {
    check("pulse-ps2-eu.chd", r"Data\Environments\16_Track\track.vex");
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn ps2_talons_junction_zone_blended_meshes_are_depth_ordered() {
    check(
        "pulse-ps2-eu.chd",
        r"Data\Environments\16_Track\zone_track.vex",
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn psp_talons_junction_zone_stays_depth_ordered() {
    check(
        "pulse-psp-eu.chd",
        r"Data\Environments\16_Track\zone_track.vex",
    );
}
