//! Unit tests for [`super`]: the render-side blast pool's spawn/advance/retire
//! shape and the billboard basis, none of which need a disc to load a model
//! from.

use super::*;
use crate::race::tests::race_with_a_grid;

/// A blast spawns in the first free slot, ages every tick, and is gone
/// exactly at [`PLASMA_BLAST_LIFETIME_SECONDS`] - the same hardcoded retire
/// time `PlasmaBlast_Update` carries, see `plasma.md`.
#[test]
fn a_blast_ages_and_retires_at_the_hardcoded_lifetime() {
    let mut race = race_with_a_grid();
    let at = Vec3::new(1.0, 2.0, 3.0);
    race.spawn_plasma_blast_model(at);

    let draws = race.plasma_blast_draws(Vec3::ZERO);
    assert!(
        draws.iter().flatten().count() == 1,
        "exactly one live blast right after spawning one"
    );

    // A couple of ticks past the exact ceiling, so f32 accumulation error in
    // repeated `age += dt` cannot leave this short of the threshold by a
    // fraction of a tick.
    let dt = 1.0 / 60.0;
    let ticks = (PLASMA_BLAST_LIFETIME_SECONDS / dt).ceil() as u32 + 2;
    for _ in 0..ticks {
        race.advance_plasma_blast_models(dt);
    }

    assert_eq!(
        race.plasma_blast_draws(Vec3::ZERO).iter().flatten().count(),
        0,
        "retired once its age reached the hardcoded 1.5s lifetime"
    );
}

/// The pool does not evict a running blast to make room for a new one -
/// the same "no evicting a running effect" rule the flare stage takes,
/// per this module's own doc comment.
#[test]
fn a_full_pool_drops_a_new_spawn_rather_than_evicting_one() {
    let mut race = race_with_a_grid();
    for slot in 0..PLASMA_BLAST_SLOTS {
        race.spawn_plasma_blast_model(Vec3::splat(slot as f32));
    }
    assert_eq!(
        race.plasma_blast_draws(Vec3::ZERO).iter().flatten().count(),
        PLASMA_BLAST_SLOTS
    );

    // One more spawn, with the pool already full - well outside the
    // `0..PLASMA_BLAST_SLOTS` fill above so it cannot alias one of those.
    let dropped = Vec3::splat(PLASMA_BLAST_SLOTS as f32 + 1000.0);
    race.spawn_plasma_blast_model(dropped);
    let draws = race.plasma_blast_draws(Vec3::ZERO);
    assert_eq!(
        draws.iter().flatten().count(),
        PLASMA_BLAST_SLOTS,
        "still full, not evicted"
    );
    assert!(
        draws
            .iter()
            .flatten()
            .all(|draw| draw.matrix.w_axis.truncate() != dropped),
        "the dropped spawn never took a slot"
    );
}

/// The billboard basis is orthonormal and puts the object's own position in
/// the matrix's translation column, for an ordinary camera placement and for
/// the straight-up degenerate case `Race::projectile_model_matrices` already
/// has to guard.
#[test]
fn the_billboard_basis_is_orthonormal_and_handles_straight_up() {
    let position = Vec3::new(2.0, 0.0, 0.0);
    let camera = Vec3::new(2.0, 0.0, -10.0);
    let matrix = billboard_matrix(position, camera);

    let right = matrix.x_axis.truncate();
    let up = matrix.y_axis.truncate();
    let forward = matrix.z_axis.truncate();
    assert!((right.length() - 1.0).abs() < 1e-5);
    assert!((up.length() - 1.0).abs() < 1e-5);
    assert!((forward.length() - 1.0).abs() < 1e-5);
    assert!(right.dot(up).abs() < 1e-5);
    assert!(right.dot(forward).abs() < 1e-5);
    assert!(up.dot(forward).abs() < 1e-5);
    assert_eq!(matrix.w_axis.truncate(), position);
    // A rotation: until 2026-09-24 this basis was a reflection, which every
    // assertion above passes and which draws the shells mirrored.
    assert!((matrix.determinant() - 1.0).abs() < 1e-5);

    // Camera directly above the blast: the `Vec3::Y` reference degenerates
    // and the fallback must still produce a finite, orthonormal basis.
    let overhead = billboard_matrix(Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0));
    assert!(overhead.x_axis.truncate().is_finite());
    assert!((overhead.x_axis.truncate().length() - 1.0).abs() < 1e-5);
    assert!((overhead.determinant() - 1.0).abs() < 1e-5);
}

/// HD's disc basis points `Z` away from the viewer, keeps the billboard's own
/// up and position, and stays a rotation - a reflection would flip every
/// triangle's winding and the back-face cull `cull_as_authored` turns on
/// would then hide exactly the faces the original shows.
#[test]
fn the_hd_basis_faces_away_from_the_camera_without_mirroring() {
    let position = Vec3::new(2.0, 0.0, 0.0);
    let camera = Vec3::new(2.0, 0.0, -10.0);
    let billboard = billboard_matrix(position, camera);
    let hd = facing_away(billboard);

    let towards_camera = (camera - position).normalize();
    assert!(hd.z_axis.truncate().dot(towards_camera) < -0.999);
    assert_eq!(hd.y_axis, billboard.y_axis);
    assert_eq!(hd.w_axis, billboard.w_axis);
    assert!((hd.determinant() - 1.0).abs() < 1e-5);
}
