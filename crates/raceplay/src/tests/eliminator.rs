//! Eliminator's per-lap refill: a fifth of the maximum, clamped, with the
//! absorb feedback - `Ship_RefillLapShield` (`0x0883de30`), see
//! `docs/ghidra/functions/psp-pulse-usa/shield.md`.

use super::*;
use crate::eliminator::LAP_REFILL_FRACTION;
use oag_gameplay::PlayerInputs;
use oag_sound::sfx::Cue;

fn eliminator() -> Race {
    race_with_weapon_table(Mode::Eliminator, enveloping_pad(), 1.0, one_mine_table())
}

#[test]
fn a_completed_lap_gives_back_a_fifth_of_the_maximum_and_plays_absorb() {
    let mut race = eliminator();
    let maximum = race.sim.world.ships[0].handling.dimensions.shield;
    race.sim.world.ships[0].physics.shield = maximum * 0.3;
    race.drain_cues();

    race.eliminator_lap_health_refill(0);

    assert_eq!(
        race.sim.world.ships[0].physics.shield,
        maximum * 0.3 + maximum * LAP_REFILL_FRACTION
    );
    assert!(
        race.pending_cues().iter().any(|cue| cue.cue == Cue::Absorb),
        "the refill must play the absorb feedback"
    );
}

#[test]
fn the_refill_is_clamped_at_the_maximum() {
    let mut race = eliminator();
    let maximum = race.sim.world.ships[0].handling.dimensions.shield;
    race.sim.world.ships[0].physics.shield = maximum * 0.9;
    race.eliminator_lap_health_refill(0);
    assert_eq!(race.sim.world.ships[0].physics.shield, maximum);
}

#[test]
fn a_craft_that_is_not_racing_and_a_mode_that_is_not_eliminator_get_nothing() {
    let mut race = eliminator();
    let maximum = race.sim.world.ships[0].handling.dimensions.shield;
    race.sim.world.ships[0].physics.shield = maximum * 0.3;
    race.sim.world.ships[0].physics.craft_state = oag_physics::damage::CraftState::Destroyed;
    race.eliminator_lap_health_refill(0);
    assert_eq!(race.sim.world.ships[0].physics.shield, maximum * 0.3);

    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_mine_table());
    race.sim.world.ships[0].physics.shield = maximum * 0.3;
    race.eliminator_lap_health_refill(0);
    assert_eq!(race.sim.world.ships[0].physics.shield, maximum * 0.3);
}

/// A wrecked opponent in a single race stays down: state 5's dwell, then
/// state 6, a bare timer nothing revives (measured live on PPSSPP, 2026-10-02,
/// 13 s past the timer's expiry - the module doc of `race::eliminator`). The
/// opponent keeps its wreck, its empty pool and its place off the circuit for
/// as long as the race runs, and the player's race goes on.
#[test]
fn a_destroyed_opponent_in_a_single_race_never_comes_back() {
    use crate::eliminator::DESTROYED_DWELL;
    use oag_physics::CraftState;

    let mut race = race_with_a_grid();
    assert!(
        race.sim.world.ships[1].active,
        "the grid fixture fields opponents"
    );
    // Straight to the out-of-the-race state, the explosion already run.
    race.sim.world.ships[1].physics.craft_state = CraftState::Eliminated;
    race.sim.world.ships[1].physics.shield = 0.0;

    // Past state 5's 1.5 s, state 6's 0.8 s, the Eliminator's longest return
    // (2.5 s) and then the 13 s the live capture watched.
    let ticks = ((DESTROYED_DWELL + 0.8 + 2.5 + 13.0) / race.dt()).round() as usize;
    for tick in 0..ticks {
        race.tick(&PlayerInputs::none());
        assert_eq!(
            race.sim.world.ships[1].physics.craft_state,
            CraftState::Eliminated,
            "the opponent came back at tick {tick}"
        );
    }
    assert_eq!(race.sim.world.ships[1].physics.shield, 0.0);
    assert_eq!(race.sim.world.ships[1].standing.deaths, 0);
    assert!(
        !race.finished(),
        "an opponent's death must not end the player's race"
    );
}

/// And the player's own destruction is still the race's end, not a respawn.
#[test]
fn the_players_destruction_in_a_single_race_still_ends_it() {
    use oag_physics::CraftState;

    let mut race = race_with_a_grid();
    race.sim.world.ships[0].physics.craft_state = CraftState::Eliminated;
    for _ in 0..300 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(
        race.sim.world.ships[0].physics.craft_state,
        CraftState::Eliminated
    );
    assert!(race.finished());
}

/// Two craft in play, `victim` recorded as last struck by `killer`, its last
/// weapon hit `age` ticks ago.
fn struck(age: u64) -> Race {
    let mut race = eliminator();
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.tick = 5000;
    race.sim.last_damager[1] = Some(0);
    race.sim.last_weapon_hit[1] = 5000 + 1 - age;
    race
}

/// `Ship_Damage` credits the kill on a weapon's fatal blow.
#[test]
fn a_kill_is_credited_when_a_weapon_hit_finished_the_craft() {
    let mut race = struck(29);
    race.credit_kill(1);
    assert_eq!(race.sim.world.ships[0].standing.kills, 1);
}

/// A wall that finishes a craft off, long after the last weapon hit, credits
/// nobody - and, unlike the rule this replaced, the earlier hit is still what
/// decides it rather than a scrape having wiped the attacker.
#[test]
fn a_death_long_after_the_last_weapon_hit_credits_nobody() {
    let mut race = struck(1800);
    race.credit_kill(1);
    assert_eq!(race.sim.world.ships[0].standing.kills, 0);
}

/// Nobody is credited with their own death.
#[test]
fn a_craft_is_never_credited_with_its_own_death() {
    let mut race = struck(10);
    race.sim.last_damager[1] = Some(1);
    race.credit_kill(1);
    assert_eq!(race.sim.world.ships[1].standing.kills, 0);
}

/// Two craft in play, the victim's shield at `shield`, a beam or wave hit from
/// slot 0 recorded this tick.
fn pending_hit(shield: f32) -> Race {
    let mut race = eliminator();
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.tick = 5000;
    race.sim.world.ships[1].physics.shield = shield;
    race.record_pending_hit(1, 0);
    race
}

/// A LeachBeam or Quake blow that empties the shield is a weapon kill: the
/// shooter goes into the victim's `+0x13c` and `Ship_Damage` credits it.
#[test]
fn a_beam_or_quake_blow_that_empties_the_shield_is_credited() {
    let mut race = pending_hit(0.0);
    race.credit_kill(1);
    assert_eq!(race.sim.world.ships[0].standing.kills, 1);
}

/// A drain that leaves the shield standing is not the fatal blow: a wall that
/// finishes the craft a moment later credits nobody, as in the original.
#[test]
fn a_beam_hit_that_left_the_shield_standing_does_not_credit_a_wall_death() {
    let mut race = pending_hit(40.0);
    race.credit_kill(1);
    assert_eq!(race.sim.world.ships[0].standing.kills, 0);
}

/// The original writes the attacker in every mode but reads it only in mode 8,
/// so no other mode's state may move.
#[test]
fn a_pending_hit_writes_nothing_outside_eliminator() {
    let mut race = race_with_a_grid();
    race.sim.world.ships[1].physics.shield = 0.0;
    let before = race.sim.state_hash();
    race.record_pending_hit(1, 0);
    assert_eq!(race.sim.state_hash(), before);
}

/// The Eliminator's own table shape: a Mine on the pads, and a Shield authored
/// with a `time` but zero odds everywhere, as `WeaponStats_Elimination.xml`
/// ships it. Invented numbers, distinct from the absorb value.
fn mine_and_unpicked_shield_table() -> oag_tables::weapons::WeaponStats {
    oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Shield"><Stats absorb="10" time="1.25"/></Weapon>
             <Weapon type="Mine"><Stats absorb="17" blastforce="18" blastradius="19"
               damage="20" slowdown_time="0.5" timetodie="3"
               trigger_radius="2"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Mine"><Stats ai="1" back="1" front="1" human="1"/></Weapon>
               <Weapon type="Shield"><Stats ai="0" back="0" front="0" human="0"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#,
    )
    .expect("the fixture table must parse")
}

/// An absorb press in the Eliminator spends the held weapon on the Shield
/// pickup for the Shield's own `time`, and pays no energy:
/// `Ship_AbsorbHeldPickup`'s mode-8 path, see [`Race::eliminator_absorb`].
/// It used to be refused, and the weapon kept.
#[test]
fn an_eliminator_absorb_spends_the_weapon_on_the_shield_and_pays_no_energy() {
    let mut race = race_with_weapon_table(
        Mode::Eliminator,
        enveloping_pad(),
        1.0,
        mine_and_unpicked_shield_table(),
    );
    let mut buttons = Buttons::new();
    race.tick(&PlayerInputs::single(buttons.tick(0)));
    assert_eq!(race.ship_pickup(), Some(oag_tables::weapons::Weapon::Mine));
    race.sim.world.ships[0].physics.shield = 10.0;

    race.tick(&PlayerInputs::single(buttons.tick(CIRCLE)));

    assert_ne!(
        race.ship_pickup(),
        Some(oag_tables::weapons::Weapon::Mine),
        "the absorb must spend the held weapon"
    );
    assert!(
        race.ship().physics.shield <= 10.0,
        "an Eliminator absorb pays nothing into the pool: {}",
        race.ship().physics.shield
    );
    let timer = race.ship().physics.shield_pickup_timer;
    assert!(
        timer > 1.25 - 2.0 * race.dt() && timer <= 1.25,
        "the Shield must run for its own authored time, timer {timer}"
    );
}

/// Every other mode still pays the absorb and raises no Shield.
#[test]
fn a_single_race_absorb_still_pays_energy_and_raises_no_shield() {
    let mut race = race_with_weapon_table(
        Mode::SingleRace,
        enveloping_pad(),
        1.0,
        mine_and_unpicked_shield_table(),
    );
    let mut buttons = Buttons::new();
    race.tick(&PlayerInputs::single(buttons.tick(0)));
    race.sim.world.ships[0].physics.shield = 10.0;
    race.tick(&PlayerInputs::single(buttons.tick(CIRCLE)));
    assert!((race.ship().physics.shield - 27.0).abs() < 1e-4);
    assert_eq!(race.ship().physics.shield_pickup_timer, 0.0);
}

/// A Mine laid by one craft and tripped by another that its blast then finishes
/// is a kill for the layer: the direct hit (`Impact::struck`) records the owner
/// as the victim's last damager, the fatal-blow window finds it, and
/// `credit_kill` pays the Eliminator tally. A layer that is itself in the
/// radius is never its own victim.
#[test]
fn a_tripped_mine_that_finishes_the_craft_credits_the_layer() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::Eliminator;
    setup.weapons = Some(one_mine_table());
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let mut race = Race::start(setup);
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Mine);
    race.sim.world.ships[0].pickup.begin_drop(1);
    race.tick(&PlayerInputs::none());
    let mine = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .find(|p| p.kind == Some(oag_tables::weapons::Weapon::Mine))
        .expect("the drop laid a mine")
        .position;
    race.sim.world.ships[1].physics.body.position = mine;
    race.sim.world.ships[1].physics.shield = 1.0;
    let mut left_racing = false;
    for _ in 0..240 {
        race.tick(&PlayerInputs::none());
        left_racing |=
            race.sim.world.ships[1].physics.craft_state != oag_physics::CraftState::Racing;
    }
    assert!(left_racing, "the blast must finish a craft on one shield point");
    assert_eq!(race.sim.world.ships[0].standing.kills, 1);
    assert_eq!(race.sim.world.ships[1].standing.kills, 0);
}
