//! Unit tests for [`super`]: the render-side blast pool's spawn/advance/retire
//! shape and the direction-facing basis, none of which need a disc to load a
//! model from.

use super::*;
use crate::tests::race_with_a_grid;

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

/// [`SHOCKWAVE_ALPHA`] eases the recovered way and the draw carries it - see this
/// module's own doc comment. The numbers stay checked against `mine.md`.
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

/// The ship explosion's ring, read off the running original: scale
/// `0.1, 1.095, 2.040, 2.938, 3.791, 4.602` on the first six ticks, on all three axes, at the
/// craft and not the explosion's dropped point, gone at `1.5 s`.
#[test]
fn a_ship_explosions_shockwave_eases_as_the_original_logged_it() {
    let mut race = race_with_a_grid();
    let at = Vec3::new(-112.61, -49.85, -195.41);
    race.spawn_ship_shockwave(at, Vec3::Y * 0.75);
    let dt = 1.0 / 60.0;
    let seen = [0.1, 1.095, 2.040, 2.938, 3.791, 4.602];
    for (tick, expect) in seen.iter().enumerate() {
        let blast = race.view.bomb_blasts[0].expect("live");
        assert!(
            (blast.shockwave_scale - expect).abs() < 2e-3,
            "tick {tick}: {} against {expect}",
            blast.shockwave_scale
        );
        race.advance_bomb_blast_models(dt);
    }
    let draw = race.bomb_blast_draws()[0].expect("live");
    assert!(!draw.hemisphere_visible, "a ship explosion has no dome");
    let scale = draw.shockwave_matrix.y_axis.truncate().length();
    assert!((scale - race.view.bomb_blasts[0].unwrap().shockwave_scale).abs() < 1e-4);
    assert!(
        (draw.shockwave_matrix.x_axis.truncate().length() - scale).abs() < 1e-4,
        "uniform: the Bomb's own ring leaves its axis alone, this one does not"
    );
    assert_eq!(draw.shockwave_matrix.w_axis.truncate(), at);

    // The ambient light alpha the original's GE read as `0xf4` early and `0xb8` later: `0.98^n`.
    let blast = race.view.bomb_blasts[0].unwrap();
    assert!((blast.shockwave_alpha - 0.98_f32.powi(6)).abs() < 1e-4);
    assert_eq!(draw.shockwave_alpha, blast.shockwave_alpha);

    let ticks = (SHIP_SHOCKWAVE_LIFETIME_SECONDS / dt).ceil() as u32 + 2;
    for _ in 0..ticks {
        race.advance_bomb_blast_models(dt);
    }
    assert_eq!(race.bomb_blast_draws().iter().flatten().count(), 0);
}

/// The basis is orthonormal, puts `dir` in its own `Y` column (the slot
/// both `.vex` files' own vertical axis expects - see [`bomb_blast_basis`]'s
/// own doc comment) and the object's position in the translation column,
/// for the ordinary case (a bomb resting upright) and for the degenerate
/// case where `dir` is parallel to the reference vector.
#[test]
fn the_basis_is_orthonormal_and_puts_dir_in_the_y_column() {
    let position = Vec3::new(2.0, 0.0, 0.0);
    let dir = Vec3::Y;
    let matrix = bomb_blast_basis(position, dir);

    let right = matrix.x_axis.truncate();
    let up = matrix.y_axis.truncate();
    let reference = matrix.z_axis.truncate();
    assert!((right.length() - 1.0).abs() < 1e-5);
    assert!((up.length() - 1.0).abs() < 1e-5);
    assert!((reference.length() - 1.0).abs() < 1e-5);
    assert!(right.dot(up).abs() < 1e-5);
    assert!(right.dot(reference).abs() < 1e-5);
    assert!(up.dot(reference).abs() < 1e-5);
    assert_eq!(up, dir, "dir occupies the basis's own Y column");
    assert_eq!(matrix.w_axis.truncate(), position);

    // A bomb frozen pointing along the world reference axis itself (`Z`):
    // the Gram-Schmidt step degenerates and the fallback must still produce
    // a finite, orthonormal basis.
    let overhead = bomb_blast_basis(Vec3::ZERO, Vec3::Z);
    assert!(overhead.x_axis.truncate().is_finite());
    assert!((overhead.x_axis.truncate().length() - 1.0).abs() < 1e-5);
}

/// Both models' texture tracks play on the blast's own age: `BombBlast_Construct`
/// seeds each with `Node_SetAnimTimeTree(0.0)` and the mesh update adds the clock's
/// delta from there, so the time is `0` at spawn and the age at rate 1 (measured
/// live: three detonations at race clocks 60.9, 157.5 and 294.1 all start at `0.000`
/// with slope `1.0000`). `write_bomb_blasts` hands `age` to `write_anims`; a draw that
/// did not carry it would leave both tracks at phase 0 for the blast's four seconds.
#[test]
fn the_draw_carries_the_age_the_texture_tracks_play_at() {
    let mut race = race_with_a_grid();
    race.spawn_bomb_blast_model(Vec3::ZERO, Quat::IDENTITY);
    assert_eq!(race.bomb_blast_draws()[0].expect("live").age, 0.0);

    let dt = 1.0 / 60.0;
    for _ in 0..90 {
        race.advance_bomb_blast_models(dt);
    }
    let draw = race.bomb_blast_draws()[0].expect("live");
    assert!(
        (draw.age - 1.5).abs() < 1e-3,
        "90 ticks of 1/60 s are 1.5 s, not {}",
        draw.age
    );
    assert_eq!(draw.age, race.view.bomb_blasts[0].unwrap().age);
}

/// The ship explosion's shockwave is the same `Bomb_Shockwave.vex` and
/// `ShipShockwave_Construct` seeds it with the same `Node_SetAnimTimeTree(0.0)`
/// (`0x0885ee44`; read statically, not measured live).
#[test]
fn a_ship_explosions_shockwave_texture_also_plays_on_its_age() {
    let mut race = race_with_a_grid();
    race.spawn_ship_shockwave(Vec3::ZERO, Vec3::Y * 0.75);
    let dt = 1.0 / 60.0;
    for _ in 0..30 {
        race.advance_bomb_blast_models(dt);
    }
    let draw = race.bomb_blast_draws()[0].expect("live");
    assert!((draw.age - 0.5).abs() < 1e-3, "{}", draw.age);
}
