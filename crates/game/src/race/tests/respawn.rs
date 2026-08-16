//! Reset volumes, the recovery pose and the scrape sparks: what happens when
//! a craft leaves the circuit or grinds along it.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`.

use super::*;

/// A respawn in flight teleports the craft and leaves the camera spring
/// catching up, so neither position describes the shot.
#[test]
fn a_respawn_in_flight_makes_everything_visible() {
    let mut race = Race::start(setup(Handling::default()));
    race.respawn_cooldown[0] = 1;
    assert_eq!(race.visibility_sections(), (UNPLACED, UNPLACED));

    // And an opponent recovering across the circuit does not blank the
    // shot: the camera is the player's, and only the player's respawn can
    // make it untrustworthy.
    race.respawn_cooldown[0] = 0;
    race.respawn_cooldown[3] = 1;
    assert_ne!(race.visibility_sections(), (UNPLACED, UNPLACED));
    race.respawn_cooldown[0] = 1;

    // And the resulting set really is everything, not merely two unknown
    // ids - this is the property the whole conservative path exists for.
    let pvs = oag_formats::pvs::TrackPvs::empty();
    let padding = SectionPadding::default();
    let (craft, camera) = race.visibility_sections();
    assert!(
        VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), craft, camera).is_everything()
    );
}

/// The feature, end to end: a ship that leaves the track and crosses `Reset`
/// geometry is put back on the spline.
///
/// The reset plane is **horizontal and below**, which is where a real one
/// sits, and the hull probes are horizontal - so only the swept ray can find
/// it. That is deliberate: it is the path a falling ship actually takes.
#[test]
fn a_ship_that_falls_through_a_reset_plane_is_put_back_on_the_track() {
    let handling = hulled_handling();
    let setup = setup_with(
        handling,
        vec![plane(1, -40.0, oag_physics::Surface::Reset, 0)],
    );
    let mut race = Race::start(setup);
    assert_eq!(race.respawns(), 0);

    // Throw it off the track. No gravity in this fixture, so nothing else can
    // move it and the only thing under test is the reset path.
    {
        let body = &mut race.world.ships[0].physics.body;
        body.position = Vec3::new(20.0, -20.0, 0.0);
        body.linear_velocity = Vec3::new(0.0, -600.0, 0.0);
    }

    for _ in 0..10 {
        race.tick(&InputSnapshot::default());
    }

    assert_eq!(race.respawns(), 1, "the reset plane never triggered");

    let body = race.world.ships[0].physics.body;
    // Back on the track: near a spline sample, above it, and stopped.
    let distance = race.spline().distance_to(body.position).expect("a sample");
    assert!(distance < 20.0, "respawned {distance} from the spline");
    assert!(body.position.y > 0.0, "respawned at {:?}", body.position);
    assert_eq!(body.linear_velocity, Vec3::ZERO);
}

/// Nothing but `Reset` geometry may respawn a ship. The same plane in the same
/// place under a different class must leave the ship where it fell.
#[test]
fn falling_through_any_other_class_does_not_respawn() {
    for surface in [
        oag_physics::Surface::Wall,
        oag_physics::Surface::Floor,
        oag_physics::Surface::MagFloor,
    ] {
        let setup = setup_with(hulled_handling(), vec![plane(1, -40.0, surface, 0)]);
        let mut race = Race::start(setup);
        {
            let body = &mut race.world.ships[0].physics.body;
            body.position = Vec3::new(20.0, -20.0, 0.0);
            body.linear_velocity = Vec3::new(0.0, -600.0, 0.0);
        }
        for _ in 0..10 {
            race.tick(&InputSnapshot::default());
        }
        assert_eq!(race.respawns(), 0, "{surface:?} respawned the ship");
    }
}

/// **The firehose trap.** A ship held against a wall for many ticks must
/// spawn one spark burst per `oag_render::sparks::COLLISION_COOLDOWN`,
/// not one burst per tick of the ensuing scrape - the shape of bug this
/// guards against is the same one `oag_physics::wall::STUN_PER_CONTACT`'s
/// doc comment records this crate cost a session of play-testing to, for
/// the collision stun rather than sparks. Ten ticks (`10/60 s`) is well
/// inside the cooldown, so this only checks the *no-refire-yet* half; see
/// `a_sustained_scrape_refires_after_the_cooldown_elapses` for the other
/// half.
///
/// The plane and approach are copied from
/// [`falling_through_any_other_class_does_not_respawn`], a known-working
/// `Surface::Wall` fixture, rather than a fresh vertical wall: getting a
/// triangle's winding backwards produces a silent "no contact" rather
/// than a loud failure, and this fixture is already proven to register.
#[test]
fn a_sustained_scrape_spawns_sparks_once_not_every_tick() {
    let handling = hulled_handling();
    let setup = setup_with(
        handling,
        vec![plane(1, -40.0, oag_physics::Surface::Wall, 0)],
    );
    let mut race = Race::start(setup);

    // Reset to an inbound approach before every tick, so each one sees a
    // fresh impact rather than the ship bouncing away after the first.
    let push_toward_wall = |race: &mut Race| {
        let body = &mut race.world.ships[0].physics.body;
        body.position = Vec3::new(20.0, -39.7, 0.0);
        body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
    };

    push_toward_wall(&mut race);
    let evaluated = race.tick(&InputSnapshot::default());
    assert!(
        evaluated.wall.impact,
        "the fixture never reaches the wall - not what this test means to check"
    );
    assert_eq!(race.spark_ignitions(), 1, "the first impact never ignited");

    // A live particle count cannot distinguish one burst from many - a
    // single burst trickles new particles for 32 ticks by design, and
    // the pool needs the disc's own `.pob`, which a headless fixture
    // has not got. The trigger cadence is this test's subject and the
    // counter is the unambiguous signal for it; that a real asset then
    // fills the pool is checked against the disc in
    // `crates/render/tests/psys_ground_truth.rs`.
    for tick in 0..10 {
        push_toward_wall(&mut race);
        race.tick(&InputSnapshot::default());
        assert_eq!(
            race.spark_ignitions(),
            1,
            "tick {tick} of the same scrape ignited another burst before the cooldown elapsed"
        );
    }
}

/// The other half of the firehose-trap guard: unlike a one-shot edge
/// latch, `ShipCollisionFx_Trigger`'s recovered behaviour is a periodic
/// re-fire - `oag_render::sparks::COLLISION_COOLDOWN` (`0.8` s) after the
/// last burst, for as long as contact continues. A burst's own trailing
/// embers can outlive the cooldown (up to `32 + 30` ticks, about
/// `1.03` s), so an empty pool is *not* a precondition of the re-fire
/// any more - the ignition counter is the unambiguous signal.
#[test]
fn a_sustained_scrape_refires_after_the_cooldown_elapses() {
    let handling = hulled_handling();
    let setup = setup_with(
        handling,
        vec![plane(1, -40.0, oag_physics::Surface::Wall, 0)],
    );
    let mut race = Race::start(setup);
    let dt = race.dt();

    let push_toward_wall = |race: &mut Race| {
        let body = &mut race.world.ships[0].physics.body;
        body.position = Vec3::new(20.0, -39.7, 0.0);
        body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
    };

    push_toward_wall(&mut race);
    let evaluated = race.tick(&InputSnapshot::default());
    assert!(
        evaluated.wall.impact,
        "the fixture never reaches the wall - not what this test means to check"
    );
    assert_eq!(race.spark_ignitions(), 1);

    // Run past the cooldown: a second burst must have ignited, and only
    // one.
    let ticks = (oag_render::sparks::COLLISION_COOLDOWN / dt).ceil() as usize + 1;
    for _ in 0..ticks {
        push_toward_wall(&mut race);
        race.tick(&InputSnapshot::default());
    }
    assert_eq!(
        race.spark_ignitions(),
        2,
        "no second burst ignited after the cooldown elapsed"
    );
}

/// The guard against the failure that would otherwise look like a hang: a
/// recovery pose that is itself inside a reset volume respawns forever.
///
/// Here the trigger is a vertical plane half a unit from the spawn, inside the
/// hull's own half-width, so every recovery lands straight back in it.
/// Respawning must stop rather than freeze the race.
#[test]
fn a_recovery_pose_inside_a_reset_volume_gives_up_instead_of_looping() {
    let handling = hulled_handling();
    let start = *setup(handling).spline.start().expect("a first sample");
    // Half a unit to the side of wherever the ship is placed, well inside the
    // one-unit hull half-width.
    let beside = Vec3::from_array(start.pos).x + 0.5;
    let setup = setup_with(
        handling,
        vec![plane(0, beside, oag_physics::Surface::Reset, 0)],
    );

    let mut race = Race::start(setup);
    for _ in 0..(RESPAWN_COOLDOWN_TICKS * (RESPAWN_GIVE_UP + 3)) {
        race.tick(&InputSnapshot::default());
    }

    assert_eq!(
        race.respawns(),
        RESPAWN_GIVE_UP,
        "respawning did not stop after {RESPAWN_GIVE_UP} tries"
    );
}

/// The cooldown is what stops one contact from respawning on every tick while
/// the ship is still overlapping the trigger.
#[test]
fn a_respawn_is_not_repeated_on_the_very_next_tick() {
    let handling = hulled_handling();
    let start = *setup(handling).spline.start().expect("a first sample");
    let beside = Vec3::from_array(start.pos).x + 0.5;
    let setup = setup_with(
        handling,
        vec![plane(0, beside, oag_physics::Surface::Reset, 0)],
    );

    let mut race = Race::start(setup);
    race.tick(&InputSnapshot::default());
    assert_eq!(race.respawns(), 1);
    for _ in 0..(RESPAWN_COOLDOWN_TICKS - 1) {
        race.tick(&InputSnapshot::default());
        assert_eq!(race.respawns(), 1, "respawned again inside the cooldown");
    }
}
