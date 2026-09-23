//! Unit tests for [`super`]: the render-side blast pool's spawn/advance/retire
//! shape and the direction-facing basis, none of which need a disc to load a
//! model from.

use super::*;
use crate::race::tests::race_with_a_grid;

/// A blast spawns in the first free slot, ages every tick, and is gone
/// exactly at [`LIFETIME_SECONDS`] - `BombBlast_Update`'s own hardcoded
/// object retire.
#[test]
fn a_blast_ages_and_retires_at_the_hardcoded_lifetime() {
    let mut race = race_with_a_grid();
    let at = Vec3::new(1.0, 2.0, 3.0);
    race.spawn_bomb_blast_model(at, Quat::IDENTITY);

    let draws = race.bomb_blast_draws();
    assert_eq!(
        draws.iter().flatten().count(),
        1,
        "exactly one live blast right after spawning one"
    );

    // A couple of ticks past the exact ceiling, so f32 accumulation error in
    // repeated `age += dt` cannot leave this short of the threshold by a
    // fraction of a tick.
    let dt = 1.0 / 60.0;
    let ticks = (LIFETIME_SECONDS / dt).ceil() as u32 + 2;
    for _ in 0..ticks {
        race.advance_bomb_blast_models(dt);
    }

    assert_eq!(
        race.bomb_blast_draws().iter().flatten().count(),
        0,
        "retired once its age reached the hardcoded 4.0s lifetime"
    );
}

/// The hemisphere hides at [`HEMISPHERE_HIDE_AT_SECONDS`] while the
/// shockwave (and the object itself) live on past it - two different
/// lifetimes inside one blast, per `BombBlast_Update`'s own two `if`s.
#[test]
fn the_hemisphere_hides_well_before_the_shockwave_does() {
    let mut race = race_with_a_grid();
    race.spawn_bomb_blast_model(Vec3::ZERO, Quat::IDENTITY);

    let dt = 1.0 / 60.0;
    let ticks = (HEMISPHERE_HIDE_AT_SECONDS / dt).ceil() as u32 + 2;
    for _ in 0..ticks {
        race.advance_bomb_blast_models(dt);
    }

    let draws = race.bomb_blast_draws();
    let draw = draws[0].expect("still inside the 4.0s lifetime");
    assert!(!draw.hemisphere_visible, "hidden past 1.55s");
    assert!(
        draw.shockwave_visible,
        "the shockwave outlives the hemisphere"
    );
}

/// The pool does not evict a running blast to make room for a new one - the
/// same "no evicting a running effect" rule [`blast_models`]'s own pool
/// takes.
#[test]
fn a_full_pool_drops_a_new_spawn_rather_than_evicting_one() {
    let mut race = race_with_a_grid();
    for slot in 0..BOMB_BLAST_SLOTS {
        race.spawn_bomb_blast_model(Vec3::splat(slot as f32), Quat::IDENTITY);
    }
    assert_eq!(
        race.bomb_blast_draws().iter().flatten().count(),
        BOMB_BLAST_SLOTS
    );

    let dropped = Vec3::splat(BOMB_BLAST_SLOTS as f32 + 1000.0);
    race.spawn_bomb_blast_model(dropped, Quat::IDENTITY);
    let draws = race.bomb_blast_draws();
    assert_eq!(
        draws.iter().flatten().count(),
        BOMB_BLAST_SLOTS,
        "still full, not evicted"
    );
    assert!(
        draws
            .iter()
            .flatten()
            .all(|draw| draw.hemisphere_matrix.w_axis.truncate() != dropped),
        "the dropped spawn never took a slot"
    );
}

/// Both eases run in the recovered direction: the hemisphere grows from its
/// own `2.0` start, and the shockwave stays at `0.0` (invisible extent)
/// until [`SHOCKWAVE_SCALE_DELAY_SECONDS`] has passed, per the `if (0.1 <
/// age)` gate `BombBlast_Update` wraps that ease alone in.
#[test]
fn the_shockwave_only_starts_growing_after_its_own_delay() {
    let mut race = race_with_a_grid();
    race.spawn_bomb_blast_model(Vec3::ZERO, Quat::IDENTITY);

    let dt = 1.0 / 60.0;
    // One tick, well inside the 0.1s gate.
    race.advance_bomb_blast_models(dt);
    let before = race.view.bomb_blasts[0].expect("just spawned");
    assert_eq!(
        before.shockwave_scale, SHOCKWAVE_SCALE.start,
        "gated shut before 0.1s"
    );
    assert!(
        before.hemisphere_scale > HEMISPHERE_SCALE.start,
        "the hemisphere's own ease is not gated"
    );

    let ticks = (SHOCKWAVE_SCALE_DELAY_SECONDS / dt).ceil() as u32 + 2;
    for _ in 0..ticks {
        race.advance_bomb_blast_models(dt);
    }
    let after = race.view.bomb_blasts[0].expect("still inside the 4.0s lifetime");
    assert!(
        after.shockwave_scale > SHOCKWAVE_SCALE.start,
        "growing once the gate has passed"
    );
}

/// [`SHOCKWAVE_ALPHA`] eases the recovered way even though nothing draws it
/// yet - see this module's own doc comment on why the fade is recovered but
/// not wired. A dedicated assertion so the constant stays exercised (and the
/// numbers stay checked against `mine.md`) rather than sitting as dead code
/// until a `Drawable::tint` scratch buffer lands for this pool.
#[test]
fn the_shockwave_alpha_ease_fades_from_opaque_to_nothing() {
    let mut alpha = SHOCKWAVE_ALPHA.start;
    assert_eq!(alpha, 1.0, "opaque at spawn - `+0xf4`'s own initial store");
    for _ in 0..600 {
        alpha = SHOCKWAVE_ALPHA.step(alpha);
    }
    assert!(
        alpha < 0.01,
        "ten seconds at the recovered 0.1/tick rate is long enough to reach \
         the `+0xf8` target of 0.0, got {alpha}"
    );
}

/// The direction-facing basis is orthonormal and puts the object's own
/// position in the matrix's translation column, for an ordinary direction
/// and for the straight-up degenerate case this shares with
/// `blast_models::billboard_matrix`.
#[test]
fn the_basis_is_orthonormal_and_handles_a_straight_up_direction() {
    let position = Vec3::new(2.0, 0.0, 0.0);
    let dir = Vec3::new(0.0, 0.0, -1.0);
    let matrix = bomb_blast_basis(position, dir);

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

    // A bomb frozen pointing straight up: the `Vec3::Y` reference degenerates
    // and the fallback must still produce a finite, orthonormal basis.
    let overhead = bomb_blast_basis(Vec3::ZERO, Vec3::Y);
    assert!(overhead.x_axis.truncate().is_finite());
    assert!((overhead.x_axis.truncate().length() - 1.0).abs() < 1e-5);
}
