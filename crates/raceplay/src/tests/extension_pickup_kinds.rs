//! Several mods' pickups in one race: each mod that declares a `[pickup]` has a kind of its
//! own (in module id order), gives, clears and rolls only that kind, sees it as `Ours` and
//! another's as a code of its own, is raised the `pickup-fire` and `pickup-absorbed` of its
//! kind alone, and draws its own `[[pickup.odds]]` at the pads.

use super::extensions::grid;
use super::extensions::on::{REPLY, STATE, module, prepared};
use super::*;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Button;
use oag_tables::pickup_look::OddsRow;
use oag_tables::weapons::{ModKind, PickupOdds, Weapon};

const CRAFTS: i32 = 1024 + oag_ext_abi::CRAFTS_AT as i32;
const EVENTS: i32 = 1024 + oag_ext_abi::EVENTS_AT as i32;

fn kind(index: usize) -> Option<Weapon> {
    Some(Weapon::Extension(ModKind::new(index).unwrap()))
}

fn pressed(button: Button) -> PlayerInputs {
    let mut buttons = oag_gameplay::input::Input::new();
    buttons.begin_frame(button.bit());
    PlayerInputs::single(oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    })
}

/// Gives `slot` its own pickup on its first call, if the craft holds nothing.
fn gives_to(id: &str, slot: i32) -> crate::extensions::Source {
    module(
        id,
        &format!(
            "(if (i32.eqz (i32.or (i32.load8_u (i32.const {craft})) (i32.load (i32.const {STATE}))))
               (then
                 (i32.store (i32.const {STATE}) (i32.const 1))
                 (i32.store (i32.const {REPLY}) (i32.const 1))
                 (i32.store8 (i32.const {cmd}) (i32.const 3))
                 (i32.store8 (i32.const {at}) (i32.const {slot}))))
             i32.const 0",
            craft = CRAFTS + slot * oag_ext_abi::CRAFT_LEN as i32,
            cmd = REPLY + 4,
            at = REPLY + 5,
        ),
    )
}

/// Records slot 1's pickup code, as the record shows it, in state byte 4 every call.
fn watches_slot_one(id: &str) -> crate::extensions::Source {
    module(
        id,
        &format!(
            "(i32.store8 (i32.const {at}) (i32.load8_u (i32.const {craft})))
             i32.const 0",
            at = STATE + 4,
            craft = CRAFTS + oag_ext_abi::CRAFT_LEN as i32,
        ),
    )
}

/// Counts the `pickup-fire` events (state word 0) and `pickup-absorbed` (word 1) it is sent.
fn counts_its_pickup_events(id: &str) -> crate::extensions::Source {
    module(
        id,
        &format!(
            "(local $i i32) (local $kind i32)
             (block $done (loop $next
               (br_if $done (i32.ge_u (local.get $i) (i32.load (i32.const {count}))))
               (local.set $kind (i32.load8_u (i32.add (i32.const {EVENTS}) (i32.mul (local.get $i) (i32.const {len})))))
               (if (i32.eq (local.get $kind) (i32.const 2))
                 (then (i32.store (i32.const {STATE}) (i32.add (i32.load (i32.const {STATE})) (i32.const 1)))))
               (if (i32.eq (local.get $kind) (i32.const 3))
                 (then (i32.store (i32.const {second}) (i32.add (i32.load (i32.const {second})) (i32.const 1)))))
               (local.set $i (i32.add (local.get $i) (i32.const 1)))
               (br $next)))
             i32.const 0",
            count = 1024 + oag_ext_abi::EVENT_COUNT_AT as i32,
            len = oag_ext_abi::EVENT_LEN,
            second = STATE + 4,
        ),
    )
}

fn word(blob: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(blob[at..at + 4].try_into().unwrap())
}

#[test]
fn two_mods_each_hand_out_a_pickup_of_their_own_kind_and_neither_faults() {
    let mut race = grid(prepared(&[gives_to("test.a", 0), gives_to("test.b", 1)]));
    race.tick(&PlayerInputs::none());
    assert_eq!(race.sim.world.ships[0].pickup.weapon, kind(0));
    assert_eq!(race.sim.world.ships[1].pickup.weapon, kind(1));
    assert!(
        race.extension_reports().iter().all(|r| r.faulted.is_none()),
        "{:?}",
        race.extension_reports()
    );
}

#[test]
fn a_module_sees_its_own_kind_as_ours_and_another_mods_as_a_code_of_its_own() {
    use oag_ext_abi::Pickup;
    let mut race = grid(prepared(&[
        watches_slot_one("test.a"),
        watches_slot_one("test.b"),
    ]));
    race.sim.world.ships[1].pickup.weapon = kind(1);
    race.tick(&PlayerInputs::none());
    let seen = |module| race.sim.extensions.blob_for_tests(module)[4];
    assert_eq!(seen(0), Pickup::Other(1).code(), "a sees b's kind");
    assert_eq!(seen(1), Pickup::Ours.code(), "b sees its own");
    assert_eq!(Pickup::from_code(seen(0)), Some(Pickup::Other(1)));
}

#[test]
fn a_fire_and_an_absorb_reach_the_module_of_the_kind_the_craft_held_and_no_other() {
    let mut race = grid(prepared(&[
        counts_its_pickup_events("test.a"),
        counts_its_pickup_events("test.b"),
    ]));
    race.sim.world.ships[0].pickup.weapon = kind(1);
    race.tick(&pressed(Button::Square));
    race.tick(&PlayerInputs::none());
    let (a, b) = (
        race.sim.extensions.blob_for_tests(0),
        race.sim.extensions.blob_for_tests(1),
    );
    assert_eq!((word(&b, 0), word(&b, 4)), (1, 0), "b was raised the fire");
    assert_eq!((word(&a, 0), word(&a, 4)), (0, 0), "a was raised nothing");
    race.tick(&pressed(Button::Circle));
    race.tick(&PlayerInputs::none());
    let (a, b) = (
        race.sim.extensions.blob_for_tests(0),
        race.sim.extensions.blob_for_tests(1),
    );
    assert_eq!((word(&b, 0), word(&b, 4)), (1, 1), "and the absorb");
    assert_eq!((word(&a, 0), word(&a, 4)), (0, 0), "a still nothing");
}

fn odds_module(id: &str, weight: f32) -> crate::extensions::Source {
    let mut source = module(id, "i32.const 0");
    source.pickup_odds = vec![OddsRow {
        title: None,
        class: Some("venom".to_string()),
        odds: PickupOdds {
            ai: weight,
            human: weight,
            front: 0.0,
            back: 0.0,
        },
    }];
    source
}

#[test]
fn the_pads_draw_every_mods_pickup_at_its_own_odds_in_kind_order() {
    let extensions = crate::extensions::Extensions::new(prepared(&[
        odds_module("test.a", 2.0),
        odds_module("test.b", 5.0),
    ]));
    let rows = extensions.pickup_odds("VENOM");
    let weights: Vec<_> = rows.iter().map(|(w, o)| (*w, o.ai)).collect();
    assert_eq!(weights, [(kind(0).unwrap(), 2.0), (kind(1).unwrap(), 5.0)]);
    // A module that faulted draws no more; the other's row stays.
    let mut race = grid(prepared(&[
        odds_module("test.a", 2.0),
        odds_module("test.b", 5.0),
    ]));
    crate::extensions::fault_for_tests(&mut race, 0);
    let rows = race.sim.extensions.pickup_odds("VENOM");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, kind(1).unwrap());
}

#[test]
fn a_pad_can_hand_out_either_mods_pickup() {
    let mut seen = [false; 2];
    for seed in 0..40_u64 {
        let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
        race.sim.world.rng = oag_core::Rng::new(seed + 1);
        race.sim.extensions = crate::extensions::Extensions::new(prepared(&[
            odds_module("test.a", 1.0e6),
            odds_module("test.b", 1.0e6),
        ]));
        race.tick(&PlayerInputs::none());
        for (i, seen) in seen.iter_mut().enumerate() {
            *seen |= race.ship_pickup() == kind(i);
        }
    }
    assert_eq!(seen, [true, true], "both kinds came off the pad");
}

#[test]
fn a_roll_turns_a_grant_into_the_rolling_modules_kind_alone() {
    // Kinds follow module ids: the idle module is kind 0, the roller (`test.roller`) kind 1.
    let mut race = grid(prepared(&[
        super::extensions::on::roller(),
        module("test.idle", "i32.const 0"),
    ]));
    race.tick(&PlayerInputs::none());
    let mut rolled = Vec::new();
    for _ in 0..2 {
        rolled.push(race.sim.extensions.rolls(0, Weapon::Turbo));
    }
    assert_eq!(
        rolled,
        [None, ModKind::new(1)],
        "every second Turbo is turned into the roller's kind (test.roller sorts after test.idle)"
    );
}

#[test]
fn the_held_kind_is_hashed_state() {
    let hash_with = |held| {
        let mut race = grid(prepared(&[
            module("test.a", "i32.const 0"),
            module("test.b", "i32.const 0"),
        ]));
        race.sim.world.ships[0].pickup.weapon = held;
        race.sim.state_hash()
    };
    assert_ne!(hash_with(kind(0)), hash_with(kind(1)));
    assert_ne!(hash_with(kind(0)), hash_with(None));
}

#[test]
fn a_race_holds_at_most_thirty_two_pickup_kinds_and_the_refusal_names_the_limit() {
    let last = ModKind::MAX - 1;
    assert_eq!(
        crate::extensions::pickup_kind_for_tests(last)
            .unwrap()
            .index(),
        last
    );
    let why = crate::extensions::pickup_kind_for_tests(ModKind::MAX).unwrap_err();
    assert!(why.contains("32"), "{why}");
}
