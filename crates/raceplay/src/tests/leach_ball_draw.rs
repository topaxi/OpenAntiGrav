//! Where the LeachBeam ball model sits, and which titles ever load one to
//! draw - see `oag_fx::beam::hd_ball` and
//! `race/weapons/visuals.rs::leach_ball_model_matrix`. The law itself
//! (`hd_ball::period`/`position`) has its own coverage in
//! `oag_fx::beam::tests::hd_ball`; these test the composition root's own
//! read of it against a live `Race`.

use super::*;
use oag_weapons::projectile::leach_beam::Beam;

fn locked_race(owner_at: Vec3, target_at: Vec3, elapsed: f32) -> Race {
    let mut race = Race::start(setup(hulled_handling()));
    let stats = one_leach_beam_table()
        .leach_beam()
        .expect("the fixture authors a LeachBeam");
    race.sim.world.leach_beam = Some(Beam::locked(0, 1, &stats));
    race.sim.world.ships[0].physics.body.position = owner_at;
    race.sim.world.ships[1].physics.body.position = target_at;
    race.view.leach_ball_elapsed = elapsed;
    race
}

/// The matrix's own translation is exactly `hd_ball::position`'s own answer
/// for the same inputs - the composition root reads owner/target off the
/// live ships and elapsed off `RaceView::leach_ball_elapsed` rather than
/// re-deriving either.
#[test]
fn the_matrix_sits_where_hd_ball_position_says_it_should() {
    let (owner, target) = (Vec3::new(30.0, 0.0, 0.0), Vec3::ZERO);
    let elapsed = 0.2;
    let race = locked_race(owner, target, elapsed);
    let length = (target - owner).length();
    let expected = oag_fx::beam::hd_ball::position(elapsed, length, owner, target);
    let matrix = race
        .leach_ball_model_matrix()
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

/// No beam at all draws no ball - the gate [`Race::advance_leach_beam_ribbon`]
/// takes for the ribbon and the burst alike.
#[test]
fn nothing_locked_draws_no_ball() {
    let race = Race::start(setup(hulled_handling()));
    assert!(race.sim.world.leach_beam.is_none());
    assert!(race.leach_ball_model_matrix().is_none());
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
    race.sim.world.leach_beam = Some(Beam::unlocked(0, &stats));
    assert!(race.leach_ball_model_matrix().is_none());
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
