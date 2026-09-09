//! Which sound cues a tick raises, and on which tick.
//!
//! The assertion that matters here is **positional**: a cue arriving is easy,
//! a cue arriving on the tick the thing that causes it happened is the part a
//! wiring bug breaks. Nothing here loads a bank or touches a mixer - what a
//! cue *sounds like* is `audio::sfx`'s problem and the disc-backed half is
//! `crates/game/tests/sfx_ground_truth.rs`.

use super::*;
use crate::audio::sfx::Cue;

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

    race.tick(&InputSnapshot::default());
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
    race.tick(&InputSnapshot::default());
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
        race.tick(&InputSnapshot::default());
    }
    assert!(
        !race.drain_cues().iter().any(|e| e.cue == Cue::SpeedupPad),
        "the pad re-fired while the craft sat on it"
    );
}

#[test]
fn draining_takes_the_queue_and_leaves_it_empty() {
    let mut race = grid_on_a_speed_pad();
    race.tick(&InputSnapshot::default());
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
        race.tick(&InputSnapshot::default());
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
    const BY_LEVEL: [Cue; 4] = [Cue::Engine, Cue::Shield, Cue::Blowup, Cue::LockOn];

    // A pad the whole grid stands on *and* a wall to scrape, so one run
    // reaches every edge. The wall is `respawn.rs`'s proven fixture - a
    // backwards-wound triangle registers no contact silently, so an invented
    // one here could make this pass by never testing anything.
    let mut setup = setup_with(
        hulled_handling(),
        vec![plane(1, -40.0, oag_physics::Surface::Wall, 0)],
    );
    setup.mode = Mode::SingleRace;
    setup.speedup_pads = enveloping_pad();
    setup.weapons = Some(one_mine_table());
    let mut race = Race::start(setup);

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
        // A Mine, dropped once - after the wall business above has already
        // had its first pass, so the impulse this leaves on `physics.shield`
        // is not mistaken for the wall's own. Its own held-weapon slot, not
        // the pickup path, for the same reason the shield and Autopilot above
        // are set directly.
        if tick == 200 {
            race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Mine);
            race.sim.world.ships[0]
                .pickup
                .begin_drop(oag_gameplay::projectile::mine::CLUSTER);
        }
        // Re-aimed at the wall every tick, so each one sees a fresh inbound
        // contact rather than the ship bouncing away after the first.
        let body = &mut race.sim.world.ships[0].physics.body;
        body.position = Vec3::new(20.0, -39.7, 0.0);
        body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
        saw_impact |= race.tick(&InputSnapshot::default()).wall.impact;
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    assert!(
        saw_impact,
        "the fixture never reached the wall - not what this test means to check"
    );

    for cue in Cue::ALL {
        assert!(
            raised.contains(&cue) || BY_LEVEL.contains(&cue),
            "{} is loaded and nothing raises it",
            cue.name()
        );
    }
    // Named individually as well, so a future `BY_LEVEL` that quietly grew
    // cannot make the loop above vacuous.
    for cue in [
        Cue::SpeedupPad,
        Cue::Collision,
        Cue::Absorb,
        Cue::ShieldActive,
        Cue::Disengaging,
        Cue::MineLaunch,
    ] {
        assert!(raised.contains(&cue), "{} was never raised", cue.name());
    }
}

#[test]
fn the_shield_announcer_fires_on_the_edge_and_not_on_the_level() {
    let mut race = grid_on_a_speed_pad();
    let mut announcements = 0;
    for tick in 0..180 {
        race.sim.world.ships[0].physics.shield_pickup_timer = if tick >= 30 { 1.0 } else { 0.0 };
        race.tick(&InputSnapshot::default());
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
    for _ in 0..1300 {
        race.tick(&InputSnapshot::default());
        raised.extend(race.drain_class_announcements());
    }
    assert!(
        race.sim.world.race.zone >= 2,
        "sanity: 1300 ticks should survive at least two zones, got {}",
        race.sim.world.race.zone
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
    for _ in 0..1300 {
        race.tick(&InputSnapshot::default());
        raised.extend(race.drain_class_announcements());
    }
    assert!(
        race.sim.world.race.zone >= 2,
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
/// `oag_gameplay::pickup::Held::begin_drop` fires the first charge on the very
/// tick it is called, so the count landed by the end of the run is exactly
/// [`oag_gameplay::projectile::mine::CLUSTER`] rather than one short.
#[test]
fn laying_a_mine_raises_its_launch_cue_once_per_charge() {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(one_mine_table());
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Mine);
    race.sim.world.ships[0]
        .pickup
        .begin_drop(oag_gameplay::projectile::mine::CLUSTER);

    let mut launches = 0;
    // Comfortably past the whole cluster: `DROP_INTERVAL` is a tenth of a
    // second, so `CLUSTER` charges take under a second at 60 Hz.
    for _ in 0..120 {
        race.tick(&InputSnapshot::default());
        launches += race
            .drain_cues()
            .iter()
            .filter(|e| e.cue == Cue::MineLaunch)
            .count();
    }
    assert_eq!(
        launches,
        usize::from(oag_gameplay::projectile::mine::CLUSTER),
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
        race.tick(&InputSnapshot::default());
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
        with.tick(&InputSnapshot::default());
        without.tick(&InputSnapshot::default());
        // One of the two has its queue drained every tick and the other never
        // does, so by the end they hold different queues entirely.
        with.drain_cues();
    }
    assert!(!without.pending_cues().is_empty(), "nothing was queued");
    assert!(with.pending_cues().is_empty());
    assert_eq!(with.sim.state_hash(), without.sim.state_hash());
}
