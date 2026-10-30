//! Unit tests for [`super`]: the render-side blast pool's spawn/advance/retire
//! shape and the billboard basis, none of which need a disc to load a model
//! from.

use super::*;
use crate::tests::race_with_a_grid;

/// A blast spawns in the first free slot, ages every tick, and is gone
/// exactly at [`PLASMA_BLAST_LIFETIME_SECONDS`] - the same hardcoded retire
/// time `PlasmaBlast_Update` carries, see `plasma.md`.
#[test]
fn a_blast_ages_and_retires_at_the_hardcoded_lifetime() {
    let mut race = race_with_a_grid();
    let at = Vec3::new(1.0, 2.0, 3.0);
    race.spawn_plasma_blast_model(at);

    let draws = race.plasma_blast_draws(Mat4::IDENTITY);
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
        race.plasma_blast_draws(Mat4::IDENTITY)
            .iter()
            .flatten()
            .count(),
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
        race.plasma_blast_draws(Mat4::IDENTITY)
            .iter()
            .flatten()
            .count(),
        PLASMA_BLAST_SLOTS
    );

    // One more spawn, with the pool already full - well outside the
    // `0..PLASMA_BLAST_SLOTS` fill above so it cannot alias one of those.
    let dropped = Vec3::splat(PLASMA_BLAST_SLOTS as f32 + 1000.0);
    race.spawn_plasma_blast_model(dropped);
    let draws = race.plasma_blast_draws(Mat4::IDENTITY);
    assert_eq!(
        draws.iter().flatten().count(),
        PLASMA_BLAST_SLOTS,
        "still full, not evicted"
    );
    assert!(
        draws
            .iter()
            .flatten()
            .all(|draw| draw.matrices[1].w_axis.truncate() != dropped),
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

/// HD hands every material the blast's age as its `UV_offset` clock
/// (`MeshImporter_SetTime` via `WeaponExplosions_Draw`); Pulse has no such
/// binding and carries zero. Fails if the draw stops carrying the age.
#[test]
fn an_hd_blast_draws_with_its_age_as_the_model_clock_and_pulse_with_none() {
    let dt = 1.0 / 60.0;
    for hd in [true, false] {
        let mut race = race_with_a_grid();
        race.view.hd_plasma_blast = hd;
        race.spawn_plasma_blast_model(Vec3::ZERO);
        for _ in 0..30 {
            race.advance_plasma_blast_models(dt);
        }
        let draw = race
            .plasma_blast_draws(Mat4::from_translation(Vec3::X))
            .into_iter()
            .flatten()
            .next()
            .expect("one live blast");
        if hd {
            assert!((draw.clock - 0.5).abs() < 1e-3, "{}", draw.clock);
        } else {
            // Pulse plays its tracks at the age too, but through the node and
            // texture scrubs, not a material-bound clock.
            assert!(!draw.hd);
        }
    }
}

const DT: f32 = 1.0 / 60.0;

fn pulse_blast_after(ticks: u32) -> (Race, PlasmaBlastDraw) {
    let mut race = race_with_a_grid();
    race.view.hd_plasma_blast = false;
    race.spawn_plasma_blast_model(Vec3::new(292.1, -42.7, -194.4));
    for _ in 0..ticks {
        race.advance_plasma_blast_models(DT);
    }
    let draw = race
        .plasma_blast_draws(Mat4::IDENTITY)
        .into_iter()
        .flatten()
        .next()
        .expect("one live blast");
    (race, draw)
}

/// The three models' scale follows `target * (1 - (1 - rate)^n)`: the first
/// drawn tick, the third and the sixth read off the original's GE dumps
/// (`hemisphere2` 9.47, 26.47, 47.76; `hemisphere1` 7.9; the halo 32.0). Fails
/// when the blast draws at the authored size (about 8 units), which is what it
/// did before the ease was wired.
#[test]
fn the_pulse_blast_scale_is_the_dumps_per_tick_ease() {
    for (ticks, halo, hemisphere2, hemisphere1) in [
        (1, Some(32.0), 9.47, 7.9),
        (3, Some(86.74), 26.47, 22.07),
        (6, Some(149.97), 47.76, 39.82),
    ] {
        let (_, draw) = pulse_blast_after(ticks);
        let scale = |m: Mat4| m.x_axis.truncate().length() / PULSE_MODEL_UNIT;
        let near = |got: f32, want: f32| (got - want).abs() < want * 0.01 + 0.05;
        assert!(
            near(scale(draw.matrices[1]), hemisphere2),
            "{ticks}: {}",
            scale(draw.matrices[1])
        );
        assert!(
            near(scale(draw.matrices[2]), hemisphere1),
            "{ticks}: {}",
            scale(draw.matrices[2])
        );
        if let Some(halo) = halo {
            assert!(
                near(scale(draw.matrices[0]), halo),
                "{ticks}: {}",
                scale(draw.matrices[0])
            );
        }
    }
}

/// Each model's grey ambient, at the ages the dumps carry its byte for:
/// `hemisphere1` 0x9d at 0.383 and 0x72 at 0.55, the halo 0xc4 at 0.383,
/// `hemisphere2` 0xc9 at 0.55 and 0x5f at 1.05; all zero from their end time.
#[test]
fn the_tints_are_the_dumps_three_linear_ramps() {
    // The GE's float-to-byte conversion truncates.
    let byte = |t: f32| (t * 255.0) as i32;
    let at = |age: f32| pulse_tints(age).map(byte);
    assert_eq!(at(0.3833)[2], 0x9d);
    assert_eq!(at(0.55)[2], 0x72);
    assert_eq!(at(0.3833)[0], 0xc4);
    assert_eq!(at(0.55)[1], 0xc9);
    assert_eq!(at(1.05)[1], 0x5f);
    assert_eq!(at(0.0), [255, 255, 255]);
    assert_eq!(pulse_tints(1.2)[2], 0.0);
    assert_eq!(pulse_tints(1.0)[0], 0.0);
}

/// The hemispheres are placed once and stay: turning the camera and moving
/// it through the shell changes neither basis, so the eye can pass through
/// `hemisphere1`'s one-sided wall. The halo takes the camera's orientation
/// and not its position.
#[test]
fn the_hemispheres_are_world_fixed_and_the_halo_follows_the_cameras_orientation() {
    let (race, _) = pulse_blast_after(5);
    let blast = Vec3::new(292.1, -42.7, -194.4);
    let [heading, up, right] = race.track_frame_at(blast);
    // The camera's world transform: local -Z is the way it looks.
    let camera = |eye: Vec3, look: Vec3| {
        let right = look.cross(Vec3::Y).normalize();
        Mat4::from_cols(
            right.extend(0.0),
            right.cross(look).extend(0.0),
            (-look).extend(0.0),
            eye.extend(1.0),
        )
    };
    let near = camera(blast - Vec3::X * 100.0, Vec3::X);
    let inside = camera(blast, (Vec3::X + Vec3::Z * 0.2).normalize());
    let (a, b) = (
        race.plasma_blast_draws(near)[0].unwrap(),
        race.plasma_blast_draws(inside)[0].unwrap(),
    );
    for model in [1, 2] {
        assert_eq!(
            a.matrices[model], b.matrices[model],
            "model {model} moved with the camera"
        );
    }
    let basis = |m: Mat4, axis: usize| m.col(axis).truncate().normalize();
    assert!(
        (basis(a.matrices[2], 0) - heading).length() < 1e-5,
        "x is the track heading"
    );
    assert!(
        (heading.cross(up) - right).length() < 1e-5 && heading.dot(up).abs() < 1e-5,
        "a rotation"
    );
    assert!(
        (basis(a.matrices[2], 1) - up).length() < 1e-5,
        "y is the up"
    );
    assert!(
        (basis(a.matrices[2], 2) - right).length() < 1e-5,
        "z is the right"
    );
    // The halo's `z` is the way the camera looks, wherever the camera is.
    assert!((basis(a.matrices[0], 2) - Vec3::X).length() < 1e-5);
    assert!((basis(b.matrices[0], 2) - (Vec3::X + Vec3::Z * 0.2).normalize()).length() < 1e-5);
    assert!((basis(b.matrices[0], 1) - Vec3::Y).length() < 1e-4);
}

/// The clock every Pulse model plays is the age itself. `age * rate` (0.07,
/// 0.1) is what it was, and spun `hemisphere2` six degrees in the time the
/// original spins it 130.
#[test]
fn the_pulse_models_play_their_node_and_texture_tracks_at_the_blast_age() {
    let (_, draw) = pulse_blast_after(43);
    assert!((draw.clock - 43.0 * DT).abs() < 1e-5);
    assert!(!draw.hd);
}

/// The halo stops drawing inside the bracket the dumps give (drawn at 0.55,
/// gone at 0.717) and the hemispheres keep drawing to the retire.
#[test]
fn the_halo_is_hidden_inside_the_measured_bracket() {
    let scale = |ticks, model: usize| pulse_blast_after(ticks).1.matrices[model].x_axis.length();
    assert!(scale(33, 0) > 0.0, "drawn at 0.55 s");
    assert_eq!(scale(43, 0), 0.0, "gone at 0.717 s");
    assert!(scale(43, 1) > 0.0 && scale(43, 2) > 0.0);
}
