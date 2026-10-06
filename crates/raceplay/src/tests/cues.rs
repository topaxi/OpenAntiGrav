//! Which sound cues a tick raises, and on which tick.
//!
//! The assertion that matters here is **positional**: a cue arriving is easy,
//! a cue arriving on the tick the thing that causes it happened is the part a
//! wiring bug breaks. Nothing here loads a bank or touches a mixer - what a
//! cue *sounds like* is `audio::sfx`'s problem and the disc-backed half is
//! `crates/game/tests/sfx_ground_truth.rs`.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_sound::sfx::Cue;

/// A race whose ship is inside a speed pad from the first tick.
fn grid_on_a_speed_pad() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.speedup_pads = enveloping_pad();
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(setup)
}

#[test]
fn crossing_a_pad_raises_its_cue_on_the_tick_the_flare_is_armed() {
    let mut race = grid_on_a_speed_pad();
    assert!(race.pending_cues().is_empty(), "a cue before any tick ran");

    race.tick(&PlayerInputs::none());
    // The same edge, the same tick: `Ship_ApplySpeedupPad` calls
    // `ExhaustFlare_OnSpeedupPad` and `Sound_Play("SPEEDUPPAD")` from one
    // branch, so a port where the flare arms and the cue does not is wired
    // wrong however good the sound is.
    assert!(
        race.view.exhaust[0].boost_timer() > 0.0,
        "the flare did not arm"
    );
    assert!(
        race.pending_cues().iter().any(|e| e.cue == Cue::SpeedupPad),
        "the flare armed and the cue did not: {:?}",
        race.pending_cues()
    );
}

#[test]
fn sitting_on_a_pad_raises_the_cue_once_and_not_every_tick() {
    let mut race = grid_on_a_speed_pad();
    race.tick(&PlayerInputs::none());
    let first: Vec<_> = race
        .drain_cues()
        .into_iter()
        .filter(|e| e.cue == Cue::SpeedupPad)
        .collect();
    // **One per craft, and the fixture's pad envelops the whole grid**, so a
    // single race raises eight - each carrying its own slot, which is what the
    // audio layer places it by. Before positional audio the seven opponents
    // were dropped at the point of raising and this asserted `1`.
    assert_eq!(first.len(), race.ship_count() as usize);
    let mut slots: Vec<_> = first.iter().map(|e| e.slot).collect();
    slots.sort_unstable();
    slots.dedup();
    assert_eq!(slots.len(), first.len(), "two cues came from one slot");
    assert_eq!(
        first.iter().filter(|e| e.is_player()).count(),
        1,
        "the player raised it more than once, or not at all"
    );

    // The edge is "entered a *new* pad", not "is on one" - the same rule the
    // Zone score and the flare already follow. A pad held for a second would
    // otherwise machine-gun sixty copies of a half-second sample.
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        !race.drain_cues().iter().any(|e| e.cue == Cue::SpeedupPad),
        "the pad re-fired while the craft sat on it"
    );
}

#[test]
fn draining_takes_the_queue_and_leaves_it_empty() {
    let mut race = grid_on_a_speed_pad();
    race.tick(&PlayerInputs::none());
    assert!(!race.pending_cues().is_empty());
    let drained = race.drain_cues();
    assert!(!drained.is_empty());
    assert!(
        race.pending_cues().is_empty(),
        "a second reader would play the same cue twice"
    );
}

#[test]
fn a_track_with_no_pads_raises_no_pad_cue() {
    let mut race = race_with_pads(Mode::SingleRace, Vec::new());
    for _ in 0..120 {
        race.tick(&PlayerInputs::none());
        assert!(!race.drain_cues().iter().any(|e| e.cue == Cue::SpeedupPad));
    }
}

/// Every cue is either raised as an edge or driven as a level - and nothing is
/// merely *loaded*.
///
/// This exists because that failed once: `shieldactive` was decoded, reported
/// in the load line and reachable through `Banks::pick`, and no code path
/// anywhere pushed it. Every test written at the time passed, because they all
/// asked whether a cue **loads** rather than whether anything plays it. So this
/// asserts over [`Cue::ALL`] and fails the day a cue is added without an
/// emitter.
#[test]
fn every_cue_has_something_that_raises_it() {
    // A cue driven from the *level* rather than from an edge: the audio layer
    // reads `Race::shield_is_up`, `Race::craft_is_exploding`, `Race::finished`
    // and `Race::sight_state` directly, so these have no entry in the queue by
    // design. `LockOn` is the odd one - it is fired on an *edge*, but the edge
    // is computed in the audio layer from that level rather than pushed here,
    // because the reticle it follows is render-only state and the queue is the
    // simulation's output. `the_reticle_says_seeking_then_locked` in
    // `race::tests::weapons` is what stops it going unraised.
    //
    // `PlasmaTravel`, `RocketTravel`, `MissileTravel` and `ShurikenTravel`
    // join this for the same reason: each is read directly off the world's
    // own projectile array every tick in `Audio::race_tick`, never pushed as
    // a `CueEvent` - see `TravelVoices`. `LeachAttach` is the same shape
    // again, read directly off `race.sim.world.leach_beam` instead of a
    // projectile slot - see `SfxVoices::leach_attach`. `Autopilot` and
    // `Engaging` join `Blowup` for the same reason it is here: both are read
    // off `Race::autopilot_is_active` directly in `Audio::race_tick`'s own
    // level-and-latch match, never pushed.
    //
    // `Magstrip` is neither: it is an edge, raised only on a title that builds
    // the HD-lineage arc wake, which this Pulse fixture does not. Its emitter is
    // `magstrip_wake::tests::the_cue_edges_follow_the_instantaneous_contact_not_the_lingering_blend`.
    const BY_LEVEL: [Cue; 12] = [
        Cue::Engine,
        Cue::Shield,
        Cue::Blowup,
        Cue::Autopilot,
        Cue::Engaging,
        Cue::LockOn,
        Cue::PlasmaTravel,
        Cue::RocketTravel,
        Cue::MissileTravel,
        Cue::QuakeTravel,
        Cue::LeachAttach,
        Cue::ShurikenTravel,
    ];

    // A pad the whole grid stands on *and* a wall to scrape, so one run
    // reaches every edge. The wall is `respawn.rs`'s proven fixture - a
    // backwards-wound triangle registers no contact silently, so an invented
    // one here could make this pass by never testing anything. The same wall
    // also closes the Plasma's own bolt: charged and released a body-length
    // above it, the downward probe finds it within the first flight tick.
    //
    // Two coincident planes since `Body_StepWorld`'s pass 1 replaced the swept
    // centre ray: the down-facing one first, which the projectiles' own
    // queries were staged against and which wins their nearest-hit tie, and
    // the up-facing one the single-sided hull narrowphase needs to meet the
    // craft pressed down onto it. See `upward_plane`.
    let mut setup = setup_with(
        hulled_handling(),
        vec![upward_plane(-40.0, oag_physics::Surface::Wall)],
    );
    setup.mode = Mode::SingleRace;
    setup.speedup_pads = enveloping_pad();
    setup.weapons = Some(wall_fixture_weapons_table());
    let mut race = Race::start(setup);

    let mut buttons = Buttons::new();
    let mut raised = std::collections::BTreeSet::new();
    let mut saw_impact = false;
    for tick in 0..240 {
        // A shield, put up mid-race and taken down again, so the rising edge,
        // the level and the shielded-contact branch are all exercised. Set
        // outside `tick` on purpose - this is a fixture, not a pickup grant.
        race.sim.world.ships[0].physics.shield_pickup_timer =
            if (60..180).contains(&tick) { 1.0 } else { 0.0 };
        // An Autopilot armed once, long enough to reach its own one-second
        // warning inside this run. Set directly for the same reason the shield
        // above is: this is a fixture reaching an edge, not a pickup grant.
        if tick == 0 {
            race.sim.world.ships[0].autopilot_timer = 2.0;
        }
        // A Plasma, pressed once the Autopilot cancel above has cleared -
        // `spend_pickup`'s fire arm cancels instead of firing while
        // `autopilot_timer > 0.0`, and `2.0` seconds clears by tick 120 at
        // 60 Hz. `Cue::Plasma` fires on this same tick; the one-second
        // wind-up releases the bolt around tick 190, comfortably inside the
        // wall it is re-aimed at every tick, and `Cue::PlasmaHitWall` follows
        // within a tick or two of that.
        if tick == 130 {
            race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Plasma);
        }
        // A Mine absorbed, before the shield goes up: `Cue::Absorb` is the
        // absorb handler's own feedback and nothing else raises it - a
        // shielded contact is silent.
        if tick == 40 {
            race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Mine);
        }
        // A Mine, dropped once - after the wall business above has already
        // had its first pass, so the impulse this leaves on `physics.shield`
        // is not mistaken for the wall's own. Its own held-weapon slot, not
        // the pickup path, for the same reason the shield and Autopilot above
        // are set directly.
        if tick == 200 {
            race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Mine);
            race.sim.world.ships[0]
                .pickup
                .begin_drop(oag_weapons::projectile::mine::CLUSTER);
        }
        // A Rocket and a Missile, each pressed once after the Plasma's own
        // wind-up has released - one bolt in the air at a time is enough,
        // and each is re-aimed at the same wall by the position reset just
        // below. `Cue::RocketHitWall` follows the Rocket immediately (no
        // bounce budget, the same probe-branch shortcut the Plasma's own
        // ending takes); `Cue::MissileHitWall` follows the Missile on its
        // first bounce, not its final ending - see that cue's own doc
        // comment. **Not the Cannon or the Shuriken**: `Missile_Init`'s own
        // launch direction is the *craft's velocity*, which this fixture
        // already points at the wall below, but `Shuriken_Init`'s and the
        // Cannon's own launch read the craft's *forward* instead, which this
        // fixture never rotates - `cannon_hits_a_wall` and
        // `shuriken_bounces_off_a_wall` stage those two independently, aimed
        // rather than dropped.
        if tick == 210 {
            race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Rocket);
        }
        if tick == 212 {
            race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Missile);
        }
        // Re-aimed at the wall every tick, so each one sees a fresh inbound
        // contact rather than the ship bouncing away after the first.
        let body = &mut race.sim.world.ships[0].physics.body;
        // Level and pressed onto the wall, except on the Missile's own tick:
        // `missile::launch` fires along the craft's *forward* from its nose,
        // so for that one press the craft is lifted ten units clear and
        // pointed nose-down at the wall.
        body.position = Vec3::new(20.0, if tick == 212 { -30.0 } else { -39.7 }, 0.0);
        body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
        body.orientation = if tick == 212 {
            oag_core::math::Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
        } else {
            oag_core::math::Quat::IDENTITY
        };
        body.angular_velocity = Vec3::ZERO;
        let snapshot = match tick {
            40 => buttons.tick(CIRCLE),
            130 | 210 | 212 => buttons.tick(SQUARE),
            _ => buttons.tick(0),
        };
        saw_impact |= race.tick(&PlayerInputs::single(snapshot)).wall.impact;
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    assert!(
        saw_impact,
        "the fixture never reached the wall - not what this test means to check"
    );
    // Each needs a struck craft, which this fixture's own bolts never meet -
    // every one of them is re-aimed at the wall every tick, and the wall is
    // the only thing in this fixture's world. The four helpers below each
    // stage their own craft-hit ending independently; see their own doc
    // comments.
    raised.extend(plasma_hits_a_craft());
    raised.extend(rocket_hits_a_craft());
    raised.extend(cannon_hits_a_craft());
    // Neither the Cannon's nor the Shuriken's own launch reads the firing
    // craft's velocity - both read its forward instead, which this fixture
    // never rotates - so neither ever crosses the floor-shaped wall above;
    // see `cannon_hits_a_wall`'s own doc comment.
    raised.extend(cannon_hits_a_wall());
    raised.extend(shuriken_bounces_off_a_wall());
    // Neither the Quake nor the LeachBeam ever meets this fixture's wall at
    // all - a travelling wave and a locked link are progress-along-the-course
    // and along-track-window questions, not raycasts against geometry - so
    // both are staged by their own two-craft helpers rather than by anything
    // in the loop above.
    raised.extend(quake_hits_a_craft());
    raised.extend(leach_fires_locked());
    // The endings and the fizzle recovered 2026-09-30, in `cue_endings.rs`.
    raised.extend(super::cue_endings::missile_hits_a_craft());
    raised.extend(super::cue_endings::missile_outlives_its_fuse());
    raised.extend(super::cue_endings::leach_fires_unlocked());
    raised.extend(super::cue_endings::shuriken_is_thrown());
    raised.extend(super::repulser::repulser_hits_a_craft());
    raised.extend(super::perfect_start::perfect_start());

    for cue in Cue::ALL {
        assert!(
            raised.contains(&cue) || BY_LEVEL.contains(&cue) || cue == Cue::Magstrip,
            "{} is loaded and nothing raises it",
            cue.name()
        );
    }
    // Named individually as well, so a future `BY_LEVEL` that quietly grew
    // cannot make the loop above vacuous.
    for cue in [
        Cue::SpeedupPad,
        Cue::Turbo,
        Cue::Collision,
        Cue::Absorb,
        Cue::ShieldActive,
        Cue::Disengaging,
        Cue::MineLaunch,
        Cue::Plasma,
        Cue::PlasmaHitWall,
        Cue::PlasmaHitShip,
        Cue::RocketHitWall,
        Cue::RocketHitShip,
        Cue::Missile,
        Cue::MissileHitWall,
        Cue::Cannon,
        Cue::CannonHitWall,
        Cue::CannonHitShip,
        Cue::QuakeHit,
        Cue::Leach,
        Cue::ShurikenHit,
    ] {
        assert!(raised.contains(&cue), "{} was never raised", cue.name());
    }
}

/// A second, stationary craft directly on a Plasma's own nose, close enough
/// that the bolt reaches it well inside this test's own tick budget - what
/// [`every_cue_has_something_that_raises_it`]'s own wall fixture cannot stage,
/// since that one only ever meets the wall it is re-aimed at every tick.
///
/// **Both craft held in place.** Neither throttles nor steers, so the
/// shooter's own nose never moves and the target sits exactly where it is
/// placed - at `forward * 15.0` off the shooter, recomputed from the shooter's
/// own `body.forward()` every tick rather than a hardcoded world axis, so this
/// does not depend on which way [`setup`]'s synthetic straight happens to
/// point. `15.0` clears [`hulled_handling`]'s own `hull_radius` (`2.0`, half
/// its `length: 4.0`) by a wide margin.
///
/// **The shooter carries a forward velocity even though its position is
/// pinned**, because since 2026-09-16 a bolt leaves at *the craft's own speed*
/// plus `launchSpeed` and only blends up to the class speed over its first
/// second (`Plasma_SpeedForClass`, `crates/weapons/src/projectile/flight.rs`).
/// A shooter held at zero velocity launches a `26 km/h` bolt that falls off
/// the synthetic straight - which has no surface under it - before it has
/// covered the fifteen units, and the hull sweep misses under the target. The
/// position write every tick keeps the shooter where it is regardless of the
/// velocity the physics would otherwise integrate, so the velocity here is
/// purely what the release reads: `60.0` units per second is `216 km/h`, an
/// ordinary Venom pace, and puts the bolt across the gap in the first few
/// ticks after release the way the pre-ramp fixture did.
///
/// **Ship count is `2`, not a full `GRID_SLOTS` grid.** `oag_weapons::projectile::step`
/// only sweeps `world.ships[..world.ship_count]` -
/// `crates/weapons/src/projectile.rs`'s own `step` - so the second craft has
/// to be counted in for the hull test to see it at all, but nothing here
/// needs the other six slots `race_with_a_grid`-style grid construction would
/// also spin up.
fn plasma_hits_a_craft() -> std::collections::BTreeSet<Cue> {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_plasma_table());
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Plasma);

    let mut buttons = Buttons::new();
    let mut raised = std::collections::BTreeSet::new();
    for tick in 0..180 {
        race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
        let forward = race.sim.world.ships[0].physics.body.forward();
        race.sim.world.ships[0].physics.body.linear_velocity = forward * 60.0;
        race.sim.world.ships[1].physics.body.position = forward * 15.0;
        race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;

        let snapshot = if tick == 0 {
            buttons.tick(SQUARE)
        } else {
            buttons.tick(0)
        };
        race.tick(&PlayerInputs::single(snapshot));
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    raised
}

/// A Rocket meeting a second, stationary craft - the same two-craft shape
/// [`plasma_hits_a_craft`] takes and for the same reason:
/// `every_cue_has_something_that_raises_it`'s own wall fixture only ever
/// meets the wall it is re-aimed at every tick. `oag_weapons::projectile::flight`
/// gives the Rocket the Plasma's own two endings - see `Cue::RocketHitShip`'s
/// own doc comment - so this is the same fixture with the weapon and its
/// table swapped, and no charge/wind-up delay to wait out.
fn rocket_hits_a_craft() -> std::collections::BTreeSet<Cue> {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_rocket_table());
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Rocket);

    let mut buttons = Buttons::new();
    let mut raised = std::collections::BTreeSet::new();
    for tick in 0..60 {
        race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
        let forward = race.sim.world.ships[0].physics.body.forward();
        race.sim.world.ships[0].physics.body.linear_velocity = forward * 60.0;
        race.sim.world.ships[1].physics.body.position = forward * 15.0;
        race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;

        let snapshot = if tick == 0 {
            buttons.tick(SQUARE)
        } else {
            buttons.tick(0)
        };
        race.tick(&PlayerInputs::single(snapshot));
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    raised
}

/// A Cannon round meeting a second, stationary craft - the same shape
/// [`rocket_hits_a_craft`] takes, held rather than pressed:
/// `Race::spend_pickup`'s own Cannon arm is a no-op, and `Race::advance_cannons`
/// reads the **held** state of the button every tick instead - see
/// `Cue::Cannon`'s own doc comment. The craft stays put, since a round's own
/// muzzle speed is the firing craft's current speed plus a fixed
/// `CannonStats::rate`-independent base, and zero is a speed like any other.
fn cannon_hits_a_craft() -> std::collections::BTreeSet<Cue> {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_cannon_table());
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Cannon);

    let mut buttons = Buttons::new();
    let mut raised = std::collections::BTreeSet::new();
    for _ in 0..60 {
        // Pinned at the origin, the same reason `plasma_hits_a_craft` pins
        // its own shooter: `forward * 15.0` is a *relative* offset, and
        // reading it against a shooter left to drift would place the target
        // somewhere that was never actually down the nose.
        race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
        let forward = race.sim.world.ships[0].physics.body.forward();
        race.sim.world.ships[1].physics.body.position = forward * 15.0;
        race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;

        // Held the whole run: `Cannon_UpdateReload` reads the held bit, not
        // the press edge.
        let snapshot = buttons.tick(SQUARE);
        race.tick(&PlayerInputs::single(snapshot));
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    raised
}

/// A Cannon round meeting a wall dead ahead - a **vertical** one, at `x =
/// 15.0` (`setup`'s own synthetic track runs along `+x`, and `Race::start`
/// spawns the craft facing its own tangent, so this is the direction its nose
/// already points), rather than [`every_cue_has_something_that_raises_it`]'s
/// own floor-shaped one: `Weapon_FireCannon`'s round reads the firing craft's
/// own *forward* for its heading, not its velocity, so a wall reached only by
/// falling (that fixture's own shape) is a wall this weapon's round never
/// crosses - it just skims parallel to it forever. A wall the craft's own
/// nose already points at needs no such trick.
fn cannon_hits_a_wall() -> std::collections::BTreeSet<Cue> {
    let mut setup = setup_with(
        hulled_handling(),
        vec![plane(0, 15.0, oag_physics::Surface::Wall, 0)],
    );
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_cannon_table());
    let mut race = Race::start(setup);
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Cannon);

    let mut buttons = Buttons::new();
    let mut raised = std::collections::BTreeSet::new();
    for _ in 0..60 {
        race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
        race.sim.world.ships[0].physics.body.linear_velocity = Vec3::ZERO;
        // Held the whole run, the same reason `cannon_hits_a_craft` holds it.
        let snapshot = buttons.tick(SQUARE);
        race.tick(&PlayerInputs::single(snapshot));
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    raised
}

/// A Shuriken glancing off the same shape of wall - see `cannon_hits_a_wall`'s
/// own doc comment for why a vertical wall dead ahead and not a floor-shaped
/// one: `Shuriken_Init` reads the craft's own forward too, twenty degrees off
/// one side or the other, and both fixed angles still meet a wall this wide
/// well inside its first few flight ticks.
fn shuriken_bounces_off_a_wall() -> std::collections::BTreeSet<Cue> {
    let mut setup = setup_with(
        hulled_handling(),
        vec![plane(0, 15.0, oag_physics::Surface::Wall, 0)],
    );
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_shuriken_table());
    let mut race = Race::start(setup);
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Shuriken);

    let mut buttons = Buttons::new();
    let mut raised = std::collections::BTreeSet::new();
    for tick in 0..60 {
        race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
        race.sim.world.ships[0].physics.body.linear_velocity = Vec3::ZERO;
        let snapshot = if tick == 0 {
            buttons.tick(SQUARE)
        } else {
            buttons.tick(0)
        };
        race.tick(&PlayerInputs::single(snapshot));
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    raised
}

/// The Quake's own travelling wave reaching a second craft.
///
/// **The radius is wide on purpose.** [`one_quake_table`]'s own `200`-unit
/// radius is bigger than [`setup`]'s whole synthetic course (a ~136.6-unit
/// ring - see that function's own doc comment), so the wave reaches every
/// craft on it the instant it launches: this is about `Cue::QuakeHit`'s own
/// edge, not the wave's travel time, and neither craft needs to be placed
/// anywhere in particular for it.
///
/// Fired on the second tick rather than the first: `Race::spend_pickup` runs
/// before `Race::update_standings` inside `Race::tick`, so a press on the
/// very first tick would still read the pre-race `None`
/// [`oag_race::Standing::progress`] starts at, and the Quake's own fire arm
/// declines rather than launches with nowhere on the course to start from.
fn quake_hits_a_craft() -> std::collections::BTreeSet<Cue> {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_quake_table());
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Quake);

    let mut buttons = Buttons::new();
    let mut raised = std::collections::BTreeSet::new();
    for tick in 0..30 {
        let snapshot = if tick == 1 {
            buttons.tick(SQUARE)
        } else {
            buttons.tick(0)
        };
        race.tick(&PlayerInputs::single(snapshot));
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    raised
}

/// The LeachBeam fired with a lock - the same nose-parked placement
/// [`crate::tests::weapons::holding_a_missile_behind_a_craft_locks_it_after_the_recovered_hold`]
/// uses, since `Ship_AcquireLock` is one function serving both weapons. No
/// hold to wait out here, unlike that test's own player-facing reticle gate:
/// `Race::spend_pickup`'s own LeachBeam arm reads `Race::sight_target`
/// directly, with no `Sight::locked` requirement layered on top of it - see
/// `Cue::Leach`'s own doc comment.
fn leach_fires_locked() -> std::collections::BTreeSet<Cue> {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_leach_beam_table());
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    let forward = race.sim.world.ships[0].physics.body.forward();
    race.sim.world.ships[1].physics.body.position =
        race.sim.world.ships[0].physics.body.position + forward * 60.0;
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::LeachBeam);

    let mut buttons = Buttons::new();
    let mut raised = std::collections::BTreeSet::new();
    for tick in 0..10 {
        let snapshot = if tick == 1 {
            buttons.tick(SQUARE)
        } else {
            buttons.tick(0)
        };
        race.tick(&PlayerInputs::single(snapshot));
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    raised
}

/// `Plasma_SweepCraftHit` plays `PLASMAHITSHIP` on a craft hit and clears the
/// bolt's own emitter before `Plasmas_Update`'s teardown would otherwise play
/// `PLASMAHITWALL` unconditionally - so the original plays exactly one of the
/// two per ending, never both. See
/// `docs/ghidra/functions/psp-pulse-usa/plasma.md`'s "a craft hit is the
/// third ending" section and `oag_sound::sfx::Cue::PlasmaHitShip`'s own
/// doc comment.
#[test]
fn a_plasma_that_hits_a_craft_raises_plasmahitship_and_not_plasmahitwall() {
    let raised = plasma_hits_a_craft();
    assert!(
        raised.contains(&Cue::PlasmaHitShip),
        "PLASMAHITSHIP never fired on a craft hit: {raised:?}"
    );
    assert!(
        !raised.contains(&Cue::PlasmaHitWall),
        "PLASMAHITWALL fired on a craft hit too - the original clears the \
         bolt's own emitter before that branch can run, so this is a double \
         sound the disc never plays: {raised:?}"
    );
}

/// The same split as the Plasma's, on `oag_weapons::projectile::Impact::struck` -
/// see `Cue::RocketHitShip`'s own doc comment.
#[test]
fn a_rocket_that_hits_a_craft_raises_rockethitship_and_not_rockethitwall() {
    let raised = rocket_hits_a_craft();
    assert!(
        raised.contains(&Cue::RocketHitShip),
        "ROCKEXPLSHIP never fired on a craft hit: {raised:?}"
    );
    assert!(
        !raised.contains(&Cue::RocketHitWall),
        "ROCKEXPLWALL fired on a craft hit too: {raised:?}"
    );
}

/// The same split again, for the Cannon - see `Cue::CannonHitShip`'s own
/// doc comment for why the cue still has to be *raised* here even though
/// `CANNONEXPLSHIP` plays `CANNONEXPLWALL`'s own waveforms rather than any
/// of its own: which waveforms come out the other end is `Banks::pick`'s
/// question, not this one's.
#[test]
fn a_cannon_round_that_hits_a_craft_raises_cannonhitship_and_not_cannonhitwall() {
    let raised = cannon_hits_a_craft();
    assert!(
        raised.contains(&Cue::CannonHitShip),
        "CANNONEXPLSHIP never fired on a craft hit: {raised:?}"
    );
    assert!(
        !raised.contains(&Cue::CannonHitWall),
        "CANNONEXPLWALL fired on a craft hit too: {raised:?}"
    );
}

#[test]
fn a_cannon_round_that_hits_a_wall_raises_cannonhitwall() {
    let raised = cannon_hits_a_wall();
    assert!(
        raised.contains(&Cue::CannonHitWall),
        "CANNONEXPLWALL never fired on a wall hit: {raised:?}"
    );
    assert!(
        !raised.contains(&Cue::CannonHitShip),
        "CANNONEXPLSHIP fired on a wall hit too: {raised:?}"
    );
}

#[test]
fn a_shuriken_bouncing_off_a_wall_raises_shurikenhit() {
    let raised = shuriken_bounces_off_a_wall();
    assert!(
        raised.contains(&Cue::ShurikenHit),
        "SHURIKENHIT never fired on a bounce: {raised:?}"
    );
}

#[test]
fn a_quake_wave_raises_quakehit_on_the_craft_it_reaches() {
    let raised = quake_hits_a_craft();
    assert!(
        raised.contains(&Cue::QuakeHit),
        "QUAKEHIT never fired: {raised:?}"
    );
}

/// `Cue::QuakeLaunch` fires on the press itself, even the one that lands
/// while a wave is already travelling and the launch itself is a no-op -
/// see that variant's own doc comment for why `Ship_FireHeldWeapon`'s own
/// gate has no busy check of its own. Two presses, not one: the first is
/// the ordinary launch, the second lands on tick 3 with the first wave
/// still up (`one_quake_table`'s wave easily outlives two ticks), so it
/// exercises `Race::spend_pickup`'s own busy-return - which must not have
/// swallowed the cue push ahead of it.
#[test]
fn a_re_press_while_a_quake_wave_travels_still_raises_quakelaunch() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_quake_table());
    let mut race = Race::start(setup);
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Quake);

    let mut buttons = Buttons::new();
    let mut launches = 0;
    for tick in 0..5 {
        // Pressed twice: the launch on tick 1, and a re-press on tick 3
        // while that same wave is still `race.sim.world.quake`.
        let snapshot = if tick == 1 || tick == 3 {
            buttons.tick(SQUARE)
        } else {
            buttons.tick(0)
        };
        // Forced back every tick: the first press's own launch spends the
        // pickup through `spend_pickup`'s ordinary `Held::take` tail, and
        // this fixture wants a Quake in the slot for the second press too,
        // to land on the busy-return path deliberately rather than on "no
        // weapon held" for an unrelated reason.
        race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Quake);
        race.tick(&PlayerInputs::single(snapshot));
        launches += race
            .drain_cues()
            .iter()
            .filter(|e| e.cue == Cue::QuakeLaunch)
            .count();
    }
    assert_eq!(
        launches, 2,
        "QUAKELAUNCH should fire on both presses, busy or not"
    );
}

#[test]
fn a_locked_leachbeam_raises_leach() {
    let raised = leach_fires_locked();
    assert!(
        raised.contains(&Cue::Leach),
        "LEACH never fired on a locked shot: {raised:?}"
    );
}

/// `Cue::LeachEnergy` off the ribbon's own pulse edge - see that variant's
/// doc comment. `Ribbon::new`'s `cursor: 0` means the very first `advance`
/// call always pulses, so a locked beam raises it immediately; the assertion
/// that matters is the second one, that a wrong wiring pushing it every tick
/// (rather than only on the wrap) gets caught rather than passing on the
/// first-tick freebie alone.
#[test]
fn a_locked_leachbeam_pulses_leachenergy_and_not_every_tick() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_leach_beam_table());
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    let forward = race.sim.world.ships[0].physics.body.forward();
    race.sim.world.ships[1].physics.body.position =
        race.sim.world.ships[0].physics.body.position + forward * 60.0;
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::LeachBeam);

    let mut buttons = Buttons::new();
    let ticks = 120;
    let mut pulse_ticks = Vec::new();
    for tick in 0..ticks {
        let snapshot = if tick == 1 {
            buttons.tick(SQUARE)
        } else {
            buttons.tick(0)
        };
        race.tick(&PlayerInputs::single(snapshot));
        if race.drain_cues().iter().any(|e| e.cue == Cue::LeachEnergy) {
            pulse_ticks.push(tick);
        }
    }
    // At least two pulses, so there is a gap to check at all.
    assert!(
        pulse_ticks.len() >= 2,
        "LEACHENERGY fired fewer than two times over {ticks} ticks: {pulse_ticks:?}"
    );
    // `Ribbon::new`'s own `cursor: 0` means the very first `advance` call -
    // the same tick the beam locks on - always pulses, so the first entry is
    // exact, not just "eventually".
    assert_eq!(
        pulse_ticks[0], 1,
        "LEACHENERGY did not fire on the fire tick itself: {pulse_ticks:?}"
    );
    // The two craft's separation is fixed for this whole fixture, so the
    // segment count - and therefore the cursor's wrap period - never
    // changes: every gap between pulses should be the same tick count. A
    // wiring bug that pushes the cue on every tick, or on some other
    // unrelated edge, would not produce this regularity.
    let gaps: Vec<usize> = pulse_ticks.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(
        gaps.iter().all(|&gap| gap == gaps[0]),
        "the gaps between LEACHENERGY pulses are not constant: {gaps:?} from {pulse_ticks:?}"
    );
    assert!(
        gaps[0] > 1,
        "LEACHENERGY pulsed every tick rather than on the ribbon's own wrap: {pulse_ticks:?}"
    );
}

#[test]
fn the_shield_announcer_fires_on_the_edge_and_not_on_the_level() {
    let mut race = grid_on_a_speed_pad();
    let mut announcements = 0;
    for tick in 0..180 {
        race.sim.world.ships[0].physics.shield_pickup_timer = if tick >= 30 { 1.0 } else { 0.0 };
        race.tick(&PlayerInputs::none());
        announcements += race
            .drain_cues()
            .iter()
            .filter(|e| e.cue == Cue::ShieldActive)
            .count();
    }
    // 150 ticks of shield, one announcement: `Shield_Activate` runs once per
    // activation, and a level-triggered version would say it 150 times.
    assert_eq!(announcements, 1);
}

/// A synthetic two-record ladder, the smallest shape that has a boundary to
/// cross: stage `1` ("A") from zone `0`, stepping to stage `2` ("B") at zone
/// `2` and holding there. Mirrors `oag_title::ZoneStages`' own real tables'
/// shape - a sentinel-free descending threshold list - without needing HD's
/// fifteen-stage one just to prove the edge.
static TEST_ZONE_STAGES: oag_title::ZoneStages = oag_title::ZoneStages {
    records: &[(2, "B"), (0, "A")],
};

fn zone_race() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::Zone;
    setup.zone_stages = Some(&TEST_ZONE_STAGES);
    Race::start(setup)
}

/// The speed-class announcer's own trigger: it fires on a stage *boundary*,
/// not on every zone survived - most zone steps do not cross one, since a
/// real ladder's bands are wider than one zone (see
/// `oag_title::ZoneStages`' own docs). `TEST_ZONE_STAGES` has exactly one
/// boundary, at zone `2`, so a wiring bug that fired on every
/// `outcome.zone_advanced` instead of on the stage changing would show up as
/// more than one announcement here.
#[test]
fn the_class_announcer_fires_on_a_stage_step_and_not_on_every_zone() {
    let mut race = zone_race();
    let mut raised = Vec::new();
    // Comfortably past two 600-tick zone steps (`zone::tests` pins
    // `STEP_SECONDS * 60.0 == 600`), with margin for the dt sum's own f32
    // slop - the boundary this asserts sits at zone 2 and every zone after
    // it names the same stage, so overshooting past it changes nothing.
    // Past the start-line countdown, through which the zone machine holds.
    for _ in 0..oag_race::COUNTDOWN_TICKS + 1300 {
        race.tick(&PlayerInputs::none());
        raised.extend(race.drain_class_announcements());
    }
    assert!(
        race.sim.world.primary_race().zone >= 2,
        "sanity: 1300 ticks should survive at least two zones, got {}",
        race.sim.world.primary_race().zone
    );
    assert_eq!(
        raised,
        vec![2],
        "one class change, to stage 2 (TEST_ZONE_STAGES' only boundary) - not one per zone \
         survived, and not fired again for every zone stage 2 goes on to cover"
    );
}

/// The same race, but for a title with no recovered zone-to-stage ladder -
/// every title but HD/Fury today. Nothing should fire: there is no stage to
/// have changed.
#[test]
fn a_title_with_no_zone_stages_never_raises_a_class_announcement() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::Zone;
    // The default `setup()` fixture already leaves this `None` - restated
    // here so the test does not depend on that staying true by accident.
    setup.zone_stages = None;
    let mut race = Race::start(setup);

    let mut raised = Vec::new();
    // Past the start-line countdown, through which the zone machine holds.
    for _ in 0..oag_race::COUNTDOWN_TICKS + 1300 {
        race.tick(&PlayerInputs::none());
        raised.extend(race.drain_class_announcements());
    }
    assert!(
        race.sim.world.primary_race().zone >= 2,
        "sanity: the zone counter itself does not need a ladder to step"
    );
    assert!(
        raised.is_empty(),
        "no ladder, no stage, nothing to announce: {raised:?}"
    );
}

/// One `MINELAUNCH` per mine that leaves the back of the craft, on the same
/// tick each one does - not one per press, and not one for the Bomb.
///
/// `oag_weapons::pickup::Held::begin_drop` fires the first charge on the very
/// tick it is called, so the count landed by the end of the run is exactly
/// [`oag_weapons::projectile::mine::CLUSTER`] rather than one short.
#[test]
fn laying_a_mine_raises_its_launch_cue_once_per_charge() {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(one_mine_table());
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Mine);
    race.sim.world.ships[0]
        .pickup
        .begin_drop(oag_weapons::projectile::mine::CLUSTER);

    let mut launches = 0;
    // Comfortably past the whole cluster: `DROP_INTERVAL` is a tenth of a
    // second, so `CLUSTER` charges take under a second at 60 Hz.
    for _ in 0..120 {
        race.tick(&PlayerInputs::none());
        launches += race
            .drain_cues()
            .iter()
            .filter(|e| e.cue == Cue::MineLaunch)
            .count();
    }
    assert_eq!(
        launches,
        usize::from(oag_weapons::projectile::mine::CLUSTER),
        "one MINELAUNCH per mine in the cluster, not more and not fewer"
    );
}

/// The Bomb shares the Mine's drop loop in this engine, and must not borrow
/// its cue: `Weapon_FireBomb` never calls the play function `Mine_Init` does,
/// so this port raises nothing for it either. See `Cue::MineLaunch`'s own doc
/// comment and `mine.md`'s 2026-09-06 section.
#[test]
fn dropping_a_bomb_raises_no_mine_launch_cue() {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(
        oag_tables::weapons::parse(
            r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Bomb"><Stats absorb="30" blastforce="31" blastradius="32"
               damage="33" slowdown_time="0.5" timetodie="6"
               trigger_radius="3"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Bomb"><Stats ai="1" back="1" front="1" human="1"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#,
        )
        .expect("the fixture table must parse"),
    );
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Bomb);
    race.sim.world.ships[0].pickup.begin_drop(1);

    let mut raised = Vec::new();
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
        raised.extend(race.drain_cues());
    }
    assert!(
        !raised.iter().any(|e| e.cue == Cue::MineLaunch),
        "the Bomb borrowed the Mine's launch cue: {raised:?}"
    );
}

/// Cues are an *output*, so a race that raises them must hash exactly like one
/// that does not. This is the guard on the committed determinism constants:
/// `docs/architecture/determinism.md` puts audio outside the simulation, and
/// the way that stays true is that nothing audio touches reaches the hasher.
#[test]
fn raising_a_cue_does_not_move_the_race_hash() {
    let mut with = grid_on_a_speed_pad();
    let mut without = grid_on_a_speed_pad();
    for _ in 0..30 {
        with.tick(&PlayerInputs::none());
        without.tick(&PlayerInputs::none());
        // One of the two has its queue drained every tick and the other never
        // does, so by the end they hold different queues entirely.
        with.drain_cues();
    }
    assert!(!without.pending_cues().is_empty(), "nothing was queued");
    assert!(with.pending_cues().is_empty());
    assert_eq!(with.sim.state_hash(), without.sim.state_hash());
}

/// A HUD message line starting to show sounds `MESSAGE` on that tick, once, and
/// not before it is raised: `Hud_UpdateMessages` plays it as a slot shows.
#[test]
fn a_message_line_showing_raises_the_message_cue_once() {
    let mut race = grid_on_a_speed_pad();
    race.tick(&PlayerInputs::none());
    race.drain_cues();
    race.tick(&PlayerInputs::none());
    let count = |race: &Race| {
        race.pending_cues()
            .iter()
            .filter(|e| e.cue == Cue::Message)
            .count()
    };
    assert_eq!(count(&race), 0, "no message raised, no cue");

    race.raise_message("ER_GMA", true);
    race.drain_cues();
    race.tick(&PlayerInputs::none());
    assert_eq!(count(&race), 1, "{:?}", race.pending_cues());
    race.drain_cues();
    race.tick(&PlayerInputs::none());
    assert_eq!(
        count(&race),
        0,
        "the cue sounds as the line shows, not each tick"
    );
}
