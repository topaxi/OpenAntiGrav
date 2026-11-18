//! Where the LeachBeam ball model sits, and which titles ever load one to
//! draw - see `oag_fx::beam::hd_ball` and
//! `race/weapons/visuals.rs::leach_ball_model_matrix`. The law itself
//! (`oag_weapons::projectile::leach_beam::trip` and `hd_ball::position`) has
//! its own coverage in those crates; these test the composition root's own
//! read of it against a live `Race`.

use super::*;
use oag_weapons::projectile::leach_beam::Beam;

fn locked_race(owner_at: Vec3, target_at: Vec3, elapsed: f32) -> Race {
    let mut race = Race::start(setup(hulled_handling()));
    let stats = one_leach_beam_table()
        .leach_beam()
        .expect("the fixture authors a LeachBeam");
    race.sim.world.leach_beams[0] = Some(Beam::locked(0, 1, &stats));
    race.sim.world.ships[0].physics.body.position = owner_at;
    race.sim.world.ships[1].physics.body.position = target_at;
    race.view.leach_ball_elapsed[0] = elapsed;
    race
}

/// The fraction a ball `elapsed` seconds into its trip has reached in `race`,
/// read the way the original does: the period from the two craft's gap along
/// the course, not their distance in space.
fn fraction(race: &Race, elapsed: f32) -> f32 {
    let ships = &race.sim.world.ships;
    let gap = oag_weapons::projectile::leach_beam::trip::gap(
        ships[0].standing.progress,
        ships[1].standing.progress,
        race.sim.leach_course_length(),
    );
    (elapsed / oag_weapons::projectile::leach_beam::trip::period(gap)).clamp(0.0, 1.0)
}

/// With no strip (a title that draws none), the ball falls back to the
/// straight line between the bodies at the fraction the track gap gives.
#[test]
fn without_a_strip_the_ball_sits_on_the_chord_at_the_track_gaps_fraction() {
    let (owner, target) = (Vec3::new(30.0, 0.0, 0.0), Vec3::ZERO);
    let mut race = locked_race(owner, target, 0.2);
    race.sim.world.ships[0].standing.progress = Some(50.0);
    race.sim.world.ships[1].standing.progress = Some(110.0);
    let expected = oag_fx::beam::hd_ball::position(fraction(&race, 0.2), owner, target);
    let matrix = race
        .leach_ball_model_matrix(0)
        .expect("a locked beam must place the ball");
    assert!(
        matrix.w_axis.truncate().distance(expected) < 1e-4,
        "{:?} vs {expected:?}",
        matrix.w_axis.truncate()
    );
    // Translation only - see `leach_ball_model_matrix`'s own doc comment for
    // why orientation and scale are chosen, not measured.
    assert_eq!(matrix.x_axis, oag_core::math::Vec4::new(1.0, 0.0, 0.0, 0.0));
    assert_eq!(matrix.y_axis, oag_core::math::Vec4::new(0.0, 1.0, 0.0, 0.0));
    assert_eq!(matrix.z_axis, oag_core::math::Vec4::new(0.0, 0.0, 1.0, 0.0));
}

/// **The ball rides the drawn strip, not the chord** (`BeamPath_PointAt`,
/// `0x00116308`): on an L-shaped strip from the target round to the shooter,
/// the ball halfway through its trip sits on the corner, and a quarter of the
/// way it sits on the first leg, far from the straight line between the ends.
#[test]
fn the_ball_follows_the_strip_from_the_target_to_the_shooter() {
    let (target, corner, shooter) = (
        Vec3::ZERO,
        Vec3::new(20.0, 0.0, 0.0),
        Vec3::new(20.0, 20.0, 0.0),
    );
    let mut race = locked_race(shooter, target, 0.0);
    race.view.leach_strip = Some(crate::leach_strip::Strip::new(
        [None; oag_gameplay::MAX_SHIPS],
    ));
    assert!(race.force_leach_path(vec![target, corner, shooter]));
    // The target ahead: the walked end, sample 0, is the target's.
    race.sim.world.ships[0].standing.progress = Some(50.0);
    race.sim.world.ships[1].standing.progress = Some(110.0);
    let period = 0.2 / fraction(&race, 0.2);
    let at = |race: &mut Race, part: f32| {
        race.view.leach_ball_elapsed[0] = part * period;
        race.leach_ball_model_matrix(0)
            .expect("a locked beam must place the ball")
            .w_axis
            .truncate()
    };
    let half = at(&mut race, 0.5);
    assert!(half.distance(corner) < 1e-3, "halfway {half} is the corner");
    let quarter = at(&mut race, 0.25);
    assert!(
        quarter.distance(Vec3::new(10.0, 0.0, 0.0)) < 1e-3,
        "a quarter in {quarter} is on the first leg"
    );
    let chord = target.lerp(shooter, 0.25);
    assert!(quarter.distance(chord) > 5.0, "not on the chord at {chord}");
    let start = at(&mut race, 0.0);
    assert!(
        start.distance(target) < 1e-3,
        "the trip starts at the target"
    );
}

/// No beam at all draws no ball - the gate [`Race::advance_leach_beam_ribbon`]
/// takes for the ribbon and the burst alike.
#[test]
fn nothing_locked_draws_no_ball() {
    let race = Race::start(setup(hulled_handling()));
    assert!(race.sim.world.leach_beams[0].is_none());
    assert!(race.leach_ball_model_matrix(0).is_none());
}

/// Fired with no lock: a real `Beam` exists (see `leach_beam.rs`'s own
/// `a_leach_beam_fired_at_nobody_is_spent_and_expires_on_its_own_clock`) but
/// it is not [`oag_weapons::projectile::leach_beam::Kind::Locked`], so it
/// draws no ball either.
#[test]
fn an_unlocked_beam_draws_no_ball_either() {
    let stats = one_leach_beam_table()
        .leach_beam()
        .expect("the fixture authors a LeachBeam");
    let mut race = Race::start(setup(hulled_handling()));
    race.sim.world.leach_beams[0] = Some(Beam::unlocked(0, &stats));
    assert!(race.leach_ball_model_matrix(0).is_none());
}

/// **Only Wipeout HD authors this model.** `Race::leach_ball_model_matrix`
/// itself is title-agnostic - it is `Scene::leach_ball` staying `None` on
/// every other title that keeps nothing drawn, through the same
/// `WeaponModels::leachbeam_ball` axis every other weapon body reads (see
/// `load::weapon_models::load_bodies`, which never reaches an archive at
/// all for an entry that is `None` on the title's own table).
#[test]
fn only_wipeout_hd_authors_a_leachbeam_ball_model() {
    assert!(oag_hd::TITLE.weapon_models.leachbeam_ball.is_some());
    assert!(oag_pulse::TITLE.weapon_models.leachbeam_ball.is_none());
    assert!(oag_pure::TITLE.weapon_models.leachbeam_ball.is_none());
}

/// **Only Wipeout HD binds no ghost static.** Its executable has no
/// `staticglow` string, so the loader never asks for the entry and HD's report
/// carries no absence line for it; every other title keeps Pulse's request.
#[test]
fn only_wipeout_hd_binds_no_ghost_static() {
    assert!(oag_hd::TITLE.weapon_models.ghost_static.is_none());
    for title in [
        &oag_pulse::TITLE,
        &oag_pure::TITLE,
        &oag_2048::TITLE,
        &oag_omega::TITLE,
    ] {
        let entry = title.weapon_models.ghost_static.map(|g| g.entry);
        assert_eq!(entry, Some(r"Data\Tex\staticglow.mip"), "{}", title.name);
    }
}
