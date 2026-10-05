//! Reset volumes, the recovery pose and the scrape sparks: what happens when
//! a craft leaves the circuit or grinds along it.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`.

use super::*;
use oag_gameplay::PlayerInputs;

/// A respawn in flight teleports the craft and leaves the camera spring
/// catching up, so neither position describes the shot.
#[test]
fn a_respawn_in_flight_makes_everything_visible() {
    let mut race = Race::start(setup(Handling::default()));
    race.sim.respawn_cooldown[0] = 1;
    assert_eq!(race.visibility_sections(), (UNPLACED, UNPLACED));

    // And an opponent recovering across the circuit does not blank the
    // shot: the camera is the player's, and only the player's respawn can
    // make it untrustworthy.
    race.sim.respawn_cooldown[0] = 0;
    race.sim.respawn_cooldown[3] = 1;
    assert_ne!(race.visibility_sections(), (UNPLACED, UNPLACED));
    race.sim.respawn_cooldown[0] = 1;

    // And the resulting set really is everything, not merely two unknown
    // ids - this is the property the whole conservative path exists for.
    let pvs = oag_vex::pvs::TrackPvs::empty();
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
        let body = &mut race.sim.world.ships[0].physics.body;
        body.position = Vec3::new(20.0, -20.0, 0.0);
        body.linear_velocity = Vec3::new(0.0, -600.0, 0.0);
    }

    for _ in 0..10 {
        race.tick(&PlayerInputs::none());
    }

    assert_eq!(race.respawns(), 1, "the reset plane never triggered");

    let body = race.sim.world.ships[0].physics.body;
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
            let body = &mut race.sim.world.ships[0].physics.body;
            body.position = Vec3::new(20.0, -20.0, 0.0);
            body.linear_velocity = Vec3::new(0.0, -600.0, 0.0);
        }
        for _ in 0..10 {
            race.tick(&PlayerInputs::none());
        }
        assert_eq!(race.respawns(), 0, "{surface:?} respawned the ship");
    }
}

/// **The second way a craft is lost, and the one the reset volumes cannot
/// catch**: the player leaves the circuit sideways into open space, where there
/// is no authored geometry of any class to touch. Nothing recovered them before
/// 2026-08-17 - a craft that came off after a Turbo fell for the rest of the
/// race. See [`PLAYER_RESCUE_HALF_WIDTHS`].
#[test]
fn a_player_that_leaves_the_circuit_is_put_back_after_the_dwell() {
    let mut race = Race::start(setup(hulled_handling()));
    // Well past the threshold and nowhere near anything: this fixture has no
    // colliders at all, so no reset volume can be what puts the craft back.
    let away = Vec3::new(20.0, 6.0, 400.0);
    race.sim.world.ships[0].physics.body.position = away;
    assert!(
        race.spline().distance_to(away).expect("a sample") > race.sim.player_rescue_distance,
        "the fixture does not put the craft off the track"
    );

    // One short of the dwell: still out there, still not touched.
    for _ in 0..PLAYER_RESCUE_TICKS - 1 {
        race.tick(&PlayerInputs::none());
        race.sim.world.ships[0].physics.body.position = away;
    }
    assert_eq!(race.respawns(), 0, "recovered before the dwell elapsed");

    race.tick(&PlayerInputs::none());
    assert_eq!(race.respawns(), 1, "the dwell elapsed and nothing happened");
    let position = race.sim.world.ships[0].physics.body.position;
    let distance = race.spline().distance_to(position).expect("a sample");
    assert!(distance < 20.0, "recovered {distance} from the spline");
}

/// **The control**: a craft on the track is never taken off it.
///
/// Held just inside the threshold for three times the dwell, which is the shape
/// of the failure that would matter - a rescue that fires during ordinary wide
/// cornering would teleport a player who was driving perfectly well.
#[test]
fn a_player_inside_the_threshold_is_left_alone() {
    let mut race = Race::start(setup(hulled_handling()));
    let inside = Vec3::new(20.0, 6.0, race.sim.player_rescue_distance - 1.0);
    for _ in 0..PLAYER_RESCUE_TICKS * 3 {
        race.sim.world.ships[0].physics.body.position = inside;
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.respawns(), 0, "a craft on the track was recovered");
}

/// **The other dwell, on an opponent rather than the player.**
///
/// This used to be a ground-truth test driving a real opponent into a
/// *sustained* stall: `05_Track`'s racing line ran above its own collision
/// surface, and a craft that came off there wedged and sat below
/// `STALL_SPEED` continuously. `oag_physics`' hover sweep closed that
/// specific shape of the gap - see `docs/gameplay/ai.md`, "the last three,
/// with a mechanism the original does not have" - and a sweep of every
/// circuit at every difficulty afterwards found no cell reaching
/// [`STALL_TICKS`] by sustained dwell any more (the closest, `07_Track` at
/// novice, tops out at 46). `crates/game/tests/stall_rescue_ground_truth.rs`
/// records that measurement and, separately, that `05_Track` and `07_Track`
/// at novice still make almost no progress - a *bounce-in-place* failure the
/// sustained-dwell counter cannot see by construction, and which is not
/// closed. So this drives [`Race::stalled`] directly rather than through a
/// real circuit either way: it proves the dwell counter itself does what it
/// was built to do, which was never in question - the open question is a
/// craft that never describes the state the counter watches for, and no
/// direct call to `stalled` can demonstrate that either way.
#[test]
fn a_stopped_opponent_is_flagged_after_the_dwell() {
    let mut race = Race::start(setup(hulled_handling()));
    let ship = &mut race.sim.world.ships[1];
    ship.physics.craft_state = oag_physics::CraftState::Racing;
    ship.physics.thrust = 100.0;
    ship.physics.body.linear_velocity = Vec3::ZERO;

    for _ in 0..STALL_TICKS - 1 {
        assert!(!race.stalled(1), "flagged before the dwell elapsed");
    }
    assert!(race.stalled(1), "the dwell elapsed and nothing happened");
}

/// **The control**: a craft asking nothing of the throttle is never flagged,
/// however long it sits - the grid before the lights being the case this
/// excludes.
#[test]
fn an_opponent_holding_no_throttle_never_stalls() {
    let mut race = Race::start(setup(hulled_handling()));
    let ship = &mut race.sim.world.ships[1];
    ship.physics.craft_state = oag_physics::CraftState::Racing;
    ship.physics.thrust = 0.0;
    ship.physics.body.linear_velocity = Vec3::ZERO;

    for _ in 0..STALL_TICKS * 3 {
        assert!(
            !race.stalled(1),
            "a craft holding no throttle was flagged as stalled"
        );
    }
}

/// **Where the craft is put back is where it left, not where it ended up.**
///
/// By the time the dwell expires the craft is a long way from the circuit, and
/// the nearest sample *there* can belong to a different part of it - recovering
/// onto that would teleport the player across the lap counter. Here the craft
/// drives to one end of the straight and is then thrown out to where the far end
/// is the nearer, which is the same shape at fixture scale.
#[test]
fn the_recovery_pose_is_the_last_place_the_craft_was_on_the_track() {
    let mut race = Race::start(setup(hulled_handling()));
    // On the line, at the far end of the straight: this is the sample the latch
    // has to keep.
    let left_from = Vec3::new(60.0, 6.0, 0.0);
    race.sim.world.ships[0].physics.body.position = left_from;
    race.tick(&PlayerInputs::none());

    // And now out beyond the *near* end, so the nearest sample to the craft is
    // sample zero rather than anything it drove past.
    let away = Vec3::new(0.0, -400.0, 0.0);
    for _ in 0..PLAYER_RESCUE_TICKS {
        race.sim.world.ships[0].physics.body.position = away;
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.respawns(), 1, "the craft was never recovered");

    let position = race.sim.world.ships[0].physics.body.position;
    assert!(
        position.x > 40.0,
        "recovered at {position:?}, which is the end of the straight the craft \
         had not reached rather than the one it left from"
    );
}

/// **The firehose trap.** A ship held against a wall for many ticks must
/// spawn one spark burst per `oag_fx::sparks::COLLISION_COOLDOWN`,
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
        vec![upward_plane(-40.0, oag_physics::Surface::Wall)],
    );
    let mut race = Race::start(setup);

    // Reset to an inbound approach before every tick, so each one sees a
    // fresh impact rather than the ship bouncing away after the first.
    let push_toward_wall = |race: &mut Race| {
        let body = &mut race.sim.world.ships[0].physics.body;
        body.position = Vec3::new(20.0, -39.7, 0.0);
        body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
        body.orientation = oag_core::math::Quat::IDENTITY;
        body.angular_velocity = Vec3::ZERO;
    };

    push_toward_wall(&mut race);
    let evaluated = race.tick(&PlayerInputs::none());
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
    // `crates/fx/tests/psys_ground_truth.rs`.
    for tick in 0..10 {
        push_toward_wall(&mut race);
        race.tick(&PlayerInputs::none());
        assert_eq!(
            race.spark_ignitions(),
            1,
            "tick {tick} of the same scrape ignited another burst before the cooldown elapsed"
        );
    }
}

/// The other half of the firehose-trap guard: unlike a one-shot edge
/// latch, `ShipCollisionFx_Trigger`'s recovered behaviour is a periodic
/// re-fire - `oag_fx::sparks::COLLISION_COOLDOWN` (`0.8` s) after the
/// last burst, for as long as contact continues. A burst's own trailing
/// embers can outlive the cooldown (up to `32 + 30` ticks, about
/// `1.03` s), so an empty pool is *not* a precondition of the re-fire
/// any more - the ignition counter is the unambiguous signal.
#[test]
fn a_sustained_scrape_refires_after_the_cooldown_elapses() {
    let handling = hulled_handling();
    let setup = setup_with(
        handling,
        vec![upward_plane(-40.0, oag_physics::Surface::Wall)],
    );
    let mut race = Race::start(setup);
    let dt = race.dt();

    let push_toward_wall = |race: &mut Race| {
        let body = &mut race.sim.world.ships[0].physics.body;
        body.position = Vec3::new(20.0, -39.7, 0.0);
        body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
        body.orientation = oag_core::math::Quat::IDENTITY;
        body.angular_velocity = Vec3::ZERO;
    };

    push_toward_wall(&mut race);
    let evaluated = race.tick(&PlayerInputs::none());
    assert!(
        evaluated.wall.impact,
        "the fixture never reaches the wall - not what this test means to check"
    );
    assert_eq!(race.spark_ignitions(), 1);

    // Run past the cooldown: a second burst must have ignited, and only
    // one.
    let ticks = (oag_fx::sparks::COLLISION_COOLDOWN / dt).ceil() as usize + 1;
    for _ in 0..ticks {
        push_toward_wall(&mut race);
        race.tick(&PlayerInputs::none());
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
        race.tick(&PlayerInputs::none());
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
    race.tick(&PlayerInputs::none());
    assert_eq!(race.respawns(), 1);
    for _ in 0..(RESPAWN_COOLDOWN_TICKS - 1) {
        race.tick(&PlayerInputs::none());
        assert_eq!(race.respawns(), 1, "respawned again inside the cooldown");
    }
}

/// A `.pob` with one root emitter, whose flags word the caller chooses.
///
/// Hand-laid bytes, no game content: the container's header, an empty slot
/// table, and a single record at the resource base carrying the smallest set
/// of fields `oag_fx::psys::Effect::parse` needs to accept it. Field
/// offsets are the ones `oag_pob::Emitter` documents.
pub(crate) fn one_emitter_pob(name: &str, flags: u32) -> Vec<u8> {
    use oag_pob::{EMITTER_LEN, HEADER_LEN, MAGIC, NAME_LEN};

    let mut record = vec![0u8; EMITTER_LEN];
    let take = name.len().min(NAME_LEN - 1);
    record[..take].copy_from_slice(&name.as_bytes()[..take]);
    let mut put = |at: usize, bits: u32| record[at..at + 4].copy_from_slice(&bits.to_le_bytes());
    put(0x20, flags);
    put(0x24, 4.0f32.to_bits()); // duration, in ticks
    put(0x5c, 10); // lifetime centre
    put(0x64, 1); // interval min
    put(0x68, 1); // interval max
    put(0x6c, 1); // per emission, min and max
    put(0x70, 1);
    put(0xa0, 64); // live cap
    put(0xb8, 2); // render mode -> draw class 3, a billboard
    put(0xc0, 2); // blend class -> additive
    put(0x4d8 + 0x10, 1.0f32.to_bits()); // size channel, upper bound
    put(0x5b8 + 0x04, 2); // alpha channel, constant
    put(0x5b8 + 0x10, 100.0f32.to_bits());
    put(0x9ac, 1); // one atlas frame
    record[0x9a0..0x9a4].copy_from_slice(&[1, 0, 1, 0]); // a 1x1 atlas grid

    let mut blob = Vec::new();
    blob.extend_from_slice(MAGIC);
    blob.extend_from_slice(&((HEADER_LEN + EMITTER_LEN - NAME_LEN) as u32).to_le_bytes());
    blob.extend_from_slice(&0u16.to_le_bytes()); // no slots
    blob.extend_from_slice(&1u16.to_le_bytes());
    blob.extend_from_slice(&1u32.to_le_bytes());
    blob.extend_from_slice(&record);
    blob
}

/// A race whose collision-spark effect is the one supplied, against a wall.
fn race_with_spark_effect(blob: &[u8]) -> Race {
    let mut setup = setup_with(
        hulled_handling(),
        vec![upward_plane(-40.0, oag_physics::Surface::Wall)],
    );
    let effect = oag_fx::psys::Effect::parse(blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    setup.effects.insert(oag_fx::sparks::DAMAGE_EFFECT, effect);
    Race::start(setup)
}

fn push_toward_wall(race: &mut Race) {
    let body = &mut race.sim.world.ships[0].physics.body;
    body.position = Vec3::new(20.0, -39.7, 0.0);
    body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
    body.orientation = oag_core::math::Quat::IDENTITY;
    body.angular_velocity = Vec3::ZERO;
}

/// **Wipeout HD's shape of this effect, and the bug it caused.** HD authors
/// all four emitters of `WO_SHIP_COLL_SPARK_DAMAGE` with
/// `pob::flags::LOOPING` set, where Pulse authors none of them looping. A
/// looping emitter has no countdown - `EmitterSpec::run_ticks` is infinite -
/// so a trigger that only ignites leaves it emitting for the rest of the
/// race, which is exactly what was seen: sparks that kept pouring off a craft
/// that had long since left the wall.
///
/// The rule is keyed on the effect's own flag, so this is one race differing
/// from the next by one bit of authored data and nothing else.
#[test]
fn a_looping_spark_effect_stops_when_the_contact_does() {
    use oag_pob::flags;

    let mut race = race_with_spark_effect(&one_emitter_pob("WO_TEST_SPARK", flags::LOOPING));

    push_toward_wall(&mut race);
    let evaluated = race.tick(&PlayerInputs::none());
    assert!(
        evaluated.wall.impact,
        "the fixture never reaches the wall - not what this test means to check"
    );
    assert_eq!(race.spark_ignitions(), 1);
    assert!(race.sparks().is_running());

    // Held against the wall: one attachment, not one per tick and not one
    // per cooldown - a chattering scrape must not go dark waiting to re-arm.
    let ticks = (oag_fx::sparks::COLLISION_COOLDOWN / race.dt()).ceil() as usize + 2;
    for _ in 0..ticks {
        push_toward_wall(&mut race);
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(
        race.spark_ignitions(),
        1,
        "an attached effect re-ignited instead of staying attached"
    );
    assert!(
        race.sparks().is_running(),
        "the scrape went dark mid-contact"
    );

    // Let go. Emission stops at once; the particles already alive finish
    // their own lives, so the system drains rather than blinking out.
    for _ in 0..90 {
        race.sim.world.ships[0].physics.body.position = Vec3::new(20.0, 0.0, 0.0);
        race.sim.world.ships[0].physics.body.linear_velocity = Vec3::ZERO;
        race.tick(&PlayerInputs::none());
    }
    assert!(
        !race.sparks().is_running(),
        "the sparks kept emitting after the contact ended"
    );
}

/// The other half of the same rule: a **non**-looping effect is Pulse's
/// burst, and releasing the wall must not cut it short. Its own duration ends
/// it, as `ShipCollisionFx_Trigger` has it.
#[test]
fn a_burst_spark_effect_is_not_cut_short_by_letting_go() {
    let mut race = race_with_spark_effect(&one_emitter_pob("WO_TEST_SPARK", 0));

    push_toward_wall(&mut race);
    race.tick(&PlayerInputs::none());
    assert_eq!(race.spark_ignitions(), 1);
    assert!(race.sparks().is_running());

    // One tick clear of the wall: the burst is still going, because nothing
    // owns it and its 4-tick schedule has not run out.
    race.sim.world.ships[0].physics.body.position = Vec3::new(20.0, 0.0, 0.0);
    race.sim.world.ships[0].physics.body.linear_velocity = Vec3::ZERO;
    race.tick(&PlayerInputs::none());
    assert!(
        race.sparks().is_running(),
        "letting go truncated a burst that owns its own schedule"
    );
}

/// A raised shield turns a wall contact into a **shell bulge and no sparks**,
/// which is the whole chain the shield exists to produce.
///
/// Both halves are recovered from the same contact loop, and both are gated on
/// the same `craft+0x1b8 & 0x10`: its third gate (`0x0884255c`) branches past
/// `Ship_DispatchCollisionFx`, and its fourth (`0x08842684`) replaces the
/// hull-damage call with `ShipShield_Hit`. See
/// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
///
/// **This is the only test that runs the whole path end to end** -
/// `damage::subtract` setting `absorbed`, `tick` turning that into
/// `ShipShield::hit`, and `hit` moving the drawn scale. The three links are
/// each covered on their own; nothing else asserts they are joined.
///
/// The fixture is [`a_sustained_scrape_spawns_sparks_once_not_every_tick`]'s,
/// for the reason that test gives: a fresh wall is easy to wind backwards and
/// fails silently.
#[test]
fn a_shielded_scrape_bulges_the_shell_and_throws_no_sparks() {
    let handling = hulled_handling();
    let setup = setup_with(
        handling,
        vec![upward_plane(-40.0, oag_physics::Surface::Wall)],
    );
    let mut race = Race::start(setup);

    race.sim.world.ships[0].physics.shield_pickup_timer = 1.0;
    race.view.shield[0].activate();
    let resting = race.shield_of(0).scale();

    let body = &mut race.sim.world.ships[0].physics.body;
    body.position = Vec3::new(20.0, -39.7, 0.0);
    body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
    body.orientation = oag_core::math::Quat::IDENTITY;
    body.angular_velocity = Vec3::ZERO;
    let evaluated = race.tick(&PlayerInputs::none());

    assert!(
        evaluated.wall.impact,
        "the fixture never reaches the wall - not what this test means to check"
    );
    assert!(
        evaluated.shield.absorbed,
        "the shield did not report swallowing the contact"
    );
    assert_eq!(
        race.spark_ignitions(),
        0,
        "a shielded craft threw hull sparks"
    );
    // One tick of the `0.2`-per-substep settle has already run, so this is a
    // little under the full tenth rather than exactly it. Well clear of the
    // `SCALE_FLICKER` the breathing moves, which is what makes the assertion
    // about the bulge and not about the phase.
    let bulge = race.shield_of(0).scale() - resting;
    assert!(
        bulge > oag_render::shield::SCALE_FLICKER * 2.0,
        "the shell did not bulge on the absorbed hit: moved {bulge}"
    );
    // And it flashes cyan: `ShipShield_Hit` drops the red channel alone, which
    // against the mesh's own blue-violet reads as a hue shift. One tick of the
    // `0.15`-per-substep settle has run, so red is a little above zero rather
    // than exactly it.
    let red = race.shield_of(0).colour()[0];
    assert!(
        red < 0.2,
        "the shell did not flash cyan on the absorbed hit: red is {red}"
    );
}

/// Puts opponent 1 `depth` below its own line point along the sample's up,
/// on the ground, and returns the race. The fixture has no floor under its
/// line, so every sample reads as a gap; `supported` rebuilds the line without
/// that mask, which is what a line over road is.
fn opponent_beneath(depth: f32, supported: bool) -> Race {
    let mut race = race_with_a_grid();
    race.tick(&PlayerInputs::none());
    if supported {
        let line = &race.sim.racing_line;
        race.sim.racing_line = oag_ai::Line::new((0..line.len()).map(|i| line.point(i)).collect());
    }
    let index = race.sim.world.ships[1].driver.index as usize;
    let sample = *race.ai_sample(index).expect("a grid race has samples");
    let up = -Vec3::from_array(sample.down).normalize_or_zero();
    let ship = &mut race.sim.world.ships[1];
    ship.physics.craft_state = oag_physics::CraftState::Racing;
    ship.physics.time_airborne = 0.0;
    ship.physics.body.position = race.sim.racing_line.point(index) - up * depth;
    race
}

/// **A craft driving a road under its own line is lost**, though it is close
/// to the line point: `05_Track` forward's pit under the upper road, 30-80
/// units down and inside the distance trigger. See [`BENEATH_LINE`].
#[test]
fn an_opponent_on_the_ground_well_below_its_line_is_lost_after_the_dwell() {
    let mut race = opponent_beneath(BENEATH_LINE + 5.0, true);
    assert!(race.beneath_the_line(1));
    for _ in 0..RESCUE_TICKS - 1 {
        assert!(
            !race.lost_off_the_circuit(1),
            "flagged before the dwell elapsed"
        );
    }
    assert!(
        race.lost_off_the_circuit(1),
        "the dwell elapsed and nothing happened"
    );
}

/// The controls: a craft a little below its line (a dip, the hover), one in
/// the air under it (a jump), and one under a line that arcs over a gap (it
/// landed early) are not beneath it.
#[test]
fn a_craft_slightly_below_airborne_or_under_a_gap_is_not_beneath_its_line() {
    let race = opponent_beneath(BENEATH_LINE - 5.0, true);
    assert!(!race.beneath_the_line(1));
    let mut race = opponent_beneath(BENEATH_LINE + 5.0, true);
    race.sim.world.ships[1].physics.time_airborne = 0.5;
    assert!(!race.beneath_the_line(1));
    let race = opponent_beneath(BENEATH_LINE + 5.0, false);
    assert!(
        race.sim
            .racing_line
            .is_unsupported(race.sim.world.ships[1].driver.index as usize)
    );
    assert!(!race.beneath_the_line(1));
}
