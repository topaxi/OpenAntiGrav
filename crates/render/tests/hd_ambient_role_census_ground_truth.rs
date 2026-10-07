//! How many of Amphiseum's resolved material slots the `NO_AMBIENT` role bit
//! covers, and how many of those are also `EMISSIVE` - the split `mesh.wgsl`'s
//! ambient/prelit-curve fix (2026-09-17) keys on.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_ambient_role_census_ground_truth)'
//! ```
//!
//! # What this pins
//!
//! `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "The per-material
//! microcode sweep" found none of Amphiseum's four ceiling materials'
//! resolved programs ever reference `constantAmbientColour`, and
//! `mesh::rcs::skin::roles` already reads that as `slots::NO_AMBIENT` on the
//! affected slots. `mesh.wgsl` now drops `scene.light.ambient` and replaces
//! the raw vertex-colour term with the curved one on every `NO_AMBIENT`
//! slot that is not also `slots::EMISSIVE` (the sign/glow family, which the
//! fix must not touch - see the shader's own comment on `vertex_light_term`).
//! This test is a regression guard on that split, not a picture check: if
//! Amphiseum's resolved-material count moves, or the `NO_AMBIENT`/`EMISSIVE`
//! counts move, that is either a resolver regression or a genuine change to
//! chase, and this is where it would show first.

mod archive_cache;

use std::path::PathBuf;

use oag_mesh::mesh::{self, slots};

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

const TRACK: &str = "/data/environments/amphiseum/track.vex";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn amphiseums_no_ambient_slots_split_from_emissive_the_way_the_shader_assumes() {
    let Some(image) = image() else { return };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let Some(data) = archive_cache::read(&spec, TRACK) else {
        return;
    };
    let Some((model, _report)) =
        mesh::rcs::scene_from(&spec, TRACK, &data).expect("Amphiseum's track should build")
    else {
        panic!("{TRACK}: not a PS3 model");
    };

    let mut no_ambient = 0usize;
    let mut no_ambient_and_emissive = 0usize;
    for &packed in &model.material_slots {
        let role = packed & slots::ROLE_MASK;
        if role & slots::NO_AMBIENT == 0 {
            continue;
        }
        no_ambient += 1;
        if role & slots::EMISSIVE == slots::EMISSIVE {
            no_ambient_and_emissive += 1;
        }
    }

    // Measured when this guard was written (2026-09-17, `lane-hd-ilevertex-ambient`):
    // 641 resolved material slots (not the 640
    // `hd_amphiseum_wall_material_ground_truth.rs` pins for
    // `report.variants_resolved` - that file's own comment says why:
    // `Model::material_slots` is one entry longer than the raw variant count
    // even built from `scene_from` alone), of which 553 carry `NO_AMBIENT`
    // and 68 of those are also `EMISSIVE` - excluded from the vertex-colour
    // curve this fix adds, as they always were from the ambient term itself.
    assert_eq!(
        model.material_slots.len(),
        641,
        "Amphiseum's resolved-material-slot count moved - re-check whether \
         this is the resolver regressing or the count genuinely growing"
    );
    assert_eq!(
        no_ambient, 553,
        "the NO_AMBIENT slot count moved - either the role-bit reader changed \
         or Amphiseum's own resolved materials did"
    );
    assert_eq!(
        no_ambient_and_emissive, 68,
        "the NO_AMBIENT-and-EMISSIVE slot count moved - these are excluded \
         from the ambient fix's vertex-colour curve (mesh.wgsl's \
         vertex_light_term), and a change here means that exclusion now \
         covers a different set of materials"
    );
}
