//! The generic per-material Edge Animation Tools curve replay
//! (`mesh::rcs::curve_track`), through the start gantry's own model - the
//! first of the 79 curved `.rcsmodel` files on the disc this path was wired
//! for, not a special case of it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_gantry_curve_replay_ground_truth)'
//! ```
//!
//! # What this is for
//!
//! `crates/rcs/tests/hd_gantry_curve_ground_truth.rs` already pins the byte
//! layout of `321go_startfinish.rcsmodel`'s four curves. This file is the
//! other half: that the *renderer* actually reaches them - a material's curve
//! becomes an [`oag_mesh::mesh::AnimTrack::Rcs`], a vertex's own `anim`
//! index selects it, and `TexAnims::sample` produces a different value at two
//! different points in time, the same three checks
//! `authored_uv_ground_truth.rs` already runs for Pulse's own mechanism.
//!
//! **All four curved materials reach the shader, but only one of them ever
//! reaches the screen.** `docs/formats/edge-animation.md`'s own 2026-09-17
//! correction found that material 2's curve alone binds geometry a player
//! sees (the digit glyph mesh and its backdrop panel); materials 1/3/4's
//! curves are real and replay correctly too, but their own geometry (the
//! chequered-flag state, slot 7's embedded `fx350` art, and `FINAL LAP`) is
//! removed from every frame by `oag_render::gantry::clip_to_panel`/
//! `::strip_fx350_art` before a draw ever reaches this test's own model. This
//! test asserts the count of four because that is what the byte layout
//! authors and what the shader table should hold - it does not, and was
//! never meant to, assert that all four are visible.

mod archive_cache;

use std::path::PathBuf;

use oag_mesh::mesh::{self, AnimTrack};
use oag_mesh::mesh_render::TexAnims;

/// The digit board's own model: five materials, four of them carrying a
/// curve - see `docs/formats/edge-animation.md`.
const GANTRY_VEX: &str = "/data/billboards/hd_adverts/321go/321go_startfinish.vex";

fn image() -> Option<PathBuf> {
    oag_testdata::image("hdfury-ps3-eu-dec.iso")
}

/// Every curved material reaches [`oag_mesh::mesh::Model::anim_tracks`] as
/// an [`AnimTrack::Rcs`], a vertex of that material selects it, and replaying
/// it at two different times actually changes the sampled table - the same
/// "wired, not frozen" bar `authored_uv_ground_truth.rs` holds Pulse's own
/// mechanism to.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_digit_boards_curves_reach_the_shaders_table_and_replay() {
    let Some(image) = image() else { return };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let data = archive_cache::read(&spec, GANTRY_VEX).expect("the .vex reads");
    let geometry =
        mesh::rcs::sibling_geometry(&spec, GANTRY_VEX, &data).expect("the .rcsmodel is there");

    let (model, report) = mesh::rcs::build(
        GANTRY_VEX,
        &data,
        &geometry,
        &mut |name| archive_cache::read(&spec, name),
        |c| c.mesh,
    )
    .expect("the gantry model builds");
    println!("{report:?}");

    // Four of the digit board's five materials carry a curve - the exact
    // count `hd_gantry_curve_ground_truth.rs` pins on the raw bytes.
    let curved = model
        .anim_tracks
        .iter()
        .filter(|t| matches!(t, AnimTrack::Rcs(_)))
        .count();
    assert_eq!(
        curved,
        4,
        "expected four curved materials on the digit board, found {curved} \
         AnimTrack::Rcs entries in {} total",
        model.anim_tracks.len()
    );

    // At least one vertex actually selects one of them - a curve nobody's
    // vertex indexes would be dead weight, not a replay.
    let animated_vertices = model.vertices.iter().filter(|v| v.anim != 0).count();
    assert!(
        animated_vertices > 0,
        "no vertex carries a non-zero anim index, so no material's curve is drawn"
    );

    // The replay itself: the shader's own per-frame table differs between two
    // points in time, and slot 0 - the identity every unanimated vertex reads
    // - never moves.
    let at_zero = TexAnims::sample(&model, 0.0);
    let later = TexAnims::sample(&model, 5.0);
    assert_ne!(
        at_zero.transform, later.transform,
        "the curve table is identical at 0 s and 5 s - it is not being replayed"
    );
    assert_eq!(
        at_zero.transform[0],
        [1.0, 1.0, 0.0, 0.0],
        "slot 0 must stay the identity"
    );
    assert_eq!(
        later.transform[0],
        [1.0, 1.0, 0.0, 0.0],
        "slot 0 must stay the identity"
    );
}
