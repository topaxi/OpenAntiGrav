//! The armour guest as plain Rust: a test fixture for the `hit`, `give-shield`,
//! `hold-shield` and `clear-shield` pieces, never an example mod.

use super::*;
use crate::test::{CraftRecord, Race};

crate::blob! {
    #[derive(Debug, Default)]
    struct Armour {
        started: u8,
        broken: u8,
        points: f32,
    }
}

/// Gives slot 0 a Shield with 50 armour points; each `hit` costs its amount, the Shield is
/// held while armour stands and cleared by the call that sees it gone.
struct Plating;

impl Extension for Plating {
    type State = Armour;

    fn tick(obs: &Observation<'_>, state: &mut Armour, reply: &mut Reply<'_>) {
        let Some(craft) = obs.craft(Slot::new(0).unwrap()) else {
            return;
        };
        if state.started == 0 {
            state.started = 1;
            state.points = 50.0;
            reply.give_shield(&craft).unwrap();
            return;
        }
        if state.broken != 0 {
            return;
        }
        for event in obs.events() {
            if let Event::Hit { slot, amount, .. } = event
                && slot == craft.slot()
            {
                state.points -= amount;
            }
        }
        if state.points <= 0.0 {
            state.broken = 1;
            reply.clear_shield(&craft).unwrap();
        } else if craft.is_shielded() {
            reply.hold_shield(&craft).unwrap();
        }
    }
}

fn slot0() -> Slot {
    Slot::new(0).unwrap()
}

fn plated(shielded: bool) -> CraftRecord {
    CraftRecord {
        racing: true,
        shielded,
        shield: 100.0,
        shield_max: 100.0,
        ..CraftRecord::default()
    }
}

fn hit(amount: f32) -> Event {
    Event::Hit {
        slot: slot0(),
        amount,
        weapon: Weapon::Cannon,
    }
}

#[test]
fn the_armour_guest_gives_holds_and_breaks_a_shield_on_the_hits_it_is_sent() {
    let mut race = Race::<Plating>::new().with_craft(slot0(), plated(false));
    assert_eq!(race.tick(), [Command::GiveShield { slot: slot0() }]);

    *race.craft_mut(slot0()) = Some(plated(true));
    assert_eq!(race.tick(), [Command::HoldShield { slot: slot0() }]);

    race.event(hit(30.0));
    assert_eq!(
        race.tick(),
        [Command::HoldShield { slot: slot0() }],
        "20 points left, still held"
    );

    race.event(hit(30.0));
    assert_eq!(
        race.tick(),
        [Command::ClearShield { slot: slot0() }],
        "the hit past the armour clears the Shield and it is not held too"
    );

    *race.craft_mut(slot0()) = Some(plated(false));
    assert_eq!(race.tick(), [], "a broken armour stays quiet");
}

#[test]
fn a_second_shield_command_for_one_craft_in_a_tick_is_refused() {
    struct Twice;
    impl Extension for Twice {
        type State = Armour;
        fn tick(obs: &Observation<'_>, _: &mut Armour, reply: &mut Reply<'_>) {
            let craft = obs.craft(slot0()).unwrap();
            reply.hold_shield(&craft).unwrap();
            assert_eq!(reply.clear_shield(&craft), Err(Refused::Repeated));
            assert_eq!(reply.give_shield(&craft), Err(Refused::Repeated));
        }
    }
    let mut race = Race::<Twice>::new().with_craft(slot0(), plated(true));
    assert_eq!(race.tick(), [Command::HoldShield { slot: slot0() }]);
}
