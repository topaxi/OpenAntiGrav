//! A mod turns the Shield into breakable armour with generic pieces: the `hit` event,
//! `give-shield`, `hold-shield` and `clear-shield`. The armour is a test fixture alone;
//! no example mod ships it. Hits are real Rocket impacts through `Race::tick`.

use std::sync::Arc;

use super::*;
use crate::extensions::{Prepared, Scope, Source, Track, TrackProps, prepare};
use oag_gameplay::PlayerInputs;
use oag_physics::damage::CraftState;

const IO: i32 = 1024;
const EVENTS: i32 = IO + oag_ext_abi::EVENTS_AT as i32;
const STATE: i32 = IO + oag_ext_abi::STATE_AT as i32;
const REPLY: i32 = IO + oag_ext_abi::REPLY_AT as i32;
const CRAFT_1: i32 = IO + oag_ext_abi::CRAFTS_AT as i32 + oag_ext_abi::CRAFT_LEN as i32;
const ROCKET_DAMAGE: f32 = 14.0;

/// The armour guest's blob, word offsets from `STATE`: phase (0 unstarted, 1 armed,
/// 2 broken), armour left, hits seen, last amount, tick the last hit arrived, tick the
/// clear was sent.
const PHASE: i32 = STATE;
const ARMOUR: i32 = STATE + 4;
const HITS: i32 = STATE + 8;
const LAST: i32 = STATE + 12;
const HIT_TICK: i32 = STATE + 16;
const CLEAR_TICK: i32 = STATE + 20;

fn source(id: &str, body: &str) -> Source {
    let wasm = wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const {word})
            (func (export "oag_io") (result i32) i32.const {IO})
            (func (export "tick") (result i32) {body}))"#,
        word = oag_ext_abi::ABI_WORD,
    ))
    .unwrap();
    Source {
        mod_id: id.to_string(),
        module: "test.wasm".to_string(),
        titles: Vec::new(),
        api: 1,
        memory_pages: 1,
        state_bytes: 32,
        scope: Scope::Rules,
        read: Arc::new(move || Ok(wasm.clone())),
        strings: Arc::new(|_| Default::default()),
        icons: Vec::new(),
        effects: Vec::new(),
        files: Arc::new(|_| Err("no files".into())),
        pickup: Some(crate::extension_pickup::a_look()),
        channels: crate::extensions::Declared::default(),
    }
}

/// One command for slot 1 as the whole reply.
fn send(kind: u8) -> String {
    format!(
        "(i32.store (i32.const {REPLY}) (i32.const 1))
         (i32.store8 (i32.const {c}) (i32.const {kind}))
         (i32.store8 (i32.const {s}) (i32.const 1))",
        c = REPLY + 4,
        s = REPLY + 5,
    )
}

/// The fixture armour: `armour` points. Its first call gives slot 1 a Shield; while the
/// Shield runs and the armour stands it holds it; each `hit` on slot 1 takes its amount
/// from the armour; the call that sees the armour gone clears the Shield and goes quiet.
fn armour(points: f32) -> Source {
    let body = format!(
        "(local $i i32) (local $broke i32)
         (block $done (loop $next
           (br_if $done (i32.ge_u (local.get $i) (i32.load (i32.const {count}))))
           (if (i32.and
                 (i32.eq (i32.load8_u (i32.add (i32.const {EVENTS}) (i32.mul (local.get $i) (i32.const {EVENT_LEN})))) (i32.const 9))
                 (i32.eq (i32.load8_u (i32.add (i32.const {EVENTS_SLOT}) (i32.mul (local.get $i) (i32.const {EVENT_LEN})))) (i32.const 1)))
             (then
               (i32.store (i32.const {HITS}) (i32.add (i32.load (i32.const {HITS})) (i32.const 1)))
               (f32.store (i32.const {LAST}) (f32.load (i32.add (i32.const {EVENTS_VALUE}) (i32.mul (local.get $i) (i32.const {EVENT_LEN})))))
               (i32.store (i32.const {HIT_TICK}) (i32.load (i32.const {TICK})))
               (if (i32.eq (i32.load (i32.const {PHASE})) (i32.const 1))
                 (then
                   (f32.store (i32.const {ARMOUR}) (f32.sub (f32.load (i32.const {ARMOUR})) (f32.load (i32.const {LAST}))))
                   (if (f32.le (f32.load (i32.const {ARMOUR})) (f32.const 0))
                     (then (local.set $broke (i32.const 1))))))))
           (local.set $i (i32.add (local.get $i) (i32.const 1)))
           (br $next)))
         (if (i32.eqz (i32.load (i32.const {PHASE})))
           (then
             (i32.store (i32.const {PHASE}) (i32.const 1))
             (f32.store (i32.const {ARMOUR}) (f32.const {points}))
             {give}))
         (if (local.get $broke)
           (then
             (i32.store (i32.const {PHASE}) (i32.const 2))
             (i32.store (i32.const {CLEAR_TICK}) (i32.load (i32.const {TICK})))
             {clear}))
         (if (i32.and
               (i32.eq (i32.load (i32.const {PHASE})) (i32.const 1))
               (i32.and (i32.eqz (local.get $broke))
                        (i32.ne (i32.and (i32.load8_u (i32.const {FLAGS})) (i32.const {SHIELDED})) (i32.const 0))))
           (then {hold}))
         i32.const 0",
        EVENT_LEN = oag_ext_abi::EVENT_LEN,
        count = IO + oag_ext_abi::EVENT_COUNT_AT as i32,
        EVENTS_SLOT = EVENTS + oag_ext_abi::EVENT_SLOT as i32,
        EVENTS_VALUE = EVENTS + oag_ext_abi::EVENT_VALUE as i32,
        TICK = IO + oag_ext_abi::TICK_AT as i32,
        FLAGS = CRAFT_1 + oag_ext_abi::CRAFT_FLAGS as i32,
        SHIELDED = oag_ext_abi::CRAFT_SHIELDED,
        give = send(24),
        hold = send(25),
        clear = send(26),
    );
    source("test.armour", &body)
}

/// Gives slot 1 a Shield on the first call, and holds it on every call up to `until`
/// (the call count in blob word 0), then goes quiet. With `hold` off it only gives.
fn shield_keeper(hold: bool, until: i32) -> Source {
    let body = format!(
        "(i32.store (i32.const {STATE}) (i32.add (i32.load (i32.const {STATE})) (i32.const 1)))
         (if (i32.eq (i32.load (i32.const {STATE})) (i32.const 1))
           (then {give})
           (else
             (if (i32.and (i32.const {hold})
                  (i32.and (i32.le_u (i32.load (i32.const {STATE})) (i32.const {until}))
                           (i32.ne (i32.and (i32.load8_u (i32.const {FLAGS})) (i32.const {SHIELDED})) (i32.const 0))))
               (then {keep}))))
         i32.const 0",
        FLAGS = CRAFT_1 + oag_ext_abi::CRAFT_FLAGS as i32,
        SHIELDED = oag_ext_abi::CRAFT_SHIELDED,
        hold = i32::from(hold),
        give = send(24),
        keep = send(25),
    );
    source("test.keeper", &body)
}

fn prepared(sources: &[Source]) -> Prepared {
    let mut report = Vec::new();
    let props = TrackProps::default();
    let track = Track {
        name: "test",
        props: &props,
        speedup_pads: &[],
        weapon_pads: &[],
        paths: &[],
        routes: &[],
    };
    let prepared = prepare(sources, "wipeout-pulse", None, track, None, &mut report);
    assert!(
        report.iter().all(|line| !line.contains("left out")),
        "{report:?}"
    );
    prepared
}

fn table() -> oag_tables::weapons::WeaponStats {
    oag_tables::weapons::parse(&format!(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Rocket"><Stats absorb="11" blastforce="12" blastradius="13"
               damage="{ROCKET_DAMAGE}" slowdown_time="15" venomspeed="600" flashspeed="700"
               rapierspeed="800" phantomspeed="900" launchSpeed="16" spread="0.2"/></Weapon>
             <Weapon type="Shield"><Stats absorb="7" time="1.25"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Rocket"><Stats ai="1" back="1" front="1" human="1"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#,
    ))
    .expect("the fixture table must parse")
}

/// Slot 0 fires Rockets at slot 1, which stands 15 units down its nose with a full pool
/// of 100.
fn duel(extensions: Prepared) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(table());
    setup.extensions = extensions;
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.sim.world.ships[1].handling.dimensions.shield = 100.0;
    race.sim.world.ships[1].physics.shield = 100.0;
    race
}

/// One tick of the duel. Slot 0 is pinned and fires a Rocket when `fire` is set; slot 1
/// is pinned too. Returns whether a Rocket reached a craft this tick.
fn step(race: &mut Race, buttons: &mut Buttons, fire: bool) -> bool {
    race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
    let forward = race.sim.world.ships[0].physics.body.forward();
    race.sim.world.ships[0].physics.body.linear_velocity = forward * 60.0;
    race.sim.world.ships[1].physics.body.position = forward * 15.0;
    race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
    if fire {
        race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Rocket);
    }
    let snapshot = buttons.tick(if fire { SQUARE } else { 0 });
    race.tick(&PlayerInputs::single(snapshot));
    race.drain_cues()
        .into_iter()
        .any(|e| e.cue == oag_gameplay::cue::Cue::RocketHitShip)
}

fn word(race: &Race, at: i32) -> [u8; 4] {
    let blob = race.sim.extensions.state(0).expect("a module runs");
    let at = (at - STATE) as usize;
    blob[at..at + 4].try_into().unwrap()
}

fn int(race: &Race, at: i32) -> u32 {
    u32::from_le_bytes(word(race, at))
}

fn float(race: &Race, at: i32) -> f32 {
    f32::from_le_bytes(word(race, at))
}

fn energy(race: &Race) -> f32 {
    race.sim.world.ships[1].physics.shield
}

fn shielded(race: &Race) -> bool {
    race.sim.world.ships[1].physics.shield_pickup_timer > 0.0
}

/// Fires a Rocket and runs until it reaches slot 1. Returns the world tick it struck in.
fn rocket(race: &mut Race, buttons: &mut Buttons) -> u64 {
    let mut fire = true;
    for _ in 0..120 {
        let at = race.sim.world.tick;
        if step(race, buttons, fire) {
            return at;
        }
        fire = false;
    }
    panic!("the Rocket never reached the craft");
}

#[test]
fn a_hit_on_a_held_shield_takes_nothing_and_reaches_the_module_one_call_later() {
    let mut race = duel(prepared(&[armour(100.0)]));
    let mut buttons = Buttons::new();
    step(&mut race, &mut buttons, false);
    assert!(
        shielded(&race),
        "the module's give-shield started the Shield"
    );
    assert_eq!(int(&race, HITS), 0);

    let struck = rocket(&mut race, &mut buttons);
    assert_eq!(energy(&race), 100.0, "the Shield absorbed the hit");
    assert_eq!(
        int(&race, HITS),
        0,
        "the hit is in the step, not yet in a call"
    );

    step(&mut race, &mut buttons, false);
    assert_eq!(int(&race, HITS), 1);
    assert_eq!(float(&race, LAST), ROCKET_DAMAGE);
    assert_eq!(
        u64::from(int(&race, HIT_TICK)),
        struck + 1,
        "a hit at step N reaches the call at N + 1"
    );
    assert_eq!(energy(&race), 100.0);
}

#[test]
fn the_hit_that_breaks_the_armour_costs_nothing_and_the_next_one_lands() {
    // 20 points: the first Rocket (14) leaves 6, the second (14) breaks it.
    let mut race = duel(prepared(&[armour(20.0)]));
    let mut buttons = Buttons::new();
    // Past the authored 75 ticks: only the module's hold keeps the Shield up.
    for _ in 0..100 {
        step(&mut race, &mut buttons, false);
    }
    assert!(shielded(&race), "held past the authored time");

    let first = rocket(&mut race, &mut buttons);
    step(&mut race, &mut buttons, false);
    assert_eq!(int(&race, HITS), 1);
    assert!((float(&race, ARMOUR) - 6.0).abs() < 1e-4);
    assert!(shielded(&race), "armour left, the Shield is held");
    assert_eq!(energy(&race), 100.0);
    assert!(first > 0);

    let second = rocket(&mut race, &mut buttons);
    assert_eq!(
        energy(&race),
        100.0,
        "the breaking hit is absorbed: no unprotected window"
    );
    assert!(shielded(&race), "the Shield breaks at the next call");

    step(&mut race, &mut buttons, false);
    assert_eq!(int(&race, HITS), 2);
    assert_eq!(u64::from(int(&race, CLEAR_TICK)), second + 1);
    assert!(
        !shielded(&race),
        "clear-shield ended the Shield before the step that followed the call"
    );
    assert_eq!(energy(&race), 100.0, "clearing costs nothing");

    rocket(&mut race, &mut buttons);
    assert_eq!(
        energy(&race),
        100.0 - ROCKET_DAMAGE,
        "the next hit lands on the energy: no double-protected window"
    );
    assert_eq!(
        race.sim.world.ships[1].physics.craft_state,
        CraftState::Racing
    );
}

#[test]
fn a_shield_the_module_holds_outlasts_its_timer_and_ends_once_it_stops() {
    let ticks_up = |sources: &[Source]| {
        let mut race = duel(prepared(sources));
        let mut buttons = Buttons::new();
        let mut up = 0;
        for _ in 0..400 {
            step(&mut race, &mut buttons, false);
            up += u32::from(shielded(&race));
            if up > 0 && !shielded(&race) {
                break;
            }
        }
        up
    };
    // The authored 1.25 s is 75 ticks at 60 Hz (`damage::advance_shield_pickup`). The run
    // stays under 4 s: the pinned craft would be stall-rescued, which resets its Shield.
    let plain = ticks_up(&[shield_keeper(false, 0)]);
    assert!(
        (74..=77).contains(&plain),
        "an unheld Shield ran {plain} ticks"
    );
    let held = ticks_up(&[shield_keeper(true, 150)]);
    assert!(
        (150..=153).contains(&held),
        "held for 150 calls it ran {held} ticks, within two of the last send"
    );
}

#[test]
fn a_race_without_a_module_hashes_the_same_run_twice_and_an_unshielded_craft_takes_the_rocket() {
    let mut race = duel(Prepared::default());
    let mut buttons = Buttons::new();
    let mut hashes = Vec::new();
    for tick in 0..90 {
        step(&mut race, &mut buttons, tick == 0);
        hashes.push(race.sim.state_hash());
    }
    let mut plain = duel(Prepared::default());
    let mut buttons = Buttons::new();
    for (tick, hash) in hashes.iter().enumerate() {
        step(&mut plain, &mut buttons, tick == 0);
        assert_eq!(plain.sim.state_hash(), *hash);
    }
    assert!(!race.is_modded());
    assert_eq!(
        energy(&race),
        100.0 - ROCKET_DAMAGE,
        "an unshielded craft takes the Rocket"
    );
}
